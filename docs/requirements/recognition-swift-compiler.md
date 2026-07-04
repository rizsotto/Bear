---
title: Recognize Swift compiler (swiftc) invocations
status: implemented
---

## Intent

When a build compiles Swift sources through `swiftc` -- make, a custom
script, or any build system other than SwiftPM/CMake -- Bear records
those compilations in the compilation database, so SourceKit-LSP and
other clangd-adjacent tooling can provide editor support for Swift code
without a dedicated build-system integration.

## Acceptance criteria

- An execution of `swiftc` naming one or more `.swift` sources yields a
  database entry per source file.
- A whole-module invocation that names several `.swift` sources in a
  single command yields one entry per source file, and every one of
  those entries carries the complete invocation's arguments (every
  source, not just the entry's own file). Swift's whole-module
  compilation gives each file's semantics a dependency on the whole
  module, unlike a separable-sources compiler, so no entry can be
  reduced to "its own file only" the way GCC/Clang entries are.
- Internal per-file frontend jobs that `swiftc` spawns (`swift-frontend`,
  or a legacy toolchain re-invoking itself as `swiftc -frontend`) yield
  no database entries.
- The recorded command is the `swiftc` driver invocation exactly as
  executed -- not an expansion of it, and not the frontend job it may
  spawn internally.
- The `swift` subcommand driver (`swift build`, `swift run`,
  `swift package`, ...) is NOT recognized as a compiler.

## Non-functional constraints

- macOS System Integrity Protection restricts `DYLD_INSERT_LIBRARIES`
  for Apple-signed binaries, including the Xcode-provided `swiftc`; on
  such toolchains Bear's wrapper interception mode applies instead of
  preload, the same constraint every other Apple-signed compiler is
  already subject to.

## Testing

Given an event file with a
`swiftc -module-name App -emit-object a.swift b.swift` execution:

> When `bear semantic` runs,
> then the database contains two entries, one for `a.swift` and one
> for `b.swift`,
> and both entries' arguments contain `a.swift`, `b.swift`, and
> `-module-name App`.

Given an event file with a `swiftc -c hello.swift` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.swift`.

Given an event file with a `swift-frontend` execution (or a
`swiftc -frontend ...` execution on a legacy toolchain that
re-invokes itself):

> When `bear semantic` runs,
> then the database contains no entry for that execution.

Given an event file with a `swiftc --version` execution:

> When `bear semantic` runs,
> then the database contains no entry for that execution.

## Notes

- `swift` (the package-manager subcommand driver: `swift build`,
  `swift run`, `swift package`, ...) is deliberately not recognized.
  Its command-line model is a subcommand dispatcher, not a compiler
  invocation -- the same model mismatch that keeps `zig cc` out of
  Bear's recognition today. Matching the bare `swift` basename would
  misfire on every non-build subcommand. Revisit only if Bear ever
  gains subcommand-argument-aware recognition (the `zig cc` /
  `dotnet exec csc.dll` class of problem).
- `-index-store-path` is a documented `swiftc` flag but Bear never
  injects it; SourceKit-LSP's cross-file indexing benefits when the
  build already passes it, but that is the build's choice, not Bear's.
- Consumer: SourceKit-LSP reads `compile_commands.json` /
  `compile_flags.txt` for Swift
  (https://github.com/swiftlang/sourcekit-lsp). CMake's native Swift
  support already emits `swiftc` entries in exactly the per-source,
  full-arguments shape this requirement adopts, and SourceKit-LSP
  consumes them
  (https://forums.swift.org/t/sourcekit-lsp-and-cmake/67956,
  https://github.com/swiftlang/sourcekit-lsp/issues/2087).

## Rationale

- [Swift whole-module entries](../rationale/swift-whole-module-entries.md) -
  why Swift needs one entry per source with the full invocation's
  arguments, rather than the stripped-per-source or combined shapes
  Bear already had.
