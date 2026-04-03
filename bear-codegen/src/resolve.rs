// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use crate::yaml_types::{EnvEntry, FlagEntry, FlagTable, IgnoreWhen};

/// Resolve flags for a compiler, with transitive inheritance and dedup.
///
/// Own flags come first, then base flags (recursively). Duplicate patterns
/// (same pattern + count) are removed, keeping the child's version. If a
/// pattern appears with conflicting results, returns an error.
pub fn resolve_flags(key: &str, raw_tables: &HashMap<String, FlagTable>) -> Result<Vec<FlagEntry>> {
    let mut visited = HashSet::new();
    let all = resolve_flags_recursive(key, raw_tables, &mut visited);
    dedup_flags(all)
}

fn resolve_flags_recursive(
    key: &str,
    raw_tables: &HashMap<String, FlagTable>,
    visited: &mut HashSet<String>,
) -> Vec<FlagEntry> {
    if !visited.insert(key.to_string()) {
        return vec![];
    }
    let table = &raw_tables[key];
    let mut entries = table.flags.clone();

    if let Some(ref base_name) = table.extends
        && raw_tables.contains_key(base_name.as_str())
    {
        entries.extend(resolve_flags_recursive(base_name, raw_tables, visited));
    }
    entries
}

/// Deduplicate flags by (pattern, count). Entries appear in priority order
/// (own first), so the first occurrence wins. If a later entry has the
/// same (pattern, count) but a different result, that is a conflict.
fn dedup_flags(flags: Vec<FlagEntry>) -> Result<Vec<FlagEntry>> {
    let mut seen: HashMap<(String, Option<u32>), String> = HashMap::new();
    let mut result = Vec::new();

    for entry in flags {
        let key = (entry.match_.pattern.clone(), entry.match_.count);
        match seen.get(&key) {
            None => {
                seen.insert(key, entry.result.clone());
                result.push(entry);
            }
            Some(prev_result) => {
                if *prev_result != entry.result {
                    bail!(
                        "flag '{}' has conflicting results: '{}' vs '{}'",
                        entry.match_.pattern,
                        prev_result,
                        entry.result
                    );
                }
            }
        }
    }

    Ok(result)
}

/// Resolve `ignore_when` for a compiler, with transitive inheritance.
///
/// Walks the extends chain. At each level, a non-empty own list takes
/// precedence over the inherited list (per field independently).
pub fn resolve_ignore_when(key: &str, raw_tables: &HashMap<String, FlagTable>) -> IgnoreWhen {
    let mut visited = HashSet::new();
    resolve_ignore_when_recursive(key, raw_tables, &mut visited)
}

fn resolve_ignore_when_recursive(
    key: &str,
    raw_tables: &HashMap<String, FlagTable>,
    visited: &mut HashSet<String>,
) -> IgnoreWhen {
    if !visited.insert(key.to_string()) {
        return IgnoreWhen::default();
    }
    let table = &raw_tables[key];
    let own = table.ignore_when.clone().unwrap_or_default();

    if let Some(ref base_name) = table.extends
        && raw_tables.contains_key(base_name.as_str())
    {
        let base = resolve_ignore_when_recursive(base_name, raw_tables, visited);
        return IgnoreWhen {
            executables: if own.executables.is_empty() { base.executables } else { own.executables },
            flags: if own.flags.is_empty() { base.flags } else { own.flags },
        };
    }
    own
}

/// Resolve `slash_prefix` for a compiler, with transitive inheritance.
///
/// Returns the first explicit value found walking up the extends chain,
/// or `false` if no table in the chain sets it.
pub fn resolve_slash_prefix(key: &str, raw_tables: &HashMap<String, FlagTable>) -> bool {
    let mut visited = HashSet::new();
    resolve_slash_prefix_recursive(key, raw_tables, &mut visited)
}

fn resolve_slash_prefix_recursive(
    key: &str,
    raw_tables: &HashMap<String, FlagTable>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(key.to_string()) {
        return false;
    }
    let table = &raw_tables[key];
    if let Some(value) = table.slash_prefix {
        return value;
    }
    if let Some(ref base_name) = table.extends
        && raw_tables.contains_key(base_name.as_str())
    {
        return resolve_slash_prefix_recursive(base_name, raw_tables, visited);
    }
    false
}

/// Resolve environment entries for a compiler, with transitive inheritance.
///
/// Walks the `extends` chain recursively, collecting environment entries.
/// Own entries override inherited ones matched by variable name.
pub fn resolve_environment(key: &str, raw_tables: &HashMap<String, FlagTable>) -> Vec<EnvEntry> {
    let mut visited = HashSet::new();
    resolve_environment_recursive(key, raw_tables, &mut visited)
}

fn resolve_environment_recursive(
    key: &str,
    raw_tables: &HashMap<String, FlagTable>,
    visited: &mut HashSet<String>,
) -> Vec<EnvEntry> {
    if !visited.insert(key.to_string()) {
        return vec![];
    }
    let table = &raw_tables[key];
    let mut entries = table.environment.clone().unwrap_or_default();

    if let Some(ref base_name) = table.extends
        && raw_tables.contains_key(base_name.as_str())
    {
        let base_entries = resolve_environment_recursive(base_name, raw_tables, visited);
        let own_vars: HashSet<String> = entries.iter().map(|e| e.variable.clone()).collect();
        for entry in base_entries {
            if !own_vars.contains(&entry.variable) {
                entries.push(entry);
            }
        }
    }
    entries
}
