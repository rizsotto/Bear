// SPDX-License-Identifier: GPL-3.0-or-later

pub mod codegen;
pub mod env_keys;
pub mod recognition;
pub mod resolve;
pub mod tables;
pub mod yaml_types;

use std::collections::HashMap;
use std::path::Path;

use codegen::{env_mapping_to_rust, flag_name_len, result_to_rust, validate_env_entry};
use env_keys::generate_env_keys;
use recognition::generate_recognition_patterns;
use resolve::{resolve_environment, resolve_ignore_when, resolve_slash_prefix};
use tables::{TABLES, TableConfig};
use yaml_types::{FlagEntry, FlagTable, IgnoreWhen};

/// Generate all flag tables, recognition patterns, and env keys.
///
/// - `flags_dir`: path to the directory containing *.yaml files
/// - `out_dir`: path to write generated .rs files
///
/// Prints `cargo:rerun-if-changed` lines to stdout (for build.rs integration).
pub fn generate(flags_dir: &Path, out_dir: &Path) {
    // Read all YAML files first so we can resolve `extends`
    let mut raw_tables: HashMap<String, FlagTable> = HashMap::new();
    for config in TABLES {
        let yaml_path = flags_dir.join(config.yaml_file);
        println!("cargo:rerun-if-changed={}", yaml_path.display());

        let content = std::fs::read_to_string(&yaml_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", yaml_path.display(), e));
        let table: FlagTable = serde_saphyr::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", yaml_path.display(), e));

        // Key by yaml filename stem (e.g., "gcc", "clang")
        let key = config.yaml_file.strip_suffix(".yaml").unwrap().to_string();
        raw_tables.insert(key, table);
    }

    // Generate recognition patterns from all tables
    let recognition = generate_recognition_patterns(&raw_tables);
    let out_path = out_dir.join("recognition.rs");
    std::fs::write(&out_path, recognition)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));

    // Generate each table
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &raw_tables[key];

        // Collect own flags + base flags (if extending)
        let mut entries: Vec<FlagEntry> = table.flags.clone();
        if let Some(ref base_name) = table.extends {
            let base = raw_tables
                .get(base_name.as_str())
                .unwrap_or_else(|| panic!("{} extends unknown table '{}'", config.yaml_file, base_name));
            entries.extend(base.flags.iter().cloned());
        }

        // Sort by flag length descending (stable sort preserves own-before-base order)
        entries.sort_by(|a, b| {
            let a_len = flag_name_len(&a.match_);
            let b_len = flag_name_len(&b.match_);
            b_len.cmp(&a_len)
        });

        // Resolve ignore_when and slash_prefix (own + base)
        let ignore_when = resolve_ignore_when(table, &raw_tables);
        let slash_prefix = resolve_slash_prefix(table, &raw_tables);

        // Resolve environment entries (transitive inheritance)
        let env_entries = resolve_environment(key, &raw_tables);

        // Generate Rust source
        let mut rust_code = generate_static_array(config, &entries);
        rust_code.push_str(&generate_ignore_arrays(config, &ignore_when));
        rust_code.push_str(&format!("static {}: bool = {};\n", config.slash_prefix_name, slash_prefix));
        rust_code.push_str(&generate_env_array(config, &env_entries));
        let out_path = out_dir.join(config.output_file);
        std::fs::write(&out_path, rust_code)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
    }

    // Generate a combined list of all compiler environment variable names
    let env_keys = generate_env_keys(&raw_tables);
    let out_path = out_dir.join("env_keys.rs");
    std::fs::write(&out_path, env_keys)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
}

/// Generate static arrays for ignore_when executables and flags.
pub fn generate_ignore_arrays(config: &TableConfig, ignore_when: &IgnoreWhen) -> String {
    let mut out = String::new();

    // Generate ignore executables array
    out.push_str(&format!(
        "static {}: [&str; {}] = [",
        config.ignore_executables_name,
        ignore_when.executables.len()
    ));
    for exe in &ignore_when.executables {
        out.push_str(&format!("\"{}\", ", exe));
    }
    out.push_str("];\n");

    // Generate ignore flags array
    out.push_str(&format!("static {}: [&str; {}] = [", config.ignore_flags_name, ignore_when.flags.len()));
    for flag in &ignore_when.flags {
        out.push_str(&format!("\"{}\", ", flag));
    }
    out.push_str("];\n");

    out
}

/// Generate a static array of EnvRule for a compiler.
pub fn generate_env_array(config: &TableConfig, entries: &[yaml_types::EnvEntry]) -> String {
    // Filter out effect: none entries (documentary only)
    let active: Vec<&yaml_types::EnvEntry> = entries.iter().filter(|e| e.effect != "none").collect();

    for entry in &active {
        validate_env_entry(entry, config.yaml_file).unwrap_or_else(|e| panic!("{}", e));
    }

    let mut out = String::new();
    out.push_str(&format!("static {}: [EnvRule; {}] = [\n", config.env_rules_name, active.len()));

    for entry in &active {
        let mapping_rust = env_mapping_to_rust(&entry.mapping);
        let effect_rust = result_to_rust(&entry.effect);
        out.push_str(&format!(
            "    EnvRule::new(\"{}\", {}, {}),\n",
            entry.variable, mapping_rust, effect_rust
        ));
    }

    out.push_str("];\n");
    out
}

/// Generate a Rust source file containing a static array of FlagRule.
pub fn generate_static_array(config: &TableConfig, entries: &[FlagEntry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("// Generated from interpreters/{} -- DO NOT EDIT\n", config.yaml_file));
    out.push_str(&format!("static {}: [FlagRule; {}] = [\n", config.static_name, entries.len()));

    for entry in entries {
        let pattern_rust = codegen::pattern_to_rust(&entry.match_.pattern, entry.match_.count);
        let result_rust = result_to_rust(&entry.result);
        out.push_str(&format!("    FlagRule::new({}, {}),\n", pattern_rust, result_rust));
    }

    out.push_str("];\n");
    out
}

/// Path to the YAML flag definitions in the workspace.
pub fn flags_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("bear/interpreters")
}

/// Load all YAML flag tables from the workspace interpreters directory.
pub fn load_tables() -> HashMap<String, FlagTable> {
    let flags_dir = flags_dir();
    let mut raw_tables = HashMap::new();
    for config in TABLES {
        let yaml_path = flags_dir.join(config.yaml_file);
        let content = std::fs::read_to_string(&yaml_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", yaml_path.display(), e));
        let table: FlagTable = serde_saphyr::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", yaml_path.display(), e));
        let key = config.yaml_file.strip_suffix(".yaml").unwrap().to_string();
        raw_tables.insert(key, table);
    }
    raw_tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::pattern_to_rust;
    use crate::yaml_types::{EnvEntry, EnvMappingYaml, FlagMatch};

    // -- pattern_to_rust tests --

    #[test]
    fn pattern_exactly_with_glued_or_sep() {
        assert_eq!(pattern_to_rust("-I{ }*", None), "FlagPattern::ExactlyWithGluedOrSep(\"-I\")");
    }

    #[test]
    fn pattern_exactly_with_eq_or_sep() {
        assert_eq!(pattern_to_rust("-std{=}*", None), "FlagPattern::ExactlyWithEqOrSep(\"-std\")");
    }

    #[test]
    fn pattern_exactly_with_colon_or_sep() {
        assert_eq!(pattern_to_rust("-MF{:}*", None), "FlagPattern::ExactlyWithColonOrSep(\"-MF\")");
    }

    #[test]
    fn pattern_exactly_with_colon() {
        assert_eq!(pattern_to_rust("-Xclang:*", None), "FlagPattern::ExactlyWithColon(\"-Xclang\")");
    }

    #[test]
    fn pattern_exactly_with_eq() {
        assert_eq!(pattern_to_rust("-std=*", None), "FlagPattern::ExactlyWithEq(\"-std\")");
    }

    #[test]
    fn pattern_prefix_with_eq_and_count() {
        assert_eq!(pattern_to_rust("-std=*", Some(2)), "FlagPattern::Prefix(\"-std=\", 2)");
    }

    #[test]
    fn pattern_prefix() {
        assert_eq!(pattern_to_rust("-Wall*", None), "FlagPattern::Prefix(\"-Wall\", 0)");
    }

    #[test]
    fn pattern_exactly() {
        assert_eq!(pattern_to_rust("-c", None), "FlagPattern::Exactly(\"-c\", 0)");
    }

    #[test]
    fn pattern_exactly_with_count() {
        assert_eq!(pattern_to_rust("-c", Some(1)), "FlagPattern::Exactly(\"-c\", 1)");
    }

    // -- result_to_rust tests --

    #[test]
    fn result_output() {
        assert_eq!(result_to_rust("output"), "ArgumentKind::Output");
    }

    #[test]
    fn result_configures_preprocessing() {
        assert_eq!(
            result_to_rust("configures_preprocessing"),
            "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))"
        );
    }

    #[test]
    fn result_configures_compiling() {
        assert_eq!(
            result_to_rust("configures_compiling"),
            "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))"
        );
    }

    #[test]
    fn result_configures_assembling() {
        assert_eq!(
            result_to_rust("configures_assembling"),
            "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Assembling))"
        );
    }

    #[test]
    fn result_configures_linking() {
        assert_eq!(
            result_to_rust("configures_linking"),
            "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))"
        );
    }

    #[test]
    fn result_stops_at_preprocessing() {
        assert_eq!(
            result_to_rust("stops_at_preprocessing"),
            "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing))"
        );
    }

    #[test]
    fn result_stops_at_compiling() {
        assert_eq!(
            result_to_rust("stops_at_compiling"),
            "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))"
        );
    }

    #[test]
    fn result_stops_at_assembling() {
        assert_eq!(
            result_to_rust("stops_at_assembling"),
            "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Assembling))"
        );
    }

    #[test]
    fn result_info_and_exit() {
        assert_eq!(result_to_rust("info_and_exit"), "ArgumentKind::Other(PassEffect::InfoAndExit)");
    }

    #[test]
    fn result_driver_option() {
        assert_eq!(result_to_rust("driver_option"), "ArgumentKind::Other(PassEffect::DriverOption)");
    }

    #[test]
    fn result_pass_through() {
        assert_eq!(result_to_rust("pass_through"), "ArgumentKind::Other(PassEffect::PassThrough)");
    }

    #[test]
    fn result_none() {
        assert_eq!(result_to_rust("none"), "ArgumentKind::Other(PassEffect::None)");
    }

    #[test]
    #[should_panic(expected = "Unknown result value")]
    fn result_unknown_panics() {
        result_to_rust("bogus");
    }

    // -- flag_name_len tests --

    #[test]
    fn flag_name_len_exact_with_glued() {
        let m = FlagMatch { pattern: "-std{=}*".to_string(), count: None };
        assert_eq!(flag_name_len(&m), 4);
    }

    #[test]
    fn flag_name_len_exact() {
        let m = FlagMatch { pattern: "-std".to_string(), count: None };
        assert_eq!(flag_name_len(&m), 4);
    }

    #[test]
    fn flag_name_len_eq_with_count() {
        let m = FlagMatch { pattern: "-o=*".to_string(), count: Some(1) };
        assert_eq!(flag_name_len(&m), 3); // "-o" + "="
    }

    #[test]
    fn flag_name_len_prefix() {
        let m = FlagMatch { pattern: "-Wall*".to_string(), count: None };
        assert_eq!(flag_name_len(&m), 5);
    }

    // -- resolve_environment tests --

    #[test]
    fn resolve_environment_no_extends() {
        let raw_tables = load_tables();
        let entries = resolve_environment("gcc", &raw_tables);
        // gcc has its own environment entries
        assert!(!entries.is_empty());
        // All entries should have valid variable names
        for entry in &entries {
            assert!(!entry.variable.is_empty());
        }
    }

    #[test]
    fn resolve_environment_with_extends() {
        let raw_tables = load_tables();
        // clang extends gcc
        let clang_entries = resolve_environment("clang", &raw_tables);
        let gcc_entries = resolve_environment("gcc", &raw_tables);
        // clang should have at least as many entries as gcc (inherits + may add)
        assert!(clang_entries.len() >= gcc_entries.len());
    }

    #[test]
    fn resolve_environment_circular_safe() {
        // Create tables with circular extends (shouldn't happen in practice, but must not loop)
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        tables.insert(
            "a".to_string(),
            FlagTable {
                extends: Some("b".to_string()),
                type_: None,
                recognize: None,
                ignore_when: None,
                slash_prefix: None,
                flags: vec![],
                environment: Some(vec![make_test_env_entry("VAR_A")]),
            },
        );
        tables.insert(
            "b".to_string(),
            FlagTable {
                extends: Some("a".to_string()),
                type_: None,
                recognize: None,
                ignore_when: None,
                slash_prefix: None,
                flags: vec![],
                environment: Some(vec![make_test_env_entry("VAR_B")]),
            },
        );
        let entries = resolve_environment("a", &tables);
        assert_eq!(entries.len(), 2);
    }

    // -- resolve_ignore_when tests --

    #[test]
    fn resolve_ignore_when_no_extends_no_ignore() {
        let table = FlagTable {
            extends: None,
            type_: None,
            recognize: None,
            ignore_when: None,
            slash_prefix: None,
            flags: vec![],
            environment: None,
        };
        let tables = HashMap::new();
        let result = resolve_ignore_when(&table, &tables);
        assert!(result.executables.is_empty());
        assert!(result.flags.is_empty());
    }

    #[test]
    fn resolve_ignore_when_own_values() {
        let table = FlagTable {
            extends: None,
            type_: None,
            recognize: None,
            ignore_when: Some(IgnoreWhen {
                executables: vec!["cpp".to_string()],
                flags: vec!["-E".to_string()],
            }),
            slash_prefix: None,
            flags: vec![],
            environment: None,
        };
        let tables = HashMap::new();
        let result = resolve_ignore_when(&table, &tables);
        assert_eq!(result.executables, vec!["cpp"]);
        assert_eq!(result.flags, vec!["-E"]);
    }

    #[test]
    fn resolve_ignore_when_inherits_from_base() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        tables.insert(
            "base".to_string(),
            FlagTable {
                extends: None,
                type_: None,
                recognize: None,
                ignore_when: Some(IgnoreWhen {
                    executables: vec!["cpp".to_string()],
                    flags: vec!["-E".to_string()],
                }),
                slash_prefix: None,
                flags: vec![],
                environment: None,
            },
        );
        let table = FlagTable {
            extends: Some("base".to_string()),
            type_: None,
            recognize: None,
            ignore_when: None,
            slash_prefix: None,
            flags: vec![],
            environment: None,
        };
        let result = resolve_ignore_when(&table, &tables);
        assert_eq!(result.executables, vec!["cpp"]);
        assert_eq!(result.flags, vec!["-E"]);
    }

    #[test]
    fn resolve_ignore_when_own_overrides_base() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        tables.insert(
            "base".to_string(),
            FlagTable {
                extends: None,
                type_: None,
                recognize: None,
                ignore_when: Some(IgnoreWhen {
                    executables: vec!["cpp".to_string()],
                    flags: vec!["-E".to_string()],
                }),
                slash_prefix: None,
                flags: vec![],
                environment: None,
            },
        );
        let table = FlagTable {
            extends: Some("base".to_string()),
            type_: None,
            recognize: None,
            ignore_when: Some(IgnoreWhen { executables: vec!["cc1".to_string()], flags: vec![] }),
            slash_prefix: None,
            flags: vec![],
            environment: None,
        };
        let result = resolve_ignore_when(&table, &tables);
        assert_eq!(result.executables, vec!["cc1"]);
        // flags is empty in own, so inherits base
        assert_eq!(result.flags, vec!["-E"]);
    }

    // -- resolve_slash_prefix tests --

    #[test]
    fn resolve_slash_prefix_default() {
        let table = FlagTable {
            extends: None,
            type_: None,
            recognize: None,
            ignore_when: None,
            slash_prefix: None,
            flags: vec![],
            environment: None,
        };
        assert!(!resolve_slash_prefix(&table, &HashMap::new()));
    }

    #[test]
    fn resolve_slash_prefix_own_value() {
        let table = FlagTable {
            extends: None,
            type_: None,
            recognize: None,
            ignore_when: None,
            slash_prefix: Some(true),
            flags: vec![],
            environment: None,
        };
        assert!(resolve_slash_prefix(&table, &HashMap::new()));
    }

    #[test]
    fn resolve_slash_prefix_inherits_from_base() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        tables.insert(
            "base".to_string(),
            FlagTable {
                extends: None,
                type_: None,
                recognize: None,
                ignore_when: None,
                slash_prefix: Some(true),
                flags: vec![],
                environment: None,
            },
        );
        let table = FlagTable {
            extends: Some("base".to_string()),
            type_: None,
            recognize: None,
            ignore_when: None,
            slash_prefix: None,
            flags: vec![],
            environment: None,
        };
        assert!(resolve_slash_prefix(&table, &tables));
    }

    // -- validate_env_entry tests --

    #[test]
    fn validate_env_entry_valid() {
        let entry = EnvEntry {
            variable: "CPATH".to_string(),
            effect: "configures_compiling".to_string(),
            mapping: EnvMappingYaml {
                flag: Some("-I".to_string()),
                expand: None,
                separator: "path".to_string(),
            },
            note: None,
        };
        assert!(validate_env_entry(&entry, "test.yaml").is_ok());
    }

    #[test]
    fn validate_env_entry_invalid_name() {
        let entry = EnvEntry {
            variable: "123BAD".to_string(),
            effect: "configures_compiling".to_string(),
            mapping: EnvMappingYaml {
                flag: Some("-I".to_string()),
                expand: None,
                separator: "path".to_string(),
            },
            note: None,
        };
        let err = validate_env_entry(&entry, "test.yaml").unwrap_err();
        assert!(err.contains("invalid environment variable name"), "{}", err);
    }

    #[test]
    fn validate_env_entry_unknown_effect() {
        let entry = EnvEntry {
            variable: "CPATH".to_string(),
            effect: "bogus_effect".to_string(),
            mapping: EnvMappingYaml {
                flag: Some("-I".to_string()),
                expand: None,
                separator: "path".to_string(),
            },
            note: None,
        };
        let err = validate_env_entry(&entry, "test.yaml").unwrap_err();
        assert!(err.contains("unknown effect value"), "{}", err);
    }

    #[test]
    fn validate_env_entry_both_flag_and_expand() {
        let entry = EnvEntry {
            variable: "CPATH".to_string(),
            effect: "configures_compiling".to_string(),
            mapping: EnvMappingYaml {
                flag: Some("-I".to_string()),
                expand: Some("prepend".to_string()),
                separator: "path".to_string(),
            },
            note: None,
        };
        let err = validate_env_entry(&entry, "test.yaml").unwrap_err();
        assert!(err.contains("has both 'flag' and 'expand'"), "{}", err);
    }

    #[test]
    fn validate_env_entry_neither_flag_nor_expand() {
        let entry = EnvEntry {
            variable: "CPATH".to_string(),
            effect: "configures_compiling".to_string(),
            mapping: EnvMappingYaml { flag: None, expand: None, separator: "path".to_string() },
            note: None,
        };
        let err = validate_env_entry(&entry, "test.yaml").unwrap_err();
        assert!(err.contains("has neither 'flag' nor 'expand'"), "{}", err);
    }

    #[test]
    fn validate_env_entry_unknown_separator() {
        let entry = EnvEntry {
            variable: "CPATH".to_string(),
            effect: "configures_compiling".to_string(),
            mapping: EnvMappingYaml {
                flag: Some("-I".to_string()),
                expand: None,
                separator: "comma".to_string(),
            },
            note: None,
        };
        let err = validate_env_entry(&entry, "test.yaml").unwrap_err();
        assert!(err.contains("unknown separator"), "{}", err);
    }

    // -- Integration test: full generation from real YAML --

    #[test]
    fn generate_from_real_yaml() {
        let out_dir = tempfile::tempdir().unwrap();
        generate(&flags_dir(), out_dir.path());

        // Check all expected output files exist and are non-empty
        for config in TABLES {
            let path = out_dir.path().join(config.output_file);
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Missing output file: {}", config.output_file));
            assert!(!content.is_empty(), "Output file is empty: {}", config.output_file);
            assert!(
                content.contains(config.static_name),
                "Output file {} does not contain static name {}",
                config.output_file,
                config.static_name
            );
        }

        // Check recognition.rs
        let recognition = std::fs::read_to_string(out_dir.path().join("recognition.rs")).unwrap();
        assert!(recognition.contains("RECOGNITION_PATTERNS"));

        // Check env_keys.rs
        let env_keys = std::fs::read_to_string(out_dir.path().join("env_keys.rs")).unwrap();
        assert!(env_keys.contains("COMPILER_ENV_KEYS"));
    }

    fn make_test_env_entry(var: &str) -> EnvEntry {
        EnvEntry {
            variable: var.to_string(),
            effect: "configures_compiling".to_string(),
            mapping: EnvMappingYaml {
                flag: Some("-I".to_string()),
                expand: None,
                separator: "path".to_string(),
            },
            note: None,
        }
    }
}
