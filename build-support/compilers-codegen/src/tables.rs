// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler-table descriptors, derived from the YAML file stem.
//!
//! Compiler files are discovered by scanning `compilers/*.yaml` (see
//! `lib.rs::load_and_index`), not listed in a hand-maintained array. Every
//! generated static name and output file name derives from the file stem by
//! a fixed convention, so a new compiler is a YAML-only addition.

/// The generated static names and output file for one compiler table, all
/// derived from its YAML file stem (`gcc` -> `GCC_FLAGS`, `flags_gcc.rs`,
/// ...). `flag_based.rs` `include!`s the output file and references the
/// static names, so this convention is load-bearing: it must match the
/// names that module expects.
pub struct TableNames {
    pub yaml_file: String,
    pub static_name: String,
    pub ignore_executables_name: String,
    pub ignore_flags_name: String,
    pub slash_prefix_name: String,
    pub env_rules_name: String,
    pub output_file: String,
}

impl TableNames {
    /// Derive every generated name from the file stem.
    pub fn from_stem(stem: &str) -> Self {
        let upper = stem.to_uppercase();
        TableNames {
            yaml_file: format!("{stem}.yaml"),
            static_name: format!("{upper}_FLAGS"),
            ignore_executables_name: format!("{upper}_IGNORE_EXECUTABLES"),
            ignore_flags_name: format!("{upper}_IGNORE_FLAGS"),
            slash_prefix_name: format!("{upper}_SLASH_PREFIX"),
            env_rules_name: format!("{upper}_ENV_RULES"),
            output_file: format!("flags_{stem}.rs"),
        }
    }
}

/// One discovered compiler-kind YAML file: its stem (packaging, drives the
/// generated names), its declared `compiler.id` (identity, drives recognition
/// and lookup), and its `extends` depth (0 for a root, one more than its
/// parent). Discovery sorts by `(depth descending, stem)`, so a specialization
/// (higher depth) is always recognized before the family it extends -- the
/// order of the generated recognition rows.
pub struct CompilerFile {
    pub stem: String,
    pub id: String,
    pub depth: usize,
}

impl CompilerFile {
    /// The generated names for this file.
    pub fn names(&self) -> TableNames {
        TableNames::from_stem(&self.stem)
    }

    /// The source YAML file name (`<stem>.yaml`).
    pub fn yaml_file(&self) -> String {
        format!("{}.yaml", self.stem)
    }
}
