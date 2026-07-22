// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{BTreeSet, HashMap};

use crate::resolve::resolve_environment;
use crate::yaml_types::FlagTable;

/// Generate a static array of all compiler environment variable names.
///
/// Returns the generated Rust source as a string. The output is sorted (a
/// `BTreeSet`), so it does not depend on the order the tables are visited.
pub fn generate_env_keys(raw_tables: &HashMap<String, FlagTable>) -> String {
    let mut all_vars = BTreeSet::new();

    for id in raw_tables.keys() {
        let entries = resolve_environment(id, raw_tables);
        for entry in &entries {
            if entry.effect != "none" {
                all_vars.insert(entry.variable.clone());
            }
        }
    }

    let mut out = String::new();
    out.push_str("// Generated from compilers/*.yaml -- DO NOT EDIT\n");
    out.push_str(&format!("static COMPILER_ENV_KEYS: [&str; {}] = [\n", all_vars.len()));
    for var in &all_vars {
        out.push_str(&format!("    \"{}\",\n", var));
    }
    out.push_str("];\n");

    out
}
