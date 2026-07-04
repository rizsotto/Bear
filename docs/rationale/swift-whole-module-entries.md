# Swift whole-module entries: one per source, full arguments

## Context

Swift's `swiftc` driver supports whole-module compilation: a single
invocation can name every `.swift` source of a module
(`swiftc -module-name App -wmo a.swift b.swift c.swift`), and the
frontend analyzes them together instead of one at a time. This is the
same "single invocation, multiple sources" shape Bear already handles
for `valac`, but the consumer that motivates Swift support wants a
different entry shape for it.

Three candidate entry shapes were considered, matching the shapes Bear
already produces for other families:

1. **Stripped per-source (the GCC/Clang shape)**: one entry per source,
   each entry's arguments containing only that source, siblings
   removed. Wrong for Swift: SourceKit-LSP reconstructs a file's
   semantic context (imports, symbols visible from other files in the
   module) from the recorded command. Stripping sibling sources from a
   whole-module entry would describe a compilation that cannot
   actually produce that file's symbols, because whole-module
   compilation only makes sense with all participating sources
   present.
2. **Combined (the valac shape)**: one entry for the whole invocation,
   `file` set to the first source, every source retained. This matches
   what a genuine single-translation-unit compiler does (valac
   produces one library from many sources), but Swift's per-file
   editor tooling looks up a compile command by file path -- only one
   file (the first source) gets an entry, so every other source in the
   module has no compile command SourceKit-LSP can find for it.
3. **Per-source, full arguments (the CMake shape)**: one entry per
   source, so every file is independently look-up-able by path, but
   every entry's arguments are the complete invocation -- no sibling
   stripping. CMake's native Swift support already emits exactly this
   shape, and SourceKit-LSP consumes CMake-generated compilation
   databases today, so this is proven prior art rather than a guess
   (https://forums.swift.org/t/sourcekit-lsp-and-cmake/67956,
   https://github.com/swiftlang/sourcekit-lsp/issues/2087).

## Decision

Bear emits shape 3 for Swift: one entry per source file, with every
entry's argument list equal to the complete invocation (all sources,
all flags -- only link-only flags are stripped, same as every other
family). This is a distinct converter mode, not a variant of the
existing separable/combined boolean: the per-invocation flag on
`semantic::Command` becomes a three-way mode (stripped-per-source /
per-source-full / combined) instead of growing a second, orthogonal
flag that every call site would have to reason about in combination.

## Consequences

- Argument duplication is O(N) per source for an N-source whole-module
  invocation: a 200-file whole-module build produces 200 entries that
  each list all 200 sources. This is accepted -- it matches what CMake
  already produces and what SourceKit-LSP already consumes; the
  combined shape leaves 199 of those files unindexable by path
  instead. Revisit only if a concrete report of database size becomes
  a problem.
- A future single-invocation, multiple-source compiler (for example a
  rustc crate-level mode, if ever added) must pick explicitly between
  the combined and per-source-full modes based on its own consumer's
  file-lookup behavior; neither is a safe default for an unresearched
  case.
- `swift-frontend` (and a legacy `swiftc -frontend` self-invocation)
  internal per-file jobs are filtered by `ignore_when`, the same
  mechanism already used for GCC's `cc1` and Clang's `-cc1` -- no new
  filtering mechanism was needed.

## References

- Requirement: `recognition-swift-compiler`
- SourceKit-LSP: https://github.com/swiftlang/sourcekit-lsp
- CMake/SourceKit-LSP Swift discussion:
  https://forums.swift.org/t/sourcekit-lsp-and-cmake/67956
- https://github.com/swiftlang/sourcekit-lsp/issues/2087
