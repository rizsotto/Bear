// SPDX-License-Identifier: GPL-3.0-or-later

/// Driver executable name (platform-dependent)
#[cfg(windows)]
const DRIVER_NAME: &str = "bear-driver.exe";
#[cfg(not(windows))]
const DRIVER_NAME: &str = "bear-driver";

/// Wrapper executable name (platform-dependent)
#[cfg(windows)]
const WRAPPER_NAME: &str = "bear-wrapper.exe";
#[cfg(not(windows))]
const WRAPPER_NAME: &str = "bear-wrapper";

/// Preload library name (platform-dependent)
#[cfg(target_os = "macos")]
const PRELOAD_NAME: &str = "libexec.dylib";
#[cfg(not(target_os = "macos"))]
const PRELOAD_NAME: &str = "libexec.so";

fn main() {
    let intercept_libdir = std::env::var("INTERCEPT_LIBDIR").unwrap_or_else(|_| "lib".to_string());
    validate_intercept_libdir(&intercept_libdir);

    println!("cargo:rustc-env=DRIVER_NAME={}", DRIVER_NAME);
    println!("cargo:rustc-env=WRAPPER_NAME={}", WRAPPER_NAME);
    println!("cargo:rustc-env=PRELOAD_NAME={}", PRELOAD_NAME);
    println!("cargo:rustc-env=INTERCEPT_LIBDIR={}", intercept_libdir);
    println!("cargo:rerun-if-env-changed=INTERCEPT_LIBDIR");

    flags::generate_flag_tables();
}

/// Validate the `INTERCEPT_LIBDIR` environment variable.
///
/// Valid values:
/// - A non-empty relative path (e.g. `"lib"`, `"lib64"`, `"lib/x86_64-linux-gnu"`).
/// - The literal string `"$LIB"` to defer directory selection to runtime/platform
///   conventions (commonly interpreted as either `"lib"` or `"lib64"` depending on
///   the system).
fn validate_intercept_libdir(value: &str) {
    let value = value.trim();

    if value.is_empty() {
        panic!("INTERCEPT_LIBDIR must not be empty or whitespace-only");
    }

    if value == "$LIB" {
        return;
    }

    let path = std::path::Path::new(value);
    if path.is_absolute() {
        panic!("INTERCEPT_LIBDIR must be a relative path, got: {}", value);
    }
}

/// Build-time code generation for compiler flag tables.
///
/// Reads YAML flag definition files from `interpreters/` and generates Rust source files
/// containing static `[FlagRule; N]` arrays that are included via `include!()` in
/// the compiler interpreter modules.
mod flags {
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[derive(Deserialize)]
    struct FlagTable {
        extends: Option<String>,
        #[serde(rename = "type")]
        type_: Option<String>,
        recognize: Option<Vec<RecognizeEntry>>,
        ignore_when: Option<IgnoreWhen>,
        /// When true, arguments starting with '/' are treated as flags (MSVC-style).
        #[serde(default)]
        slash_prefix: Option<bool>,
        flags: Vec<FlagEntry>,
        #[serde(default)]
        environment: Option<Vec<EnvEntry>>,
    }

    #[derive(Deserialize, Clone)]
    struct RecognizeEntry {
        executables: Vec<String>,
        #[serde(default)]
        cross_compilation: bool,
        #[serde(default)]
        versioned: bool,
    }

    #[derive(Deserialize, Clone, Default)]
    struct IgnoreWhen {
        #[serde(default)]
        executables: Vec<String>,
        #[serde(default)]
        flags: Vec<String>,
    }

    #[derive(Deserialize, Clone)]
    struct FlagEntry {
        #[serde(rename = "match")]
        match_: FlagMatch,
        result: String,
    }

    #[derive(Deserialize, Clone)]
    struct FlagMatch {
        pattern: String,
        count: Option<u32>,
    }

    #[derive(Deserialize, Clone)]
    struct EnvEntry {
        variable: String,
        effect: String,
        mapping: EnvMappingYaml,
        #[serde(default)]
        #[allow(dead_code)]
        note: Option<String>,
    }

    #[derive(Deserialize, Clone)]
    struct EnvMappingYaml {
        #[serde(default)]
        flag: Option<String>,
        #[serde(default)]
        expand: Option<String>,
        separator: String,
    }

    /// Table metadata: name of the static, which file to generate.
    struct TableConfig {
        yaml_file: &'static str,
        static_name: &'static str,
        ignore_executables_name: &'static str,
        ignore_flags_name: &'static str,
        slash_prefix_name: &'static str,
        env_rules_name: &'static str,
        output_file: &'static str,
    }

    // Table order matters: it determines recognition pattern priority.
    // More specific compilers (whose executable names could be mistaken for
    // cross-compilation variants of general compilers) must come first.
    // E.g., ibm-clang must match IbmXl before Clang's cross-compilation pattern.
    const TABLES: &[TableConfig] = &[
        TableConfig {
            yaml_file: "gcc.yaml",
            static_name: "GCC_FLAGS",
            ignore_executables_name: "GCC_IGNORE_EXECUTABLES",
            ignore_flags_name: "GCC_IGNORE_FLAGS",
            slash_prefix_name: "GCC_SLASH_PREFIX",
            env_rules_name: "GCC_ENV_RULES",
            output_file: "flags_gcc.rs",
        },
        // IBM XL before Clang: ibm-clang looks like cross-compilation clang
        TableConfig {
            yaml_file: "ibm_xl.yaml",
            static_name: "IBM_XL_FLAGS",
            ignore_executables_name: "IBM_XL_IGNORE_EXECUTABLES",
            ignore_flags_name: "IBM_XL_IGNORE_FLAGS",
            slash_prefix_name: "IBM_XL_SLASH_PREFIX",
            env_rules_name: "IBM_XL_ENV_RULES",
            output_file: "flags_ibm_xl.rs",
        },
        // clang-cl before Clang: clang-cl is versioned and could match clang's pattern
        TableConfig {
            yaml_file: "clang_cl.yaml",
            static_name: "CLANG_CL_FLAGS",
            ignore_executables_name: "CLANG_CL_IGNORE_EXECUTABLES",
            ignore_flags_name: "CLANG_CL_IGNORE_FLAGS",
            slash_prefix_name: "CLANG_CL_SLASH_PREFIX",
            env_rules_name: "CLANG_CL_ENV_RULES",
            output_file: "flags_clang_cl.rs",
        },
        TableConfig {
            yaml_file: "clang.yaml",
            static_name: "CLANG_FLAGS",
            ignore_executables_name: "CLANG_IGNORE_EXECUTABLES",
            ignore_flags_name: "CLANG_IGNORE_FLAGS",
            slash_prefix_name: "CLANG_SLASH_PREFIX",
            env_rules_name: "CLANG_ENV_RULES",
            output_file: "flags_clang.rs",
        },
        TableConfig {
            yaml_file: "flang.yaml",
            static_name: "FLANG_FLAGS",
            ignore_executables_name: "FLANG_IGNORE_EXECUTABLES",
            ignore_flags_name: "FLANG_IGNORE_FLAGS",
            slash_prefix_name: "FLANG_SLASH_PREFIX",
            env_rules_name: "FLANG_ENV_RULES",
            output_file: "flags_flang.rs",
        },
        TableConfig {
            yaml_file: "cuda.yaml",
            static_name: "CUDA_FLAGS",
            ignore_executables_name: "CUDA_IGNORE_EXECUTABLES",
            ignore_flags_name: "CUDA_IGNORE_FLAGS",
            slash_prefix_name: "CUDA_SLASH_PREFIX",
            env_rules_name: "CUDA_ENV_RULES",
            output_file: "flags_cuda.rs",
        },
        TableConfig {
            yaml_file: "intel_fortran.yaml",
            static_name: "INTEL_FORTRAN_FLAGS",
            ignore_executables_name: "INTEL_FORTRAN_IGNORE_EXECUTABLES",
            ignore_flags_name: "INTEL_FORTRAN_IGNORE_FLAGS",
            slash_prefix_name: "INTEL_FORTRAN_SLASH_PREFIX",
            env_rules_name: "INTEL_FORTRAN_ENV_RULES",
            output_file: "flags_intel_fortran.rs",
        },
        TableConfig {
            yaml_file: "cray_fortran.yaml",
            static_name: "CRAY_FORTRAN_FLAGS",
            ignore_executables_name: "CRAY_FORTRAN_IGNORE_EXECUTABLES",
            ignore_flags_name: "CRAY_FORTRAN_IGNORE_FLAGS",
            slash_prefix_name: "CRAY_FORTRAN_SLASH_PREFIX",
            env_rules_name: "CRAY_FORTRAN_ENV_RULES",
            output_file: "flags_cray_fortran.rs",
        },
        TableConfig {
            yaml_file: "msvc.yaml",
            static_name: "MSVC_FLAGS",
            ignore_executables_name: "MSVC_IGNORE_EXECUTABLES",
            ignore_flags_name: "MSVC_IGNORE_FLAGS",
            slash_prefix_name: "MSVC_SLASH_PREFIX",
            env_rules_name: "MSVC_ENV_RULES",
            output_file: "flags_msvc.rs",
        },
        TableConfig {
            yaml_file: "intel_cc.yaml",
            static_name: "INTEL_CC_FLAGS",
            ignore_executables_name: "INTEL_CC_IGNORE_EXECUTABLES",
            ignore_flags_name: "INTEL_CC_IGNORE_FLAGS",
            slash_prefix_name: "INTEL_CC_SLASH_PREFIX",
            env_rules_name: "INTEL_CC_ENV_RULES",
            output_file: "flags_intel_cc.rs",
        },
        TableConfig {
            yaml_file: "nvidia_hpc.yaml",
            static_name: "NVIDIA_HPC_FLAGS",
            ignore_executables_name: "NVIDIA_HPC_IGNORE_EXECUTABLES",
            ignore_flags_name: "NVIDIA_HPC_IGNORE_FLAGS",
            slash_prefix_name: "NVIDIA_HPC_SLASH_PREFIX",
            env_rules_name: "NVIDIA_HPC_ENV_RULES",
            output_file: "flags_nvidia_hpc.rs",
        },
        TableConfig {
            yaml_file: "armclang.yaml",
            static_name: "ARMCLANG_FLAGS",
            ignore_executables_name: "ARMCLANG_IGNORE_EXECUTABLES",
            ignore_flags_name: "ARMCLANG_IGNORE_FLAGS",
            slash_prefix_name: "ARMCLANG_SLASH_PREFIX",
            env_rules_name: "ARMCLANG_ENV_RULES",
            output_file: "flags_armclang.rs",
        },
    ];

    pub fn generate_flag_tables() {
        let flags_dir = Path::new("interpreters");
        let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();

        // Read all YAML files first so we can resolve `extends`
        let mut raw_tables: HashMap<String, FlagTable> = HashMap::new();
        for config in TABLES {
            let yaml_path = flags_dir.join(config.yaml_file);
            println!("cargo:rerun-if-changed={}", yaml_path.display());

            let content = fs::read_to_string(&yaml_path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", yaml_path.display(), e));
            let table: FlagTable = serde_saphyr::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", yaml_path.display(), e));

            // Key by yaml filename stem (e.g., "gcc", "clang")
            let key = config.yaml_file.strip_suffix(".yaml").unwrap().to_string();
            raw_tables.insert(key, table);
        }

        // Generate recognition patterns from all tables
        generate_recognition_patterns(&raw_tables, &out_dir);

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
            fs::write(&out_path, rust_code)
                .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
        }

        // Generate a combined list of all compiler environment variable names
        generate_env_keys(&raw_tables, &out_dir);
    }

    /// Resolve `ignore_when` for a table, inheriting from base if extending.
    fn resolve_ignore_when(table: &FlagTable, raw_tables: &HashMap<String, FlagTable>) -> IgnoreWhen {
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
    fn resolve_slash_prefix(table: &FlagTable, raw_tables: &HashMap<String, FlagTable>) -> bool {
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

    /// Generate static arrays for ignore_when executables and flags.
    fn generate_ignore_arrays(config: &TableConfig, ignore_when: &IgnoreWhen) -> String {
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
        out.push_str(&format!(
            "static {}: [&str; {}] = [",
            config.ignore_flags_name,
            ignore_when.flags.len()
        ));
        for flag in &ignore_when.flags {
            out.push_str(&format!("\"{}\", ", flag));
        }
        out.push_str("];\n");

        out
    }

    /// Generate a static array of recognition pattern data from all YAML files.
    ///
    /// Produces `recognition.rs` containing `RECOGNITION_PATTERNS`, a static array of
    /// `(&str, &[&str], bool, bool)` tuples: (compiler_type, executables, cross_compilation, versioned).
    ///
    /// Executables listed in `ignore_when.executables` are automatically added as
    /// recognition entries with `(false, false)` so the recognizer can route them
    /// to the right compiler type (where the interpreter will then ignore them).
    fn generate_recognition_patterns(raw_tables: &HashMap<String, FlagTable>, out_dir: &Path) {
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
                    let names_str: Vec<String> =
                        entry.executables.iter().map(|n| format!("\"{}\"", n)).collect();
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
            // Only use the table's own list, not inherited — inherited executables are already
            // recognized under the base compiler type.
            let own_ignore = table.ignore_when.as_ref();
            if own_ignore.is_some_and(|iw| !iw.executables.is_empty()) {
                let exes = &own_ignore.unwrap().executables;
                let names_str: Vec<String> = exes.iter().map(|n| format!("\"{}\"", n)).collect();
                out.push_str(&format!(
                    "    (\"{}\", &[{}], false, false),\n",
                    type_name,
                    names_str.join(", "),
                ));
            }
        }

        out.push_str("];\n");

        let out_path = out_dir.join("recognition.rs");
        fs::write(&out_path, out).unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
    }

    /// Compute the flag name length as `FlagPattern::flag()` would return it.
    fn flag_name_len(m: &FlagMatch) -> usize {
        let pattern = &m.pattern;
        if let Some(flag) = pattern.strip_suffix("{ }*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix("{=}*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix("{:}*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix(":*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix("=*") {
            if m.count.is_some() {
                flag.len() + 1 // "=" is part of the flag name
            } else {
                flag.len()
            }
        } else if let Some(flag) = pattern.strip_suffix('*') {
            flag.len()
        } else {
            pattern.len()
        }
    }

    /// Parse a pattern string into a FlagPattern Rust expression.
    fn pattern_to_rust(pattern: &str, count: Option<u32>) -> String {
        if let Some(flag) = pattern.strip_suffix("{ }*") {
            format!("FlagPattern::ExactlyWithGluedOrSep(\"{}\")", flag)
        } else if let Some(flag) = pattern.strip_suffix("{=}*") {
            format!("FlagPattern::ExactlyWithEqOrSep(\"{}\")", flag)
        } else if let Some(flag) = pattern.strip_suffix("{:}*") {
            format!("FlagPattern::ExactlyWithColonOrSep(\"{}\")", flag)
        } else if let Some(flag) = pattern.strip_suffix(":*") {
            format!("FlagPattern::ExactlyWithColon(\"{}\")", flag)
        } else if let Some(flag) = pattern.strip_suffix("=*") {
            if let Some(n) = count {
                // "=*" with count means Prefix where "=" is part of the flag name
                format!("FlagPattern::Prefix(\"{}=\", {})", flag, n)
            } else {
                format!("FlagPattern::ExactlyWithEq(\"{}\")", flag)
            }
        } else if let Some(flag) = pattern.strip_suffix('*') {
            format!("FlagPattern::Prefix(\"{}\", {})", flag, count.unwrap_or(0))
        } else {
            format!("FlagPattern::Exactly(\"{}\", {})", pattern, count.unwrap_or(0))
        }
    }

    /// Map a result string to its Rust ArgumentKind expression.
    fn result_to_rust(result: &str) -> &'static str {
        match result {
            "output" => "ArgumentKind::Output",
            "configures_preprocessing" => {
                "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))"
            }
            "configures_compiling" => "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))",
            "configures_assembling" => {
                "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Assembling))"
            }
            "configures_linking" => "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))",
            "stops_at_preprocessing" => {
                "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing))"
            }
            "stops_at_compiling" => "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))",
            "stops_at_assembling" => "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Assembling))",
            "info_and_exit" => "ArgumentKind::Other(PassEffect::InfoAndExit)",
            "driver_option" => "ArgumentKind::Other(PassEffect::DriverOption)",
            "pass_through" => "ArgumentKind::Other(PassEffect::PassThrough)",
            "none" => "ArgumentKind::Other(PassEffect::None)",
            other => panic!("Unknown result value: '{}'", other),
        }
    }

    /// Resolve environment entries for a compiler, with transitive inheritance.
    ///
    /// Walks the `extends` chain recursively, collecting environment entries.
    /// Own entries override inherited ones matched by variable name.
    fn resolve_environment(key: &str, raw_tables: &HashMap<String, FlagTable>) -> Vec<EnvEntry> {
        let mut visited = std::collections::HashSet::new();
        resolve_environment_recursive(key, raw_tables, &mut visited)
    }

    fn resolve_environment_recursive(
        key: &str,
        raw_tables: &HashMap<String, FlagTable>,
        visited: &mut std::collections::HashSet<String>,
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
            let own_vars: std::collections::HashSet<String> =
                entries.iter().map(|e| e.variable.clone()).collect();
            for entry in base_entries {
                if !own_vars.contains(&entry.variable) {
                    entries.push(entry);
                }
            }
        }
        entries
    }

    /// Validate an environment entry at build time.
    fn validate_env_entry(entry: &EnvEntry, yaml_file: &str) {
        let var_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
        assert!(
            var_re.is_match(&entry.variable),
            "{}: invalid environment variable name: '{}'",
            yaml_file,
            entry.variable
        );

        // Validate effect is a known value
        match entry.effect.as_str() {
            "configures_preprocessing"
            | "configures_compiling"
            | "configures_assembling"
            | "configures_linking"
            | "stops_at_preprocessing"
            | "stops_at_compiling"
            | "stops_at_assembling"
            | "info_and_exit"
            | "driver_option"
            | "none" => {}
            other => panic!("{}: unknown effect value: '{}'", yaml_file, other),
        }

        // Validate mapping
        let mapping = &entry.mapping;
        if mapping.flag.is_some() && mapping.expand.is_some() {
            panic!("{}: environment entry '{}' has both 'flag' and 'expand'", yaml_file, entry.variable);
        }
        if mapping.flag.is_none() && mapping.expand.is_none() && entry.effect != "none" {
            panic!(
                "{}: environment entry '{}' has neither 'flag' nor 'expand' (and effect is not 'none')",
                yaml_file, entry.variable
            );
        }

        // Validate separator
        match mapping.separator.as_str() {
            "path" | "space" | ";" => {}
            other => panic!(
                "{}: environment entry '{}' has unknown separator: '{}'",
                yaml_file, entry.variable, other
            ),
        }

        // Validate expand position
        if let Some(ref expand) = mapping.expand {
            match expand.as_str() {
                "prepend" | "append" => {}
                other => panic!(
                    "{}: environment entry '{}' has unknown expand position: '{}'",
                    yaml_file, entry.variable, other
                ),
            }
        }
    }

    /// Map an environment entry's mapping to a Rust EnvMapping expression.
    fn env_mapping_to_rust(mapping: &EnvMappingYaml) -> String {
        if let Some(ref flag) = mapping.flag {
            let sep = match mapping.separator.as_str() {
                "path" => "EnvSeparator::Path".to_string(),
                ";" => "EnvSeparator::Fixed(\";\")".to_string(),
                other => format!("EnvSeparator::Fixed(\"{}\")", other),
            };
            format!("EnvMapping::Flag {{ flag: \"{}\", separator: {} }}", flag, sep)
        } else if let Some(ref expand) = mapping.expand {
            let pos = match expand.as_str() {
                "prepend" => "EnvPosition::Prepend",
                "append" => "EnvPosition::Append",
                _ => unreachable!(),
            };
            format!("EnvMapping::Expand {{ position: {} }}", pos)
        } else {
            unreachable!()
        }
    }

    /// Generate a static array of EnvRule for a compiler.
    fn generate_env_array(config: &TableConfig, entries: &[EnvEntry]) -> String {
        // Filter out effect: none entries (documentary only)
        let active: Vec<&EnvEntry> = entries.iter().filter(|e| e.effect != "none").collect();

        for entry in &active {
            validate_env_entry(entry, config.yaml_file);
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

    /// Generate a static array of all compiler environment variable names.
    ///
    /// Used by `environment.rs` to replace the hardcoded GCC include key set.
    fn generate_env_keys(raw_tables: &HashMap<String, FlagTable>, out_dir: &Path) {
        let mut all_vars = std::collections::BTreeSet::new();

        for config in TABLES {
            let key = config.yaml_file.strip_suffix(".yaml").unwrap();
            let entries = resolve_environment(key, raw_tables);
            for entry in &entries {
                if entry.effect != "none" {
                    all_vars.insert(entry.variable.clone());
                }
            }
        }

        let mut out = String::new();
        out.push_str("// Generated from interpreters/*.yaml -- DO NOT EDIT\n");
        out.push_str(&format!("static COMPILER_ENV_KEYS: [&str; {}] = [\n", all_vars.len()));
        for var in &all_vars {
            out.push_str(&format!("    \"{}\",\n", var));
        }
        out.push_str("];\n");

        let out_path = out_dir.join("env_keys.rs");
        fs::write(&out_path, &out)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
    }

    /// Generate a Rust source file containing a static array of FlagRule.
    fn generate_static_array(config: &TableConfig, entries: &[FlagEntry]) -> String {
        let mut out = String::new();
        out.push_str(&format!("// Generated from interpreters/{} -- DO NOT EDIT\n", config.yaml_file));
        out.push_str(&format!("static {}: [FlagRule; {}] = [\n", config.static_name, entries.len()));

        for entry in entries {
            let pattern_rust = pattern_to_rust(&entry.match_.pattern, entry.match_.count);
            let result_rust = result_to_rust(&entry.result);
            out.push_str(&format!("    FlagRule::new({}, {}),\n", pattern_rust, result_rust));
        }

        out.push_str("];\n");
        out
    }
}
