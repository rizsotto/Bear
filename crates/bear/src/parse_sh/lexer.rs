// SPDX-License-Identifier: GPL-3.0-or-later

//! A pure lexer for the POSIX `sh` subset described by
//! `docs/requirements/interception-events-from-shell-text.md`. No I/O: text
//! in, structured tokens out. The mode that reads stdin/files and folds this
//! output into execution events lives in a later stage.

use std::fmt;

/// One simple command: a leading run of `VAR=value` assignments, followed
/// by the executable and its arguments (redirections already stripped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCommand {
    pub assignments: Vec<(String, String)>,
    pub words: Vec<String>,
    /// 1-based line where the command starts, for diagnostics.
    pub line: usize,
}

/// Why a whole logical command was skipped instead of lexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// `( ... )` subshell or `{ ... }` group starting a command.
    Subshell,
    /// Backtick or `$( ... )` command substitution.
    CommandSubstitution,
    /// `$VAR` or `${...}` parameter expansion.
    ParameterExpansion,
    /// `*`, `?`, or `[` in the executable word.
    GlobInExecutable,
    /// `<<` here-document.
    HereDoc,
    /// A shell keyword (`case`, `for`, `while`, ...) in command position.
    Keyword,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Subshell => "subshell or group",
            Self::CommandSubstitution => "command substitution",
            Self::ParameterExpansion => "parameter expansion",
            Self::GlobInExecutable => "glob in executable",
            Self::HereDoc => "here-document",
            Self::Keyword => "shell keyword",
        };
        f.write_str(text)
    }
}

/// One lexed unit: either a simple command, or a loud skip with a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexedCommand {
    Command(SimpleCommand),
    Skipped { line: usize, reason: SkipReason },
}

/// Shell keywords that put a command in "not a simple command" territory
/// when found in the executable position.
const KEYWORDS: &[&str] =
    &["case", "for", "while", "until", "if", "do", "done", "then", "else", "elif", "fi", "esac", "function"];

/// Lexes shell command text into a flat, ordered list of simple commands or
/// loud skips. Comments and blank (or comment-only) segments produce
/// nothing at all.
pub fn lex(input: &str) -> Vec<LexedCommand> {
    let chars = strip_line_continuations(input);
    Scanner::new(&chars).run()
}

/// Removes backslash-newline line continuations, joining physical lines,
/// while tracking the physical line number of every remaining character.
///
/// Continuations are stripped textually regardless of quote context: real
/// `sh` would treat a backslash-newline inside single quotes as a literal
/// two-character sequence, not a continuation, but that distinction is not
/// exercised by the dry-run text this lexer targets, so we take the simpler
/// unconditional rule rather than threading quote state through this
/// preprocessing pass.
fn strip_line_continuations(input: &str) -> Vec<(char, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut line = 1usize;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && chars.get(i + 1) == Some(&'\n') {
            // The continuation itself contributes no character; the newline
            // it swallows still advances the physical line counter.
            line += 1;
            i += 2;
            continue;
        }
        if chars[i] == '\n' {
            out.push((chars[i], line));
            line += 1;
            i += 1;
            continue;
        }
        out.push((chars[i], line));
        i += 1;
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    None,
    Single,
    Double,
}

/// A word as read by [`Scanner::read_word`], carrying the flags needed to
/// classify it (assignment / executable / skip) without a second pass.
struct Word {
    text: String,
    /// Set when an unescaped, non-single-quoted `$` (other than `$(`) or an
    /// unescaped, non-single-quoted backtick / `$(` was seen in this word.
    skip: Option<SkipReason>,
    /// Set when `*`, `?`, or `[` appeared while genuinely unquoted.
    has_unquoted_glob: bool,
}

/// Splits a fully-resolved word into a `(name, value)` assignment pair when
/// it looks like `IDENT=value`. Operates on the already quote-resolved text:
/// a valid assignment's name portion can never itself contain quotes, so
/// checking the resolved text is equivalent to checking the raw prefix.
fn split_assignment(text: &str) -> Option<(String, String)> {
    let eq = text.find('=')?;
    let name = &text[..eq];
    let mut chars = name.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some((name.to_string(), text[eq + 1..].to_string()))
}

struct Scanner<'a> {
    chars: &'a [(char, usize)],
    pos: usize,
    out: Vec<LexedCommand>,
}

impl<'a> Scanner<'a> {
    fn new(chars: &'a [(char, usize)]) -> Self {
        Self { chars, pos: 0, out: Vec::new() }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|&(c, _)| c)
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).map(|&(c, _)| c)
    }

    fn peek_line(&self) -> Option<usize> {
        self.chars.get(self.pos).map(|&(_, l)| l)
    }

    fn advance(&mut self) -> Option<(char, usize)> {
        let item = self.chars.get(self.pos).copied();
        if item.is_some() {
            self.pos += 1;
        }
        item
    }

    /// Width in characters of a top-level command separator at the current
    /// position (`;`, newline, `&&`, `||`, `&`, `|`), or `None`.
    fn at_top_level_separator(&self) -> Option<usize> {
        match self.peek() {
            Some(';') | Some('\n') => Some(1),
            Some('&') => Some(if self.peek_at(1) == Some('&') { 2 } else { 1 }),
            Some('|') => Some(if self.peek_at(1) == Some('|') { 2 } else { 1 }),
            _ => None,
        }
    }

    fn run(mut self) -> Vec<LexedCommand> {
        loop {
            self.skip_command_separators();
            if self.peek().is_none() {
                break;
            }
            self.scan_command();
        }
        self.out
    }

    /// Consumes whitespace and any run of top-level separators between
    /// commands (covers blank lines and repeated separators alike).
    fn skip_command_separators(&mut self) {
        loop {
            if matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.advance();
                continue;
            }
            if let Some(width) = self.at_top_level_separator() {
                for _ in 0..width {
                    self.advance();
                }
                continue;
            }
            break;
        }
    }

    /// Scans one candidate command up to (not including) the next top-level
    /// separator or end of input, and pushes the resulting `Command` or
    /// `Skipped` onto `out`. A comment-only or blank segment pushes nothing.
    fn scan_command(&mut self) {
        let cmd_line = self.peek_line();
        let mut assignments: Vec<(String, String)> = Vec::new();
        let mut words: Vec<String> = Vec::new();
        let mut executable_seen = false;
        let mut skip: Option<SkipReason> = None;

        loop {
            if self.peek().is_none() || self.at_top_level_separator().is_some() {
                break;
            }
            if matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.advance();
                continue;
            }
            if self.peek() == Some('#') {
                // A '#' at a word boundary starts a comment to end of line;
                // '#' inside an already-started word is handled inside
                // read_word and never reaches here.
                self.skip_to_eol();
                break;
            }
            if words.is_empty() && matches!(self.peek(), Some('(') | Some('{')) {
                // `(` or `{` starting a command: subshell / group. See the
                // module report for why this takes precedence over treating
                // `{` as a Keyword.
                let open = self.peek().expect("guarded by matches! above");
                self.advance();
                skip.get_or_insert(SkipReason::Subshell);
                self.skip_balanced(open);
                continue;
            }
            if self.looks_like_redirect_start() {
                self.scan_redirect(&mut skip);
                continue;
            }

            let word = self.read_word();
            if let Some(reason) = word.skip {
                skip.get_or_insert(reason);
            }
            if !executable_seen {
                if let Some((name, value)) = split_assignment(&word.text) {
                    assignments.push((name, value));
                    continue;
                }
                executable_seen = true;
                if skip.is_none() {
                    if word.has_unquoted_glob {
                        skip = Some(SkipReason::GlobInExecutable);
                    } else if KEYWORDS.contains(&word.text.as_str()) {
                        skip = Some(SkipReason::Keyword);
                    }
                }
            }
            words.push(word.text);
        }

        if let Some(reason) = skip {
            // Structurally guaranteed: skip is only ever set after at least
            // one character of this command has been consumed.
            let line = cmd_line.expect("skip implies a first character was read");
            self.out.push(LexedCommand::Skipped { line, reason });
        } else if !words.is_empty() {
            let line = cmd_line.expect("non-empty words implies a first character was read");
            self.out.push(LexedCommand::Command(SimpleCommand { assignments, words, line }));
        }
        // Blank, comment-only, or assignment-only (no executable) segments
        // produce nothing: there is no command to report an event for.
    }

    fn skip_to_eol(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// Consumes a `(` / `{` ... `)` / `}` span, tracking nesting depth and
    /// quote state so that separators inside it (e.g. `||` inside a
    /// subshell) are not mistaken for top-level separators. The caller has
    /// already consumed the opening character.
    fn skip_balanced(&mut self, open: char) {
        let close = if open == '(' { ')' } else { '}' };
        let mut depth = 1usize;
        let mut quote = Quote::None;
        while let Some(c) = self.peek() {
            match quote {
                Quote::Single => {
                    self.advance();
                    if c == '\'' {
                        quote = Quote::None;
                    }
                }
                Quote::Double => {
                    if c == '\\' {
                        self.advance();
                        self.advance();
                    } else {
                        self.advance();
                        if c == '"' {
                            quote = Quote::None;
                        }
                    }
                }
                Quote::None => {
                    if c == '\\' {
                        self.advance();
                        self.advance();
                        continue;
                    }
                    self.advance();
                    match c {
                        '\'' => quote = Quote::Single,
                        '"' => quote = Quote::Double,
                        c if c == open => depth += 1,
                        c if c == close => {
                            depth -= 1;
                            if depth == 0 {
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // Unterminated group at end of input: nothing more to consume.
    }

    fn looks_like_redirect_start(&self) -> bool {
        match self.peek() {
            Some('<') | Some('>') => true,
            Some(c) if c.is_ascii_digit() => {
                let mut off = 0usize;
                while matches!(self.peek_at(off), Some(c) if c.is_ascii_digit()) {
                    off += 1;
                }
                let _ = c;
                matches!(self.peek_at(off), Some('<') | Some('>'))
            }
            _ => false,
        }
    }

    /// Consumes one redirection (optional fd-prefix, operator, and target or
    /// dup-form) and strips it entirely: nothing from a redirect is ever
    /// added to `words`. `<<<` (herestring) is treated as an ordinary
    /// redirect to strip; only `<<` (here-document) sets `skip`.
    fn scan_redirect(&mut self, skip: &mut Option<SkipReason>) {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }

        let op = if self.peek() == Some('<') && self.peek_at(1) == Some('<') && self.peek_at(2) == Some('<') {
            self.advance();
            self.advance();
            self.advance();
            "<<<"
        } else if self.peek() == Some('<') && self.peek_at(1) == Some('<') {
            self.advance();
            self.advance();
            "<<"
        } else if self.peek() == Some('>') && self.peek_at(1) == Some('>') {
            self.advance();
            self.advance();
            ">>"
        } else if self.peek() == Some('<') {
            self.advance();
            "<"
        } else {
            // Only '<' and '>' (plus their doubled/tripled forms above) can
            // reach here: looks_like_redirect_start already checked this.
            self.advance();
            ">"
        };

        if op == "<<" {
            skip.get_or_insert(SkipReason::HereDoc);
            self.skip_optional_target();
            return;
        }

        // Dup form: `N>&M`, `>&2`, `>&-` -- the whole token is the redirect,
        // no separate target word follows.
        if self.peek() == Some('&') {
            self.advance();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.advance();
            }
            if self.peek() == Some('-') {
                self.advance();
            }
            return;
        }

        self.skip_optional_target();
    }

    /// Consumes and discards a redirect's target word, whether fused
    /// (`>file`) or separated by whitespace (`> file`). Constructs inside a
    /// discarded target (e.g. a stray `$(...)`) are not inspected: the
    /// target never reaches `words`, so there is nothing to guess at.
    fn skip_optional_target(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace() && c != '\n') {
            self.advance();
        }
        if self.peek().is_some() && self.at_top_level_separator().is_none() && self.peek() != Some('#') {
            let _ = self.read_word();
        }
    }

    /// Reads one whitespace/metacharacter-delimited word, resolving quotes
    /// and backslash escapes, and flagging an unsupported construct or an
    /// unquoted glob character as it goes. Assumes the caller has already
    /// ruled out comment, redirect, and subshell starts at this position.
    fn read_word(&mut self) -> Word {
        let mut text = String::new();
        let mut skip: Option<SkipReason> = None;
        let mut has_unquoted_glob = false;
        let mut quote = Quote::None;

        while let Some(c) = self.peek() {
            match quote {
                Quote::Single => {
                    self.advance();
                    if c == '\'' {
                        quote = Quote::None;
                    } else {
                        text.push(c);
                    }
                }
                Quote::Double => match c {
                    '"' => {
                        self.advance();
                        quote = Quote::None;
                    }
                    '\\' => {
                        self.advance();
                        if let Some((next, _)) = self.advance() {
                            text.push(next);
                        }
                    }
                    '$' => {
                        let reason = if self.peek_at(1) == Some('(') {
                            SkipReason::CommandSubstitution
                        } else {
                            SkipReason::ParameterExpansion
                        };
                        skip.get_or_insert(reason);
                        self.advance();
                        text.push(c);
                    }
                    '`' => {
                        skip.get_or_insert(SkipReason::CommandSubstitution);
                        self.advance();
                        text.push(c);
                    }
                    _ => {
                        self.advance();
                        text.push(c);
                    }
                },
                Quote::None => {
                    if c.is_whitespace() || matches!(c, ';' | '&' | '|' | '<' | '>') {
                        break;
                    }
                    match c {
                        '\'' => {
                            self.advance();
                            quote = Quote::Single;
                        }
                        '"' => {
                            self.advance();
                            quote = Quote::Double;
                        }
                        '\\' => {
                            self.advance();
                            if let Some((next, _)) = self.advance() {
                                text.push(next);
                            }
                        }
                        '$' => {
                            let reason = if self.peek_at(1) == Some('(') {
                                SkipReason::CommandSubstitution
                            } else {
                                SkipReason::ParameterExpansion
                            };
                            skip.get_or_insert(reason);
                            self.advance();
                            text.push(c);
                        }
                        '`' => {
                            skip.get_or_insert(SkipReason::CommandSubstitution);
                            self.advance();
                            text.push(c);
                        }
                        '*' | '?' | '[' => {
                            has_unquoted_glob = true;
                            self.advance();
                            text.push(c);
                        }
                        _ => {
                            self.advance();
                            text.push(c);
                        }
                    }
                }
            }
        }

        Word { text, skip, has_unquoted_glob }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(words: &[&str], line: usize) -> LexedCommand {
        LexedCommand::Command(SimpleCommand {
            assignments: Vec::new(),
            words: words.iter().map(|w| w.to_string()).collect(),
            line,
        })
    }

    fn command_with_assignments(assignments: &[(&str, &str)], words: &[&str], line: usize) -> LexedCommand {
        LexedCommand::Command(SimpleCommand {
            assignments: assignments.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            words: words.iter().map(|w| w.to_string()).collect(),
            line,
        })
    }

    fn skipped(line: usize, reason: SkipReason) -> LexedCommand {
        LexedCommand::Skipped { line, reason }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn lexes_single_commands() {
        let cases: Vec<(&str, Vec<LexedCommand>)> = vec![
            (
                "gcc -O3 -I. -include zconf.h -c -o adler32.o ../zlib-1.3.1/adler32.c",
                vec![command(
                    &[
                        "gcc",
                        "-O3",
                        "-I.",
                        "-include",
                        "zconf.h",
                        "-c",
                        "-o",
                        "adler32.o",
                        "../zlib-1.3.1/adler32.c",
                    ],
                    1,
                )],
            ),
            ("ar rc libz.a a.o b.o ", vec![command(&["ar", "rc", "libz.a", "a.o", "b.o"], 1)]),
            (
                "gcc -Wl,-soname,libz.so.1,--version-script,x.map -o libz.so.1 a.lo -lc",
                vec![command(
                    &[
                        "gcc",
                        "-Wl,-soname,libz.so.1,--version-script,x.map",
                        "-o",
                        "libz.so.1",
                        "a.lo",
                        "-lc",
                    ],
                    1,
                )],
            ),
            ("gcc -DNAME='a b' foo.c", vec![command(&["gcc", "-DNAME=a b", "foo.c"], 1)]),
            ("gcc \"-DX=y z\" foo.c", vec![command(&["gcc", "-DX=y z", "foo.c"], 1)]),
            ("gcc foo\\ bar.c", vec![command(&["gcc", "foo bar.c"], 1)]),
            ("gcc -c \\\n foo.c", vec![command(&["gcc", "-c", "foo.c"], 1)]),
            ("gcc foo.c # build it", vec![command(&["gcc", "foo.c"], 1)]),
            ("# comment", vec![]),
            ("", vec![]),
            ("   \n  \n", vec![]),
            (
                "CC=gcc CFLAGS=-O2 gcc -c foo.c",
                vec![command_with_assignments(
                    &[("CC", "gcc"), ("CFLAGS", "-O2")],
                    &["gcc", "-c", "foo.c"],
                    1,
                )],
            ),
            ("gcc X=y foo.c", vec![command(&["gcc", "X=y", "foo.c"], 1)]),
            ("gcc *.c", vec![command(&["gcc", "*.c"], 1)]),
            ("gcc '$CFLAGS' foo.c", vec![command(&["gcc", "$CFLAGS", "foo.c"], 1)]),
        ];

        for (input, expected) in cases {
            let sut = lex(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn splits_on_command_separators() {
        let cases: Vec<(&str, Vec<LexedCommand>)> = vec![
            (
                "mkdir objs 2>/dev/null || test -d objs",
                vec![command(&["mkdir", "objs"], 1), command(&["test", "-d", "objs"], 1)],
            ),
            ("echo a; echo b", vec![command(&["echo", "a"], 1), command(&["echo", "b"], 1)]),
            ("echo a\necho b", vec![command(&["echo", "a"], 1), command(&["echo", "b"], 2)]),
            ("echo a && echo b", vec![command(&["echo", "a"], 1), command(&["echo", "b"], 1)]),
            ("echo a | echo b", vec![command(&["echo", "a"], 1), command(&["echo", "b"], 1)]),
            ("echo a &", vec![command(&["echo", "a"], 1)]),
        ];

        for (input, expected) in cases {
            let sut = lex(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn strips_redirections_from_words() {
        let cases: Vec<(&str, Vec<LexedCommand>)> = vec![
            (
                "CC=gcc gcc -c foo.c -o foo.o >/dev/null 2>&1",
                vec![command_with_assignments(&[("CC", "gcc")], &["gcc", "-c", "foo.c", "-o", "foo.o"], 1)],
            ),
            ("cmd < input.txt", vec![command(&["cmd"], 1)]),
            ("cmd >> log.txt", vec![command(&["cmd"], 1)]),
            ("cmd 1>out 2>err", vec![command(&["cmd"], 1)]),
            ("cmd >&2", vec![command(&["cmd"], 1)]),
            ("cmd <<< input", vec![command(&["cmd"], 1)]),
        ];

        for (input, expected) in cases {
            let sut = lex(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn skips_the_zlib_subshell_line_loudly() {
        let sut = lex("(ranlib libz.a || true) >/dev/null 2>&1");
        assert_eq!(sut, vec![skipped(1, SkipReason::Subshell)], "zlib subshell line");
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn skips_unsupported_constructs_with_the_right_reason() {
        let cases: Vec<(&str, Vec<LexedCommand>)> = vec![
            ("gcc $(pkg-config --cflags x) foo.c", vec![skipped(1, SkipReason::CommandSubstitution)]),
            ("gcc `date`", vec![skipped(1, SkipReason::CommandSubstitution)]),
            ("gcc $CFLAGS foo.c", vec![skipped(1, SkipReason::ParameterExpansion)]),
            ("gcc ${CFLAGS} foo.c", vec![skipped(1, SkipReason::ParameterExpansion)]),
            ("*.sh arg", vec![skipped(1, SkipReason::GlobInExecutable)]),
            ("cat <<EOF", vec![skipped(1, SkipReason::HereDoc)]),
            ("for i in a b; do", vec![skipped(1, SkipReason::Keyword), skipped(1, SkipReason::Keyword)]),
            ("if true; then", vec![skipped(1, SkipReason::Keyword), skipped(1, SkipReason::Keyword)]),
            ("{ echo a; }", vec![skipped(1, SkipReason::Subshell)]),
        ];

        for (input, expected) in cases {
            let sut = lex(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }
}
