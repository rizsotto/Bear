## Preload interception library

`README.md` in this directory is the reference for what this crate is, the
C/Rust split and the reasons for it, the supported-platform matrix, and the
build instructions. This file covers only the constraints an agent needs.

## Architecture constraint

The C shim (`src/c/shim.c`) and Rust (`src/implementation.rs`) split is
mandatory; the rationale is in `README.md`. Do not merge the C and Rust parts.

## Unsafe code

This crate is where almost all of Bear's `unsafe` lives (FFI, `dlsym`,
async-signal-safe handlers). Annotate it consistently:

- Every `unsafe fn` carries a `/// # Safety` doc comment stating the
  caller's obligations.
- Each `unsafe {}` block gets an inline `// SAFETY:` comment when its
  reasoning is not already covered by the enclosing function's `# Safety`
  section.

## Build script

On ELF platforms, `build.rs` picks the linker. The order and why GNU ld is out
are in `docs/rationale/preload-linker-selection.md`; the user-facing
linker prerequisite is in `INSTALL.md`. If the linker set or order
changes, update the code and the rationale. If the set changes, update
`INSTALL.md` too.

`INTERCEPT_FAMILY` in `build.rs` lists the symbols `src/c/shim.c`
exports. Source of truth is `src/c/shim.c` itself.

Adding or removing an intercepted function is a three-place edit:

1. `src/c/shim.c` - the `EXPORT` wrapper itself (and the matching
   `extern` declaration of the `rust_<name>` implementation).
2. `intercept-preload/build.rs::INTERCEPT_FAMILY` - so the symbol
   passes the filter against `platform_checks::DETECTED_SYMBOLS`
   and is written into the version script / exports list.
3. `platform-checks/build.rs::SYMBOL_PROBES` - so the symbol is
   probed on the host at all. If it is not listed here it never
   enters `DETECTED_SYMBOLS`, and the `INTERCEPT_FAMILY` filter
   silently drops it, leaving the wrapper hidden by `local: *;`.

Miss any one of the three and the wrapper compiles but is invisible
to the dynamic linker, so libc handles the call instead of us.
Pairs that travel together (e.g. `popen`/`pclose`) each need their
own entry in all three places.

## Before modifying

- The "doctor" logic that maintains interception across `exec` calls is subtle - read it fully before changing
- Reports go to the TCP collector in `crates/intercept-supervisor`, in the wire format from `crates/intercept` (`tcp.rs`) - do not change the protocol without updating the collector
