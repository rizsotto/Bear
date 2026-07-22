// SPDX-License-Identifier: GPL-3.0-or-later

pub mod codegen;
pub mod env_keys;
pub mod families;
pub mod recognition;
pub mod resolve;
pub mod tables;
pub mod wrappers;
pub mod yaml_types;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

use codegen::{pattern_to_rust, result_to_rust};
use env_keys::generate_env_keys;
use families::generate_families;
use recognition::{generate_compiler_ids, generate_recognition_patterns};
use resolve::{resolve_environment, resolve_flags, resolve_ignore_when, resolve_slash_prefix};
use tables::{CompilerFile, TableNames};
use wrappers::generate_wrappers;
use yaml_types::{EnvEntry, FlagEntry, FlagTable, IgnoreWhen, Kind, WrapperTable};

/// A compiler flag table with all inheritance resolved and ready for code generation.
pub struct ResolvedTable {
    pub names: TableNames,
    pub flags: Vec<FlagEntry>,
    pub ignore_when: IgnoreWhen,
    pub slash_prefix: bool,
    pub env_entries: Vec<EnvEntry>,
}

impl ResolvedTable {
    /// Resolve a single compiler table by merging inherited flags, ignore_when,
    /// slash_prefix, and environment entries from the extends chain. Flags are
    /// resolved by the file's `compiler.id`; the generated names come from its
    /// stem.
    pub fn new(compiler_file: &CompilerFile, raw_tables: &HashMap<String, FlagTable>) -> Result<Self> {
        let key = compiler_file.id.as_str();
        if !raw_tables.contains_key(key) {
            bail!("no table found for '{}'", key);
        }

        let mut flags = resolve_flags(key, raw_tables)
            .with_context(|| format!("resolving flags for {}", compiler_file.yaml_file()))?;
        flags.sort_by_key(|b| std::cmp::Reverse(b.match_.name_len()));

        Ok(ResolvedTable {
            names: compiler_file.names(),
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
                .with_context(|| format!("generating flags for {}", self.names.yaml_file))?,
        );
        out.push_str(&self.generate_ignore_arrays());
        out.push_str(&format!("static {}: bool = {};\n", self.names.slash_prefix_name, self.slash_prefix));
        out.push_str(
            &self
                .generate_env_array()
                .with_context(|| format!("generating env rules for {}", self.names.yaml_file))?,
        );
        Ok(out)
    }

    fn generate_flag_array(&self) -> Result<String> {
        let mut out = String::new();
        out.push_str(&format!("// Generated from compilers/{} -- DO NOT EDIT\n", self.names.yaml_file));
        out.push_str(&format!("static {}: [FlagRule; {}] = [\n", self.names.static_name, self.flags.len()));
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
            self.names.ignore_executables_name,
            self.ignore_when.executables.len()
        ));
        for exe in &self.ignore_when.executables {
            out.push_str(&format!("\"{}\", ", exe));
        }
        out.push_str("];\n");

        out.push_str(&format!(
            "static {}: [&str; {}] = [",
            self.names.ignore_flags_name,
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
            entry.validate().with_context(|| format!("environment entry in {}", self.names.yaml_file))?;
        }

        let mut out = String::new();
        out.push_str(&format!("static {}: [EnvRule; {}] = [\n", self.names.env_rules_name, active.len()));
        for entry in &active {
            let mapping_rust = entry
                .mapping
                .to_rust()
                .with_context(|| format!("variable '{}' in {}", entry.variable, self.names.yaml_file))?;
            let effect_rust = result_to_rust(&entry.effect)
                .with_context(|| format!("variable '{}' in {}", entry.variable, self.names.yaml_file))?;
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
    // A per-file rerun-if-changed (printed by load_and_index/load_wrapper_tables
    // below) only ever covers files that already existed at the previous build.
    // Wrapper files need no other Rust-side registration to be picked up, so
    // adding a brand-new one is exactly the case a per-file-only watch misses:
    // cargo has no reason to rerun this build script, and the new launcher goes
    // unrecognized until something else forces a rebuild. Watch the whole
    // directory so a new (or removed) file is never silently missed.
    println!("cargo:rerun-if-changed={}", flags_dir.display());

    let (raw_tables, compiler_files) = load_tables_from(flags_dir)?;

    // Load compiler-launcher (wrapper) tables first: both the recognition
    // patterns and the wrapper tables output need the same loaded value.
    let wrapper_tables = load_wrapper_tables(flags_dir, true)?;

    // Generate recognition patterns (compiler rows in discovery order --
    // specializations before their base -- then wrapper rows)
    let recognition = generate_recognition_patterns(&raw_tables, &compiler_files, &wrapper_tables)?;
    write_output(out_dir, "recognition.rs", recognition)?;

    // Generate the compiler-id data the config module's `as:` deserializer
    // validates against (KNOWN_IDS plus the wrapper launcher basenames).
    let compiler_ids = generate_compiler_ids(&compiler_files, &wrapper_tables)?;
    write_output(out_dir, "compiler_ids.rs", compiler_ids)?;

    // Generate each compiler's flag table
    for compiler_file in &compiler_files {
        let resolved = ResolvedTable::new(compiler_file, &raw_tables)?;
        write_output(out_dir, &compiler_file.names().output_file, resolved.generate()?)?;
    }

    // Generate the family registry the semantic layer loops over to register
    // one interpreter per family (includes the flag tables above).
    let families = generate_families(&compiler_files, &raw_tables);
    write_output(out_dir, "families.rs", families)?;

    // Generate combined environment variable keys
    let env_keys = generate_env_keys(&raw_tables);
    write_output(out_dir, "env_keys.rs", env_keys)?;

    // Generate compiler-launcher (wrapper) tables: ccache, distcc, ...
    let wrappers = generate_wrappers(&wrapper_tables);
    write_output(out_dir, "wrappers.rs", wrappers)?;

    Ok(())
}

/// Generate only the combined environment variable keys.
///
/// Used by the `intercept` crate, which needs `COMPILER_ENV_KEYS` for its
/// agent-side environment filtering but none of the flag tables or recognition
/// patterns. Reads `*.yaml` from `flags_dir` and writes `env_keys.rs` into
/// `out_dir`.
///
/// Prints `cargo:rerun-if-changed` lines to stdout (for build.rs integration).
pub fn generate_env_keys_only(flags_dir: &Path, out_dir: &Path) -> Result<()> {
    let (raw_tables, _compiler_files) = load_tables_from(flags_dir)?;
    let env_keys = generate_env_keys(&raw_tables);
    write_output(out_dir, "env_keys.rs", env_keys)?;
    Ok(())
}

/// Parse a single YAML table, attaching the source file name to any parse
/// error so a schema violation names the offending file.
pub fn parse_table(yaml_file: &str, content: &str) -> Result<FlagTable> {
    serde_saphyr::from_str(content).with_context(|| format!("parsing {}", yaml_file))
}

/// Parse a single wrapper table, attaching the source file name to any
/// parse error so a schema violation names the offending file.
pub fn parse_wrapper_table(yaml_file: &str, content: &str) -> Result<WrapperTable> {
    serde_saphyr::from_str(content).with_context(|| format!("parsing {}", yaml_file))
}

/// Insert a parsed table into an id-keyed map, keyed by `table.compiler.id`.
///
/// Bails with a clear error naming both the id and the file that redefines
/// it if the id collides with a table already present. Returns the id on
/// success, so callers that also need a `yaml_file -> id` side index don't
/// have to clone it themselves.
pub fn insert_by_id(
    raw_tables: &mut HashMap<String, FlagTable>,
    yaml_file: &str,
    table: FlagTable,
) -> Result<String> {
    let id = table.compiler.id.clone();
    if raw_tables.contains_key(&id) {
        bail!("duplicate compiler id '{}': {} redefines an id already used by another table", id, yaml_file);
    }
    raw_tables.insert(id.clone(), table);
    Ok(id)
}

/// Tables keyed by `compiler.id`, plus the discovered compiler files in
/// generated-output order (`(extends depth desc, stem)`). Callers that need output
/// order (recognition rows, flag files) iterate the `Vec`; callers that only
/// need a table by id use the map.
type LoadedTables = (HashMap<String, FlagTable>, Vec<CompilerFile>);

/// True if `s` is a valid Rust identifier fragment: non-empty, first char an
/// ASCII letter or `_`, the rest ASCII alphanumeric or `_`.
fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Discover and load every compiler-kind YAML file in `flags_dir`, keyed by
/// id, validated, and ordered for recognition.
///
/// Files are found by scanning the directory and peeking each file's `type:`
/// kind; wrapper-kind files are skipped here (they are loaded separately by
/// [`load_wrapper_tables`]). There is no hand-maintained file list: a new
/// compiler is picked up by the scan alone.
///
/// Order is `(extends depth descending, stem ascending)`: a table that
/// `extends` another is a specialization of it (its executable names may be
/// caught by the base's broadened cross-compilation pattern), so it must be
/// recognized first. Depth encodes exactly that -- a child's depth is always
/// greater than its parent's -- so the "recognize specific before general"
/// invariant falls out of the `extends` graph with no per-file annotation.
///
/// `print_rerun` controls whether `cargo:rerun-if-changed` lines are
/// printed (only meaningful when called from a `build.rs`).
fn load_and_index(flags_dir: &Path, print_rerun: bool) -> Result<LoadedTables> {
    let mut yaml_files: Vec<String> = std::fs::read_dir(flags_dir)
        .with_context(|| format!("reading directory {}", flags_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".yaml"))
        .collect();
    yaml_files.sort();

    let mut raw_tables = HashMap::new();
    let mut compiler_files = Vec::new();
    for yaml_file in yaml_files {
        let yaml_path = flags_dir.join(&yaml_file);
        if print_rerun {
            println!("cargo:rerun-if-changed={}", yaml_path.display());
        }

        let content = std::fs::read_to_string(&yaml_path)
            .with_context(|| format!("reading {}", yaml_path.display()))?;
        let peek: KindPeek =
            serde_saphyr::from_str(&content).with_context(|| format!("parsing {}", yaml_file))?;
        // Wrapper-kind files are loaded by load_wrapper_tables, not here.
        if peek.type_ != Kind::Compiler {
            continue;
        }

        let stem = yaml_file.strip_suffix(".yaml").expect("filtered to .yaml above").to_string();
        // The stem becomes an identifier fragment in generated names
        // (`gcc` -> `GCC_FLAGS`), so it must be a valid Rust identifier.
        // Catch a bad file name here with a clear message instead of a
        // cryptic rustc error in the consuming crate.
        if !is_ident(&stem) {
            bail!(
                "compiler file '{}' has an invalid stem '{}': the stem becomes part of a generated \
                 identifier, so it must be ASCII letters, digits, and underscores, not starting with a digit",
                yaml_file,
                stem
            );
        }
        let table = parse_table(&yaml_file, &content)?;
        let id = insert_by_id(&mut raw_tables, &yaml_file, table)?;
        // Depth is filled in below, once every table is loaded and extends
        // targets can be resolved.
        compiler_files.push(CompilerFile { stem, id, depth: 0 });
    }

    validate_extends(&raw_tables, &compiler_files)?;

    // Recognition order: specializations (higher extends depth) first, then
    // stem. See the doc comment above; this replaces the old hand-ordered
    // TABLES array with an order derived from the extends graph.
    for compiler_file in &mut compiler_files {
        compiler_file.depth = extends_depth(&raw_tables, &compiler_file.id)?;
    }
    compiler_files.sort_by(|a, b| b.depth.cmp(&a.depth).then_with(|| a.stem.cmp(&b.stem)));

    Ok((raw_tables, compiler_files))
}

/// The `extends` depth of `id`: 0 for a root, one more than its parent
/// otherwise. Walks the extends chain, which `validate_extends` has already
/// proven leads only to existing tables; bails on a cycle so codegen (not
/// just a test) rejects one.
fn extends_depth(raw_tables: &HashMap<String, FlagTable>, id: &str) -> Result<usize> {
    let mut seen: Vec<&str> = Vec::new();
    let mut cursor = id;
    while let Some(parent) = raw_tables
        .get(cursor)
        .expect("extends targets exist after validate_extends")
        .compiler
        .extends
        .as_deref()
    {
        if seen.contains(&cursor) {
            bail!("extends cycle detected at '{}'", cursor);
        }
        seen.push(cursor);
        cursor = parent;
    }
    Ok(seen.len())
}

/// Validate that every table's `compiler.extends` (if set) names an id
/// that was actually loaded. Bails with a message naming the offending
/// file so a dangling `extends:` target fails codegen, not just a test.
pub fn validate_extends(
    raw_tables: &HashMap<String, FlagTable>,
    compiler_files: &[CompilerFile],
) -> Result<()> {
    for compiler_file in compiler_files {
        let table = &raw_tables[&compiler_file.id];
        if let Some(ref base) = table.compiler.extends
            && !raw_tables.contains_key(base.as_str())
        {
            bail!("{} extends '{}', which does not exist", compiler_file.yaml_file(), base);
        }
    }
    Ok(())
}

/// Load YAML flag tables from a directory, printing cargo:rerun-if-changed.
fn load_tables_from(flags_dir: &Path) -> Result<LoadedTables> {
    load_and_index(flags_dir, true)
}

/// Just enough of a YAML file's shape to decide which full schema to parse
/// it as. Serde ignores unknown fields by default, so this deserializes
/// correctly regardless of the rest of the file.
#[derive(serde::Deserialize)]
struct KindPeek {
    #[serde(rename = "type")]
    type_: Kind,
}

/// Load every `type: wrapper` YAML file in `flags_dir`, sorted by file name
/// for deterministic generated output.
///
/// Like [`load_and_index`], this scans the directory and peeks each file's
/// kind; it collects the wrapper-kind files and skips the compiler-kind ones
/// (which `load_and_index` loads). Kind is declared data, so nothing needs a
/// hand-maintained list to tell the two apart.
///
/// `print_rerun` controls whether `cargo:rerun-if-changed` lines are
/// printed (only meaningful when called from a `build.rs`).
pub fn load_wrapper_tables(flags_dir: &Path, print_rerun: bool) -> Result<Vec<(String, WrapperTable)>> {
    let mut yaml_files: Vec<String> = std::fs::read_dir(flags_dir)
        .with_context(|| format!("reading directory {}", flags_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".yaml"))
        .collect();
    yaml_files.sort();

    let mut wrapper_tables = Vec::new();
    for yaml_file in yaml_files {
        let yaml_path = flags_dir.join(&yaml_file);
        if print_rerun {
            println!("cargo:rerun-if-changed={}", yaml_path.display());
        }

        let content = std::fs::read_to_string(&yaml_path)
            .with_context(|| format!("reading {}", yaml_path.display()))?;
        let peek: KindPeek =
            serde_saphyr::from_str(&content).with_context(|| format!("parsing {}", yaml_file))?;

        match peek.type_ {
            Kind::Wrapper => {
                let table = parse_wrapper_table(&yaml_file, &content)?;
                table.validate(&yaml_file)?;
                wrapper_tables.push((yaml_file, table));
            }
            // Compiler-kind files are loaded by load_and_index.
            Kind::Compiler => continue,
        }
    }
    Ok(wrapper_tables)
}

fn write_output(out_dir: &Path, filename: &str, content: String) -> Result<()> {
    let out_path = out_dir.join(filename);
    std::fs::write(&out_path, content).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Path to the YAML flag definitions in the workspace.
///
/// `CARGO_MANIFEST_DIR` is `<root>/build-support/compilers-codegen`; the YAML
/// lives at `<root>/crates/bear/compilers`, so walk up two levels to the
/// workspace root.
pub(crate) fn flags_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .ancestors()
        .nth(2)
        .expect("CARGO_MANIFEST_DIR is build-support/compilers-codegen, two levels below the workspace root");
    workspace_root.join("crates/bear/compilers")
}

/// Load all YAML flag tables from the workspace compilers directory, keyed
/// by `compiler.id`.
pub fn load_tables() -> Result<HashMap<String, FlagTable>> {
    let (raw_tables, _compiler_files) = load_and_index(&flags_dir(), false)?;
    Ok(raw_tables)
}

/// Discover the compiler-kind files in the workspace compilers directory, in
/// generated-output order (`(extends depth desc, stem)`). Used by tests and codegen
/// stages that need the ordered file list without the full flag tables.
pub fn load_compiler_files() -> Result<Vec<CompilerFile>> {
    let (_raw_tables, compiler_files) = load_and_index(&flags_dir(), false)?;
    Ok(compiler_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::pattern_to_rust;
    use crate::yaml_types::{
        CompilerIdentity, EnvEntry, EnvMapping, FlagMatch, Kind, RecognizeEntry, SourceMode, Syntax,
    };

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
                type_: Kind::Compiler,
                compiler: CompilerIdentity { id: "a".to_string(), extends: Some("b".to_string()) },
                recognize: None,
                ignore_when: None,
                slash_prefix: None,
                flags: vec![],
                environment: Some(vec![make_test_env_entry("VAR_A")]),
                source_mode: SourceMode::default(),
                response_file_syntax: Syntax::default(),
            },
        );
        tables.insert(
            "b".to_string(),
            FlagTable {
                type_: Kind::Compiler,
                compiler: CompilerIdentity { id: "b".to_string(), extends: Some("a".to_string()) },
                recognize: None,
                ignore_when: None,
                slash_prefix: None,
                flags: vec![],
                environment: Some(vec![make_test_env_entry("VAR_B")]),
                source_mode: SourceMode::default(),
                response_file_syntax: Syntax::default(),
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
        parent.compiler.extends = Some("gp".to_string());
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.compiler.extends = Some("parent".to_string());
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
        child.compiler.extends = Some("base".to_string());
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
        parent.compiler.extends = Some("gp".to_string());
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.compiler.extends = Some("parent".to_string());
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
        parent.compiler.extends = Some("gp".to_string());
        parent.flags = vec![make_test_flag("-p", "output")];
        tables.insert("parent".to_string(), parent);
        let mut child = make_empty_table();
        child.compiler.extends = Some("parent".to_string());
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
        child.compiler.extends = Some("parent".to_string());
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
        child.compiler.extends = Some("parent".to_string());
        child.flags = vec![make_test_flag("-c", "output")];
        tables.insert("child".to_string(), child);
        let err = resolve_flags("child", &tables).unwrap_err();
        assert!(err.to_string().contains("conflicting"), "{}", err);
    }

    #[test]
    fn resolve_flags_real_no_conflicts() {
        let raw_tables = load_tables().unwrap();
        for compiler_file in load_compiler_files().unwrap() {
            resolve_flags(&compiler_file.id, &raw_tables).unwrap();
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

    // -- RecognizeEntry::validate tests --

    #[test]
    fn validate_recognize_entry_valid() {
        let e = RecognizeEntry {
            executables: vec!["gcc".to_string()],
            cross_compilation: true,
            versioned: true,
            description: "GCC compiler".to_string(),
            references: vec!["https://gcc.gnu.org/".to_string()],
        };
        assert!(e.validate().is_ok());
    }

    #[test]
    fn validate_recognize_entry_empty_description() {
        let e = RecognizeEntry {
            executables: vec!["gcc".to_string()],
            cross_compilation: false,
            versioned: false,
            description: "   ".to_string(),
            references: vec!["https://x".to_string()],
        };
        assert!(e.validate().unwrap_err().to_string().contains("description must not be empty"));
    }

    #[test]
    fn validate_recognize_entry_empty_reference() {
        let e = RecognizeEntry {
            executables: vec!["gcc".to_string()],
            cross_compilation: false,
            versioned: false,
            description: "GCC compiler".to_string(),
            references: vec![],
        };
        assert!(e.validate().unwrap_err().to_string().contains("references must list"));
    }

    #[test]
    fn validate_recognize_entry_bad_url() {
        let e = RecognizeEntry {
            executables: vec!["gcc".to_string()],
            cross_compilation: false,
            versioned: false,
            description: "GCC compiler".to_string(),
            references: vec!["gcc.gnu.org".to_string()],
        };
        assert!(e.validate().unwrap_err().to_string().contains("http(s) URL"));
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

    // -- EnvMapping::to_rust tests --

    #[test]
    fn env_mapping_to_rust_no_flag_no_expand_is_err() {
        let mapping = EnvMapping { flag: None, expand: None, separator: "path".to_string() };
        let err = mapping.to_rust().unwrap_err();
        assert!(err.to_string().contains("neither 'flag' nor 'expand'"), "{}", err);
    }

    #[test]
    fn env_mapping_to_rust_unknown_expand_is_err() {
        let mapping =
            EnvMapping { flag: None, expand: Some("middle".to_string()), separator: "path".to_string() };
        let err = mapping.to_rust().unwrap_err();
        assert!(err.to_string().contains("unknown expand position"), "{}", err);
    }

    // -- Integration test --

    #[test]
    fn generate_from_real_yaml() {
        let out_dir = tempfile::tempdir().unwrap();
        generate(&flags_dir(), out_dir.path()).unwrap();

        for compiler_file in load_compiler_files().unwrap() {
            let names = compiler_file.names();
            let path = out_dir.path().join(&names.output_file);
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Missing output file: {}", names.output_file));
            assert!(!content.is_empty());
            assert!(content.contains(&names.static_name));
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
        let wrappers_rs = std::fs::read_to_string(out_dir.path().join("wrappers.rs")).unwrap();
        assert!(wrappers_rs.contains("WRAPPER_NAMES"));
        assert!(wrappers_rs.contains("WRAPPER_OPTIONS"));
        assert!(wrappers_rs.contains("\"ccache\""));
        assert!(wrappers_rs.contains("\"distcc\""));
        let compiler_ids_rs = std::fs::read_to_string(out_dir.path().join("compiler_ids.rs")).unwrap();
        assert!(compiler_ids_rs.contains("KNOWN_IDS"));
        assert!(compiler_ids_rs.contains("WRAPPER_AS_NAMES"));
        assert!(compiler_ids_rs.contains("\"gcc\""));
        assert!(compiler_ids_rs.contains("\"ccache\""));
    }

    #[test]
    fn generate_compiler_ids_lists_every_family_and_launcher() {
        let compiler_files = load_compiler_files().unwrap();
        let wrapper_tables = load_wrapper_tables(&flags_dir(), false).unwrap();

        let sut = generate_compiler_ids(&compiler_files, &wrapper_tables).unwrap();

        // Every discovered compiler id appears; the four launcher basenames
        // appear as wrapper as-names; "wrapper" is the kind, never an id.
        for compiler_file in &compiler_files {
            assert!(sut.contains(&format!("\"{}\"", compiler_file.id)), "missing id {}", compiler_file.id);
        }
        for launcher in ["ccache", "distcc", "icecc", "sccache"] {
            assert!(sut.contains(&format!("\"{}\"", launcher)), "missing launcher {launcher}");
        }
        assert!(sut.contains("KNOWN_IDS"));
        assert!(sut.contains("WRAPPER_AS_NAMES"));
    }

    #[test]
    fn discovery_orders_specializations_before_the_base() {
        // A family that `extends` another is recognized before it: ibm_xl and
        // clang_cl extend clang/msvc, so they precede their bases, and clang
        // (extends gcc) precedes gcc. The real constraint this protects is
        // ibm_xl before clang -- "ibm-clang" matches clang's cross-compilation
        // pattern, so clang must not be tried first.
        let compiler_files = load_compiler_files().unwrap();
        let position = |id: &str| compiler_files.iter().position(|c| c.id == id).unwrap();

        assert!(position("ibm_xl") < position("clang"), "ibm_xl must precede clang");
        assert!(position("clang_cl") < position("msvc"), "clang_cl must precede msvc");
        assert!(position("clang") < position("gcc"), "clang must precede gcc");
    }

    #[test]
    fn extends_depth_of_a_chain_counts_hops() {
        let raw_tables = load_tables().unwrap();

        // gcc is a root (0); clang extends gcc (1); ibm_xl extends clang (2).
        assert_eq!(extends_depth(&raw_tables, "gcc").unwrap(), 0);
        assert_eq!(extends_depth(&raw_tables, "clang").unwrap(), 1);
        assert_eq!(extends_depth(&raw_tables, "ibm_xl").unwrap(), 2);
    }

    #[test]
    fn extends_depth_bails_on_a_cycle() {
        let mut tables = HashMap::new();
        for (id, parent) in [("a", "b"), ("b", "a")] {
            tables.insert(
                id.to_string(),
                FlagTable {
                    type_: Kind::Compiler,
                    compiler: CompilerIdentity { id: id.to_string(), extends: Some(parent.to_string()) },
                    recognize: None,
                    ignore_when: None,
                    slash_prefix: None,
                    flags: vec![],
                    environment: None,
                    source_mode: SourceMode::default(),
                    response_file_syntax: Syntax::default(),
                },
            );
        }

        let err = extends_depth(&tables, "a").unwrap_err();

        assert!(err.to_string().contains("cycle"), "{}", err);
    }

    #[test]
    fn invalid_stem_fails_codegen() {
        // A file whose stem is not a valid identifier fails codegen with a
        // clear message, not a cryptic rustc error in the consuming crate.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("my-compiler.yaml"),
            "type: compiler\ncompiler:\n  id: mycompiler\nflags: []\n",
        )
        .unwrap();

        let err = load_and_index(dir.path(), false).err().unwrap();

        assert!(err.to_string().contains("my-compiler"), "{}", err);
        assert!(err.to_string().contains("invalid stem"), "{}", err);
    }

    #[test]
    fn compiler_id_reserved_wrapper_fails() {
        let compiler_files = vec![CompilerFile { stem: "wrapper".into(), id: "wrapper".into(), depth: 0 }];

        let err = generate_compiler_ids(&compiler_files, &[]).unwrap_err();

        assert!(err.to_string().contains("reserved"), "{}", err);
    }

    #[test]
    fn compiler_id_colliding_with_a_launcher_fails() {
        let compiler_files = vec![CompilerFile { stem: "ccache".into(), id: "ccache".into(), depth: 0 }];
        let wrapper_tables = vec![(
            "ccache.yaml".to_string(),
            WrapperTable {
                type_: Kind::Wrapper,
                recognize: vec![RecognizeEntry {
                    description: "Compiler cache".into(),
                    references: vec!["https://ccache.dev/".into()],
                    executables: vec!["ccache".into()],
                    versioned: false,
                    cross_compilation: false,
                }],
                options: vec![],
            },
        )];

        let err = generate_compiler_ids(&compiler_files, &wrapper_tables).unwrap_err();

        assert!(err.to_string().contains("ccache"), "{}", err);
        assert!(err.to_string().contains("collides"), "{}", err);
    }

    // -- load_wrapper_tables tests --

    #[test]
    fn load_wrapper_tables_finds_the_four_real_launcher_files() {
        let wrapper_tables = load_wrapper_tables(&flags_dir(), false).unwrap();
        let mut names: Vec<&str> = wrapper_tables.iter().map(|(f, _)| f.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["ccache.yaml", "distcc.yaml", "icecc.yaml", "sccache.yaml"]);
    }

    #[test]
    fn load_wrapper_tables_skips_compiler_kind_files() {
        // A compiler-kind file is discovered by load_and_index, not the
        // wrapper pass; the wrapper pass simply skips it (no error).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("stray.yaml"), "type: compiler\ncompiler:\n  id: stray\nflags: []\n")
            .unwrap();

        let wrapper_tables = load_wrapper_tables(dir.path(), false).unwrap();

        assert!(
            wrapper_tables.is_empty(),
            "compiler-kind file should be skipped, got {} rows",
            wrapper_tables.len()
        );
    }

    // -- helpers --

    fn make_empty_table() -> FlagTable {
        FlagTable {
            type_: Kind::Compiler,
            compiler: CompilerIdentity { id: "test".to_string(), extends: None },
            recognize: None,
            ignore_when: None,
            slash_prefix: None,
            flags: vec![],
            environment: None,
            source_mode: SourceMode::default(),
            response_file_syntax: Syntax::default(),
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
            mapping: EnvMapping { flag: Some("-I".to_string()), expand: None, separator: "path".to_string() },
            note: None,
        }
    }
}
