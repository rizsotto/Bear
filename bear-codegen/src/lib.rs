// SPDX-License-Identifier: GPL-3.0-or-later

pub mod codegen;
pub mod env_keys;
pub mod recognition;
pub mod resolve;
pub mod tables;
pub mod yaml_types;

use std::collections::HashMap;
use std::path::Path;

use codegen::{pattern_to_rust, result_to_rust};
use env_keys::generate_env_keys;
use recognition::generate_recognition_patterns;
use resolve::{resolve_environment, resolve_flags, resolve_ignore_when, resolve_slash_prefix};
use tables::{TABLES, TableConfig};
use yaml_types::{EnvEntry, FlagEntry, FlagTable, IgnoreWhen};

/// A compiler flag table with all inheritance resolved and ready for code generation.
pub struct ResolvedTable {
    pub config: &'static TableConfig,
    pub flags: Vec<FlagEntry>,
    pub ignore_when: IgnoreWhen,
    pub slash_prefix: bool,
    pub env_entries: Vec<EnvEntry>,
}

impl ResolvedTable {
    /// Resolve a single compiler table by merging inherited flags, ignore_when,
    /// slash_prefix, and environment entries from the extends chain.
    pub fn new(key: &str, config: &'static TableConfig, raw_tables: &HashMap<String, FlagTable>) -> Self {
        let mut flags = resolve_flags(key, raw_tables);
        // Sort by flag length descending (stable sort preserves own-before-base order)
        flags.sort_by(|a, b| b.match_.name_len().cmp(&a.match_.name_len()));

        ResolvedTable {
            config,
            flags,
            ignore_when: resolve_ignore_when(key, raw_tables),
            slash_prefix: resolve_slash_prefix(key, raw_tables),
            env_entries: resolve_environment(key, raw_tables),
        }
    }

    /// Generate the complete Rust source file for this compiler's flag table.
    pub fn generate(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.generate_flag_array());
        out.push_str(&self.generate_ignore_arrays());
        out.push_str(&format!("static {}: bool = {};\n", self.config.slash_prefix_name, self.slash_prefix));
        out.push_str(&self.generate_env_array());
        out
    }

    fn generate_flag_array(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("// Generated from interpreters/{} -- DO NOT EDIT\n", self.config.yaml_file));
        out.push_str(&format!("static {}: [FlagRule; {}] = [\n", self.config.static_name, self.flags.len()));
        for entry in &self.flags {
            let pattern_rust = pattern_to_rust(&entry.match_.pattern, entry.match_.count);
            let result_rust = result_to_rust(&entry.result);
            out.push_str(&format!("    FlagRule::new({}, {}),\n", pattern_rust, result_rust));
        }
        out.push_str("];\n");
        out
    }

    fn generate_ignore_arrays(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "static {}: [&str; {}] = [",
            self.config.ignore_executables_name,
            self.ignore_when.executables.len()
        ));
        for exe in &self.ignore_when.executables {
            out.push_str(&format!("\"{}\", ", exe));
        }
        out.push_str("];\n");

        out.push_str(&format!(
            "static {}: [&str; {}] = [",
            self.config.ignore_flags_name,
            self.ignore_when.flags.len()
        ));
        for flag in &self.ignore_when.flags {
            out.push_str(&format!("\"{}\", ", flag));
        }
        out.push_str("];\n");
        out
    }

    fn generate_env_array(&self) -> String {
        let active: Vec<&EnvEntry> = self.env_entries.iter().filter(|e| e.effect != "none").collect();

        for entry in &active {
            entry.validate(self.config.yaml_file).unwrap_or_else(|e| panic!("{}", e));
        }

        let mut out = String::new();
        out.push_str(&format!("static {}: [EnvRule; {}] = [\n", self.config.env_rules_name, active.len()));
        for entry in &active {
            let mapping_rust = entry.mapping.to_rust();
            let effect_rust = result_to_rust(&entry.effect);
            out.push_str(&format!(
                "    EnvRule::new(\"{}\", {}, {}),\n",
                entry.variable, mapping_rust, effect_rust
            ));
        }
        out.push_str("];\n");
        out
    }
}

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

        let key = config.yaml_file.strip_suffix(".yaml").unwrap().to_string();
        raw_tables.insert(key, table);
    }

    // Generate recognition patterns
    let recognition = generate_recognition_patterns(&raw_tables);
    write_output(out_dir, "recognition.rs", recognition);

    // Generate each compiler's flag table
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let resolved = ResolvedTable::new(key, config, &raw_tables);
        write_output(out_dir, config.output_file, resolved.generate());
    }

    // Generate combined environment variable keys
    let env_keys = generate_env_keys(&raw_tables);
    write_output(out_dir, "env_keys.rs", env_keys);
}

fn write_output(out_dir: &Path, filename: &str, content: String) {
    let out_path = out_dir.join(filename);
    std::fs::write(&out_path, content)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
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

    // -- FlagMatch::name_len tests --

    #[test]
    fn name_len_exact_with_glued() {
        let m = FlagMatch { pattern: "-std{=}*".to_string(), count: None };
        assert_eq!(m.name_len(), 4);
    }

    #[test]
    fn name_len_exact() {
        let m = FlagMatch { pattern: "-std".to_string(), count: None };
        assert_eq!(m.name_len(), 4);
    }

    #[test]
    fn name_len_eq_with_count() {
        let m = FlagMatch { pattern: "-o=*".to_string(), count: Some(1) };
        assert_eq!(m.name_len(), 3); // "-o" + "="
    }

    #[test]
    fn name_len_prefix() {
        let m = FlagMatch { pattern: "-Wall*".to_string(), count: None };
        assert_eq!(m.name_len(), 5);
    }

    // -- resolve_environment tests --

    #[test]
    fn resolve_environment_no_extends() {
        let raw_tables = load_tables();
        let entries = resolve_environment("gcc", &raw_tables);
        assert!(!entries.is_empty());
        for entry in &entries {
            assert!(!entry.variable.is_empty());
        }
    }

    #[test]
    fn resolve_environment_with_extends() {
        let raw_tables = load_tables();
        let clang_entries = resolve_environment("clang", &raw_tables);
        let gcc_entries = resolve_environment("gcc", &raw_tables);
        assert!(clang_entries.len() >= gcc_entries.len());
    }

    #[test]
    fn resolve_environment_circular_safe() {
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
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        tables.insert("leaf".to_string(), make_empty_table());
        let result = resolve_ignore_when("leaf", &tables);
        assert!(result.executables.is_empty());
        assert!(result.flags.is_empty());
    }

    #[test]
    fn resolve_ignore_when_own_values() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut t = make_empty_table();
        t.ignore_when =
            Some(IgnoreWhen { executables: vec!["cpp".to_string()], flags: vec!["-E".to_string()] });
        tables.insert("leaf".to_string(), t);
        let result = resolve_ignore_when("leaf", &tables);
        assert_eq!(result.executables, vec!["cpp"]);
        assert_eq!(result.flags, vec!["-E"]);
    }

    #[test]
    fn resolve_ignore_when_inherits_from_base() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut base = make_empty_table();
        base.ignore_when =
            Some(IgnoreWhen { executables: vec!["cpp".to_string()], flags: vec!["-E".to_string()] });
        tables.insert("base".to_string(), base);
        let mut child = make_empty_table();
        child.extends = Some("base".to_string());
        tables.insert("child".to_string(), child);
        let result = resolve_ignore_when("child", &tables);
        assert_eq!(result.executables, vec!["cpp"]);
        assert_eq!(result.flags, vec!["-E"]);
    }

    #[test]
    fn resolve_ignore_when_own_overrides_base() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut base = make_empty_table();
        base.ignore_when =
            Some(IgnoreWhen { executables: vec!["cpp".to_string()], flags: vec!["-E".to_string()] });
        tables.insert("base".to_string(), base);
        let mut child = make_empty_table();
        child.extends = Some("base".to_string());
        child.ignore_when = Some(IgnoreWhen { executables: vec!["cc1".to_string()], flags: vec![] });
        tables.insert("child".to_string(), child);
        let result = resolve_ignore_when("child", &tables);
        assert_eq!(result.executables, vec!["cc1"]);
        // flags is empty in own, so inherits base
        assert_eq!(result.flags, vec!["-E"]);
    }

    #[test]
    fn resolve_ignore_when_transitive_three_levels() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut grandparent = make_empty_table();
        grandparent.ignore_when =
            Some(IgnoreWhen { executables: vec!["cpp".to_string()], flags: vec!["-E".to_string()] });
        tables.insert("grandparent".to_string(), grandparent);
        let mut parent = make_empty_table();
        parent.extends = Some("grandparent".to_string());
        // parent has no ignore_when - should inherit from grandparent
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        tables.insert("child".to_string(), child);
        let result = resolve_ignore_when("child", &tables);
        assert_eq!(result.executables, vec!["cpp"]);
        assert_eq!(result.flags, vec!["-E"]);
    }

    // -- resolve_slash_prefix tests --

    #[test]
    fn resolve_slash_prefix_default() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        tables.insert("leaf".to_string(), make_empty_table());
        assert!(!resolve_slash_prefix("leaf", &tables));
    }

    #[test]
    fn resolve_slash_prefix_own_value() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut t = make_empty_table();
        t.slash_prefix = Some(true);
        tables.insert("leaf".to_string(), t);
        assert!(resolve_slash_prefix("leaf", &tables));
    }

    #[test]
    fn resolve_slash_prefix_inherits_from_base() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut base = make_empty_table();
        base.slash_prefix = Some(true);
        tables.insert("base".to_string(), base);
        let mut child = make_empty_table();
        child.extends = Some("base".to_string());
        tables.insert("child".to_string(), child);
        assert!(resolve_slash_prefix("child", &tables));
    }

    #[test]
    fn resolve_slash_prefix_transitive_three_levels() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut grandparent = make_empty_table();
        grandparent.slash_prefix = Some(true);
        tables.insert("grandparent".to_string(), grandparent);
        let mut parent = make_empty_table();
        parent.extends = Some("grandparent".to_string());
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        tables.insert("child".to_string(), child);
        assert!(resolve_slash_prefix("child", &tables));
    }

    // -- resolve_flags tests --

    #[test]
    fn resolve_flags_no_extends() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut t = make_empty_table();
        t.flags = vec![make_test_flag("-c", "output")];
        tables.insert("leaf".to_string(), t);
        let flags = resolve_flags("leaf", &tables);
        assert_eq!(flags.len(), 1);
    }

    #[test]
    fn resolve_flags_transitive_three_levels() {
        let mut tables: HashMap<String, FlagTable> = HashMap::new();
        let mut grandparent = make_empty_table();
        grandparent.flags = vec![make_test_flag("-gp", "output")];
        tables.insert("grandparent".to_string(), grandparent);
        let mut parent = make_empty_table();
        parent.extends = Some("grandparent".to_string());
        parent.flags = vec![make_test_flag("-p", "output")];
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        child.flags = vec![make_test_flag("-ch", "output")];
        tables.insert("child".to_string(), child);

        let flags = resolve_flags("child", &tables);
        assert_eq!(flags.len(), 3);
        // Own flags first, then parent, then grandparent
        assert_eq!(flags[0].match_.pattern, "-ch");
        assert_eq!(flags[1].match_.pattern, "-p");
        assert_eq!(flags[2].match_.pattern, "-gp");
    }

    #[test]
    fn resolve_flags_real_ibm_xl_includes_gcc() {
        let raw_tables = load_tables();
        let ibm_flags = resolve_flags("ibm_xl", &raw_tables);
        let gcc_flags = resolve_flags("gcc", &raw_tables);
        // ibm_xl extends clang extends gcc - must include all gcc flags
        for gcc_flag in &gcc_flags {
            assert!(
                ibm_flags.iter().any(|f| f.match_.pattern == gcc_flag.match_.pattern),
                "ibm_xl is missing gcc flag: {}",
                gcc_flag.match_.pattern
            );
        }
    }

    // -- EnvEntry::validate tests --

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
        assert!(entry.validate("test.yaml").is_ok());
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
        let err = entry.validate("test.yaml").unwrap_err();
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
        let err = entry.validate("test.yaml").unwrap_err();
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
        let err = entry.validate("test.yaml").unwrap_err();
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
        let err = entry.validate("test.yaml").unwrap_err();
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
        let err = entry.validate("test.yaml").unwrap_err();
        assert!(err.contains("unknown separator"), "{}", err);
    }

    // -- Integration test: full generation from real YAML --

    #[test]
    fn generate_from_real_yaml() {
        let out_dir = tempfile::tempdir().unwrap();
        generate(&flags_dir(), out_dir.path());

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

        let recognition = std::fs::read_to_string(out_dir.path().join("recognition.rs")).unwrap();
        assert!(recognition.contains("RECOGNITION_PATTERNS"));

        let env_keys = std::fs::read_to_string(out_dir.path().join("env_keys.rs")).unwrap();
        assert!(env_keys.contains("COMPILER_ENV_KEYS"));
    }

    fn make_empty_table() -> FlagTable {
        FlagTable {
            extends: None,
            type_: None,
            recognize: None,
            ignore_when: None,
            slash_prefix: None,
            flags: vec![],
            environment: None,
        }
    }

    fn make_test_flag(pattern: &str, result: &str) -> FlagEntry {
        FlagEntry {
            match_: FlagMatch { pattern: pattern.to_string(), count: None },
            result: result.to_string(),
        }
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
