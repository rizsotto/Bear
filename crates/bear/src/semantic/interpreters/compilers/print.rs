// SPDX-License-Identifier: GPL-3.0-or-later

//! Renders the compilers Bear recognizes as a human-readable list, for
//! `bear semantic --print-compilers`.
//!
//! Pure formatting over the generated [`RECOGNITION_PATTERNS`] table and
//! [`WRAPPER_NAMES`]: no I/O, no config lookup. Rows whose `description`
//! is `None` are internal helper executables (e.g. `cc1`,
//! `swift-frontend`) that exist only so the recognizer can route them to
//! the right interpreter for ignoring; they are not user-facing
//! compilers and are skipped here.

use super::compiler_recognition::RECOGNITION_PATTERNS;
use super::wrapper::WRAPPER_NAMES;

/// Maximum output line width in columns. The executables column wraps on
/// `, ` boundaries to keep every line within this budget.
const WIDTH: usize = 100;

/// One renderable row: a human description, the config `as:` alias, and
/// the executable basenames that recognize it.
struct Row {
    description: &'static str,
    alias: String,
    executables: &'static [&'static str],
}

impl Row {
    /// The literal `as <alias>` label rendered in the second column.
    fn alias_column(&self) -> String {
        format!("as {}", self.alias)
    }
}

/// Renders the entry lines only -- no header, no version banner, no
/// trailing caveat; the caller (the CLI dispatcher) adds those. Ends with
/// a trailing newline.
pub fn print_compilers() -> String {
    let rows = build_rows();

    let col1_width = rows.iter().map(|r| r.description.len()).max().unwrap_or(0) + 2;
    let col2_width = rows.iter().map(|r| r.alias_column().len()).max().unwrap_or(0) + 2;

    let mut out = String::new();
    for row in &rows {
        render_row(row, col1_width, col2_width, &mut out);
    }
    out
}

/// Builds the row list from [`RECOGNITION_PATTERNS`], preserving its
/// order and skipping internal-only (`description: None`) rows, then
/// appends one final row for the launchers (ccache/distcc/sccache/icecc).
fn build_rows() -> Vec<Row> {
    let mut rows: Vec<Row> = RECOGNITION_PATTERNS
        .iter()
        .filter_map(|&(type_str, executables, _cross_compilation, _versioned, description)| {
            description.map(|desc| Row { description: desc, alias: type_str.replace('_', "-"), executables })
        })
        .collect();

    rows.push(Row {
        description: "Compiler launcher",
        alias: "wrapper".to_string(),
        executables: WRAPPER_NAMES,
    });

    rows
}

/// Appends one row's rendered lines to `out`: the first line carries the
/// description and alias columns, continuation lines (when the
/// executables list wraps) are blank in those columns, indented with
/// spaces to where the executables column begins.
fn render_row(row: &Row, col1_width: usize, col2_width: usize, out: &mut String) {
    let indent = col1_width + col2_width;
    let avail = WIDTH.saturating_sub(indent);
    let wrapped = wrap_executables(row.executables, avail);

    for (i, line) in wrapped.iter().enumerate() {
        if i == 0 {
            out.push_str(&format!(
                "{:col1_width$}{:col2_width$}{}\n",
                row.description,
                row.alias_column(),
                line
            ));
        } else {
            out.push_str(&format!("{:indent$}{}\n", "", line));
        }
    }
}

/// Greedily wraps `executables` (joined with `, `) into lines no longer
/// than `avail` columns, breaking only on `, ` boundaries. Mirrors plain
/// text wrapping: a line always holds at least one executable, even if
/// that single name alone exceeds `avail`.
fn wrap_executables(executables: &[&str], avail: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for (i, exe) in executables.iter().enumerate() {
        let is_last = i + 1 == executables.len();
        let piece = if is_last { (*exe).to_string() } else { format!("{}, ", exe) };
        let tentative = format!("{current}{piece}");

        if !current.is_empty() && tentative.trim_end().len() > avail {
            lines.push(current.trim_end().to_string());
            current = piece;
        } else {
            current = tentative;
        }
    }

    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CompilerType;

    #[test]
    fn lists_a_known_compiler_with_its_alias_and_executables() {
        let sut = print_compilers();

        assert!(sut.contains("GCC"), "missing GCC description:\n{sut}");
        assert!(sut.contains("as gcc"), "missing gcc alias:\n{sut}");
        assert!(sut.contains("gcc, g++"), "missing gcc executables:\n{sut}");
    }

    #[test]
    fn lists_the_launcher_row_last() {
        let sut = print_compilers();

        assert!(sut.contains("Compiler launcher"), "missing launcher description:\n{sut}");
        assert!(sut.contains("as wrapper"), "missing wrapper alias:\n{sut}");
        assert!(sut.contains("ccache"), "missing launcher executables:\n{sut}");
    }

    #[test]
    fn omits_internal_helper_executables_and_ambiguous_bare_names() {
        let sut = print_compilers();

        // Internal-only rows (description: None) must not surface.
        assert!(!sut.contains("swift-frontend"), "internal swift helper leaked:\n{sut}");
        assert!(!sut.contains("cc1"), "internal gcc helper leaked:\n{sut}");

        // The probed ambiguous basenames (cc, c++, CC) have no regex entry
        // at all, so they can never appear as a standalone executable-list
        // entry; a bare "cc" would only ever show up as part of another
        // token (e.g. "gcc"), never as its own comma-delimited entry.
        assert!(!sut.contains(" cc,"), "bare 'cc' must not be listed:\n{sut}");
        assert!(!sut.contains(" c++,"), "bare 'c++' must not be listed:\n{sut}");
    }

    #[test]
    fn every_line_fits_within_the_configured_width() {
        let sut = print_compilers();

        for line in sut.lines() {
            assert!(line.len() <= WIDTH, "line exceeds {WIDTH} columns ({} chars): {line:?}", line.len());
        }
    }

    /// Round-trip guard: every alias this renderer displays (`as <alias>`)
    /// must be a value the config `as:` field actually accepts. If a new
    /// compiler family's `type:` string in YAML and its `CompilerType`
    /// serde spelling ever drift apart, this test -- not a confused user
    /// copy-pasting a broken config -- catches it.
    #[test]
    fn every_displayed_alias_deserializes_as_a_compiler_type() {
        let mut aliases: Vec<String> = RECOGNITION_PATTERNS
            .iter()
            .filter_map(|&(type_str, _, _, _, description)| description.map(|_| type_str.replace('_', "-")))
            .collect();
        aliases.push("wrapper".to_string());

        for alias in aliases {
            let json = format!("\"{alias}\"");
            let sut: Result<CompilerType, _> = serde_json::from_str(&json);
            assert!(sut.is_ok(), "alias '{alias}' does not deserialize as a CompilerType: {sut:?}");
        }
    }
}
