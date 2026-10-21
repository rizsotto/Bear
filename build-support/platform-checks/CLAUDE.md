## platform-checks

Build-time helper library. Each consumer calls it from its own
`build.rs`, and it probes the C toolchain of that consumer's target for
symbols. Probes run once per consumer. The probe list and the API are in
`src/lib.rs`. `SYSTEM_EXTENSIONS` there is the feature-macro list
for probes and consumer C sources.

## Constraints

- A consumer takes this crate as a `[build-dependencies]` entry and calls
  `platform_checks::detect()`, then `platform_checks::emit_cfg()` with the
  result and `platform_checks::emit_check_cfg()`, from its own `build.rs`.
  The `cfg` keys apply only to the calling crate.
- The crate has no `build.rs`. Probing there would see the host as the
  target in cross builds.
- `detect()` returns nothing on non-unix targets and panics on a unix
  target where `execve` is missing.

## Adding a probe

1. Add the symbol name to `SYMBOL_PROBES` in `src/lib.rs`. The probe links a
   program that calls it, so it passes only if the target libc exports it.
2. Reference the resulting `cfg(has_symbol_X)` from consumer source.
3. Provided the consumer's `build.rs` calls
   `platform_checks::emit_check_cfg()`, the lint allowlist is covered
   automatically; no `Cargo.toml` `[lints.rust]` editing needed.

## What this crate does NOT decide

It detects target capabilities, not consumer behavior. Which symbols the
preload shim exports is decided in `crates/intercept-preload`. Adding an
intercepted function touches `SYMBOL_PROBES` here as one of three edits;
`crates/intercept-preload/CLAUDE.md` lists them.
