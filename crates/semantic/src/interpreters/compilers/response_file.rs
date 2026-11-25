// SPDX-License-Identifier: GPL-3.0-or-later

//! Response-file (`@file`) inlining for intercepted compiler invocations.
//!
//! Some build systems (notably Xcode) keep part of a compiler's flags in a
//! separate "response file" and pass it as `@/path/to/args.resp`. Unless the
//! user turns it off (`format.arguments.from_response_files`), the interpreter
//! classifies argv first. Each `@file` it finds is then read with [`read`] and
//! its tokens are classified on their own, so they are link-stripped and split
//! per source like any other argument. Flag values never count as `@file`.
//!
//! Tokenization follows the compiler family Bear already identified for the
//! entry; no new detection is introduced here. See the
//! `output-response-file-inlining` requirement for the contract.

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

/// Reads and tokenizes one `@file` reference.
///
/// `raw_path` is the path as written after the `@`, resolved relative to
/// `working_dir` like the compiler does. `depth` is how many `@file` levels
/// deep this reference is, starting at 1. Returns `None`, with a warning, when
/// the file cannot be read or `depth` is past [`MAX_DEPTH`], so a single stale
/// build artefact never fails the whole database. The caller keeps the
/// reference as an item then.
pub(super) fn read(raw_path: &str, working_dir: &Path, syntax: Syntax, depth: usize) -> Option<Vec<String>> {
    if depth > MAX_DEPTH {
        warn!(
            "response file expansion exceeded depth limit ({MAX_DEPTH}); leaving @{raw_path} literal (possible cycle)"
        );
        return None;
    }

    let path = resolve(working_dir, raw_path);
    match std::fs::read_to_string(&path) {
        Ok(content) => Some(tokenize(&content, syntax)),
        Err(error) => {
            warn!("cannot read response file {}: {error}; leaving @{raw_path} literal", path.display());
            None
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
    fn read_tokenizes_a_file_relative_to_the_working_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("flags.resp"), "-I/opt/include -DEXTRA=2").unwrap();

        let sut = read("flags.resp", dir.path(), Syntax::GnuClang, 1);

        assert_eq!(sut, Some(vec!["-I/opt/include".to_string(), "-DEXTRA=2".to_string()]));
    }

    #[test]
    fn read_uses_absolute_paths_as_is() {
        let dir = tempfile::tempdir().unwrap();
        let resp = dir.path().join("abs.resp");
        fs::write(&resp, "-DONLY").unwrap();

        let sut = read(&resp.display().to_string(), Path::new("/elsewhere"), Syntax::GnuClang, 1);

        assert_eq!(sut, Some(vec!["-DONLY".to_string()]));
    }

    #[test]
    fn read_gives_none_for_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();

        let sut = read("gone.resp", dir.path(), Syntax::GnuClang, 1);

        assert_eq!(sut, None);
    }

    #[test]
    fn read_stops_past_the_depth_limit() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.resp"), "-DA").unwrap();

        assert!(read("a.resp", dir.path(), Syntax::GnuClang, MAX_DEPTH).is_some());
        assert_eq!(read("a.resp", dir.path(), Syntax::GnuClang, MAX_DEPTH + 1), None);
    }
}
