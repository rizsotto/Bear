// SPDX-License-Identifier: GPL-3.0-or-later

//! Generic flag-based compiler interpreter.
//!
//! This module provides a single interpreter type that handles all flag-table-driven
//! compilers (GCC, Clang, Flang, CUDA, Intel Fortran, Cray Fortran). Each compiler
//! is parameterized by its generated flag table and optional ignore filters, eliminating
//! the need for per-compiler structs and trait implementations.

use super::super::matchers::source::file_type_of;
use super::super::matchers::{
    EnvMapping, EnvPosition, EnvRule, EnvSeparator, FlagAnalyzer, FlagPattern, FlagRule, RuleKind,
};
use super::response_file::{Syntax, expand, tokenize_msvc};
use crate::interpretation::Provenance;
use crate::{
    Arguments, CompilerPass, Execution, FileRef, Interpretation, OutputSpelling, PassEffect, RecognizeResult,
    Recognized, Role, SourceMode,
};
use std::path::{Path, PathBuf};

/// A generic compiler interpreter parameterized by a flag table and ignore filters.
///
/// This replaces the individual per-compiler interpreter structs (GccInterpreter,
/// ClangInterpreter, etc.) with a single type driven by build-time-generated data.
pub(super) struct FlagBasedInterpreter {
    /// The family id this interpreter was built for.
    id: &'static str,
    analyzer: FlagAnalyzer,
    ignore_executables: &'static [&'static str],
    ignore_flags: &'static [&'static str],
    /// When true, arguments starting with '/' are treated as flags (MSVC-style).
    /// When false (default), only '-' prefixed arguments are treated as flags.
    slash_prefix: bool,
    env_rules: &'static [EnvRule],
    /// When true, environment variables in `env_rules` are folded into the
    /// recognized arguments (`format.arguments.from_environment`).
    from_environment: bool,
    /// How `env_rules` values that expand to words are split.
    syntax: Syntax,
    /// How this family's invocations map to compilation database entries.
    /// See [`SourceMode`]. Comes from the family's YAML `source_mode:` (via the
    /// generated [`FamilyDef`]); it is a per-family datum, consumed at the
    /// converter (post-parse).
    source_mode: SourceMode,
}

/// One call handed to a family interpreter.
pub(super) struct Call {
    /// The execution as observed. It goes back in the result.
    pub execution: Execution,
    /// How many argv tokens come before the compiler (a launcher and its options).
    pub launcher_len: usize,
    /// Whether `@file` response files are read before classification.
    pub inline_response_files: bool,
}

impl FlagBasedInterpreter {
    /// Creates a new flag-based interpreter from one family's generated data.
    fn new(def: &FamilyDef, from_environment: bool) -> Self {
        Self {
            id: def.id,
            analyzer: FlagAnalyzer::new(def.flags),
            ignore_executables: def.ignore_executables,
            ignore_flags: def.ignore_flags,
            slash_prefix: def.slash_prefix,
            env_rules: def.env_rules,
            from_environment,
            syntax: def.response_file_syntax,
            source_mode: def.source_mode,
        }
    }

    /// `executable` and `arguments` are the compiler call as the family sees
    /// it: without the launcher, with response files read.
    fn should_ignore(&self, executable: &Path, arguments: &[String]) -> Option<&'static str> {
        // Check executable name against ignore list
        if !self.ignore_executables.is_empty()
            && let Some(filename) = executable.file_name()
            && let Some(filename_str) = filename.to_str()
            && self.ignore_executables.contains(&filename_str)
        {
            return Some("internal executable");
        }

        // Check arguments against ignore flags
        if !self.ignore_flags.is_empty()
            && self.ignore_flags.iter().any(|flag| arguments.iter().any(|arg| arg == flag))
        {
            return Some("internal invocation");
        }

        None
    }

    /// Reads the call as an [`Interpretation`].
    pub(super) fn recognize(&self, call: Call) -> RecognizeResult {
        let Call { execution, launcher_len, inline_response_files } = call;
        let argv = execution.arguments.get(launcher_len..).unwrap_or_default();

        // The compiler token, then everything after it. Under a launcher the
        // compiler runs as the token was written.
        let executable = match argv.first() {
            Some(token) if launcher_len > 0 => PathBuf::from(token),
            _ => execution.executable.clone(),
        };
        // Response files are read before classification, so their tokens are
        // classified like any other argument. The family's own syntax splits them.
        let (stream, provenances): (Vec<String>, Option<Vec<Provenance>>) = if inline_response_files {
            let after_compiler = argv.get(1..).unwrap_or_default();
            let tokens = expand(after_compiler, &execution.working_dir, self.syntax);
            let (tokens, provenances) = tokens.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
            (argv.first().cloned().into_iter().chain(tokens).collect(), Some(provenances))
        } else {
            (argv.to_vec(), None)
        };

        if let Some(reason) = self.should_ignore(&executable, &stream) {
            return RecognizeResult::Ignored { execution, reason };
        }

        let interpretation = self.interpretation(&execution, launcher_len, &executable, &stream, provenances);
        RecognizeResult::Recognized(Recognized { execution, interpretation })
    }

    fn interpretation(
        &self,
        execution: &Execution,
        launcher_len: usize,
        executable: &Path,
        stream: &[String],
        provenances: Option<Vec<Provenance>>,
    ) -> Interpretation {
        let (before, after) = if self.from_environment {
            interpret_environment(&execution.environment, self.env_rules, self.syntax)
        } else {
            (Vec::new(), Vec::new())
        };

        let mut arguments: Tagged = Vec::with_capacity(stream.len() + before.len() + after.len() + 1);
        if launcher_len > 0 {
            let launcher = execution.arguments.get(..launcher_len).unwrap_or_default().to_vec();
            arguments.push((Provenance::CommandLine, Arguments { args: launcher, role: Role::Launcher }));
        }
        // Tokens after the compiler carry the provenance of their own position.
        let provenance_of = |index: usize| match (&provenances, index.checked_sub(1)) {
            (Some(provenances), Some(position)) => {
                provenances.get(position).cloned().unwrap_or(Provenance::CommandLine)
            }
            _ => Provenance::CommandLine,
        };
        let mut parsed = classify(&self.analyzer, stream, self.slash_prefix).into_iter();
        // The compiler stays first (classify yields it first, and a launcher
        // always has one after it); environment words that go first follow it.
        if let Some(compiler) = parsed.next() {
            arguments.push((Provenance::CommandLine, compiler.into_arguments(executable)));
        }
        arguments.extend(before);
        for item in parsed {
            let provenance = provenance_of(item.first);
            arguments.push((provenance, item.into_arguments(executable)));
        }
        arguments.extend(after);

        Interpretation::new(self.id, self.source_mode, arguments)
    }
}

/// What the flag tables say about one run of tokens.
enum Class {
    Compiler,
    /// A file named on the command line, with its path as written.
    Source(String),
    Output {
        flag: String,
        path: String,
        spelling: OutputSpelling,
    },
    Flag(PassEffect),
    /// A `@file` token, with the path as written after the `@`.
    ArgumentsFile(String),
}

/// Items tagged with where they came from, in output order.
type Tagged = Vec<(Provenance, Arguments)>;

/// A run of argv tokens that mean one thing, and where the run starts.
struct Parsed {
    first: usize,
    tokens: Vec<String>,
    class: Class,
}

impl Parsed {
    fn into_arguments(self, executable: &Path) -> Arguments {
        let role = match self.class {
            Class::Compiler => Role::Compiler { path: executable.to_path_buf() },
            Class::Source(path) => {
                let file_type = file_type_of(&path);
                Role::Input(FileRef::Resolved { path: PathBuf::from(path), file_type })
            }
            Class::Output { flag, path, spelling } => {
                Role::Output { flag, path: PathBuf::from(path), spelling }
            }
            Class::Flag(effect) => Role::Flag(effect),
            Class::ArgumentsFile(path) => Role::ArgumentsFile { path: PathBuf::from(path) },
        };
        Arguments { args: self.tokens, role }
    }
}

/// Split argv (compiler first) into runs of tokens, each with its meaning.
///
/// A token at a token position that starts with `@` and has more after it is
/// an `@file` item, checked before the flag table. A value a flag consumes
/// never reaches that check.
fn classify(flag_analyzer: &FlagAnalyzer, args: &[String], slash_prefix: bool) -> Vec<Parsed> {
    let mut result: Vec<Parsed> = Vec::with_capacity(args.len());
    let mut i = 0;

    while i < args.len() {
        // Handle the first argument (compiler name)
        if i == 0 {
            result.push(Parsed { first: 0, tokens: vec![args[0].clone()], class: Class::Compiler });
            i += 1;
            continue;
        }

        let token = &args[i];
        if let Some(path) = token.strip_prefix('@') {
            // A bare `@` names no file; keep it as a plain flag so the entry
            // still holds it.
            let class = if path.is_empty() {
                Class::Flag(PassEffect::None)
            } else {
                Class::ArgumentsFile(path.to_string())
            };
            result.push(Parsed { first: i, tokens: vec![token.clone()], class });
            i += 1;
            continue;
        }

        if let Some(match_result) = flag_analyzer.match_flag(&args[i..]) {
            // Handle pass-through first (early exit)
            if matches!(match_result.rule.kind, RuleKind::Effect(PassEffect::PassThrough)) {
                result.push(Parsed {
                    first: i,
                    tokens: vec![token.clone()],
                    class: Class::Flag(PassEffect::PassThrough),
                });
                i += 1;
                while i < args.len() {
                    result.push(Parsed {
                        first: i,
                        tokens: vec![args[i].clone()],
                        class: Class::Flag(PassEffect::Configures(CompilerPass::Linking)),
                    });
                    i += 1;
                }
                break;
            }

            let consumed_count = match_result.consumed_count;
            let tokens = args[i..i + consumed_count].to_vec();
            let class = match match_result.rule.kind {
                RuleKind::Output => match consumed_count {
                    1 => {
                        let flag_str = match_result.rule.pattern.flag();
                        let after_flag = &token[flag_str.len()..];
                        // Remember how the build joined the flag and its
                        // value, so the entry can spell it back the way this
                        // compiler accepts.
                        let (spelling, path) = match after_flag.strip_prefix('=') {
                            Some(rest) => (OutputSpelling::Equals, rest),
                            None => match after_flag.strip_prefix(':') {
                                Some(rest) => (OutputSpelling::Colon, rest),
                                None => (OutputSpelling::Glued, after_flag),
                            },
                        };
                        Class::Output { flag: flag_str.to_string(), path: path.to_string(), spelling }
                    }
                    2 => Class::Output {
                        flag: token.clone(),
                        path: args[i + 1].clone(),
                        spelling: OutputSpelling::Separate,
                    },
                    _ => {
                        unreachable!("Output file should be specified with glued or separate value")
                    }
                },
                RuleKind::Effect(effect) => Class::Flag(effect),
            };

            result.push(Parsed { first: i, tokens, class });
            i += consumed_count;
        } else if token.starts_with('-') || (slash_prefix && token.starts_with('/')) {
            result.push(Parsed {
                first: i,
                tokens: vec![token.clone()],
                class: Class::Flag(PassEffect::None),
            });
            i += 1;
        } else {
            result.push(Parsed {
                first: i,
                tokens: vec![token.clone()],
                class: Class::Source(token.clone()),
            });
            i += 1;
        }
    }

    result
}

/// The words one rule makes from its environment variable.
struct EnvWords {
    variable: &'static str,
    position: EnvPosition,
    words: Vec<String>,
    effect: PassEffect,
}

/// Turn the rules' variables into words, in rule order. `Flag` rules make a
/// `flag value` pair per element, `Expand` rules one word per shell word.
fn environment_words(
    environment: &std::collections::HashMap<String, String>,
    rules: &[EnvRule],
    syntax: Syntax,
) -> Vec<EnvWords> {
    let mut result = Vec::new();

    for rule in rules {
        let Some(value) = environment.get(rule.variable) else {
            continue;
        };
        match rule.mapping {
            EnvMapping::Flag { flag, separator } => {
                for part in split_env_value(value, separator) {
                    if !part.is_empty() {
                        result.push(EnvWords {
                            variable: rule.variable,
                            position: EnvPosition::Append,
                            words: vec![flag.to_string(), part],
                            effect: rule.effect,
                        });
                    }
                }
            }
            EnvMapping::Expand { position } => {
                let words = match syntax {
                    Syntax::Msvc => tokenize_msvc(value),
                    Syntax::GnuClang => shell_words::split(value).unwrap_or_else(|_| vec![value.clone()]),
                };
                for word in words {
                    result.push(EnvWords {
                        variable: rule.variable,
                        position,
                        words: vec![word],
                        effect: rule.effect,
                    });
                }
            }
        }
    }

    result
}

/// Environment words, each tagged with its variable, as `(before, after)`.
/// Words that go before the command line args (e.g. MSVC `CL`) are `before`;
/// the rest (include paths, MSVC `_CL_`) are `after`.
fn interpret_environment(
    environment: &std::collections::HashMap<String, String>,
    rules: &[EnvRule],
    syntax: Syntax,
) -> (Tagged, Tagged) {
    let mut before = Vec::new();
    let mut after = Vec::new();

    for item in environment_words(environment, rules, syntax) {
        let target = match item.position {
            EnvPosition::Prepend => &mut before,
            EnvPosition::Append => &mut after,
        };
        let words = Arguments { args: item.words, role: Role::Flag(item.effect) };
        target.push((Provenance::Environment(item.variable.to_string()), words));
    }

    (before, after)
}

/// Split an environment variable value by the given separator type.
fn split_env_value(value: &str, separator: EnvSeparator) -> Vec<String> {
    match separator {
        EnvSeparator::Path => std::env::split_paths(value).map(|p| p.to_string_lossy().to_string()).collect(),
        EnvSeparator::Fixed(sep) => value.split(sep).map(|s| s.to_string()).collect(),
    }
}

// Flag tables and the family registry, generated at build time from
// compilers/*.yaml. families.rs include!s each per-family flag-table file
// (bringing that family's X_FLAGS / X_IGNORE_* / X_SLASH_PREFIX / X_ENV_RULES
// statics into scope) and then defines FAMILIES, one row per family in
// recognition order.
include!(concat!(env!("OUT_DIR"), "/families.rs"));

/// One compiler family's generated data: references to its flag-table statics
/// plus the two per-family behavior selectors. The generated `FAMILIES` array
/// is a slice of these; `register_all` (in the parent module) loops over it,
/// and `FlagBasedInterpreter::new` reads `response_file_syntax` to split
/// response files and environment-derived words.
pub(super) struct FamilyDef {
    pub(super) id: &'static str,
    pub(super) flags: &'static [FlagRule],
    pub(super) ignore_executables: &'static [&'static str],
    pub(super) ignore_flags: &'static [&'static str],
    pub(super) slash_prefix: bool,
    pub(super) env_rules: &'static [EnvRule],
    pub(super) source_mode: SourceMode,
    pub(super) response_file_syntax: Syntax,
}

/// Build the interpreter for one family.
pub(super) fn interpreter(def: &FamilyDef, from_environment: bool) -> FlagBasedInterpreter {
    FlagBasedInterpreter::new(def, from_environment)
}

#[cfg(test)]
mod flag_table_invariants {
    use super::*;

    fn assert_invariants(flags: &[FlagRule]) {
        assert!(!flags.is_empty(), "Flag table must not be empty");

        // Sorted by flag length descending
        for window in flags.windows(2) {
            assert!(
                window[0].pattern.flag().len() >= window[1].pattern.flag().len(),
                "Flags not sorted by length: {:?} (len {}) before {:?} (len {})",
                window[0].pattern.flag(),
                window[0].pattern.flag().len(),
                window[1].pattern.flag(),
                window[1].pattern.flag().len(),
            );
        }

        for rule in flags {
            let flag = rule.pattern.flag();
            assert!(
                flag.starts_with('-') || flag.starts_with('/'),
                "Flag {:?} must start with '-' or '/'",
                flag
            );

            if matches!(rule.kind, RuleKind::Output) {
                match rule.pattern {
                    FlagPattern::Exactly(_, n) => {
                        assert!(n <= 1, "Output rule {:?} must take 0 or 1 extra args", flag)
                    }
                    FlagPattern::ExactlyWithEq(_)
                    | FlagPattern::ExactlyWithEqOrSep(_)
                    | FlagPattern::ExactlyWithColon(_)
                    | FlagPattern::ExactlyWithColonOrSep(_)
                    | FlagPattern::ExactlyWithGluedOrSep(_) => {}
                    FlagPattern::Prefix(_, n) => {
                        assert!(n <= 1, "Output rule {:?} must take 0 or 1 extra args", flag)
                    }
                }
            }
        }
    }

    #[test]
    fn gcc() {
        assert_invariants(&GCC_FLAGS);
    }

    #[test]
    fn clang() {
        assert_invariants(&CLANG_FLAGS);
    }

    #[test]
    fn flang() {
        assert_invariants(&FLANG_FLAGS);
    }

    #[test]
    fn cuda() {
        assert_invariants(&CUDA_FLAGS);
    }

    #[test]
    fn intel_fortran() {
        assert_invariants(&INTEL_FORTRAN_FLAGS);
    }

    #[test]
    fn cray_fortran() {
        assert_invariants(&CRAY_FORTRAN_FLAGS);
    }

    #[test]
    fn msvc() {
        assert_invariants(&MSVC_FLAGS);
    }

    #[test]
    fn clang_cl() {
        assert_invariants(&CLANG_CL_FLAGS);
    }

    #[test]
    fn intel_cc() {
        assert_invariants(&INTEL_CC_FLAGS);
    }

    #[test]
    fn nvidia_hpc() {
        assert_invariants(&NVIDIA_HPC_FLAGS);
    }

    #[test]
    fn armclang() {
        assert_invariants(&ARMCLANG_FLAGS);
    }

    #[test]
    fn ibm_xl() {
        assert_invariants(&IBM_XL_FLAGS);
    }

    #[test]
    fn vala() {
        assert_invariants(&VALA_FLAGS);
    }

    #[test]
    fn mpi() {
        assert_invariants(&MPI_FLAGS);
    }

    #[test]
    fn cray_cc() {
        assert_invariants(&CRAY_CC_FLAGS);
    }

    #[test]
    fn qnx() {
        assert_invariants(&QNX_FLAGS);
    }

    #[test]
    fn nasm() {
        assert_invariants(&NASM_FLAGS);
    }

    #[test]
    fn fasm() {
        assert_invariants(&FASM_FLAGS);
    }

    #[test]
    fn swift() {
        assert_invariants(&SWIFT_FLAGS);
    }

    #[test]
    fn clang_inherits_all_gcc_flags() {
        let gcc_flag_strings: std::collections::HashSet<&str> =
            GCC_FLAGS.iter().map(|f| f.pattern.flag()).collect();
        let clang_flag_strings: std::collections::HashSet<&str> =
            CLANG_FLAGS.iter().map(|f| f.pattern.flag()).collect();

        assert!(
            CLANG_FLAGS.len() > GCC_FLAGS.len(),
            "Clang should have more flags than GCC, got gcc: {}, clang: {}",
            GCC_FLAGS.len(),
            CLANG_FLAGS.len()
        );

        let missing_flags: Vec<&str> = gcc_flag_strings.difference(&clang_flag_strings).cloned().collect();
        assert!(missing_flags.is_empty(), "These GCC flags are missing from Clang: {:?}", missing_flags);
    }
}

#[cfg(test)]
mod pass_through_tests {
    use super::*;
    use crate::interpreters::matchers::{FlagAnalyzer, FlagPattern, FlagRule};

    #[test]
    fn test_pass_through_flag_stops_parsing() {
        static PASS_THROUGH_FLAGS: std::sync::LazyLock<Vec<FlagRule>> = std::sync::LazyLock::new(|| {
            let mut flags = vec![
                FlagRule::new(
                    FlagPattern::Exactly("-c", 0),
                    RuleKind::Effect(PassEffect::StopsAt(CompilerPass::Assembling)),
                ),
                FlagRule::new(FlagPattern::Exactly("/link", 0), RuleKind::Effect(PassEffect::PassThrough)),
                FlagRule::new(FlagPattern::Exactly("-o", 1), RuleKind::Output),
            ];
            flags.sort_by_key(|b| std::cmp::Reverse(b.pattern.flag().len()));
            flags
        });

        let analyzer = FlagAnalyzer::new(&PASS_THROUGH_FLAGS);
        let args = vec![
            "cl".to_string(),
            "-c".to_string(),
            "foo.c".to_string(),
            "/link".to_string(),
            "/SUBSYSTEM:CONSOLE".to_string(),
            "/OUT:foo.exe".to_string(),
        ];

        let result = classify(&analyzer, &args, false);

        // cl (compiler)
        assert!(matches!(result[0].class, Class::Compiler));
        // -c (stops at assembling)
        assert!(
            matches!(result[1].class, Class::Flag(effect) if effect == PassEffect::StopsAt(CompilerPass::Assembling))
        );
        // foo.c (source)
        assert!(matches!(result[2].class, Class::Source(_)));
        // /link (pass-through marker)
        assert!(matches!(result[3].class, Class::Flag(effect) if effect == PassEffect::PassThrough));
        // /SUBSYSTEM:CONSOLE (linker arg)
        assert!(
            matches!(result[4].class, Class::Flag(effect) if effect == PassEffect::Configures(CompilerPass::Linking))
        );
        // /OUT:foo.exe (linker arg)
        assert!(
            matches!(result[5].class, Class::Flag(effect) if effect == PassEffect::Configures(CompilerPass::Linking))
        );
    }
}

#[cfg(test)]
mod vala_tests {
    use super::*;
    use crate::interpreters::matchers::FlagAnalyzer;

    // `-X` forwards exactly one token to the C compiler, and that token usually
    // starts with '-' (e.g. `-X -lm`). Because Bear treats any bare argument as
    // a source file, the danger is that the value leaks in as a phantom source.
    // `count: 1` must consume it regardless of its leading dash.
    #[test]
    fn dash_prefixed_x_value_is_consumed_not_a_source() {
        let analyzer = FlagAnalyzer::new(&VALA_FLAGS);
        let args = vec!["valac".to_string(), "-X".to_string(), "-lm".to_string(), "foo.vala".to_string()];

        let result = classify(&analyzer, &args, false);

        // valac (compiler), [-X -lm] (one option), foo.vala (the only source)
        assert_eq!(result.len(), 3, "expected exactly 3 parsed arguments, got {}", result.len());
        assert!(matches!(result[0].class, Class::Compiler));
        assert!(
            matches!(&result[1], Parsed { tokens, .. } if tokens == &vec!["-X".to_string(), "-lm".to_string()]),
            "-X must consume -lm as its value, got {:?}",
            result[1].tokens
        );
        assert!(
            matches!(&result[2].class, Class::Source(path) if path == "foo.vala"),
            "foo.vala must be the sole source, got {:?}",
            result[2].tokens
        );
    }
}

#[cfg(test)]
mod slash_prefix_tests {
    use super::*;
    use crate::interpreters::matchers::{FlagAnalyzer, FlagRule};

    #[test]
    fn slash_prefixed_args_treated_as_source_without_slash_support() {
        let flags: &[FlagRule] = &[];
        let analyzer = FlagAnalyzer::new(flags);
        let args = vec!["cl".to_string(), "/c".to_string(), "foo.c".to_string()];
        let result = classify(&analyzer, &args, false);
        // /c should be a source file since slash_prefix is false
        assert!(matches!(result[1].class, Class::Source(_)));
    }

    #[test]
    fn slash_prefixed_args_treated_as_flags_with_slash_support() {
        let flags: &[FlagRule] = &[];
        let analyzer = FlagAnalyzer::new(flags);
        let args = vec!["cl".to_string(), "/c".to_string(), "foo.c".to_string()];
        let result = classify(&analyzer, &args, true);
        // /c should be an unrecognized flag (effect None) since slash_prefix is true
        assert!(matches!(result[1].class, Class::Flag(PassEffect::None)));
    }

    #[test]
    fn output_extraction_works_with_glued_eq() {
        use crate::interpreters::matchers::FlagPattern;
        use std::sync::LazyLock;

        static OUTPUT_FLAGS: LazyLock<Vec<FlagRule>> =
            LazyLock::new(|| vec![FlagRule::new(FlagPattern::ExactlyWithEq("-o"), RuleKind::Output)]);

        let analyzer = FlagAnalyzer::new(&OUTPUT_FLAGS);
        let args = vec!["gcc".to_string(), "-o=foo.o".to_string()];
        let result = classify(&analyzer, &args, false);
        assert!(matches!(result[1].class, Class::Output { ref flag, ref path, spelling }
                if flag == "-o" && path == "foo.o" && spelling == OutputSpelling::Equals));
    }

    #[test]
    fn output_extraction_works_with_glued_colon() {
        use crate::interpreters::matchers::FlagPattern;
        use std::sync::LazyLock;

        static OUTPUT_FLAGS: LazyLock<Vec<FlagRule>> =
            LazyLock::new(|| vec![FlagRule::new(FlagPattern::ExactlyWithColon("/Fo"), RuleKind::Output)]);

        let analyzer = FlagAnalyzer::new(&OUTPUT_FLAGS);
        let args = vec!["cl".to_string(), "/Fo:foo.obj".to_string()];
        let result = classify(&analyzer, &args, true);
        assert!(matches!(result[1].class, Class::Output { ref flag, ref path, spelling }
                if flag == "/Fo" && path == "foo.obj" && spelling == OutputSpelling::Colon));
    }

    #[test]
    fn output_extraction_works_with_glued_value() {
        use crate::interpreters::matchers::FlagPattern;
        use std::sync::LazyLock;

        static OUTPUT_FLAGS: LazyLock<Vec<FlagRule>> =
            LazyLock::new(|| vec![FlagRule::new(FlagPattern::ExactlyWithGluedOrSep("-o"), RuleKind::Output)]);

        let analyzer = FlagAnalyzer::new(&OUTPUT_FLAGS);
        let args = vec!["gcc".to_string(), "-ofoo.o".to_string()];
        let result = classify(&analyzer, &args, false);
        assert!(matches!(result[1].class, Class::Output { ref flag, ref path, spelling }
                if flag == "-o" && path == "foo.o" && spelling == OutputSpelling::Glued));
    }

    /// The separate spelling is the only one that may re-emit as two tokens.
    #[test]
    fn output_extraction_records_the_separate_form() {
        use crate::interpreters::matchers::FlagPattern;
        use std::sync::LazyLock;

        static OUTPUT_FLAGS: LazyLock<Vec<FlagRule>> =
            LazyLock::new(|| vec![FlagRule::new(FlagPattern::ExactlyWithGluedOrSep("-o"), RuleKind::Output)]);

        let analyzer = FlagAnalyzer::new(&OUTPUT_FLAGS);
        let args = vec!["gcc".to_string(), "-o".to_string(), "foo.o".to_string()];
        let result = classify(&analyzer, &args, false);
        assert!(matches!(result[1].class, Class::Output { ref flag, ref path, spelling }
                if flag == "-o" && path == "foo.o" && spelling == OutputSpelling::Separate));
    }
}

#[cfg(test)]
mod environment_mapping_tests {
    use super::*;
    use std::collections::HashMap;

    fn collect_args(words: &[(Provenance, Arguments)]) -> Vec<String> {
        words.iter().flat_map(|(_, item)| item.args.clone()).collect()
    }

    #[test]
    fn path_separator_mapping() {
        let rules = &[EnvRule::new(
            "CPATH",
            EnvMapping::Flag { flag: "-I", separator: EnvSeparator::Path },
            PassEffect::Configures(CompilerPass::Preprocessing),
        )];
        let mut env = HashMap::new();
        // std::env::split_paths uses `:` on Unix, `;` on Windows
        let (value, expected_a, expected_b) =
            if cfg!(windows) { (r"C:\a;C:\b", r"C:\a", r"C:\b") } else { ("/a:/b", "/a", "/b") };
        env.insert("CPATH".to_string(), value.to_string());

        let (prepend, append) = interpret_environment(&env, rules, Syntax::GnuClang);
        assert!(prepend.is_empty());
        let args = collect_args(&append);
        assert_eq!(args, vec!["-I", expected_a, "-I", expected_b]);
    }

    #[test]
    fn path_separator_filters_empty_elements() {
        let rules = &[EnvRule::new(
            "CPATH",
            EnvMapping::Flag { flag: "-I", separator: EnvSeparator::Path },
            PassEffect::Configures(CompilerPass::Preprocessing),
        )];
        let mut env = HashMap::new();
        // Leading/trailing/double separators produce empty elements that must be filtered
        let (value, expected_a, expected_b) =
            if cfg!(windows) { (r";C:\a;;C:\b;", r"C:\a", r"C:\b") } else { (":/a::/b:", "/a", "/b") };
        env.insert("CPATH".to_string(), value.to_string());

        let (_prepend, append) = interpret_environment(&env, rules, Syntax::GnuClang);
        let args = collect_args(&append);
        assert_eq!(args, vec!["-I", expected_a, "-I", expected_b]);
    }

    #[test]
    fn fixed_separator_mapping() {
        let rules = &[EnvRule::new(
            "INCLUDE",
            EnvMapping::Flag { flag: "/I", separator: EnvSeparator::Fixed(";") },
            PassEffect::Configures(CompilerPass::Preprocessing),
        )];
        let mut env = HashMap::new();
        env.insert("INCLUDE".to_string(), r"C:\a;C:\b".to_string());

        let (_prepend, append) = interpret_environment(&env, rules, Syntax::Msvc);
        let args = collect_args(&append);
        assert_eq!(args, vec!["/I", r"C:\a", "/I", r"C:\b"]);
    }

    // Requirements: output-env-derived-flags
    #[test]
    fn expand_prepend() {
        let rules = &[EnvRule::new(
            "CL",
            EnvMapping::Expand { position: EnvPosition::Prepend },
            PassEffect::Configures(CompilerPass::Compiling),
        )];
        let mut env = HashMap::new();
        env.insert("CL".to_string(), "/O2 /W4".to_string());

        let (prepend, append) = interpret_environment(&env, rules, Syntax::Msvc);
        assert!(append.is_empty());
        let args = collect_args(&prepend);
        assert_eq!(args, vec!["/O2", "/W4"]);
    }

    // Requirements: output-env-derived-flags
    #[test]
    fn expand_append() {
        let rules = &[EnvRule::new(
            "_CL_",
            EnvMapping::Expand { position: EnvPosition::Append },
            PassEffect::Configures(CompilerPass::Compiling),
        )];
        let mut env = HashMap::new();
        env.insert("_CL_".to_string(), "/link foo.lib".to_string());

        let (prepend, append) = interpret_environment(&env, rules, Syntax::Msvc);
        assert!(prepend.is_empty());
        let args = collect_args(&append);
        assert_eq!(args, vec!["/link", "foo.lib"]);
    }

    fn family(id: &str) -> &'static FamilyDef {
        FAMILIES.iter().find(|def| def.id == id).unwrap()
    }

    #[test]
    fn response_file_syntax_follows_compiler_family() {
        for id in ["msvc", "clang_cl"] {
            assert!(matches!(family(id).response_file_syntax, Syntax::Msvc), "{id} should use MSVC syntax");
        }
        for id in ["gcc", "clang", "cuda", "vala"] {
            assert!(
                matches!(family(id).response_file_syntax, Syntax::GnuClang),
                "{id} should use GNU/Clang syntax"
            );
        }
    }

    fn msvc() -> FlagBasedInterpreter {
        FlagBasedInterpreter::new(family("msvc"), true)
    }

    fn armclang() -> FlagBasedInterpreter {
        FlagBasedInterpreter::new(family("armclang"), true)
    }

    // Requirements: output-env-derived-flags, output-json-compilation-database
    #[test]
    fn prepended_environment_words_follow_the_compiler() {
        let cases: Vec<(FlagBasedInterpreter, Execution, Vec<&str>)> = vec![
            (
                msvc(),
                Execution::from_strings(
                    "cl",
                    vec!["cl", "/c", "foo.c"],
                    "/home/user",
                    HashMap::from([("CL", "/O2 /W4"), ("_CL_", "/DX")]),
                ),
                vec!["cl", "/O2", "/W4", "/c", "foo.c", "/DX"],
            ),
            (
                armclang(),
                Execution::from_strings(
                    "armclang",
                    vec!["armclang", "-c", "foo.c"],
                    "/home/user",
                    HashMap::from([("ARMCOMPILER6_CLANGOPT", "-O2")]),
                ),
                vec!["armclang", "-O2", "-c", "foo.c"],
            ),
        ];

        for (interpreter, execution, expected) in cases {
            let input = execution.to_string();

            let call = Call { execution, launcher_len: 0, inline_response_files: false };
            let sut: Vec<String> = match interpreter.recognize(call) {
                RecognizeResult::Recognized(Recognized { interpretation, .. }) => {
                    interpretation.arguments().flat_map(|item| item.args.clone()).collect()
                }
                _ => panic!("expected a recognized command for {input}"),
            };

            assert_eq!(sut, expected, "argument order for {input}");
        }
    }

    // Requirements: output-env-derived-flags
    #[test]
    fn environment_words_split_with_the_family_syntax() {
        let cl = |value: &str| {
            Execution::from_strings(
                "cl",
                vec!["cl", "/c", "foo.c"],
                "/home/user",
                HashMap::from([("CL", value)]),
            )
        };
        let armclang_opt = |value: &str| {
            Execution::from_strings(
                "armclang",
                vec!["armclang", "-c", "foo.c"],
                "/home/user",
                HashMap::from([("ARMCOMPILER6_CLANGOPT", value)]),
            )
        };
        let cases: Vec<(FlagBasedInterpreter, Execution, Vec<&str>)> = vec![
            (msvc(), cl(r"/IC:\foo /W4"), vec!["cl", r"/IC:\foo", "/W4", "/c", "foo.c"]),
            (
                msvc(),
                cl(r#"/I"C:\Program Files\x" /W4"#),
                vec!["cl", r"/IC:\Program Files\x", "/W4", "/c", "foo.c"],
            ),
            (msvc(), cl(r#"/DNAME=\"v\""#), vec!["cl", r#"/DNAME="v""#, "/c", "foo.c"]),
            (
                armclang(),
                armclang_opt(r#"-I/a\ b -DX=\"v\""#),
                vec!["armclang", "-I/a b", r#"-DX="v""#, "-c", "foo.c"],
            ),
        ];

        for (interpreter, execution, expected) in cases {
            let input = execution.to_string();

            let call = Call { execution, launcher_len: 0, inline_response_files: false };
            let sut: Vec<String> = match interpreter.recognize(call) {
                RecognizeResult::Recognized(Recognized { interpretation, .. }) => {
                    interpretation.arguments().flat_map(|item| item.args.clone()).collect()
                }
                _ => panic!("expected a recognized command for {input}"),
            };

            assert_eq!(sut, expected, "environment words for {input}");
        }
    }

    #[test]
    fn missing_variable_produces_no_args() {
        let rules = &[EnvRule::new(
            "CPATH",
            EnvMapping::Flag { flag: "-I", separator: EnvSeparator::Path },
            PassEffect::Configures(CompilerPass::Preprocessing),
        )];
        let env = HashMap::new();

        let (prepend, append) = interpret_environment(&env, rules, Syntax::GnuClang);
        assert!(prepend.is_empty());
        assert!(append.is_empty());
    }

    #[test]
    fn nvidia_hpc_has_no_gcc_env_rules() {
        assert!(
            NVIDIA_HPC_ENV_RULES.is_empty(),
            "NVIDIA HPC should not inherit GCC environment variables, got {} rules",
            NVIDIA_HPC_ENV_RULES.len()
        );
    }

    #[test]
    fn clang_inherits_gcc_env_rules() {
        let gcc_vars: std::collections::HashSet<&str> = GCC_ENV_RULES.iter().map(|r| r.variable).collect();
        let clang_vars: std::collections::HashSet<&str> =
            CLANG_ENV_RULES.iter().map(|r| r.variable).collect();

        let missing: Vec<&str> = gcc_vars.difference(&clang_vars).cloned().collect();
        assert!(missing.is_empty(), "Clang should inherit all GCC env vars, missing: {:?}", missing);
    }

    #[test]
    fn armclang_inherits_gcc_env_rules_transitively() {
        let gcc_vars: std::collections::HashSet<&str> = GCC_ENV_RULES.iter().map(|r| r.variable).collect();
        let arm_vars: std::collections::HashSet<&str> =
            ARMCLANG_ENV_RULES.iter().map(|r| r.variable).collect();

        let missing: Vec<&str> = gcc_vars.difference(&arm_vars).cloned().collect();
        assert!(
            missing.is_empty(),
            "ARMClang should transitively inherit all GCC env vars, missing: {:?}",
            missing
        );
        // Also has its own variable
        assert!(arm_vars.contains("ARMCOMPILER6_CLANGOPT"));
    }

    #[test]
    fn msvc_env_rules_present() {
        let vars: Vec<&str> = MSVC_ENV_RULES.iter().map(|r| r.variable).collect();
        assert!(vars.contains(&"CL"));
        assert!(vars.contains(&"_CL_"));
        assert!(vars.contains(&"INCLUDE"));
        assert!(vars.contains(&"LIB"));
    }

    #[test]
    fn clang_cl_inherits_msvc_env_rules() {
        let msvc_vars: std::collections::HashSet<&str> = MSVC_ENV_RULES.iter().map(|r| r.variable).collect();
        let clang_cl_vars: std::collections::HashSet<&str> =
            CLANG_CL_ENV_RULES.iter().map(|r| r.variable).collect();

        let missing: Vec<&str> = msvc_vars.difference(&clang_cl_vars).cloned().collect();
        assert!(missing.is_empty(), "Clang-CL should inherit all MSVC env vars, missing: {:?}", missing);
    }

    #[test]
    fn expand_with_quoted_values() {
        let rules = &[EnvRule::new(
            "CL",
            EnvMapping::Expand { position: EnvPosition::Prepend },
            PassEffect::Configures(CompilerPass::Compiling),
        )];
        let mut env = HashMap::new();
        env.insert("CL".to_string(), r#"/DPATH="C:\Program Files" /W4"#.to_string());

        let (prepend, _append) = interpret_environment(&env, rules, Syntax::Msvc);
        let args = collect_args(&prepend);
        assert_eq!(args, vec![r"/DPATH=C:\Program Files", "/W4"]);
    }
}
