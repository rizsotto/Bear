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
/// Reads YAML flag definition files from `flags/` and generates Rust source files
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
        flags: Vec<FlagEntry>,
    }

    #[derive(Deserialize, Clone)]
    struct RecognizeEntry {
        names: Vec<String>,
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

    /// Table metadata: name of the static, visibility, which file to generate.
    struct TableConfig {
        yaml_file: &'static str,
        static_name: &'static str,
        ignore_executables_name: &'static str,
        ignore_flags_name: &'static str,
        visibility: &'static str, // "pub " or ""
        output_file: &'static str,
    }

    const TABLES: &[TableConfig] = &[
        TableConfig {
            yaml_file: "gcc.yaml",
            static_name: "GCC_FLAGS",
            ignore_executables_name: "GCC_IGNORE_EXECUTABLES",
            ignore_flags_name: "GCC_IGNORE_FLAGS",
            visibility: "pub ",
            output_file: "flags_gcc.rs",
        },
        TableConfig {
            yaml_file: "clang.yaml",
            static_name: "CLANG_FLAGS",
            ignore_executables_name: "CLANG_IGNORE_EXECUTABLES",
            ignore_flags_name: "CLANG_IGNORE_FLAGS",
            visibility: "pub ",
            output_file: "flags_clang.rs",
        },
        TableConfig {
            yaml_file: "flang.yaml",
            static_name: "FLANG_FLAGS",
            ignore_executables_name: "FLANG_IGNORE_EXECUTABLES",
            ignore_flags_name: "FLANG_IGNORE_FLAGS",
            visibility: "pub ",
            output_file: "flags_flang.rs",
        },
        TableConfig {
            yaml_file: "cuda.yaml",
            static_name: "CUDA_FLAGS",
            ignore_executables_name: "CUDA_IGNORE_EXECUTABLES",
            ignore_flags_name: "CUDA_IGNORE_FLAGS",
            visibility: "pub ",
            output_file: "flags_cuda.rs",
        },
        TableConfig {
            yaml_file: "intel_fortran.yaml",
            static_name: "INTEL_FORTRAN_FLAGS",
            ignore_executables_name: "INTEL_FORTRAN_IGNORE_EXECUTABLES",
            ignore_flags_name: "INTEL_FORTRAN_IGNORE_FLAGS",
            visibility: "pub ",
            output_file: "flags_intel_fortran.rs",
        },
        TableConfig {
            yaml_file: "cray_fortran.yaml",
            static_name: "CRAY_FORTRAN_FLAGS",
            ignore_executables_name: "CRAY_FORTRAN_IGNORE_EXECUTABLES",
            ignore_flags_name: "CRAY_FORTRAN_IGNORE_FLAGS",
            visibility: "pub ",
            output_file: "flags_cray_fortran.rs",
        },
    ];

    pub fn generate_flag_tables() {
        let flags_dir = Path::new("flags");
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

            // Resolve ignore_when (own + base)
            let ignore_when = resolve_ignore_when(table, &raw_tables);

            // Generate Rust source
            let mut rust_code = generate_static_array(config, &entries);
            rust_code.push_str(&generate_ignore_arrays(config, &ignore_when));
            let out_path = out_dir.join(config.output_file);
            fs::write(&out_path, rust_code)
                .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
        }
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

    /// Generate static arrays for ignore_when executables and flags.
    fn generate_ignore_arrays(config: &TableConfig, ignore_when: &IgnoreWhen) -> String {
        let mut out = String::new();

        // Generate ignore executables array
        out.push_str(&format!(
            "{}static {}: [&str; {}] = [",
            config.visibility,
            config.ignore_executables_name,
            ignore_when.executables.len()
        ));
        for exe in &ignore_when.executables {
            out.push_str(&format!("\"{}\", ", exe));
        }
        out.push_str("];\n");

        // Generate ignore flags array
        out.push_str(&format!(
            "{}static {}: [&str; {}] = [",
            config.visibility,
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
    /// `(&str, &[&str], bool, bool)` tuples: (compiler_type, names, cross_compilation, versioned).
    fn generate_recognition_patterns(raw_tables: &HashMap<String, FlagTable>, out_dir: &Path) {
        let mut out = String::new();
        out.push_str("// Generated from flags/*.yaml -- DO NOT EDIT\n");
        out.push_str(
            "pub static RECOGNITION_PATTERNS: &[(&str, &[&str], bool, bool)] = &[\n",
        );

        // Collect entries in a deterministic order (by TABLES order)
        for config in TABLES {
            let key = config.yaml_file.strip_suffix(".yaml").unwrap();
            let table = &raw_tables[key];

            let Some(ref type_name) = table.type_ else {
                continue;
            };
            let Some(ref recognize_entries) = table.recognize else {
                continue;
            };

            for entry in recognize_entries {
                let names_str: Vec<String> =
                    entry.names.iter().map(|n| format!("\"{}\"", n)).collect();
                out.push_str(&format!(
                    "    (\"{}\", &[{}], {}, {}),\n",
                    type_name,
                    names_str.join(", "),
                    entry.cross_compilation,
                    entry.versioned,
                ));
            }
        }

        out.push_str("];\n");

        let out_path = out_dir.join("recognition.rs");
        fs::write(&out_path, out)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
    }

    /// Compute the flag name length as `FlagPattern::flag()` would return it.
    fn flag_name_len(m: &FlagMatch) -> usize {
        let pattern = &m.pattern;
        if let Some(flag) = pattern.strip_suffix("{ }*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix("{=}*") {
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
            "none" => "ArgumentKind::Other(PassEffect::None)",
            other => panic!("Unknown result value: '{}'", other),
        }
    }

    /// Generate a Rust source file containing a static array of FlagRule.
    fn generate_static_array(config: &TableConfig, entries: &[FlagEntry]) -> String {
        let mut out = String::new();
        out.push_str(&format!("// Generated from flags/{} -- DO NOT EDIT\n", config.yaml_file));
        out.push_str(&format!(
            "{}static {}: [FlagRule; {}] = [\n",
            config.visibility,
            config.static_name,
            entries.len()
        ));

        for entry in entries {
            let pattern_rust = pattern_to_rust(&entry.match_.pattern, entry.match_.count);
            let result_rust = result_to_rust(&entry.result);
            out.push_str(&format!("    FlagRule::new({}, {}),\n", pattern_rust, result_rust));
        }

        out.push_str("];\n");
        out
    }
}
