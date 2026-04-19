// SPDX-License-Identifier: GPL-3.0-or-later

pub mod codegen;
pub mod env_keys;
pub mod recognition;
pub mod resolve;
pub mod tables;
pub mod yaml_types;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

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
    pub fn new(
        key: &str,
        config: &'static TableConfig,
        raw_tables: &HashMap<String, FlagTable>,
    ) -> Result<Self> {
        if !raw_tables.contains_key(key) {
            bail!("no table found for '{}'", key);
        }

        let mut flags = resolve_flags(key, raw_tables)
            .with_context(|| format!("resolving flags for {}", config.yaml_file))?;
        flags.sort_by_key(|b| std::cmp::Reverse(b.match_.name_len()));

        Ok(ResolvedTable {
            config,
            flags,
            ignore_when: resolve_ignore_when(key, raw_tables),
            slash_prefix: resolve_slash_prefix(key, raw_tables),
            env_entries: resolve_environment(key, raw_tables),
        })
    }

    /// Generate the complete Rust source file for this compiler's flag table.
    pub fn generate(&self) -> Result<String> {
        let mut out = String::new();
        out.push_str(
            &self
                .generate_flag_array()
                .with_context(|| format!("generating flags for {}", self.config.yaml_file))?,
        );
        out.push_str(&self.generate_ignore_arrays());
        out.push_str(&format!("static {}: bool = {};\n", self.config.slash_prefix_name, self.slash_prefix));
        out.push_str(
            &self
                .generate_env_array()
                .with_context(|| format!("generating env rules for {}", self.config.yaml_file))?,
        );
        Ok(out)
    }

    fn generate_flag_array(&self) -> Result<String> {
        let mut out = String::new();
        out.push_str(&format!("// Generated from interpreters/{} -- DO NOT EDIT\n", self.config.yaml_file));
        out.push_str(&format!("static {}: [FlagRule; {}] = [\n", self.config.static_name, self.flags.len()));
        for entry in &self.flags {
            let pattern_rust = pattern_to_rust(&entry.match_.pattern, entry.match_.count);
            let result_rust =
                result_to_rust(&entry.result).with_context(|| format!("flag '{}'", entry.match_.pattern))?;
            out.push_str(&format!("    FlagRule::new({}, {}),\n", pattern_rust, result_rust));
        }
        out.push_str("];\n");
        Ok(out)
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

    fn generate_env_array(&self) -> Result<String> {
        let active: Vec<&EnvEntry> = self.env_entries.iter().filter(|e| e.effect != "none").collect();

        for entry in &active {
            entry.validate().with_context(|| format!("environment entry in {}", self.config.yaml_file))?;
        }

        let mut out = String::new();
        out.push_str(&format!("static {}: [EnvRule; {}] = [\n", self.config.env_rules_name, active.len()));
        for entry in &active {
            let mapping_rust = entry
                .mapping
                .to_rust()
                .with_context(|| format!("variable '{}' in {}", entry.variable, self.config.yaml_file))?;
            let effect_rust = result_to_rust(&entry.effect)
                .with_context(|| format!("variable '{}' in {}", entry.variable, self.config.yaml_file))?;
            out.push_str(&format!(
                "    EnvRule::new(\"{}\", {}, {}),\n",
                entry.variable, mapping_rust, effect_rust
            ));
        }
        out.push_str("];\n");
        Ok(out)
    }
}

/// Generate all flag tables, recognition patterns, and env keys.
///
/// - `flags_dir`: path to the directory containing *.yaml files
/// - `out_dir`: path to write generated .rs files
///
/// Prints `cargo:rerun-if-changed` lines to stdout (for build.rs integration).
pub fn generate(flags_dir: &Path, out_dir: &Path) -> Result<()> {
    let raw_tables = load_tables_from(flags_dir)?;

    // Generate recognition patterns
    let recognition = generate_recognition_patterns(&raw_tables);
    write_output(out_dir, "recognition.rs", recognition)?;

    // Generate each compiler's flag table
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let resolved = ResolvedTable::new(key, config, &raw_tables)?;
        write_output(out_dir, config.output_file, resolved.generate()?)?;
    }

    // Generate combined environment variable keys
    let env_keys = generate_env_keys(&raw_tables);
    write_output(out_dir, "env_keys.rs", env_keys)?;

    Ok(())
}

/// Load YAML flag tables from a directory, printing cargo:rerun-if-changed.
fn load_tables_from(flags_dir: &Path) -> Result<HashMap<String, FlagTable>> {
    let mut raw_tables = HashMap::new();
    for config in TABLES {
        let yaml_path = flags_dir.join(config.yaml_file);
        println!("cargo:rerun-if-changed={}", yaml_path.display());

        let content = std::fs::read_to_string(&yaml_path)
            .with_context(|| format!("reading {}", yaml_path.display()))?;
        let table: FlagTable =
            serde_saphyr::from_str(&content).with_context(|| format!("parsing {}", yaml_path.display()))?;

        let key = config.yaml_file.strip_suffix(".yaml").unwrap().to_string();
        raw_tables.insert(key, table);
    }
    Ok(raw_tables)
}

fn write_output(out_dir: &Path, filename: &str, content: String) -> Result<()> {
    let out_path = out_dir.join(filename);
    std::fs::write(&out_path, content).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Path to the YAML flag definitions in the workspace.
pub fn flags_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("bear/interpreters")
}

/// Load all YAML flag tables from the workspace interpreters directory.
pub fn load_tables() -> Result<HashMap<String, FlagTable>> {
    let flags_dir = flags_dir();
    let mut raw_tables = HashMap::new();
    for config in TABLES {
        let yaml_path = flags_dir.join(config.yaml_file);
        let content = std::fs::read_to_string(&yaml_path)
            .with_context(|| format!("reading {}", yaml_path.display()))?;
        let table: FlagTable =
            serde_saphyr::from_str(&content).with_context(|| format!("parsing {}", yaml_path.display()))?;
        let key = config.yaml_file.strip_suffix(".yaml").unwrap().to_string();
        raw_tables.insert(key, table);
    }
    Ok(raw_tables)
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
    fn result_known_values() {
        assert!(result_to_rust("output").is_ok());
        assert!(result_to_rust("configures_preprocessing").is_ok());
        assert!(result_to_rust("configures_compiling").is_ok());
        assert!(result_to_rust("configures_assembling").is_ok());
        assert!(result_to_rust("configures_linking").is_ok());
        assert!(result_to_rust("stops_at_preprocessing").is_ok());
        assert!(result_to_rust("stops_at_compiling").is_ok());
        assert!(result_to_rust("stops_at_assembling").is_ok());
        assert!(result_to_rust("info_and_exit").is_ok());
        assert!(result_to_rust("driver_option").is_ok());
        assert!(result_to_rust("pass_through").is_ok());
        assert!(result_to_rust("none").is_ok());
    }

    #[test]
    fn result_unknown_is_err() {
        let err = result_to_rust("bogus").unwrap_err();
        assert!(err.to_string().contains("unknown result value"), "{}", err);
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
        assert_eq!(m.name_len(), 3);
    }

    #[test]
    fn name_len_prefix() {
        let m = FlagMatch { pattern: "-Wall*".to_string(), count: None };
        assert_eq!(m.name_len(), 5);
    }

    // -- resolve tests --

    #[test]
    fn resolve_environment_no_extends() {
        let raw_tables = load_tables().unwrap();
        let entries = resolve_environment("gcc", &raw_tables);
        assert!(!entries.is_empty());
    }

    #[test]
    fn resolve_environment_with_extends() {
        let raw_tables = load_tables().unwrap();
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
        assert_eq!(resolve_environment("a", &tables).len(), 2);
    }

    #[test]
    fn resolve_ignore_when_no_extends() {
        let mut tables = HashMap::new();
        tables.insert("leaf".to_string(), make_empty_table());
        let result = resolve_ignore_when("leaf", &tables);
        assert!(result.executables.is_empty());
        assert!(result.flags.is_empty());
    }

    #[test]
    fn resolve_ignore_when_transitive() {
        let mut tables = HashMap::new();
        let mut gp = make_empty_table();
        gp.ignore_when =
            Some(IgnoreWhen { executables: vec!["cpp".to_string()], flags: vec!["-E".to_string()] });
        tables.insert("gp".to_string(), gp);
        let mut parent = make_empty_table();
        parent.extends = Some("gp".to_string());
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        tables.insert("child".to_string(), child);
        let result = resolve_ignore_when("child", &tables);
        assert_eq!(result.executables, vec!["cpp"]);
        assert_eq!(result.flags, vec!["-E"]);
    }

    #[test]
    fn resolve_ignore_when_own_overrides() {
        let mut tables = HashMap::new();
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
        assert_eq!(result.flags, vec!["-E"]);
    }

    #[test]
    fn resolve_slash_prefix_default_is_false() {
        let mut tables = HashMap::new();
        tables.insert("leaf".to_string(), make_empty_table());
        assert!(!resolve_slash_prefix("leaf", &tables));
    }

    #[test]
    fn resolve_slash_prefix_transitive() {
        let mut tables = HashMap::new();
        let mut gp = make_empty_table();
        gp.slash_prefix = Some(true);
        tables.insert("gp".to_string(), gp);
        let mut parent = make_empty_table();
        parent.extends = Some("gp".to_string());
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        tables.insert("child".to_string(), child);
        assert!(resolve_slash_prefix("child", &tables));
    }

    #[test]
    fn resolve_flags_transitive() {
        let mut tables = HashMap::new();
        let mut gp = make_empty_table();
        gp.flags = vec![make_test_flag("-gp", "output")];
        tables.insert("gp".to_string(), gp);
        let mut parent = make_empty_table();
        parent.extends = Some("gp".to_string());
        parent.flags = vec![make_test_flag("-p", "output")];
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        child.flags = vec![make_test_flag("-ch", "output")];
        tables.insert("child".to_string(), child);

        let flags = resolve_flags("child", &tables).unwrap();
        assert_eq!(flags.len(), 3);
        assert_eq!(flags[0].match_.pattern, "-ch");
        assert_eq!(flags[1].match_.pattern, "-p");
        assert_eq!(flags[2].match_.pattern, "-gp");
    }

    #[test]
    fn resolve_flags_dedup_same_result() {
        let mut tables = HashMap::new();
        let mut parent = make_empty_table();
        parent.flags = vec![make_test_flag("-c", "stops_at_compiling")];
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        child.flags = vec![make_test_flag("-c", "stops_at_compiling")];
        tables.insert("child".to_string(), child);
        let flags = resolve_flags("child", &tables).unwrap();
        assert_eq!(flags.len(), 1);
    }

    #[test]
    fn resolve_flags_conflict_is_err() {
        let mut tables = HashMap::new();
        let mut parent = make_empty_table();
        parent.flags = vec![make_test_flag("-c", "stops_at_compiling")];
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.extends = Some("parent".to_string());
        child.flags = vec![make_test_flag("-c", "output")];
        tables.insert("child".to_string(), child);
        let err = resolve_flags("child", &tables).unwrap_err();
        assert!(err.to_string().contains("conflicting"), "{}", err);
    }

    #[test]
    fn resolve_flags_real_no_conflicts() {
        let raw_tables = load_tables().unwrap();
        for config in TABLES {
            let key = config.yaml_file.strip_suffix(".yaml").unwrap();
            resolve_flags(key, &raw_tables).unwrap();
        }
    }

    #[test]
    fn resolve_flags_real_ibm_xl_includes_gcc() {
        let raw_tables = load_tables().unwrap();
        let ibm = resolve_flags("ibm_xl", &raw_tables).unwrap();
        let gcc = resolve_flags("gcc", &raw_tables).unwrap();
        for gf in &gcc {
            assert!(
                ibm.iter().any(|f| f.match_.pattern == gf.match_.pattern),
                "ibm_xl missing gcc flag: {}",
                gf.match_.pattern
            );
        }
    }

    // -- EnvEntry::validate tests --

    #[test]
    fn validate_env_entry_valid() {
        let entry = make_test_env_entry("CPATH");
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn validate_env_entry_invalid_name() {
        let mut entry = make_test_env_entry("CPATH");
        entry.variable = "123BAD".to_string();
        let err = entry.validate().unwrap_err();
        assert!(err.to_string().contains("invalid environment variable name"), "{}", err);
    }

    #[test]
    fn validate_env_entry_unknown_effect() {
        let mut entry = make_test_env_entry("CPATH");
        entry.effect = "bogus_effect".to_string();
        let err = entry.validate().unwrap_err();
        assert!(err.to_string().contains("unknown effect"), "{}", err);
    }

    #[test]
    fn validate_env_entry_both_flag_and_expand() {
        let mut entry = make_test_env_entry("CPATH");
        entry.mapping.expand = Some("prepend".to_string());
        let err = entry.validate().unwrap_err();
        assert!(err.to_string().contains("both 'flag' and 'expand'"), "{}", err);
    }

    #[test]
    fn validate_env_entry_neither_flag_nor_expand() {
        let mut entry = make_test_env_entry("CPATH");
        entry.mapping.flag = None;
        let err = entry.validate().unwrap_err();
        assert!(err.to_string().contains("neither 'flag' nor 'expand'"), "{}", err);
    }

    #[test]
    fn validate_env_entry_unknown_separator() {
        let mut entry = make_test_env_entry("CPATH");
        entry.mapping.separator = "comma".to_string();
        let err = entry.validate().unwrap_err();
        assert!(err.to_string().contains("unknown separator"), "{}", err);
    }

    // -- EnvMappingYaml::to_rust tests --

    #[test]
    fn env_mapping_to_rust_no_flag_no_expand_is_err() {
        let mapping = EnvMappingYaml { flag: None, expand: None, separator: "path".to_string() };
        let err = mapping.to_rust().unwrap_err();
        assert!(err.to_string().contains("neither 'flag' nor 'expand'"), "{}", err);
    }

    #[test]
    fn env_mapping_to_rust_unknown_expand_is_err() {
        let mapping =
            EnvMappingYaml { flag: None, expand: Some("middle".to_string()), separator: "path".to_string() };
        let err = mapping.to_rust().unwrap_err();
        assert!(err.to_string().contains("unknown expand position"), "{}", err);
    }

    // -- Integration test --

    #[test]
    fn generate_from_real_yaml() {
        let out_dir = tempfile::tempdir().unwrap();
        generate(&flags_dir(), out_dir.path()).unwrap();

        for config in TABLES {
            let path = out_dir.path().join(config.output_file);
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Missing output file: {}", config.output_file));
            assert!(!content.is_empty());
            assert!(content.contains(config.static_name));
        }
        assert!(
            std::fs::read_to_string(out_dir.path().join("recognition.rs"))
                .unwrap()
                .contains("RECOGNITION_PATTERNS")
        );
        assert!(
            std::fs::read_to_string(out_dir.path().join("env_keys.rs"))
                .unwrap()
                .contains("COMPILER_ENV_KEYS")
        );
    }

    // -- helpers --

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
