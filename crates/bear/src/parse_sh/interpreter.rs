// SPDX-License-Identifier: GPL-3.0-or-later

//! Folds the lexed shell-command stream (see [`crate::parse_sh::lexer`])
//! into [`intercept::Execution`] values, tracking the working directory
//! (via `cd` and recursive-make `Entering/Leaving directory` markers) and
//! fabricating a per-command environment overlay.
//!
//! Pure: no I/O, no `std::env`, no `std::fs`. The initial working directory
//! and base environment are inputs supplied by the caller through
//! [`Context`]; a later stage (not implemented here) is responsible for
//! reading them from the real process.

use super::lexer::{LexedCommand, SkipReason, lex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Inputs the caller controls: the working directory in effect at the
/// start of the input, and the base environment to overlay per-command
/// assignments onto.
pub struct Context {
    pub working_dir: PathBuf,
    pub environment: HashMap<String, String>,
}

/// One line that was not turned into an execution event, with the reason
/// the lexer gave and the absolute physical line it started on.
pub struct SkippedLine {
    pub line: usize,
    pub reason: SkipReason,
}

/// The result of interpreting a whole input: the emitted execution events,
/// in order, plus every skipped line, in order.
pub struct Interpretation {
    pub executions: Vec<intercept::Execution>,
    pub skipped: Vec<SkippedLine>,
}

/// Interprets shell command text into an [`Interpretation`], starting from
/// `context.working_dir` and `context.environment`.
pub fn interpret(input: &str, context: &Context) -> Interpretation {
    let mut state = State {
        working_dir: context.working_dir.clone(),
        directory_stack: Vec::new(),
        base_environment: &context.environment,
        executions: Vec::new(),
        skipped: Vec::new(),
        initial_working_dir: context.working_dir.clone(),
        heredoc_terminator: None,
    };

    for (logical_line, start_line) in logical_lines(input) {
        state.process_logical_line(&logical_line, start_line);
    }

    Interpretation { executions: state.executions, skipped: state.skipped }
}

struct State<'a> {
    working_dir: PathBuf,
    /// Directories pushed by `Entering directory` markers, popped by
    /// `Leaving directory` markers; the best-effort recursive-make model.
    directory_stack: Vec<PathBuf>,
    base_environment: &'a HashMap<String, String>,
    executions: Vec<intercept::Execution>,
    skipped: Vec<SkippedLine>,
    /// Fallback for an unmatched `Leaving directory` (empty stack).
    initial_working_dir: PathBuf,
    /// Set while consuming a here-document body: the delimiter that ends
    /// it. `None` means "not currently inside a here-document".
    heredoc_terminator: Option<String>,
}

impl State<'_> {
    fn process_logical_line(&mut self, logical_line: &str, start_line: usize) {
        if let Some(terminator) = self.heredoc_terminator.take() {
            // Here-document body lines (and the terminator line itself) are
            // data, not commands: they must never reach the lexer, or they
            // would fabricate execution events. Pragmatic match: a plain
            // `.trim()` of the candidate line against the delimiter, which
            // also covers `<<-`'s "strip leading tabs" rule without
            // threading a separate dash flag through the state.
            if logical_line.trim() != terminator {
                self.heredoc_terminator = Some(terminator);
            }
            return;
        }

        if let Some(marker) = parse_make_marker(logical_line) {
            self.apply_marker(marker);
            return;
        }

        if let Some(delimiter) = detect_heredoc(logical_line) {
            self.heredoc_terminator = Some(delimiter);
        }

        for item in lex(logical_line) {
            match item {
                LexedCommand::Skipped { reason, .. } => {
                    self.skipped.push(SkippedLine { line: start_line, reason });
                }
                LexedCommand::Command(cmd) => {
                    // `words` is non-empty for every emitted `Command`: the
                    // lexer only pushes a `Command` once an executable word
                    // has been seen (see `scan_command`).
                    let executable = cmd.words.first().expect("lexer guarantees a non-empty word list");
                    if executable == "cd" {
                        self.apply_cd(&cmd.words);
                        continue;
                    }
                    self.executions.push(self.build_execution(cmd));
                }
            }
        }
    }

    fn build_execution(&self, cmd: super::lexer::SimpleCommand) -> intercept::Execution {
        let mut environment = self.base_environment.clone();
        for (name, value) in cmd.assignments {
            environment.insert(name, value);
        }
        let executable = PathBuf::from(&cmd.words[0]);
        intercept::Execution {
            executable,
            arguments: cmd.words,
            working_dir: self.working_dir.clone(),
            environment,
        }
    }

    /// `cd` is a shell builtin: it never reaches `exec()`, so it must not
    /// become an event. A zero-argument `cd` (bare `cd`, or `cd $HOME`)
    /// does not reach this point -- the lexer already routes those to a
    /// skip via parameter expansion, or `words` is `["cd"]` and we simply
    /// leave the working directory unchanged.
    fn apply_cd(&mut self, words: &[String]) {
        let Some(target) = words.get(1) else {
            return;
        };
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

/// Splits `input` into `\`-newline-joined logical lines, paired with the
/// 1-based physical line each one starts on. Uses the lexer's own naive
/// continuation rule (a `\` immediately followed by `\n`) so that line
/// numbers reported here agree with what the lexer would compute, and
/// mirrors its documented rationale: continuations are stripped textually
/// regardless of quote context.
fn logical_lines(input: &str) -> Vec<(String, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_start: Option<usize> = None;
    let mut line = 1usize;
    let mut i = 0;

    while i < chars.len() {
        if current_start.is_none() {
            current_start = Some(line);
        }
        if chars[i] == '\\' && chars.get(i + 1) == Some(&'\n') {
            line += 1;
            i += 2;
            continue;
        }
        if chars[i] == '\n' {
            lines.push((std::mem::take(&mut current), current_start.take().expect("set above")));
            line += 1;
            i += 1;
            continue;
        }
        current.push(chars[i]);
        i += 1;
    }
    if let Some(start) = current_start {
        lines.push((current, start));
    }

    lines
}

enum Marker {
    Entering(String),
    /// The target path is not needed: leaving pops back to whatever
    /// directory was pushed on entry, so the printed path is only
    /// consumed here to validate the marker's shape.
    Leaving,
}

/// Recognizes a make recursive-directory marker on an already
/// continuation-joined logical line. Requires a `: ` immediately before
/// `Entering directory ` / `Leaving directory ` so an ordinary command that
/// merely mentions those words is not mis-detected; the optional `[<n>]`
/// job-number suffix (`make[1]: ...`) is allowed but not required.
fn parse_make_marker(line: &str) -> Option<Marker> {
    let trimmed = line.trim();

    const ENTERING: &str = ": Entering directory ";
    const LEAVING: &str = ": Leaving directory ";

    if let Some(rest) = find_after_marker(trimmed, ENTERING) {
        return extract_quoted_path(rest).map(Marker::Entering);
    }
    if let Some(rest) = find_after_marker(trimmed, LEAVING) {
        return extract_quoted_path(rest).map(|_| Marker::Leaving);
    }
    None
}

/// Returns the text after `marker` when `line` starts with a program-name
/// prefix (optionally followed by `[<n>]`) immediately followed by
/// `marker`. The prefix itself is not validated beyond "non-empty,
/// contains no ':'" -- `make`, `gmake`, `make[1]` all qualify.
fn find_after_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let idx = line.find(marker)?;
    let prefix = &line[..idx];
    if prefix.is_empty() || prefix.contains(':') {
        return None;
    }
    Some(&line[idx + marker.len()..])
}

/// Strips one layer of quoting from the start of `rest` -- `'...'`,
/// `` `...' `` (older GNU make's backtick-open/single-close form), or
/// `"..."` -- and returns the inner path. Trailing text after the closing
/// quote (there should be none) is ignored.
fn extract_quoted_path(rest: &str) -> Option<String> {
    let mut chars = rest.chars();
    let open = chars.next()?;
    let close = match open {
        '\'' => '\'',
        '`' => '\'',
        '"' => '"',
        _ => return None,
    };
    let body = chars.as_str();
    let end = body.find(close)?;
    Some(body[..end].to_string())
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

/// Finds the first unquoted, unescaped here-document redirection (`<<`,
/// never the `<<<` herestring) in `line` and returns its delimiter, with
/// any surrounding quotes stripped. Pragmatic, not a full shell tokenizer:
/// it walks the raw line tracking only enough quote state to skip a `<<`
/// that appears inside quotes or is backslash-escaped, then reads the word
/// that follows as the delimiter. Handles `<<WORD`, `<< WORD`, `<<-WORD`,
/// `<<'WORD'`, `<<"WORD"`. Multiple here-documents on one line are not
/// supported: only the first is honored, which is enough for the dry-run
/// build logs this lexer targets.
fn detect_heredoc(line: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut quote: Option<char> = None;

    while i < chars.len() {
        let c = chars[i];
        match quote {
            Some(q) => {
                // Single quotes take everything literally, including
                // backslash; only double quotes let backslash escape the
                // next character.
                if c == '\\' && q == '"' {
                    i += 2;
                    continue;
                }
                if c == q {
                    quote = None;
                }
                i += 1;
            }
            None => {
                if c == '\\' {
                    i += 2;
                    continue;
                }
                if c == '\'' || c == '"' {
                    quote = Some(c);
                    i += 1;
                    continue;
                }
                if c == '<' {
                    let mut run = 0usize;
                    while chars.get(i + run) == Some(&'<') {
                        run += 1;
                    }
                    if run == 2 {
                        let mut j = i + 2;
                        if chars.get(j) == Some(&'-') {
                            j += 1;
                        }
                        while chars.get(j).is_some_and(|c| c.is_whitespace() && *c != '\n') {
                            j += 1;
                        }
                        return read_heredoc_delimiter(&chars, j);
                    }
                    // A single `<` is an ordinary redirect, `<<<` is a
                    // herestring: neither is a here-document. Skip the
                    // whole run so `<<<` is not re-scanned as `<<` starting
                    // one character in.
                    i += run.max(1);
                    continue;
                }
                i += 1;
            }
        }
    }
    None
}

/// Reads the here-document delimiter word starting at `chars[start]`:
/// either a bare word, or a single run of `'...'` / `"..."` with the
/// quotes stripped. Returns `None` if there is no word there at all.
fn read_heredoc_delimiter(chars: &[char], start: usize) -> Option<String> {
    let mut i = start;
    let quote = match chars.get(i) {
        Some(q @ ('\'' | '"')) => {
            i += 1;
            Some(*q)
        }
        _ => None,
    };

    let mut word = String::new();
    while let Some(&c) = chars.get(i) {
        match quote {
            Some(q) => {
                if c == q {
                    // `i` is not consulted after the loop; only `word`
                    // escapes this function.
                    break;
                }
                word.push(c);
                i += 1;
            }
            None => {
                if c.is_whitespace() || matches!(c, ';' | '&' | '|' | '<' | '>') {
                    break;
                }
                word.push(c);
                i += 1;
            }
        }
    }

    if word.is_empty() { None } else { Some(word) }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn cd_on_same_line_affects_the_following_command_only() {
        let sut = interpret("cd sub && gcc -c foo.c", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build/sub", &[])]);
        assert!(sut.skipped.is_empty());
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn cd_persists_across_logical_lines() {
        let sut = interpret("cd sub\ngcc -c foo.c", &context("/build", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "-c", "foo.c"], "/build/sub", &[])]);
    }

    // Requirements: interception-events-from-shell-text
    #[test]
    fn relative_cd_with_dotdot_normalizes_lexically() {
        let sut = interpret("cd ../bar && gcc x.c", &context("/build/foo", &[]));

        assert_eq!(sut.executions, vec![execution("gcc", &["gcc", "x.c"], "/build/bar", &[])]);
    }

    // Requirements: interception-events-from-shell-text
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
        // `gcc 'a` / `b' foo.c` -- the interpreter feeds one physical line
        // at a time, so the quote that spans the newline never gets to
        // close: each half must be skipped loudly as UnterminatedQuote, and
        // neither half may fabricate a command.
        let input = "gcc 'a\nb' foo.c\n";

        let sut = interpret(input, &context("/build", &[]));

        assert!(sut.executions.is_empty(), "must not fabricate a command from a split quote");
        assert_eq!(sut.skipped.len(), 2);
        assert_eq!(sut.skipped[0].reason, SkipReason::UnterminatedQuote);
        assert_eq!(sut.skipped[1].reason, SkipReason::UnterminatedQuote);
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
}
