// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::path::Path;

use crate::tables::TABLES;
use crate::yaml_types::FlagTable;

/// Generate a static array of recognition pattern data from all YAML files.
///
/// Produces `recognition.rs` containing `RECOGNITION_PATTERNS`, a static array of
/// `(&str, &[&str], bool, bool)` tuples: (compiler_type, executables, cross_compilation, versioned).
///
/// Executables listed in `ignore_when.executables` are automatically added as
/// recognition entries with `(false, false)` so the recognizer can route them
/// to the right compiler type (where the interpreter will then ignore them).
pub fn generate_recognition_patterns(raw_tables: &HashMap<String, FlagTable>, out_dir: &Path) {
    let mut out = String::new();
    out.push_str("// Generated from interpreters/*.yaml -- DO NOT EDIT\n");
    out.push_str("pub static RECOGNITION_PATTERNS: &[(&str, &[&str], bool, bool)] = &[\n");

    // Collect entries in a deterministic order (by TABLES order)
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &raw_tables[key];

        let Some(ref type_name) = table.type_ else {
            continue;
        };

        // Emit explicit recognize entries
        if let Some(ref recognize_entries) = table.recognize {
            for entry in recognize_entries {
                let names_str: Vec<String> = entry.executables.iter().map(|n| format!("\"{}\"", n)).collect();
                out.push_str(&format!(
                    "    (\"{}\", &[{}], {}, {}),\n",
                    type_name,
                    names_str.join(", "),
                    entry.cross_compilation,
                    entry.versioned,
                ));
            }
        }

        // Auto-add own ignore_when.executables as recognition entries (no cross-compilation, no version).
        // Only use the table's own list, not inherited - inherited executables are already
        // recognized under the base compiler type.
        let own_ignore = table.ignore_when.as_ref();
        if own_ignore.is_some_and(|iw| !iw.executables.is_empty()) {
            let exes = &own_ignore.unwrap().executables;
            let names_str: Vec<String> = exes.iter().map(|n| format!("\"{}\"", n)).collect();
            out.push_str(&format!("    (\"{}\", &[{}], false, false),\n", type_name, names_str.join(", "),));
        }
    }

    out.push_str("];\n");

    let out_path = out_dir.join("recognition.rs");
    std::fs::write(&out_path, out)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
}
