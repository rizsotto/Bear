// SPDX-License-Identifier: GPL-3.0-or-later

//! The single tokenizer for the `parse-sh` producer mode: the only code in
//! this module that reads characters. It owns quotes and escapes (words are
//! emitted fully assembled), comments, backslash-newline continuations,
//! redirections (consumed, never tokens), here-documents (the pending
//! delimiter queue and body consumption are internal; bodies are discarded,
//! not stored), and the recursive-make directory markers. See
//! `docs/rationale/parse-sh-single-tokenizer.md` for why every lexical rule
//! lives here and nowhere else.
//!
//! The iterator yields `Result<Token, TokenError>`: a [`TokenError::Skip`]
//! marks an unsupported construct and is a recoverable item, not stream
//! termination. The tokenizer keeps scanning the rest of the line after
//! yielding one -- stopping would blind it to a later `<<` on the same
//! line, and the here-document body would then be lexed as commands.
//! Deciding what to discard after a skip is the parser's job (it drains to
//! the next [`Token::Newline`]). A [`TokenError::Io`] read failure is
//! fatal: it is the stream's last item.

use std::collections::VecDeque;
use std::fmt;
use std::io;

/// One token of the supported shell subset. Redirections and here-document
/// bodies never appear: both are consumed inside the tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(Word),
    /// `;`, `&&`, `||`, `|`, or `&`: ends the current command.
    Separator,
    /// End of a logical line: ends the current command and bounds the
    /// reach of a skip (a skip poisons its whole line, nothing beyond).
    Newline,
    /// A recursive-make `Entering directory` / `Leaving directory` line.
    Marker(Marker),
}

/// A fully assembled word: quotes resolved, escapes applied, and adjacent
/// quoted segments concatenated (`foo'bar'baz` is one word).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    /// Set when `*`, `?`, or `[` appeared while genuinely unquoted. Only
    /// the parser knows whether this word ends up in executable position,
    /// where an unexpanded glob would fabricate a wrong event.
    pub has_unquoted_glob: bool,
    /// 1-based physical line the word's logical line starts on.
    pub line: usize,
}

/// A recursive-make directory marker, recognized on its own line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marker {
    Entering(String),
    /// The printed path is not carried: leaving pops back to whatever
    /// directory was in effect on entry.
    Leaving,
}

/// The error half of a tokenizer item. Only `Io` ends the stream.
#[derive(Debug)]
pub enum TokenError {
    /// An unsupported construct: recoverable, scanning continues on the
    /// same line.
    Skip(Skip),
    /// The reader failed; no further items follow.
    Io(io::Error),
}

/// A construct outside the supported subset, reported loudly per the
/// contract: the physical line it starts on and the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skip {
    pub line: usize,
    pub reason: SkipReason,
}

/// Why a line was skipped instead of parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// `( ... )` subshell (a child shell whose state must not leak back
    /// out, so it stays unsupported; a `{ ...; }` group runs in the
    /// current shell and is parsed transparently instead).
    Subshell,
    /// A `}` that closes no open group, or a `{` left open at end of
    /// input: the brace nesting does not balance.
    UnbalancedBrace,
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
    /// A `'` or `"` quote opened but never closed before end of line.
    UnterminatedQuote,
    /// A `cd` form the working-directory model cannot follow
    /// (`cd -` before any previous directory is known).
    UnsupportedCd,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Subshell => "subshell",
            Self::UnbalancedBrace => "unbalanced brace",
            Self::CommandSubstitution => "command substitution",
            Self::ParameterExpansion => "parameter expansion",
            Self::GlobInExecutable => "glob in executable",
            Self::HereDoc => "here-document",
            Self::Keyword => "shell keyword",
            Self::UnterminatedQuote => "unterminated quote",
            Self::UnsupportedCd => "unsupported cd",
        };
        f.write_str(text)
    }
}

/// Streaming tokenizer over shell command text, consuming any buffered
/// reader one logical line at a time (backslash-newline continuations
/// joined), so memory is bounded by the longest logical line -- nothing is
/// accumulated across lines but the here-doc delimiter queue. Quote state
/// can never leak across a newline: an unclosed quote at end of line is an
/// [`SkipReason::UnterminatedQuote`] skip, per the line-oriented contract.
pub struct Tokenizer<R> {
    input: R,
    /// 1-based physical line of the next unread character.
    line: usize,
    /// Tokens and skips already produced for the current logical line.
    pending: VecDeque<Result<Token, Skip>>,
    /// Delimiters of here-documents whose bodies start after the current
    /// line, in redirect order. While non-empty, incoming lines are body
    /// data: consumed, matched against the front delimiter, never tokens.
    heredocs: VecDeque<String>,
    /// Set at end of input or after a read error: the stream is over.
    finished: bool,
}

impl<R: io::BufRead> Tokenizer<R> {
    pub fn new(input: R) -> Self {
        Self { input, line: 1, pending: VecDeque::new(), heredocs: VecDeque::new(), finished: false }
    }

    /// Reads the next logical line: physical lines joined over
    /// backslash-newline, paired with the 1-based physical line the result
    /// starts on. Continuations are stripped textually regardless of quote
    /// context: real `sh` would treat a backslash-newline inside single
    /// quotes as literal, but that distinction is not exercised by the
    /// dry-run text this tokenizer targets, so the simpler unconditional
    /// rule wins over threading quote state through line assembly.
    fn next_logical_line(&mut self) -> io::Result<Option<(String, usize)>> {
        let start = self.line;
        let mut text = String::new();
        let mut read_any = false;
        loop {
            let read = self.input.read_line(&mut text)?;
            if read == 0 {
                return Ok(if read_any { Some((text, start)) } else { None });
            }
            read_any = true;
            if !text.ends_with('\n') {
                // Last line of the input, without a trailing newline; a
                // trailing backslash here escapes nothing and stays.
                return Ok(Some((text, start)));
            }
            text.pop();
            self.line += 1;
            if text.ends_with('\\') {
                text.pop();
                continue;
            }
            return Ok(Some((text, start)));
        }
    }

    fn process_line(&mut self, text: &str, start_line: usize) {
        if let Some(delimiter) = self.heredocs.front() {
            // Here-document body lines (and the terminator itself) are
            // data, not commands: they must never be tokenized, or they
            // would fabricate execution events. Pragmatic match: leading
            // tabs are stripped before comparing (the `<<-` rule, applied
            // without tracking the dash), but nothing else -- to real sh a
            // space-indented or trailing-decorated delimiter is body, and
            // ending the body early would tokenize its remaining lines.
            if text.trim_start_matches('\t') == delimiter {
                self.heredocs.pop_front();
            }
            return;
        }

        if let Some(marker) = parse_make_marker(text) {
            self.pending.push_back(Ok(Token::Marker(marker)));
            self.pending.push_back(Ok(Token::Newline));
            return;
        }

        LineScanner {
            chars: text.chars().collect(),
            pos: 0,
            line: start_line,
            out: &mut self.pending,
            heredocs: &mut self.heredocs,
        }
        .run();
        self.pending.push_back(Ok(Token::Newline));
    }
}

impl<R: io::BufRead> Iterator for Tokenizer<R> {
    type Item = Result<Token, TokenError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.pending.pop_front() {
                return Some(item.map_err(TokenError::Skip));
            }
            if self.finished {
                return None;
            }
            match self.next_logical_line() {
                Err(error) => {
                    self.finished = true;
                    return Some(Err(TokenError::Io(error)));
                }
                Ok(None) => {
                    self.finished = true;
                    return None;
                }
                Ok(Some((text, start_line))) => self.process_line(&text, start_line),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    None,
    Single,
    Double,
}

/// A word as read by [`LineScanner::read_word`], before it is turned into
/// a token or a skip.
struct ScannedWord {
    text: String,
    skip: Option<SkipReason>,
    has_unquoted_glob: bool,
}

/// Tokenizes one logical line. All emitted skips carry the line's starting
/// physical line number, which is also what the parser reports.
struct LineScanner<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    out: &'a mut VecDeque<Result<Token, Skip>>,
    heredocs: &'a mut VecDeque<String>,
}

impl LineScanner<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip(&mut self, reason: SkipReason) {
        self.out.push_back(Err(Skip { line: self.line, reason }));
    }

    fn word(&mut self, text: String, has_unquoted_glob: bool) {
        self.out.push_back(Ok(Token::Word(Word { text, has_unquoted_glob, line: self.line })));
    }

    fn at_separator(&self) -> bool {
        matches!(self.peek(), Some(';') | Some('&') | Some('|'))
    }

    fn run(mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.advance();
            }
            let Some(c) = self.peek() else { break };
            match c {
                // A '#' at a word boundary starts a comment to end of
                // line; '#' inside an already-started word is handled by
                // read_word and never reaches here. Nothing after it is
                // scanned, so a `<<` inside a comment cannot queue a
                // here-document.
                '#' => break,
                ';' => {
                    self.advance();
                    self.out.push_back(Ok(Token::Separator));
                }
                '&' | '|' => {
                    self.advance();
                    if self.peek() == Some(c) {
                        self.advance();
                    }
                    self.out.push_back(Ok(Token::Separator));
                }
                // An unquoted parenthesis at word start is subshell
                // grouping (in `sh` it is an operator; a command like
                // `gcc (a)` is a syntax error, never a real invocation).
                // Mid-word parentheses stay word characters, see
                // read_word. `{`/`}` are NOT handled here: they are
                // reserved words, special only in command position,
                // which only the parser can see.
                '(' | ')' => {
                    self.advance();
                    self.skip(SkipReason::Subshell);
                }
                _ if self.looks_like_redirect_start() => self.scan_redirect(),
                _ => {
                    let word = self.read_word();
                    match word.skip {
                        Some(reason) => self.skip(reason),
                        None => self.word(word.text, word.has_unquoted_glob),
                    }
                }
            }
        }
    }

    fn looks_like_redirect_start(&self) -> bool {
        match self.peek() {
            Some('<') | Some('>') => true,
            Some(c) if c.is_ascii_digit() => {
                let mut off = 1;
                while matches!(self.peek_at(off), Some(d) if d.is_ascii_digit()) {
                    off += 1;
                }
                matches!(self.peek_at(off), Some('<') | Some('>'))
            }
            _ => false,
        }
    }

    /// Consumes one redirection (optional fd-prefix, operator, and target
    /// or dup-form) and strips it entirely: nothing from a redirect ever
    /// becomes a token. `<<<` (herestring) is an ordinary redirect to
    /// strip; only `<<` (here-document) yields a skip and queues its
    /// delimiter for body consumption.
    fn scan_redirect(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }

        if self.peek() == Some('<') && self.peek_at(1) == Some('<') && self.peek_at(2) == Some('<') {
            self.pos += 3;
            self.discard_target();
            return;
        }

        if self.peek() == Some('<') && self.peek_at(1) == Some('<') {
            self.pos += 2;
            if self.peek() == Some('-') {
                self.advance();
            }
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.advance();
            }
            // A missing delimiter word queues nothing: there is no body
            // to consume, only the `<<` line itself to skip.
            if let Some(delimiter) = self.read_heredoc_delimiter() {
                self.heredocs.push_back(delimiter);
            }
            self.skip(SkipReason::HereDoc);
            return;
        }

        if self.peek() == Some('>') && self.peek_at(1) == Some('>') {
            self.pos += 2;
        } else if self.peek() == Some('>') {
            self.advance();
            // `>|` is the POSIX clobber redirect (`set -C` override): the
            // `|` belongs to the operator, not to a pipe.
            if self.peek() == Some('|') {
                self.advance();
            }
        } else {
            // Only '<' can reach here: the doubled/tripled forms and '>'
            // were consumed above, and looks_like_redirect_start proved
            // the operator character.
            self.advance();
        }

        // Dup form: `N>&M`, `>&2`, `2>&-` -- the whole token is the
        // redirect, no separate target word follows.
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

        self.discard_target();
    }

    /// Consumes and discards a redirect's target word, whether fused
    /// (`>file`) or separated by whitespace (`> file`). Targets are
    /// discarded uninspected -- an expansion or substitution inside one
    /// does not skip the line, because the command's argv is still fully
    /// known and only the redirect path is unresolved (see the contract's
    /// redirection clause). The one exception is an unterminated quote or
    /// substitution: that means the line boundary itself was misread, so
    /// it must surface as a loud skip rather than let the construct's
    /// content fabricate commands on the next line.
    fn discard_target(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.advance();
        }
        if self.peek().is_none() || self.at_separator() || self.peek() == Some('#') {
            return;
        }
        let word = self.read_word();
        if word.skip == Some(SkipReason::UnterminatedQuote) {
            self.skip(SkipReason::UnterminatedQuote);
        }
    }

    /// Reads the here-document delimiter word: either a bare word, or a
    /// single run of `'...'` / `"..."` with the quotes stripped. A bare
    /// delimiter ends at whitespace or at any shell operator character --
    /// including `(`, `)`, and a backtick, so a here-doc inside a
    /// subshell (`(cat <<EOF)`) queues `EOF`, not `EOF)`.
    fn read_heredoc_delimiter(&mut self) -> Option<String> {
        let mut word = String::new();
        match self.peek() {
            Some(q @ ('\'' | '"')) => {
                self.advance();
                while let Some(c) = self.peek() {
                    if c == q {
                        self.advance();
                        break;
                    }
                    word.push(c);
                    self.advance();
                }
            }
            _ => {
                while let Some(c) = self.peek() {
                    if c.is_whitespace() || matches!(c, ';' | '&' | '|' | '<' | '>' | '(' | ')' | '`') {
                        break;
                    }
                    word.push(c);
                    self.advance();
                }
            }
        }
        if word.is_empty() { None } else { Some(word) }
    }

    /// Reads one whitespace/metacharacter-delimited word, resolving quotes
    /// and backslash escapes, and flagging an unsupported construct or an
    /// unquoted glob character as it goes. Assumes the caller has already
    /// ruled out comment, separator, redirect, and parenthesis starts at
    /// this position.
    fn read_word(&mut self) -> ScannedWord {
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
                        if let Some(next) = self.advance() {
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
                            if let Some(next) = self.advance() {
                                text.push(next);
                            }
                        }
                        '$' if self.peek_at(1) == Some('(') => {
                            skip.get_or_insert(SkipReason::CommandSubstitution);
                            self.advance();
                            self.advance();
                            text.push_str("$(");
                            // The whole substitution is one shell word even
                            // when it contains spaces: consume to the
                            // matching parenthesis so no fragment of it can
                            // leak out as a separate word -- or, after a
                            // discarded redirect target, as a fabricated
                            // argument.
                            if !self.consume_balanced(&mut text, '(', ')') {
                                skip = Some(SkipReason::UnterminatedQuote);
                            }
                        }
                        '$' if self.peek_at(1) == Some('{') => {
                            skip.get_or_insert(SkipReason::ParameterExpansion);
                            self.advance();
                            self.advance();
                            text.push_str("${");
                            if !self.consume_balanced(&mut text, '{', '}') {
                                skip = Some(SkipReason::UnterminatedQuote);
                            }
                        }
                        '$' => {
                            skip.get_or_insert(SkipReason::ParameterExpansion);
                            self.advance();
                            text.push(c);
                        }
                        '`' => {
                            skip.get_or_insert(SkipReason::CommandSubstitution);
                            self.advance();
                            text.push(c);
                            if !self.consume_balanced(&mut text, '`', '`') {
                                skip = Some(SkipReason::UnterminatedQuote);
                            }
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

        if quote != Quote::None {
            // The read loop only exits mid-quote by running out of line
            // (the `Quote::None` arm is the only one that breaks on a
            // delimiter). Quote state never crosses a newline: the
            // contract is line-oriented, so each half of a quote that a
            // real newline split must be skipped loudly rather than lexed
            // into a fabricated command.
            skip = Some(SkipReason::UnterminatedQuote);
        }

        ScannedWord { text, skip, has_unquoted_glob }
    }

    /// Consumes a substitution/expansion span up to the matching `close`,
    /// appending everything to `text`, quote- and escape-aware so a close
    /// character inside quotes does not end the span. Returns false when
    /// the line ends before the span closes: the construct continues on
    /// the next physical line, which this line-oriented reader must treat
    /// like an unterminated quote, or the span's tail would fabricate
    /// commands there.
    fn consume_balanced(&mut self, text: &mut String, open: char, close: char) -> bool {
        let mut depth = 1usize;
        let mut quote = Quote::None;
        while let Some(c) = self.advance() {
            text.push(c);
            match quote {
                Quote::Single => {
                    if c == '\'' {
                        quote = Quote::None;
                    }
                }
                Quote::Double => match c {
                    '\\' => {
                        if let Some(next) = self.advance() {
                            text.push(next);
                        }
                    }
                    '"' => quote = Quote::None,
                    _ => {}
                },
                Quote::None => match c {
                    '\\' => {
                        if let Some(next) = self.advance() {
                            text.push(next);
                        }
                    }
                    '\'' => quote = Quote::Single,
                    '"' => quote = Quote::Double,
                    // Close is checked first so an open==close pair (a
                    // backtick span) terminates on its second character.
                    c if c == close => {
                        depth -= 1;
                        if depth == 0 {
                            return true;
                        }
                    }
                    c if c == open => depth += 1,
                    _ => {}
                },
            }
        }
        false
    }
}

/// Recognizes a make recursive-directory marker on an already
/// continuation-joined logical line. Requires a `: ` immediately before
/// `Entering directory ` / `Leaving directory `, with the optional `[<n>]`
/// job-number suffix (`make[1]: ...`) allowed but not required. Real `make`
/// prints these markers on their own line and never on a command line, so
/// dry-run input is not misread; a hand-crafted line that embeds the exact
/// `<prefix>: Entering directory '<path>'` shape (e.g. an `echo` of it)
/// would be, but that is outside the supported dry-run contract.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: &str) -> Vec<Result<Token, Skip>> {
        Tokenizer::new(input.as_bytes())
            .map(|item| {
                item.map_err(|error| match error {
                    TokenError::Skip(skip) => skip,
                    // An in-memory reader cannot fail.
                    TokenError::Io(error) => panic!("unexpected read error: {error}"),
                })
            })
            .collect()
    }

    fn word(text: &str, line: usize) -> Result<Token, Skip> {
        Ok(Token::Word(Word { text: text.to_string(), has_unquoted_glob: false, line }))
    }

    fn glob_word(text: &str, line: usize) -> Result<Token, Skip> {
        Ok(Token::Word(Word { text: text.to_string(), has_unquoted_glob: true, line }))
    }

    fn sep() -> Result<Token, Skip> {
        Ok(Token::Separator)
    }

    fn nl() -> Result<Token, Skip> {
        Ok(Token::Newline)
    }

    fn skipped(line: usize, reason: SkipReason) -> Result<Token, Skip> {
        Err(Skip { line, reason })
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn assembles_words_across_quotes_and_escapes() {
        let cases: Vec<(&str, Vec<Result<Token, Skip>>)> = vec![
            ("gcc -c foo.c", vec![word("gcc", 1), word("-c", 1), word("foo.c", 1), nl()]),
            ("gcc -DNAME='a b' foo.c", vec![word("gcc", 1), word("-DNAME=a b", 1), word("foo.c", 1), nl()]),
            ("gcc \"-DX=y z\" foo.c", vec![word("gcc", 1), word("-DX=y z", 1), word("foo.c", 1), nl()]),
            ("gcc foo\\ bar.c", vec![word("gcc", 1), word("foo bar.c", 1), nl()]),
            ("gcc foo'bar'baz", vec![word("gcc", 1), word("foobarbaz", 1), nl()]),
            ("gcc '$CFLAGS' foo.c", vec![word("gcc", 1), word("$CFLAGS", 1), word("foo.c", 1), nl()]),
            ("gcc foo#bar", vec![word("gcc", 1), word("foo#bar", 1), nl()]),
            ("gcc *.c", vec![word("gcc", 1), glob_word("*.c", 1), nl()]),
        ];

        for (input, expected) in cases {
            let sut = tokens(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn joins_line_continuations_and_numbers_physical_lines() {
        let cases: Vec<(&str, Vec<Result<Token, Skip>>)> = vec![
            ("gcc -c \\\n foo.c", vec![word("gcc", 1), word("-c", 1), word("foo.c", 1), nl()]),
            ("gcc a\ngcc b", vec![word("gcc", 1), word("a", 1), nl(), word("gcc", 2), word("b", 2), nl()]),
            ("a \\\n b\ngcc c", vec![word("a", 1), word("b", 1), nl(), word("gcc", 3), word("c", 3), nl()]),
        ];

        for (input, expected) in cases {
            let sut = tokens(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    //
    // A build log saved on Windows (or fetched through a CRLF-translating
    // channel) carries `\r\n` line endings; the carriage return must not
    // survive into the last word of the line.
    #[test]
    fn crlf_line_endings_leave_no_carriage_return_in_words() {
        let cases: Vec<(&str, Vec<Result<Token, Skip>>)> = vec![
            ("gcc -c foo.c\r\n", vec![word("gcc", 1), word("-c", 1), word("foo.c", 1), nl()]),
            (
                "gcc a\r\ngcc b\r\n",
                vec![word("gcc", 1), word("a", 1), nl(), word("gcc", 2), word("b", 2), nl()],
            ),
        ];

        for (input, expected) in cases {
            let sut = tokens(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    //
    // Documented deviation (see `next_logical_line`): continuations are
    // stripped textually regardless of quote context, so a backslash-newline
    // inside single quotes joins instead of staying literal as real `sh`
    // would keep it. Pin the joined form so a change to the rule is a
    // conscious one.
    #[test]
    fn backslash_newline_inside_single_quotes_joins_the_lines() {
        let sut = tokens("gcc '-DX=a\\\nb' foo.c");

        assert_eq!(sut, vec![word("gcc", 1), word("-DX=ab", 1), word("foo.c", 1), nl()]);
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn emits_separators_for_the_command_operators() {
        let cases: Vec<(&str, Vec<Result<Token, Skip>>)> = vec![
            ("a; b", vec![word("a", 1), sep(), word("b", 1), nl()]),
            ("a && b", vec![word("a", 1), sep(), word("b", 1), nl()]),
            ("a || b", vec![word("a", 1), sep(), word("b", 1), nl()]),
            ("a | b", vec![word("a", 1), sep(), word("b", 1), nl()]),
            ("a &", vec![word("a", 1), sep(), nl()]),
        ];

        for (input, expected) in cases {
            let sut = tokens(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn comments_and_blank_lines_produce_only_newlines() {
        let cases: Vec<(&str, Vec<Result<Token, Skip>>)> = vec![
            ("# comment", vec![nl()]),
            ("", vec![]),
            ("   \n  \n", vec![nl(), nl()]),
            ("gcc foo.c # build it", vec![word("gcc", 1), word("foo.c", 1), nl()]),
        ];

        for (input, expected) in cases {
            let sut = tokens(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn consumes_redirections_without_emitting_tokens() {
        let cases: Vec<(&str, Vec<Result<Token, Skip>>)> = vec![
            ("cmd < input.txt", vec![word("cmd", 1), nl()]),
            ("cmd >> log.txt", vec![word("cmd", 1), nl()]),
            ("cmd 1>out 2>err", vec![word("cmd", 1), nl()]),
            ("cmd >&2", vec![word("cmd", 1), nl()]),
            ("cmd 2>&-", vec![word("cmd", 1), nl()]),
            ("cmd <<< input", vec![word("cmd", 1), nl()]),
            // `>|` is one operator (POSIX clobber): the `|` must not be
            // read as a pipe, and the target must not become a word.
            ("cmd >| out.txt", vec![word("cmd", 1), nl()]),
            ("cmd >/dev/null 2>&1", vec![word("cmd", 1), nl()]),
        ];

        for (input, expected) in cases {
            let sut = tokens(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn an_unterminated_quote_in_a_redirect_target_is_a_loud_skip() {
        let sut = tokens("cmd > 'oops");

        assert_eq!(sut, vec![word("cmd", 1), skipped(1, SkipReason::UnterminatedQuote), nl()]);
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn substitution_in_a_discarded_redirect_target_is_not_a_skip() {
        let cases: Vec<&str> = vec![
            "gcc -c foo.c > $(logdir)/x.log",
            // The substitution is one shell word despite the space inside
            // it: no fragment (`x)/log`) may leak back as an argument.
            "gcc -c foo.c > $(dirname x)/log",
            "gcc -c foo.c > `dirname x`/log",
            "gcc -c foo.c > ${LOGDIR}/x.log",
        ];

        for input in cases {
            let sut = tokens(input);
            assert_eq!(sut, vec![word("gcc", 1), word("-c", 1), word("foo.c", 1), nl()], "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn an_unterminated_substitution_in_a_redirect_target_is_a_loud_skip() {
        // The `$(` never closes on this line, so the line boundary is
        // misread: same loud-skip treatment as an unterminated quote.
        let sut = tokens("cmd > $(oops");

        assert_eq!(sut, vec![word("cmd", 1), skipped(1, SkipReason::UnterminatedQuote), nl()]);
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn skips_unsupported_constructs_but_keeps_scanning_the_line() {
        let cases: Vec<(&str, Vec<Result<Token, Skip>>)> = vec![
            (
                "gcc $CFLAGS foo.c",
                vec![word("gcc", 1), skipped(1, SkipReason::ParameterExpansion), word("foo.c", 1), nl()],
            ),
            (
                // The whole `$(...)` span is one (skipped) word; scanning
                // resumes at `foo.c`.
                "gcc $(pkg-config --cflags x) foo.c",
                vec![word("gcc", 1), skipped(1, SkipReason::CommandSubstitution), word("foo.c", 1), nl()],
            ),
            ("gcc `date`", vec![word("gcc", 1), skipped(1, SkipReason::CommandSubstitution), nl()]),
            ("gcc 'unterminated", vec![word("gcc", 1), skipped(1, SkipReason::UnterminatedQuote), nl()]),
        ];

        for (input, expected) in cases {
            let sut = tokens(input);
            assert_eq!(sut, expected, "case: {input:?}");
        }
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn a_heredoc_is_a_skip_and_its_body_produces_no_tokens() {
        let sut = tokens("cat <<EOF\ngcc -c fake.c\nEOF\ngcc -c real.c\n");

        assert_eq!(
            sut,
            vec![
                word("cat", 1),
                skipped(1, SkipReason::HereDoc),
                nl(),
                word("gcc", 4),
                word("-c", 4),
                word("real.c", 4),
                nl(),
            ]
        );
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn a_heredoc_inside_a_trailing_comment_is_not_detected() {
        let sut = tokens("gcc -c a.c # see <<EOF docs\ngcc -c b.c\n");

        assert_eq!(
            sut,
            vec![
                word("gcc", 1),
                word("-c", 1),
                word("a.c", 1),
                nl(),
                word("gcc", 2),
                word("-c", 2),
                word("b.c", 2),
                nl(),
            ]
        );
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn a_heredoc_delimiter_ends_at_a_closing_parenthesis() {
        // `(cat <<EOF)`: the delimiter is `EOF`, not `EOF)`, so the body
        // ends at the `EOF` line and later commands are tokenized again.
        let sut = tokens("(cat <<EOF)\nbody\nEOF\ngcc -c after.c\n");

        assert_eq!(
            sut,
            vec![
                skipped(1, SkipReason::Subshell),
                word("cat", 1),
                skipped(1, SkipReason::HereDoc),
                skipped(1, SkipReason::Subshell),
                nl(),
                word("gcc", 4),
                word("-c", 4),
                word("after.c", 4),
                nl(),
            ]
        );
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn queues_every_heredoc_on_a_line_in_redirect_order() {
        let sut = tokens("cat <<A <<B\nbody a\nA\nbody b\nB\ngcc -c x.c\n");

        assert_eq!(
            sut,
            vec![
                word("cat", 1),
                skipped(1, SkipReason::HereDoc),
                skipped(1, SkipReason::HereDoc),
                nl(),
                word("gcc", 6),
                word("-c", 6),
                word("x.c", 6),
                nl(),
            ]
        );
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn a_read_failure_is_the_last_item_of_the_stream() {
        struct FailingReader<'a>(&'a [u8]);

        impl io::Read for FailingReader<'_> {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if self.0.is_empty() {
                    return Err(io::Error::other("read failure"));
                }
                let n = self.0.len().min(buf.len());
                buf[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }

        let mut sut = Tokenizer::new(io::BufReader::new(FailingReader(b"gcc a.c\n")));

        assert!(matches!(sut.next(), Some(Ok(Token::Word(_)))));
        assert!(matches!(sut.next(), Some(Ok(Token::Word(_)))));
        assert!(matches!(sut.next(), Some(Ok(Token::Newline))));
        assert!(matches!(sut.next(), Some(Err(TokenError::Io(_)))));
        assert!(sut.next().is_none(), "the stream must be over after a read error");
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn emits_markers_for_recursive_make_directory_lines() {
        let sut =
            tokens("make[1]: Entering directory '/build/lib'\nmake[1]: Leaving directory '/build/lib'\n");

        assert_eq!(
            sut,
            vec![
                Ok(Token::Marker(Marker::Entering("/build/lib".to_string()))),
                nl(),
                Ok(Token::Marker(Marker::Leaving)),
                nl(),
            ]
        );
    }
}
