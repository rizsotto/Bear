// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared logging initialization for Bear's binaries.
//!
//! Bear's log output has two audiences, so this initializer picks a format
//! from the environment:
//!
//! - **User** (default, `RUST_LOG` unset): UNIX-style `prog: message`
//!   diagnostics on stderr, matching `cc`, `git`, and `make`. The default
//!   filter is `warn`, so a user sees only warnings and errors, prefixed
//!   `prog: warning:` / `prog: error:`.
//! - **Developer** (`RUST_LOG` set): a rich trace carrying timestamp, level,
//!   the **process** identity (name + pid), and the source module, so lines
//!   from the driver, wrapper, and preload processes can be told apart even
//!   when they interleave on the same terminal.
//!
//! `program` is the process identity: `bear`, `wrapper`, `preload`. All
//! binaries route through [`init`]; initialization is idempotent so the
//! preload library may call it more than once without panicking.

use std::fmt;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

/// Initialize the process-wide logger.
///
/// `program` names the process for diagnostics (see the module docs).
/// Idempotent: a second call is a no-op rather than a panic.
pub fn init(program: &str) {
    // An empty `RUST_LOG` counts as unset: it would otherwise select the
    // developer view while `env_logger` parses "" down to an `error`
    // threshold, i.e. quieter than the user view's `warn` default.
    let developer = std::env::var_os("RUST_LOG").is_some_and(|value| !value.is_empty());
    let program = program.to_string();
    let pid = std::process::id();

    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"));

    if developer {
        builder.format(move |buf, record| {
            write_developer(buf, &timestamp(), &program, pid, record.level(), record.target(), record.args())
        });
    } else {
        builder.format(move |buf, record| write_user(buf, &program, record.level(), record.args()));
    }

    // `try_init`, not `init`: this runs from a library constructor inside
    // processes Bear does not control (any intercepted compiler), so a second
    // registration must be a silent no-op rather than a panic.
    let _ = builder.try_init();
}

/// User format: `prog: message`, with a severity qualifier for the two levels
/// a user sees by default.
fn write_user(
    out: &mut impl io::Write,
    program: &str,
    level: log::Level,
    message: impl fmt::Display,
) -> io::Result<()> {
    match level {
        log::Level::Error => writeln!(out, "{program}: error: {message}"),
        log::Level::Warn => writeln!(out, "{program}: warning: {message}"),
        _ => writeln!(out, "{program}: {message}"),
    }
}

/// Developer format: timestamp, level, process identity (name + pid), source
/// module, then the message.
fn write_developer(
    out: &mut impl io::Write,
    timestamp: &str,
    program: &str,
    pid: u32,
    level: log::Level,
    target: &str,
    message: impl fmt::Display,
) -> io::Result<()> {
    writeln!(out, "[{timestamp} {level:5} {program}[{pid}] {target}] {message}")
}

/// UTC `HH:MM:SS.mmm` for the developer format.
///
/// `env_logger`'s timestamp support lives behind its `humantime` feature,
/// which the workspace disables (`default-features = false`), so the time of
/// day is derived here from the epoch directly. It is UTC, not local time;
/// this is developer-only diagnostic output where an absolute wall clock is
/// not needed.
fn timestamp() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = now.as_secs();
    let ms = now.subsec_millis();
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Requirements: cli-diagnostic-format
    #[test]
    fn user_format_prefixes_program_and_qualifies_severity() {
        // arrange: one message, every level, expected line per level
        let cases = [
            (log::Level::Error, "bear: error: hi\n"),
            (log::Level::Warn, "bear: warning: hi\n"),
            (log::Level::Info, "bear: hi\n"),
            (log::Level::Debug, "bear: hi\n"),
            (log::Level::Trace, "bear: hi\n"),
        ];

        for (level, expected) in cases {
            // act
            let mut buf = Vec::new();
            write_user(&mut buf, "bear", level, "hi").unwrap();
            let sut = String::from_utf8(buf).unwrap();

            // assert
            assert_eq!(sut, expected, "level {level}");
        }
    }

    // Requirements: cli-diagnostic-format
    #[test]
    fn user_format_carries_the_process_identity() {
        // arrange / act: a warning from the wrapper process
        let mut buf = Vec::new();
        write_user(&mut buf, "wrapper", log::Level::Warn, "report failed").unwrap();
        let sut = String::from_utf8(buf).unwrap();

        // assert: the emitting process is named, not the crate
        assert_eq!(sut, "wrapper: warning: report failed\n");
    }

    // Requirements: cli-diagnostic-format
    #[test]
    fn developer_format_tags_process_identity_pid_and_source() {
        // arrange / act
        let mut buf = Vec::new();
        write_developer(
            &mut buf,
            "01:02:03.004",
            "preload",
            4242,
            log::Level::Debug,
            "intercept::tcp",
            "sent",
        )
        .unwrap();
        let sut = String::from_utf8(buf).unwrap();

        // assert
        assert_eq!(sut, "[01:02:03.004 DEBUG preload[4242] intercept::tcp] sent\n");
    }

    // Requirements: cli-diagnostic-format
    #[test]
    fn developer_format_pads_level_for_column_alignment() {
        // arrange / act: the shorter level names pad to five columns
        let mut buf = Vec::new();
        write_developer(&mut buf, "01:02:03.004", "bear", 7, log::Level::Info, "bear::modes", "cfg").unwrap();
        let sut = String::from_utf8(buf).unwrap();

        // assert: "INFO" padded to width 5, then the field separator
        assert_eq!(sut, "[01:02:03.004 INFO  bear[7] bear::modes] cfg\n");
    }

    // Requirements: cli-diagnostic-format
    #[test]
    fn format_accepts_deferred_arguments() {
        // arrange / act: the real call path passes `record.args()`, i.e.
        // `fmt::Arguments`; make sure that type is accepted, not just `&str`
        let mut buf = Vec::new();
        write_user(&mut buf, "bear", log::Level::Error, format_args!("code {}", 42)).unwrap();
        let sut = String::from_utf8(buf).unwrap();

        // assert
        assert_eq!(sut, "bear: error: code 42\n");
    }
}
