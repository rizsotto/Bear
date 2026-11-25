// SPDX-License-Identifier: GPL-3.0-or-later

//! Response-file (`@file`) inlining for intercepted compiler invocations.
//!
//! Some build systems (notably Xcode) keep part of a compiler's flags in a
//! separate "response file" and pass it as `@/path/to/args.resp`. When the
//! user opts in (`format.arguments.from_response_files`), [`expand`] replaces
//! each `@file` token with the file's tokenized contents *before* flag
//! classification, so the inlined tokens are classified, link-stripped, and
//! split per source exactly like any other argument. Each token carries the
//! file it came from.
//!
//! Tokenization follows the compiler family Bear already identified for the
//! entry; no new detection is introduced here. See the
//! `output-response-file-inlining` requirement for the contract.

use super::identity::CompilerType;
use crate::interpretation::Origin;
use log::warn;
use std::path::{Path, PathBuf};

/// Maximum number of nested `@file` levels to follow. Guards against cycles
/// (a response file that references itself, directly or transitively).
const MAX_DEPTH: usize = 16;

/// Tokenization conventions for a response file's contents.
#[derive(Clone, Copy)]
pub(super) enum Syntax {
    /// GCC/Clang family: whitespace separates tokens; single or double quotes
    /// group whitespace; a backslash quotes the next character.
    GnuClang,
    /// MSVC and clang-cl: Windows command-line rules; only double quotes group
    /// tokens; backslash escaping is positional, meaningful next to quotes.
    Msvc,
}

/// Selects the tokenization syntax for the compiler family Bear identified,
/// reading the family's `response_file_syntax` from the generated
/// [`super::flag_based::FAMILIES`] registry. A wrapper (no family id) and any
/// id absent from the registry default to GNU/Clang syntax.
pub(super) fn syntax_for(compiler_type: CompilerType) -> Syntax {
    match compiler_type {
        CompilerType::Compiler(id) => super::flag_based::FAMILIES
            .iter()
            .find(|def| def.id == id.as_str())
            .map(|def| def.response_file_syntax)
            .unwrap_or(Syntax::GnuClang),
        CompilerType::Wrapper => Syntax::GnuClang,
    }
}

/// Replaces every `@file` argument with the file's tokenized contents, each
/// token tagged with where it came from.
///
/// `arguments` is the argv after the compiler token, so the compiler is never
/// treated as a response file. `@file` paths are resolved relative to
/// `working_dir`, matching how the compiler itself opens them. A token from a
/// file is tagged with that file as written (the innermost one when nested).
/// Missing/unreadable files and references past [`MAX_DEPTH`] are left as the
/// literal `@path` token with a warning, so a single stale build artefact
/// never fails the whole database.
pub(super) fn expand(arguments: &[String], working_dir: &Path, syntax: Syntax) -> Vec<(String, Origin)> {
    let mut expanded = Vec::with_capacity(arguments.len());
    for argument in arguments {
        match argument.strip_prefix('@') {
            Some(path) if !path.is_empty() => {
                expand_reference(&mut expanded, path, working_dir, syntax, 1, &Origin::CommandLine);
            }
            _ => expanded.push((argument.clone(), Origin::CommandLine)),
        }
    }
    expanded
}

/// Reads, tokenizes, and recursively expands a single `@file` reference into
/// `out`. On any failure the `@path` token is appended unchanged, tagged with
/// `parent`, the place the reference was written in.
fn expand_reference(
    out: &mut Vec<(String, Origin)>,
    raw_path: &str,
    base: &Path,
    syntax: Syntax,
    depth: usize,
    parent: &Origin,
) {
    if depth > MAX_DEPTH {
        warn!(
            "response file expansion exceeded depth limit ({MAX_DEPTH}); leaving @{raw_path} literal (possible cycle)"
        );
        out.push((format!("@{raw_path}"), parent.clone()));
        return;
    }

    let path = resolve(base, raw_path);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            warn!("cannot read response file {}: {error}; leaving @{raw_path} literal", path.display());
            out.push((format!("@{raw_path}"), parent.clone()));
            return;
        }
    };

    let origin = Origin::ResponseFile(PathBuf::from(raw_path));
    for token in tokenize(&content, syntax) {
        match token.strip_prefix('@') {
            Some(nested) if !nested.is_empty() => {
                expand_reference(out, nested, base, syntax, depth + 1, &origin);
            }
            _ => out.push((token, origin.clone())),
        }
    }
}

/// Resolves a response-file path relative to the invocation's working
/// directory. Absolute paths are used as-is.
fn resolve(base: &Path, raw_path: &str) -> PathBuf {
    let path = Path::new(raw_path);
    if path.is_absolute() { path.to_path_buf() } else { base.join(path) }
}

fn tokenize(content: &str, syntax: Syntax) -> Vec<String> {
    match syntax {
        Syntax::GnuClang => tokenize_gnu(content),
        Syntax::Msvc => tokenize_msvc(content),
    }
}

/// Tokenizes GCC/Clang-style response-file text: whitespace separates tokens,
/// single and double quotes group whitespace, and a backslash quotes the next
/// character. Matching libiberty/clang, a backslash is literal inside single
/// quotes and, inside double quotes, only escapes a following `"` or `\`.
fn tokenize_gnu(content: &str) -> Vec<String> {
    enum Quote {
        None,
        Single,
        Double,
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote = Quote::None;
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        match quote {
            Quote::Single => {
                if c == '\'' {
                    quote = Quote::None;
                } else {
                    current.push(c);
                }
            }
            Quote::Double => match c {
                '"' => quote = Quote::None,
                '\\' => match chars.peek() {
                    Some('"') | Some('\\') => current.push(chars.next().expect("peeked")),
                    _ => current.push('\\'),
                },
                _ => current.push(c),
            },
            Quote::None => {
                if c.is_whitespace() {
                    if started {
                        tokens.push(std::mem::take(&mut current));
                        started = false;
                    }
                    continue;
                }
                started = true;
                match c {
                    '\'' => quote = Quote::Single,
                    '"' => quote = Quote::Double,
                    '\\' => match chars.next() {
                        Some(next) => current.push(next),
                        None => current.push('\\'),
                    },
                    _ => current.push(c),
                }
            }
        }
    }

    if started {
        tokens.push(current);
    }
    tokens
}

/// Tokenizes MSVC/clang-cl response-file text following Windows
/// `CommandLineToArgv` rules: whitespace separates tokens, double quotes group,
/// and backslashes are literal except when they precede a quote (`2n` -> `n`
/// backslashes and a quote toggle; `2n+1` -> `n` backslashes and a literal
/// quote).
pub(super) fn tokenize_msvc(content: &str) -> Vec<String> {
    let chars: Vec<char> = content.chars().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut in_quotes = false;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            let mut backslashes = 0;
            while i < chars.len() && chars[i] == '\\' {
                backslashes += 1;
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                for _ in 0..backslashes / 2 {
                    current.push('\\');
                }
                started = true;
                if backslashes % 2 == 1 {
                    current.push('"');
                } else {
                    in_quotes = !in_quotes;
                }
                i += 1;
            } else {
                for _ in 0..backslashes {
                    current.push('\\');
                }
                started = true;
            }
        } else if c == '"' {
            started = true;
            // Inside a quoted segment, a doubled quote ("") is a literal quote
            // and keeps the segment open (CommandLineToArgv rule).
            if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                current.push('"');
                i += 2;
            } else {
                in_quotes = !in_quotes;
                i += 1;
            }
        } else if c.is_whitespace() && !in_quotes {
            if started {
                tokens.push(std::mem::take(&mut current));
                started = false;
            }
            i += 1;
        } else {
            current.push(c);
            started = true;
            i += 1;
        }
    }

    if started {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    //! Requirements: output-response-file-inlining
    use super::*;
    use std::fs;

    /// Expands the arguments after the first one, which stands for the compiler.
    fn args_of(arguments: Vec<&str>, working_dir: &Path) -> Vec<String> {
        tokens_of(arguments, working_dir).into_iter().map(|(token, _)| token).collect()
    }

    fn tokens_of(arguments: Vec<&str>, working_dir: &Path) -> Vec<(String, Origin)> {
        let argv: Vec<String> = arguments.iter().map(|a| a.to_string()).collect();
        expand(&argv[1..], working_dir, Syntax::GnuClang)
    }

    #[test]
    fn gnu_tokenizer_splits_on_whitespace() {
        let sut = tokenize_gnu("-I/opt/include -DEXTRA=2");

        assert_eq!(sut, vec!["-I/opt/include", "-DEXTRA=2"]);
    }

    #[test]
    fn gnu_tokenizer_strips_quotes_and_groups() {
        let sut = tokenize_gnu("'-std=gnu++20' -fmodules");

        assert_eq!(sut, vec!["-std=gnu++20", "-fmodules"]);
    }

    #[test]
    fn gnu_tokenizer_double_quotes_group_whitespace() {
        let sut = tokenize_gnu("-I \"/opt/my includes\"");

        assert_eq!(sut, vec!["-I", "/opt/my includes"]);
    }

    #[test]
    fn gnu_tokenizer_backslash_quotes_next_char() {
        let sut = tokenize_gnu(r"-DName=a\ b");

        assert_eq!(sut, vec!["-DName=a b"]);
    }

    #[test]
    fn msvc_tokenizer_applies_windows_rules() {
        let sut = tokenize_msvc(r#"/I "C:\Program Files\inc" /DFOO=1"#);

        assert_eq!(sut, vec!["/I", r"C:\Program Files\inc", "/DFOO=1"]);
    }

    #[test]
    fn msvc_tokenizer_doubled_quote_is_literal_inside_quotes() {
        // CommandLineToArgv: "" inside a quoted segment yields a literal " and
        // keeps the segment open.
        let sut = tokenize_msvc(r#"/DGREETING="say ""hi"""#);

        assert_eq!(sut, vec![r#"/DGREETING=say "hi""#]);
    }

    #[test]
    fn msvc_tokenizer_backslash_before_quote_escapes() {
        // Backslashes not before a quote are literal (a\\b -> a\\b); an odd
        // run before a quote (\") yields a literal quote.
        let sut = tokenize_msvc(r#"/DPATH="a\\b" \"x\""#);

        assert_eq!(sut, vec![r"/DPATH=a\\b", r#""x""#]);
    }

    #[test]
    fn gnu_tokenizer_double_quote_backslash_only_escapes_quote_and_backslash() {
        // Inside double quotes \" -> ", \\ -> \, but \d (any other char) keeps
        // the backslash literal (libiberty/clang).
        let sut = tokenize_gnu(r#""a\"b\\c\d""#);

        assert_eq!(sut, vec![r#"a"b\c\d"#]);
    }

    #[test]
    fn syntax_selection_follows_compiler_family() {
        for ty in [CompilerType::compiler("msvc"), CompilerType::compiler("clang_cl")] {
            assert!(matches!(syntax_for(ty), Syntax::Msvc), "{ty} should use MSVC syntax");
        }
        for ty in [
            CompilerType::compiler("gcc"),
            CompilerType::compiler("clang"),
            CompilerType::compiler("cuda"),
            CompilerType::compiler("vala"),
        ] {
            assert!(matches!(syntax_for(ty), Syntax::GnuClang), "{ty} should use GNU/Clang syntax");
        }
    }

    #[test]
    fn expand_inlines_response_file_at_token_position() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("flags.resp"), "-I/opt/include -DEXTRA=2").unwrap();

        let sut = args_of(vec!["cc", "-DBASE=1", "@flags.resp", "-c", "src.c"], dir.path());

        assert_eq!(sut, vec!["-DBASE=1", "-I/opt/include", "-DEXTRA=2", "-c", "src.c"]);
        assert!(sut.iter().all(|a| !a.starts_with('@')));
    }

    #[test]
    fn expand_tags_tokens_with_the_file_they_came_from() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("outer.resp"), "-DOUTER @inner.resp -DTAIL").unwrap();
        fs::write(dir.path().join("inner.resp"), "-DINNER").unwrap();

        let sut = tokens_of(vec!["cc", "-DBASE", "@outer.resp", "src.c"], dir.path());

        let file = |name: &str| Origin::ResponseFile(PathBuf::from(name));
        let expected = vec![
            ("-DBASE".to_string(), Origin::CommandLine),
            ("-DOUTER".to_string(), file("outer.resp")),
            ("-DINNER".to_string(), file("inner.resp")),
            ("-DTAIL".to_string(), file("outer.resp")),
            ("src.c".to_string(), Origin::CommandLine),
        ];
        assert_eq!(sut, expected);
    }

    #[test]
    fn expand_resolves_absolute_paths_as_is() {
        let dir = tempfile::tempdir().unwrap();
        let resp = dir.path().join("abs.resp");
        fs::write(&resp, "-DONLY").unwrap();

        let sut = args_of(vec!["cc", &format!("@{}", resp.display()), "src.c"], Path::new("/elsewhere"));

        assert_eq!(sut, vec!["-DONLY", "src.c"]);
    }

    #[test]
    fn expand_follows_nested_references() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("outer.resp"), "-DOUTER @inner.resp -DTAIL").unwrap();
        fs::write(dir.path().join("inner.resp"), "-DINNER").unwrap();

        let sut = args_of(vec!["cc", "@outer.resp", "src.c"], dir.path());

        assert_eq!(sut, vec!["-DOUTER", "-DINNER", "-DTAIL", "src.c"]);
        assert!(sut.iter().all(|a| !a.starts_with('@')));
    }

    #[test]
    fn expand_keeps_missing_file_literal() {
        let dir = tempfile::tempdir().unwrap();

        let sut = tokens_of(vec!["cc", "@gone.resp", "src.c"], dir.path());

        assert_eq!(
            sut,
            vec![("@gone.resp".to_string(), Origin::CommandLine), ("src.c".to_string(), Origin::CommandLine)]
        );
    }

    #[test]
    fn expand_stops_at_depth_limit_on_cycle() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("loop.resp"), "-DBEFORE @loop.resp").unwrap();

        let sut = args_of(vec!["cc", "@loop.resp", "src.c"], dir.path());

        // The self-reference is left literal once the depth limit is hit; the
        // tokens before it on each level are still emitted, and the trailing
        // source argument is preserved.
        assert_eq!(sut.last().map(String::as_str), Some("src.c"));
        assert!(sut.iter().any(|a| a == "@loop.resp"), "offending token should remain literal: {sut:?}");
        assert!(sut.iter().filter(|a| *a == "-DBEFORE").count() <= MAX_DEPTH);
    }
}
