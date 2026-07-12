// SPDX-License-Identifier: GPL-3.0-or-later

//! Folds the token stream (see [`super::tokenizer`]) into
//! [`intercept::Execution`] values, as a streaming iterator: each event is
//! yielded as soon as its line completes, nothing is accumulated across
//! lines. The parser owns all interpreter state: the working directory
//! (driven by `cd` and by the recursive-make `Entering/Leaving directory`
//! markers), the per-command environment overlay, and the positional
//! classification of words (leading `VAR=value` assignments, keywords and
//! group braces in command position). It never looks at characters:
//! everything lexical was resolved by the tokenizer.
//!
//! No ambient I/O: the parser reads only the supplied reader, never
//! `std::env` or `std::fs`. The initial working directory and base
//! environment are inputs supplied by the caller through [`Context`]; the
//! mode layer is responsible for reading them from the real process.

use super::tokenizer::{Marker, SkipReason, Token, TokenError, Tokenizer, Word};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};

/// Inputs the caller controls: the working directory in effect at the
/// start of the input, and the base environment to overlay per-command
/// assignments onto.
pub struct Context {
    pub working_dir: PathBuf,
    pub environment: HashMap<String, String>,
}

/// One line that was not turned into an execution event, with the reason
/// and the absolute physical line it started on.
pub struct SkippedLine {
    pub line: usize,
    pub reason: SkipReason,
}

/// One streamed result of parsing: a recognized command, or a line that
/// was loudly skipped. Both are ordinary items so the caller can forward
/// executions and report skips in input order without buffering either.
pub enum Event {
    Execution(intercept::Execution),
    Skipped(SkippedLine),
}

/// Parses shell command text from `input`, starting from
/// `context.working_dir` and `context.environment`. Returns an iterator
/// of [`Event`]s; a read failure surfaces as an `Err` item and ends the
/// stream. Memory is bounded by the longest logical line.
pub fn parse<R: io::BufRead>(input: R, context: &Context) -> Parser<'_, R> {
    Parser {
        tokens: Tokenizer::new(input),
        working_dir: context.working_dir.clone(),
        previous_working_dir: None,
        directory_stack: Vec::new(),
        initial_working_dir: context.working_dir.clone(),
        base_environment: &context.environment,
        ready: VecDeque::new(),
        line_executions: Vec::new(),
        draining: false,
        assignments: Vec::new(),
        words: Vec::new(),
        command_line: 0,
        finished: false,
    }
}

/// Shell keywords that put a command in "not a simple command" territory
/// when found in the executable position.
const KEYWORDS: &[&str] =
    &["case", "for", "while", "until", "if", "do", "done", "then", "else", "elif", "fi", "esac", "function"];

/// The streaming parser: an [`Iterator`] of parsed [`Event`]s over the
/// tokenizer's output. A lightweight state machine -- its only buffers are
/// the current command's words, the current line's not-yet-committed
/// executions (a skip must still be able to poison the line), and the
/// `ready` handover queue, all bounded by one logical line.
pub struct Parser<'a, R> {
    tokens: Tokenizer<R>,
    working_dir: PathBuf,
    /// The directory before the last `cd`, so `cd -` can restore it; the
    /// parse-time model of the shell's OLDPWD.
    previous_working_dir: Option<PathBuf>,
    /// Directories pushed by `Entering directory` markers, popped by
    /// `Leaving directory` markers; the best-effort recursive-make model.
    directory_stack: Vec<PathBuf>,
    /// Fallback for an unmatched `Leaving directory` (empty stack).
    initial_working_dir: PathBuf,
    base_environment: &'a HashMap<String, String>,
    /// Events completed and ready to be yielded, in input order.
    ready: VecDeque<Event>,
    /// Executions completed on the current line, not yet committed. A
    /// skip poisons its whole line, so nothing is emitted until the
    /// line's Newline proves the line held no unsupported construct.
    line_executions: Vec<intercept::Execution>,
    /// Set from a skip until the next Newline: the rest of the line is
    /// discarded, and further skips on it are not reported again.
    draining: bool,
    assignments: Vec<(String, String)>,
    words: Vec<String>,
    /// Line of the current command's first word, for parser-side skips.
    command_line: usize,
    /// Set when the token stream ended or a read error was yielded.
    finished: bool,
}

impl<R: io::BufRead> Iterator for Parser<'_, R> {
    type Item = io::Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.ready.pop_front() {
                return Some(Ok(event));
            }
            if self.finished {
                return None;
            }
            match self.tokens.next() {
                None => {
                    self.finished = true;
                    // The tokenizer ends every line with a Newline token,
                    // so these buffers are already committed; drain them
                    // defensively and let the loop yield anything left.
                    self.finish_command();
                    self.commit_line();
                }
                Some(Err(TokenError::Io(error))) => {
                    self.finished = true;
                    return Some(Err(error));
                }
                Some(Err(TokenError::Skip(skip))) => self.skip_line(skip.line, skip.reason),
                Some(Ok(token)) => self.accept(token),
            }
        }
    }
}

impl<R> Parser<'_, R> {
    fn accept(&mut self, token: Token) {
        match token {
            Token::Newline => {
                self.finish_command();
                self.commit_line();
                self.draining = false;
            }
            Token::Separator => {
                if !self.draining {
                    self.finish_command();
                }
            }
            Token::Marker(marker) => self.apply_marker(marker),
            Token::Word(word) => {
                if !self.draining {
                    self.accept_word(word);
                }
            }
        }
    }

    /// The line held no unsupported construct: its executions become
    /// yieldable events.
    fn commit_line(&mut self) {
        for execution in self.line_executions.drain(..) {
            self.ready.push_back(Event::Execution(execution));
        }
    }

    /// Records one loud skip for the current line (the first reason wins)
    /// and discards everything the line produced or still produces: a
    /// construct the parser cannot follow makes the whole line untrusted,
    /// per the contract's "never guesses" clause.
    fn skip_line(&mut self, line: usize, reason: SkipReason) {
        if !self.draining {
            self.ready.push_back(Event::Skipped(SkippedLine { line, reason }));
            self.draining = true;
        }
        self.clear_command();
        self.line_executions.clear();
    }

    fn clear_command(&mut self) {
        self.assignments.clear();
        self.words.clear();
    }

    fn accept_word(&mut self, word: Word) {
        if self.words.is_empty() {
            if let Some((name, value)) = split_assignment(&word.text) {
                self.assignments.push((name, value));
                return;
            }
            // The executable position: the one place where an unexpanded
            // glob or a shell control word means the line is not a simple
            // command.
            if word.has_unquoted_glob {
                self.skip_line(word.line, SkipReason::GlobInExecutable);
                return;
            }
            if KEYWORDS.contains(&word.text.as_str()) {
                self.skip_line(word.line, SkipReason::Keyword);
                return;
            }
            if word.text == "{" || word.text == "}" {
                self.skip_line(word.line, SkipReason::Subshell);
                return;
            }
            self.command_line = word.line;
        }
        self.words.push(word.text);
    }

    /// Completes the current command: `cd` mutates the working directory
    /// and produces no event (it is a shell builtin that never reaches
    /// `exec()`), an assignment-only or empty segment produces nothing,
    /// and everything else becomes an execution on the line buffer.
    fn finish_command(&mut self) {
        if self.words.is_empty() {
            self.assignments.clear();
            return;
        }
        if self.words[0] == "cd" {
            self.apply_cd();
            self.clear_command();
            return;
        }
        let execution = self.build_execution();
        self.line_executions.push(execution);
        self.clear_command();
    }

    fn build_execution(&mut self) -> intercept::Execution {
        let mut environment = self.base_environment.clone();
        for (name, value) in self.assignments.drain(..) {
            environment.insert(name, value);
        }
        let executable = PathBuf::from(&self.words[0]);
        intercept::Execution {
            executable,
            arguments: std::mem::take(&mut self.words),
            working_dir: self.working_dir.clone(),
            environment,
        }
    }

    /// A bare `cd` leaves the model unchanged (the real target would be
    /// `$HOME`, which an expansion-bearing line never reaches anyway).
    /// `cd -` restores the previous directory when one is known and is a
    /// loud skip otherwise -- the `-` is an OLDPWD reference, never a
    /// directory name to join.
    fn apply_cd(&mut self) {
        let Some(target) = self.words.get(1) else {
            return;
        };
        if target == "-" {
            match self.previous_working_dir.take() {
                Some(previous) => {
                    self.previous_working_dir = Some(self.working_dir.clone());
                    self.working_dir = previous;
                }
                None => self.skip_line(self.command_line, SkipReason::UnsupportedCd),
            }
            return;
        }
        self.previous_working_dir = Some(self.working_dir.clone());
        self.working_dir = resolve_path(&self.working_dir, target);
    }

    fn apply_marker(&mut self, marker: Marker) {
        match marker {
            Marker::Entering(path) => {
                self.directory_stack.push(self.working_dir.clone());
                self.working_dir = PathBuf::from(path);
            }
            Marker::Leaving => {
                self.working_dir =
                    self.directory_stack.pop().unwrap_or_else(|| self.initial_working_dir.clone());
            }
        }
    }
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

/// Lexically resolves `target` against `base`: an absolute target replaces
/// `base` outright, a relative one is joined onto it; `.` components are
/// dropped and `..` pops the previous normal component. Purely textual --
/// no filesystem access, so symlinks are not resolved.
///
/// The result is only ever built from a stack of "normal" (real) path
/// segments; root-ness is tracked separately as a flag rather than as a
/// poppable stack entry. That is what pins `..` at the filesystem root:
/// popping an empty stack is a no-op, so an absolute base can never be
/// popped past its root (real `cd /..` stays at `/`, it does not become a
/// relative or empty path) while `cd /abs/x` still replaces the base
/// outright.
fn resolve_path(base: &Path, target: &str) -> PathBuf {
    let target_path = Path::new(target);
    let target_is_absolute = target_path.is_absolute();
    let result_is_absolute = target_is_absolute || base.is_absolute();

    let mut stack: Vec<std::ffi::OsString> = Vec::new();
    if !target_is_absolute {
        push_components(base, &mut stack);
    }
    push_components(target_path, &mut stack);

    let mut resolved = if result_is_absolute {
        PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
    } else {
        PathBuf::new()
    };
    for part in &stack {
        resolved.push(part);
    }
    resolved
}

/// Folds `path`'s components onto `stack`: `.` is dropped, `..` pops the
/// previous normal segment (a no-op on an empty stack), and root/prefix
/// markers contribute nothing (root-ness is tracked separately by the
/// caller).
fn push_components(path: &Path, stack: &mut Vec<std::ffi::OsString>) {
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                stack.pop();
            }
            std::path::Component::Normal(part) => stack.push(part.to_os_string()),
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The collected form of a whole parse, for assertion convenience;
    /// production callers consume [`parse`] incrementally instead.
    struct Interpretation {
        executions: Vec<intercept::Execution>,
        skipped: Vec<SkippedLine>,
    }

    fn interpret(input: &str, context: &Context) -> Interpretation {
        let mut executions = Vec::new();
        let mut skipped = Vec::new();
        for event in parse(input.as_bytes(), context) {
            match event.expect("an in-memory reader cannot fail") {
                Event::Execution(execution) => executions.push(execution),
                Event::Skipped(line) => skipped.push(line),
            }
        }
        Interpretation { executions, skipped }
    }

    fn context(working_dir: &str, environment: &[(&str, &str)]) -> Context {
        Context {
            working_dir: PathBuf::from(working_dir),
            environment: environment.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    fn execution(
        executable: &str,
        arguments: &[&str],
        working_dir: &str,
        environment: &[(&str, &str)],
    ) -> intercept::Execution {
        intercept::Execution {
            executable: PathBuf::from(executable),
            arguments: arguments.iter().map(|a| a.to_string()).collect(),
            working_dir: PathBuf::from(working_dir),
            environment: environment.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    /// Shorthand for the common shape: commands from `/build` with an
    /// empty base environment, asserting only the argument vectors.
    fn arguments_of(input: &str) -> Vec<Vec<String>> {
        let sut = interpret(input, &context("/build", &[]));
        assert!(sut.skipped.is_empty(), "unexpected skips for {input:?}");
        sut.executions.into_iter().map(|e| e.arguments).collect()
    }

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn events_stream_in_input_order() {
        let input = "gcc -c a.c\ngcc $X b.c\ngcc -c c.c\n";

        let sut: Vec<Event> = parse(input.as_bytes(), &context("/build", &[]))
            .map(|event| event.expect("an in-memory reader cannot fail"))
            .collect();

        assert_eq!(sut.len(), 3);
        assert!(matches!(&sut[0], Event::Execution(e) if e.arguments == args(&["gcc", "-c", "a.c"])));
        assert!(matches!(&sut[1], Event::Skipped(s) if s.line == 2));
        assert!(matches!(&sut[2], Event::Execution(e) if e.arguments == args(&["gcc", "-c", "c.c"])));
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn a_read_failure_surfaces_as_the_last_event() {
        struct FailingReader;

        impl std::io::Read for FailingReader {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("read failure"))
            }
        }

        let context = context("/build", &[]);
        let mut sut = parse(std::io::BufReader::new(FailingReader), &context);

        assert!(matches!(sut.next(), Some(Err(_))));
        assert!(sut.next().is_none(), "the stream must be over after a read error");
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn emits_one_execution_per_simple_command() {
        let sut = interpret("mv objs/a.o a.lo", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("mv", &["mv", "objs/a.o", "a.lo"], "/build", &[])]);
        assert!(sut.skipped.is_empty());
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn emits_execution_for_a_compile_command() {
        let sut = interpret("gcc -c -o foo.o foo.c", &context("/build", &[]));

        assert_eq!(
            sut.executions,
            vec![execution("gcc", &["gcc", "-c", "-o", "foo.o", "foo.c"], "/build", &[])]
        );
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn resolves_quotes_escapes_and_continuations_into_arguments() {
        let cases: Vec<(&str, Vec<Vec<String>>)> = vec![
            (
                "gcc -O3 -I. -include zconf.h -c -o adler32.o ../zlib-1.3.1/adler32.c",
                vec![args(&[
                    "gcc",
                    "-O3",
                    "-I.",
                    "-include",
                    "zconf.h",
                    "-c",
                    "-o",
                    "adler32.o",
                    "../zlib-1.3.1/adler32.c",
                ])],
            ),
            ("ar rc libz.a a.o b.o ", vec![args(&["ar", "rc", "libz.a", "a.o", "b.o"])]),
            ("gcc -DNAME='a b' foo.c", vec![args(&["gcc", "-DNAME=a b", "foo.c"])]),
            ("gcc \"-DX=y z\" foo.c", vec![args(&["gcc", "-DX=y z", "foo.c"])]),
            ("gcc foo\\ bar.c", vec![args(&["gcc", "foo bar.c"])]),
            ("gcc -c \\\n foo.c", vec![args(&["gcc", "-c", "foo.c"])]),
            ("gcc foo.c # build it", vec![args(&["gcc", "foo.c"])]),
            ("# comment", vec![]),
            ("", vec![]),
            ("   \n  \n", vec![]),
            ("gcc X=y foo.c", vec![args(&["gcc", "X=y", "foo.c"])]),
            ("gcc *.c", vec![args(&["gcc", "*.c"])]),
            ("gcc '$CFLAGS' foo.c", vec![args(&["gcc", "$CFLAGS", "foo.c"])]),
        ];

        for (input, expected) in cases {
            let sut = arguments_of(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn splits_commands_on_separators() {
        let cases: Vec<(&str, Vec<Vec<String>>)> = vec![
            (
                "mkdir objs 2>/dev/null || test -d objs",
                vec![args(&["mkdir", "objs"]), args(&["test", "-d", "objs"])],
            ),
            ("echo a; echo b", vec![args(&["echo", "a"]), args(&["echo", "b"])]),
            ("echo a\necho b", vec![args(&["echo", "a"]), args(&["echo", "b"])]),
            ("echo a && echo b", vec![args(&["echo", "a"]), args(&["echo", "b"])]),
            ("echo a | echo b", vec![args(&["echo", "a"]), args(&["echo", "b"])]),
            ("echo a &", vec![args(&["echo", "a"])]),
        ];

        for (input, expected) in cases {
            let sut = arguments_of(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn strips_redirections_from_arguments() {
        let cases: Vec<(&str, Vec<Vec<String>>)> = vec![
            ("cmd < input.txt", vec![args(&["cmd"])]),
            ("cmd >> log.txt", vec![args(&["cmd"])]),
            ("cmd 1>out 2>err", vec![args(&["cmd"])]),
            ("cmd >&2", vec![args(&["cmd"])]),
            ("cmd <<< input", vec![args(&["cmd"])]),
            ("cmd >| out.txt", vec![args(&["cmd"])]),
        ];

        for (input, expected) in cases {
            let sut = arguments_of(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn skips_unsupported_constructs_with_the_right_reason() {
        let cases: Vec<(&str, SkipReason)> = vec![
            ("gcc $(pkg-config --cflags x) foo.c", SkipReason::CommandSubstitution),
            ("gcc `date`", SkipReason::CommandSubstitution),
            ("gcc $CFLAGS foo.c", SkipReason::ParameterExpansion),
            ("gcc ${CFLAGS} foo.c", SkipReason::ParameterExpansion),
            ("*.sh arg", SkipReason::GlobInExecutable),
            ("cat <<EOF", SkipReason::HereDoc),
            ("(ranlib libz.a || true) >/dev/null 2>&1", SkipReason::Subshell),
            ("{ echo a; }", SkipReason::Subshell),
            ("gcc 'unterminated", SkipReason::UnterminatedQuote),
            ("gcc \"also unterminated", SkipReason::UnterminatedQuote),
        ];

        for (input, reason) in cases {
            let sut = interpret(input, &context("/build", &[]));
            assert!(sut.executions.is_empty(), "no executions expected for {input:?}");
            assert_eq!(sut.skipped.len(), 1, "exactly one skip expected for {input:?}");
            assert_eq!(sut.skipped[0].reason, reason, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn a_line_with_several_unsupported_constructs_is_one_skip() {
        // Two keyword segments (`for ...; do`) and a `; done` would each
        // have been a skip per segment; the line-granular rule reports the
        // line once, with the first reason.
        let sut = interpret("for i in a b; do", &context("/build", &[]));

        assert!(sut.executions.is_empty());
        assert_eq!(sut.skipped.len(), 1, "one skip per line, not one per segment");
        assert_eq!(sut.skipped[0].reason, SkipReason::Keyword);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn a_skip_poisons_the_whole_line_in_both_directions() {
        // The contract skips the whole line, so a supported command
        // sharing a line with an unsupported construct is dropped no
        // matter which side of the separator it is on.
        let cases: Vec<&str> = vec!["gcc -c ok.c && gcc $X bad.c", "gcc $X bad.c && gcc -c ok.c"];

        for input in cases {
            let sut = interpret(input, &context("/build", &[]));
            assert!(sut.executions.is_empty(), "whole line must be dropped for {input:?}");
            assert_eq!(sut.skipped.len(), 1, "case: {input:?}");
            assert_eq!(sut.skipped[0].reason, SkipReason::ParameterExpansion, "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    // `cd`/`..` resolution goes through `std::path`, so these POSIX
    // absolute-path assertions hold on Unix only; on Windows the same
    // input yields backslash-separated, drive-relative paths. parse-sh
    // targets POSIX `sh` dry-run output, so Unix is where this matters.
    #[cfg(unix)]
    #[test]
    fn cd_on_same_line_affects_the_following_command_only() {
        let sut = interpret("cd sub && gcc -c foo.c", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build/sub", &[])]);
        assert!(sut.skipped.is_empty());
    }

    // Requirements: interception-events-from-shell-text
    #[cfg(unix)]
    #[test]
    fn cd_persists_across_logical_lines() {
        let sut = interpret("cd sub\ngcc -c foo.c", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build/sub", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[cfg(unix)]
    #[test]
    fn relative_cd_with_dotdot_normalizes_lexically() {
        let sut = interpret("cd ../bar && gcc x.c", &context("/build/foo", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "x.c"], "/build/bar", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[cfg(unix)]
    #[test]
    fn cd_dotdot_stays_pinned_at_the_filesystem_root() {
        let cases: Vec<(&str, &str, &str)> = vec![
            ("/build/foo", "cd ../bar && gcc x.c", "/build/bar"),
            ("/build", "cd ../../other && gcc x.c", "/other"),
            ("/", "cd .. && gcc x.c", "/"),
            ("/build", "cd /abs/x && gcc x.c", "/abs/x"),
            ("/build", "cd . && gcc x.c", "/build"),
        ];

        for (working_dir, input, expected_dir) in cases {
            let sut = interpret(input, &context(working_dir, &[]));
            assert_eq!(
                sut.executions,
                vec![execution("gcc", &["gcc", "x.c"], expected_dir, &[])],
                "case: {input:?} from {working_dir:?}"
            );
        }
    }

    // Requirements: interception-events-from-shell-text
    #[cfg(unix)]
    #[test]
    fn cd_dash_restores_the_previous_working_directory() {
        let sut = interpret("cd sub\ncd -\ngcc x.c", &context("/build", &[]));

        assert_eq!(
            sut.executions,
            vec![execution("gcc", &["gcc", "x.c"], "/build", &[])],
            "cd - must restore the previous directory, never join a l l `-`"
        );
        assert!(sut.skipped.is_empty());
    }

    // Requirements: interception-events-from-shell-text
    #[cfg(unix)]
    #[test]
    fn cd_dash_toggles_between_the_last_two_directories() {
        let sut = interpret("cd sub\ncd -\ncd -\ngcc x.c", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "x.c"], "/build/sub", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[cfg(unix)]
    #[test]
    fn cd_dash_without_a_previous_directory_is_a_loud_skip() {
        let sut = interpret("cd -\ngcc x.c", &context("/build", &[]));

        assert_eq!(
            sut.executions,
            vec![execution("gcc", &["gcc", "x.c"], "/build", &[])],
            "the working directory must stay unchanged"
        );
        assert_eq!(sut.skipped.len(), 1);
        assert_eq!(sut.skipped[0].reason, SkipReason::UnsupportedCd);
        assert_eq!(sut.skipped[0].line, 1);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn make_markers_push_and_pop_the_working_directory() {
        let input = "make[1]: Entering directory '/build/lib'\n\
                     gcc -c a.c\n\
                     make[1]: Leaving directory '/build/lib'\n\
                     gcc -c b.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(
            sut.executions,
            vec![
                execution("gcc", &["gcc", "-c", "a.c"], "/build/lib", &[]),
                execution("gcc", &["gcc", "-c", "b.c"], "/build", &[]),
            ]
        );
        assert!(sut.skipped.is_empty(), "markers must not be reported as skips");
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn recognizes_entering_marker_without_job_number() {
        let input = "make: Entering directory `/build/lib'\ngcc -c a.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "a.c"], "/build/lib", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn leaving_marker_with_empty_stack_falls_back_to_initial_working_dir() {
        let input = "make[1]: Leaving directory '/build/lib'\ngcc -c a.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "a.c"], "/build", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn per_command_assignment_overlays_the_base_environment() {
        let sut = interpret("CC=gcc gcc -c foo.c", &context("/build", &[("PATH", "/usr/bin")]));

        assert_eq!(
            sut.executions,
            vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build", &[("PATH", "/usr/bin"), ("CC", "gcc")])]
        );
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn a_plain_command_keeps_the_base_environment_unchanged() {
        let sut = interpret("gcc -c foo.c", &context("/build", &[("PATH", "/usr/bin")]));

        assert_eq!(
            sut.executions,
            vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build", &[("PATH", "/usr/bin")])]
        );
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn skips_propagate_with_the_absolute_physical_line() {
        let input = "gcc -c foo.c\n(ranlib libz.a || true) >/dev/null 2>&1\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build", &[])]);
        assert_eq!(sut.skipped.len(), 1);
        assert_eq!(
            sut.skipped[0].line, 2,
            "skip must report the physical start line, not the lexer's line-1"
        );
        assert_eq!(sut.skipped[0].reason, SkipReason::Subshell);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn a_quoted_value_split_by_a_real_newline_yields_no_fabricated_executions() {
        // `gcc 'a` / `b' foo.c` -- quote state never crosses a newline, so
        // the quote that spans one never gets to close: each half must be
        // skipped loudly as UnterminatedQuote, and neither half may
        // fabricate a command.
        let input = "gcc 'a\nb' foo.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert!(sut.executions.is_empty(), "must not fabricate a command from a split quote");
        assert_eq!(sut.skipped.len(), 2);
        assert_eq!(sut.skipped[0].reason, SkipReason::UnterminatedQuote);
        assert_eq!(sut.skipped[1].reason, SkipReason::UnterminatedQuote);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn an_unterminated_quote_in_a_redirect_target_is_a_loud_skip() {
        let sut = interpret("cmd > 'oops", &context("/build", &[]));

        assert!(sut.executions.is_empty(), "the command before the redirect must not survive");
        assert_eq!(sut.skipped.len(), 1);
        assert_eq!(sut.skipped[0].reason, SkipReason::UnterminatedQuote);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn a_quoted_redirect_target_spanning_a_newline_fabricates_nothing() {
        // The double-quoted target swallows the newline in real sh; here
        // each physical line must be an UnterminatedQuote skip, and the
        // `gcc -c fake.c` text inside the quote must not become an event.
        let input = "cmd > \"a\ngcc -c fake.c > b\"\n";

        let sut = interpret(input, &context("/build", &[]));

        assert!(sut.executions.is_empty(), "must not fabricate commands from quoted redirect text");
        assert_eq!(sut.skipped.len(), 2);
        assert_eq!(sut.skipped[0].line, 1);
        assert_eq!(sut.skipped[0].reason, SkipReason::UnterminatedQuote);
        assert_eq!(sut.skipped[1].line, 2);
        assert_eq!(sut.skipped[1].reason, SkipReason::UnterminatedQuote);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn substitution_in_a_discarded_redirect_target_is_not_a_skip() {
        // The redirect target never reaches the argv, so an unresolved
        // path there loses nothing the database needs: the event is
        // emitted cleanly, and no fragment of the target (even one with
        // a space inside the substitution) leaks into the arguments.
        let cases: Vec<&str> = vec!["gcc -c foo.c > $(logdir)/x.log", "gcc -c foo.c > $(dirname x)/log"];

        for input in cases {
            let sut = interpret(input, &context("/build", &[]));
            assert_eq!(
                sut.executions,
                vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build", &[])],
                "case: {input:?}"
            );
            assert!(sut.skipped.is_empty(), "case: {input:?}");
        }
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn an_unterminated_substitution_in_a_redirect_target_is_a_loud_skip() {
        let sut = interpret("cmd > $(oops\ngcc -c a.c\n", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "a.c"], "/build", &[])]);
        assert_eq!(sut.skipped.len(), 1);
        assert_eq!(sut.skipped[0].line, 1);
        assert_eq!(sut.skipped[0].reason, SkipReason::UnterminatedQuote);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn all_skipped_input_yields_no_executions() {
        let input = "(ranlib libz.a || true) >/dev/null 2>&1\ngcc $CFLAGS foo.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert!(sut.executions.is_empty());
        assert_eq!(sut.skipped.len(), 2);
        assert_eq!(sut.skipped[0].reason, SkipReason::Subshell);
        assert_eq!(sut.skipped[1].reason, SkipReason::ParameterExpansion);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn heredoc_body_lines_are_swallowed_not_fabricated_into_commands() {
        let input = "cat <<EOF\nsome random text\nEOF\ngcc -c a.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "a.c"], "/build", &[])]);
        assert_eq!(sut.skipped.len(), 1, "only the `<<` line itself is a loud skip");
        assert_eq!(sut.skipped[0].reason, SkipReason::HereDoc);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn heredoc_with_quoted_delimiter_swallows_its_body() {
        let input = "cat <<'EOF'\n$notexpanded\nEOF\ngcc -c b.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "b.c"], "/build", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn a_space_indented_delimiter_line_is_body_not_terminator() {
        // Real sh ends a `<<EOF` body only at an undecorated delimiter
        // line; `  EOF` and `echo pwned` are body text and must not be
        // tokenized into commands.
        let input = "cat <<EOF\n  EOF\necho pwned\nEOF\ngcc -c a.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "a.c"], "/build", &[])]);
        assert_eq!(sut.skipped.len(), 1);
        assert_eq!(sut.skipped[0].reason, SkipReason::HereDoc);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn dash_heredoc_matches_a_tab_indented_terminator() {
        let input = "cat <<-EOF\n\tbody\n\tEOF\ngcc -c c.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "c.c"], "/build", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn herestring_is_not_mistaken_for_a_heredoc() {
        let sut = interpret("cmd <<< input\n", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("cmd", &["cmd"], "/build", &[])]);
        assert!(sut.skipped.is_empty());
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn heredoc_inside_a_trailing_comment_swallows_nothing() {
        let input = "gcc -c a.c # see <<EOF docs\ngcc -c b.c\ngcc -c c.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(
            sut.executions,
            vec![
                execution("gcc", &["gcc", "-c", "a.c"], "/build", &[]),
                execution("gcc", &["gcc", "-c", "b.c"], "/build", &[]),
                execution("gcc", &["gcc", "-c", "c.c"], "/build", &[]),
            ]
        );
        assert!(sut.skipped.is_empty());
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn heredoc_inside_a_subshell_does_not_swallow_the_rest_of_the_input() {
        let input = "(cat <<EOF)\ngcc -c real.c\nEOF\ngcc -c after.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert_eq!(
            sut.executions,
            vec![execution("gcc", &["gcc", "-c", "after.c"], "/build", &[])],
            "the delimiter is EOF, not EOF), so the body ends at line 3"
        );
        assert_eq!(sut.skipped.len(), 1);
        assert_eq!(sut.skipped[0].reason, SkipReason::Subshell);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn clobber_redirect_does_not_fabricate_an_execution() {
        let sut = interpret("cc -c x.c >| out.txt", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("cc", &["cc", "-c", "x.c"], "/build", &[])]);
        assert!(sut.skipped.is_empty());
    }
}
