// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{HashMap, HashSet};

use crate::yaml_types::{EnvEntry, FlagTable, IgnoreWhen};

/// Resolve `ignore_when` for a table, inheriting from base if extending.
pub fn resolve_ignore_when(table: &FlagTable, raw_tables: &HashMap<String, FlagTable>) -> IgnoreWhen {
    let own = table.ignore_when.clone().unwrap_or_default();
    if let Some(ref base_name) = table.extends
        && let Some(base_table) = raw_tables.get(base_name.as_str())
    {
        let base = base_table.ignore_when.clone().unwrap_or_default();
        // Own values take precedence; only inherit if own list is empty
        return IgnoreWhen {
            executables: if own.executables.is_empty() { base.executables } else { own.executables },
            flags: if own.flags.is_empty() { base.flags } else { own.flags },
        };
    }
    own
}

/// Resolve `slash_prefix` for a table, inheriting from base if extending.
pub fn resolve_slash_prefix(table: &FlagTable, raw_tables: &HashMap<String, FlagTable>) -> bool {
    if let Some(value) = table.slash_prefix {
        return value;
    }
    if let Some(ref base_name) = table.extends
        && let Some(base_table) = raw_tables.get(base_name.as_str())
    {
        return base_table.slash_prefix.unwrap_or(false);
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
