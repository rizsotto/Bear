// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::tables::TABLES;
use crate::yaml_types::{FlagTable, Kind};

/// Generate a static array of recognition pattern data from all YAML files.
///
/// Returns the generated Rust source as a string containing `RECOGNITION_PATTERNS`,
/// a static array of `(&str, &[&str], bool, bool, Option<&str>)` tuples:
/// (compiler_id, executables, cross_compilation, versioned, description).
///
/// Executables listed in `ignore_when.executables` are automatically added as
/// recognition entries with `(false, false)` so the recognizer can route them
/// to the right compiler id (where the interpreter will then ignore them).
///
/// `file_to_id` maps each `TableConfig::yaml_file` to the `compiler.id` the
/// file declared, so this can walk `TABLES` (which fixes output order) while
/// looking `raw_tables` (keyed by id) up correctly, without assuming a file's
/// stem equals its id.
pub fn generate_recognition_patterns(
    raw_tables: &HashMap<String, FlagTable>,
    file_to_id: &HashMap<&'static str, String>,
) -> Result<String> {
    let mut out = String::new();
    out.push_str("// Generated from compilers/*.yaml -- DO NOT EDIT\n");
    // The 5-tuple row shape trips clippy::type_complexity; it is plain
    // generated data, not an API to simplify with a type alias.
    out.push_str("#[allow(clippy::type_complexity)]\n");
    out.push_str("pub static RECOGNITION_PATTERNS: &[(&str, &[&str], bool, bool, Option<&str>)] = &[\n");

    // Collect entries in a deterministic order (by TABLES order)
    for config in TABLES {
        let id = file_to_id
            .get(config.yaml_file)
            .with_context(|| format!("no table loaded for {}", config.yaml_file))?;
        let table = &raw_tables[id.as_str()];

        // No wrapper recognize rows exist yet; skip anything that is not
        // `type: compiler` (wrapper-kind recognition is a later commit).
        if !matches!(table.type_, Kind::Compiler) {
            continue;
        }
        let id_str = table.compiler.id.as_str();

        // Emit explicit recognize entries
        if let Some(ref recognize_entries) = table.recognize {
            for entry in recognize_entries {
                entry.validate().with_context(|| format!("recognize entry in {}", config.yaml_file))?;

                let names_str: Vec<String> = entry.executables.iter().map(|n| format!("\"{}\"", n)).collect();
                out.push_str(&format!(
                    "    (\"{}\", &[{}], {}, {}, Some(\"{}\")),\n",
                    id_str,
                    names_str.join(", "),
                    entry.cross_compilation,
                    entry.versioned,
                    escape(&entry.description),
                ));
            }
        }

        // Auto-add own ignore_when.executables as recognition entries (no cross-compilation, no version).
        // Only use the table's own list, not inherited - inherited executables are already
        // recognized under the base compiler id.
        let own_ignore = table.ignore_when.as_ref();
        if own_ignore.is_some_and(|iw| !iw.executables.is_empty()) {
            let exes = &own_ignore.unwrap().executables;
            let names_str: Vec<String> = exes.iter().map(|n| format!("\"{}\"", n)).collect();
            out.push_str(&format!(
                "    (\"{}\", &[{}], false, false, None),\n",
                id_str,
                names_str.join(", "),
            ));
        }
    }

    out.push_str("];\n");

    Ok(out)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
