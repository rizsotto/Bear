---
paths:
  - "**/*.rs"
---

# Rust house style

Conventions for Rust code across every workspace crate. The per-crate
`CLAUDE.md` files add only crate-specific constraints on top of these.

- **Edition**: Rust 2024 idioms (`edition = "2024"` in `Cargo.toml`).
- **File header**: every `.rs` file starts with
  `// SPDX-License-Identifier: GPL-3.0-or-later` as its first line, before
  any `//!` module doc. Bear is GPL-3.0-or-later (see `COPYING`).
- **Error handling**: domain errors are `thiserror` enums in the library
  code; the binaries (`crates/bear-driver/src/`, `crates/bear-wrapper/src/`)
  use `anyhow::Result` with
  `.context(...)` at the boundary and turn any `Err` into a non-zero exit.
- **Panicking macros**: `unwrap()` is for test code only. In production use
  `.expect("short reason")` only when a prior-stage invariant makes the
  `None`/`Err` structurally impossible, and name that invariant in the
  string; propagate everything else with `?`. `panic!` / `unreachable!` are
  for unambiguous programmer bugs (violated API contract, malformed
  generated data), with a one-line comment stating the invariant.
- **Module structure**: modules are organised into directories by
  responsibility (`output/`, `config/`, `modes/`, `interpreters/`), plus a
  few top-level modules. Extend an existing module before
  adding one; keep each module's public surface as small as the crate needs.
- **Abstraction**: introduce a trait only for a real polymorphism seam with
  a second implementation in sight. No speculative abstractions.
- **Logging**: use the `log` crate via qualified `log::level!(...)` calls,
  initialised once with `env_logger` at the binary entry. Levels: `info`
  for startup/config summaries; `debug` for per-event trace; `warn` for
  recoverable anomalies; `error` for non-fatal failures that are logged and
  swallowed. No `trace!`.
- **Platform gating**: split OSes with `#[cfg(target_os = "macos")]` /
  `#[cfg(not(target_os = "macos"))]` / `#[cfg(unix)]`; gate optional host
  capabilities with the build-probed `#[cfg(has_symbol_X)]` /
  `#[cfg(has_executable_X)]` keys, never on `target_os = "linux"`.
- **Lint suppressions**: only `#[allow(...)]` when there is no better fix,
  always with a trailing `// reason` comment.
- **Comments** explain *why*, not *what*; default to none unless a subtle
  invariant would otherwise need re-deriving.
