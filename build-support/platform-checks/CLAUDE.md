## platform-checks

Build-time helper crate. Probes the host for symbols once
per workspace build and exposes the results to consumer build scripts.
The probe lists are in `build.rs`; the API is in `src/lib.rs`.

## Constraints

- A consumer takes this crate as a `[build-dependencies]` entry and calls
  both `platform_checks::emit_cfg()` and `platform_checks::emit_check_cfg()`
  from its own `build.rs`. The `cfg` keys apply only to the calling crate.
- The library has no dependencies. `cc` and `tempfile` are
  `[build-dependencies]` only.

## Adding a probe

1. Add an entry to `SYMBOL_PROBES` in `build.rs`.
2. Reference the resulting `cfg(has_symbol_X)` from consumer source.
3. Provided the consumer's `build.rs` calls
   `platform_checks::emit_check_cfg()`, the lint allowlist is covered
   automatically; no `Cargo.toml` `[lints.rust]` editing needed.

## What this crate does NOT decide

It detects host capabilities, not consumer behavior. Which symbols the
preload shim exports is decided in `crates/intercept-preload`. Adding an
intercepted function touches `SYMBOL_PROBES` here as one of three edits;
`crates/intercept-preload/CLAUDE.md` lists them.
