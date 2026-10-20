# Linker selection for the preload library

## Context

The preload library must export exactly the interposer symbols that the
C shim defines (the `exec` family, `posix_spawn*`, `popen`, `pclose`,
`system`) plus `LIBEXEC_VERSION`, and hide everything else. On ELF
platforms the build script does this with a generated linker version
script, `exports.map`.

rustc adds its own version script to every cdylib link: an anonymous tag
that exports the crate's `#[no_mangle]` symbols and ends in `local: *`.
Two facts follow, both checked by experiment (2026-07):

- Bear's script cannot be dropped. rustc's script hides every symbol from
  a linked native static archive. Without `exports.map` the interposers
  leave the dynamic table and interception silently stops. This holds
  even with rustc's own `-l static:+whole-archive=...`.
- The link line always has two version scripts, and GNU ld refuses that:
  "anonymous version tag cannot be combined with other version tags"
  (binutils 2.46). A named tag in Bear's script does not help, because
  rustc's tag is the anonymous one. This is the failure in
  [#662](https://github.com/rizsotto/Bear/issues/662) (Ubuntu 24.04
  aarch64).

lld and mold accept the pair and export the union of both scripts'
globals. On `x86_64-unknown-linux-gnu`, stable rustc links cdylibs with
its bundled rust-lld by default. Other targets, such as aarch64, use the
system `cc` default linker, GNU ld. That is why
[#662](https://github.com/rizsotto/Bear/issues/662) hit ARM64 only. The
build used to hard-code `-fuse-ld=lld`
([#664](https://github.com/rizsotto/Bear/pull/664)), which fails where
no lld is installed.

Options considered:

- Drive the final link with `cc` from a staticlib. One version script
  links fine under GNU ld (checked). But stable cargo cannot do it: a
  build script cannot depend on a sibling crate's staticlib (the gap
  that RFC 3028 "artifact dependencies" fills), a crate's own build
  script runs before the crate compiles, and linking outside cargo
  breaks the single `cargo build` and adds a packaging step.
- A named version tag in `exports.map`. GNU ld rejects any pair that
  includes an anonymous tag (checked).
- Drop `exports.map` and rely on rustc's export handling. Native-archive
  symbols are never in rustc's export list (checked).
- A fallback chain of linkers that accept two scripts. Chosen.

## Decision

The build script picks the ELF linker in this order: the toolchain's
bundled rust-lld, a system `ld.lld`, a system `mold`. If none is found,
it adds no linker flag and prints a build warning that names the GNU ld
error and the fix, in case the default linker copes anyway.

## Consequences

- rustup toolchains build the library on every architecture with no
  extra linker package.
- A Rust toolchain without bundled rust-lld (some distro packages) needs
  `lld` or `mold`. Both are widely packaged. Without them the user gets
  an early warning before the cryptic linker error.
- The system probes search `PATH` only. An `ld.lld` that `cc` finds
  through its own exec prefix falls through to the warning. Revisit, for
  example with `cc -print-prog-name=ld.lld`, if a user reports that case.
- The union means the `rust_<name>` implementation symbols also show up
  in the dynamic table, despite `local: *` in `exports.map`. This is
  cosmetic: the names are prefixed and calls resolve inside the library.
  Removing them needs a function-pointer indirection between the shim
  and the Rust side, or a rustc option to exclude them. Revisit if a
  strict export surface becomes a requirement.
- Revisit the staticlib option when cargo artifact dependencies are
  stable.

## References

- [interception-preload-mechanism](../requirements/interception-preload-mechanism.md), the requirement this supports
- [#662](https://github.com/rizsotto/Bear/issues/662), the GNU ld link failure on aarch64
- [#664](https://github.com/rizsotto/Bear/pull/664), the original hard-coded `-fuse-ld=lld`
- [How to use linker version scripts in Rust 1.54](https://users.rust-lang.org/t/how-to-use-linker-version-scripts-in-rust-1-54/64033), the rustc version-script behavior
