// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use crate::tables::CompilerFile;
use crate::yaml_types::FlagTable;

/// Generate `families.rs`: the registry the semantic layer loops over to
/// register one flag-based interpreter per compiler family.
///
/// It `include!`s each per-family flag-table file (bringing the generated
/// statics into scope) and then emits `FAMILIES`, one `FamilyDef` per
/// discovered compiler, in discovery order. Each row carries references to
/// that family's generated statics plus its `source_mode` and
/// `response_file_syntax` (taken from the family's own table, not resolved
/// through `extends`). `FamilyDef` itself is hand-written in `flag_based.rs`,
/// which `include!`s this file.
pub fn generate_families(compiler_files: &[CompilerFile], raw_tables: &HashMap<String, FlagTable>) -> String {
    let mut out = String::new();
    out.push_str("// Generated from compilers/*.yaml -- DO NOT EDIT\n");

    // Bring every family's generated flag-table statics into scope.
    for compiler_file in compiler_files {
        out.push_str(&format!(
            "include!(concat!(env!(\"OUT_DIR\"), \"/{}\"));\n",
            compiler_file.names().output_file
        ));
    }

    out.push_str("pub(super) static FAMILIES: &[FamilyDef] = &[\n");
    for compiler_file in compiler_files {
        let table = &raw_tables[&compiler_file.id];
        let names = compiler_file.names();
        out.push_str(&format!(
            "    FamilyDef {{ id: \"{}\", flags: &{}, ignore_executables: &{}, ignore_flags: &{}, slash_prefix: {}, env_rules: &{}, source_mode: {}, response_file_syntax: {} }},\n",
            compiler_file.id,
            names.static_name,
            names.ignore_executables_name,
            names.ignore_flags_name,
            names.slash_prefix_name,
            names.env_rules_name,
            table.source_mode.to_rust(),
            table.response_file_syntax.to_rust(),
        ));
    }
    out.push_str("];\n");
    out
}
