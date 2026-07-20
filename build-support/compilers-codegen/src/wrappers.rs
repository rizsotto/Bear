// SPDX-License-Identifier: GPL-3.0-or-later

use crate::yaml_types::WrapperTable;

/// Generate `WRAPPER_NAMES` (every wrapper's recognized executables,
/// flattened and sorted for deterministic output) and `WRAPPER_OPTIONS`
/// (per-executable argv options to skip before the real compiler, emitted
/// only for wrappers that declare any -- today, distcc).
///
/// `wrapper_tables` is `(yaml_file, WrapperTable)` pairs, sorted by file
/// name by the caller; wrapper tables carry no identity block, so "wrapper
/// name" here means an executable listed under `recognize[].executables`.
pub fn generate_wrappers(wrapper_tables: &[(String, WrapperTable)]) -> String {
    let mut names: Vec<&str> = wrapper_tables
        .iter()
        .flat_map(|(_, table)| table.recognize.iter())
        .flat_map(|entry| entry.executables.iter())
        .map(|s| s.as_str())
        .collect();
    names.sort();

    let mut out = String::new();
    out.push_str("// Generated from compilers/*.yaml -- DO NOT EDIT\n");
    out.push_str(&format!(
        "pub(super) static WRAPPER_NAMES: &[&str] = &[{}];\n",
        names.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(", ")
    ));

    out.push_str("// (name, &[(option, arity)]) -- arity = following argv tokens consumed\n");
    out.push_str("pub(super) static WRAPPER_OPTIONS: &[(&str, &[(&str, u32)])] = &[\n");
    for (_, table) in wrapper_tables {
        if table.options.is_empty() {
            continue;
        }
        let opts: Vec<String> = table
            .options
            .iter()
            .map(|opt| format!("(\"{}\", {})", opt.match_.pattern, opt.match_.count.unwrap_or(0)))
            .collect();
        for entry in &table.recognize {
            for exe in &entry.executables {
                out.push_str(&format!("    (\"{}\", &[{}]),\n", exe, opts.join(", ")));
            }
        }
    }
    out.push_str("];\n");

    out
}
