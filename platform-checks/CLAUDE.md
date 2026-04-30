## platform-checks

Build-time helper crate. Probes the host for headers and symbols
(`dlfcn.h`, `unistd.h`, `execve`, `posix_spawn`, `popen`, ...) once
per workspace build and exposes the results to consumer build scripts.

## How it works

- `build.rs` invokes `cc::Build::try_compile` on tiny C probes
  inside `OUT_DIR`, writes the outcome to `OUT_DIR/detected.rs`,
  and the library `include!()`s that file.
- Consumers (`intercept-preload`, `integration-tests`) declare
  `platform-checks` as a `[build-dependencies]`. Cargo guarantees
  this crate's `build.rs` runs before any consumer's, exactly once
  per `cargo build`.
- Consumers call `platform_checks::emit_cfg()` and
  `platform_checks::emit_check_cfg()` from their own `build.rs`.
  Those functions print `cargo:rustc-cfg=` and
  `cargo:rustc-check-cfg=` directives, which apply to the calling
  crate.

## Public API

| Item | Purpose |
|---|---|
| `DETECTED_HEADERS: &[&str]` | Headers compilable on this host (e.g. `"dlfcn_h"`) |
| `DETECTED_SYMBOLS: &[&str]` | Symbols linkable on this host (e.g. `"execve"`) |
| `KNOWN_HEADERS: &[&str]` | Every probed header, found or not |
| `KNOWN_SYMBOLS: &[&str]` | Every probed symbol, found or not |
| `emit_cfg()` | `cargo:rustc-cfg=has_*_X` for everything detected |
| `emit_check_cfg()` | `cargo:rustc-check-cfg=cfg(has_*_X)` for everything probed |

`cc` and `tempfile` are `[build-dependencies]` only; the library
proper is dependency-free.

## Adding a probe

1. Add an entry to `HEADER_PROBES` or `SYMBOL_PROBES` in `build.rs`.
2. Reference the resulting `cfg(has_*_X)` from consumer source.
3. Provided the consumer's `build.rs` calls
   `platform_checks::emit_check_cfg()`, the lint allowlist is covered
   automatically; no `Cargo.toml` `[lints.rust]` editing needed.

## What this crate does NOT decide

It detects host capabilities, not consumer behavior. The list of
symbols that the preload C shim actually exports lives in
`intercept-preload/build.rs::INTERCEPT_FAMILY`; the source of truth
is `intercept-preload/src/c/shim.c`.
