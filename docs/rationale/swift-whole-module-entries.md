# Swift whole-module entries: one per source, full arguments

## Context

`swiftc` supports whole-module compilation. One call can name every
`.swift` source of a module
(`swiftc -module-name App -wmo a.swift b.swift c.swift`), and the
frontend analyzes them together. `valac` has the same many-sources,
one-call shape, but the consumer here,
[SourceKit-LSP](https://github.com/swiftlang/sourcekit-lsp), needs a
different entry shape.

Three entry shapes were considered, all of which Bear already makes for
some family:

- Per source, siblings stripped (the GCC and Clang shape). Rejected:
  SourceKit-LSP rebuilds a file's semantic context (imports, symbols
  from other files of the module) from the recorded command. Without the
  sibling sources the command describes a compilation that cannot
  produce that file's symbols.
- One combined entry, `file` set to the first source (the `valac`
  shape). Rejected: SourceKit-LSP looks up a command by file path, so
  every source but the first would have no command.
- Per source, full arguments (the CMake shape): one entry per source,
  each with the whole call. Chosen. CMake's Swift support already
  writes this, and SourceKit-LSP reads CMake databases today
  ([forum thread](https://forums.swift.org/t/sourcekit-lsp-and-cmake/67956),
  [sourcekit-lsp#2087](https://github.com/swiftlang/sourcekit-lsp/issues/2087)).

The per-call setting could have grown a second flag next to the
existing separable/combined one. Rejected: every call site would have
to reason about the two flags together.

## Decision

Bear writes one entry per Swift source, each with the complete call:
all sources and all flags, minus link-only flags as for every family.
The per-call source mode is a three-way choice (stripped per source,
full per source, combined).

## Consequences

- An N-source whole-module call gives N entries that each list all N
  sources. Accepted: it is what CMake writes and SourceKit-LSP reads,
  and the combined shape would leave N-1 files with no command. Revisit
  only if someone reports database size as a real problem.
- A future many-sources, one-call compiler must choose between the
  combined and full-per-source modes from how its consumer looks up
  files. Neither is a safe default.
- The per-file `swift-frontend` jobs (and the legacy
  `swiftc -frontend` self-call) are filtered by the same `ignore_when`
  rule used for GCC's `cc1` and Clang's `-cc1`.

## References

- [recognition-compiler-names](../requirements/recognition-compiler-names.md), the requirement that adds `swiftc`
- [output-compilation-entries](../requirements/output-compilation-entries.md), the entry-shape contract
- [compilation-entry-shapes](compilation-entry-shapes.md), the entry shapes per compiler kind
- [vala-transpiler-database](vala-transpiler-database.md), the combined shape
- [SourceKit-LSP](https://github.com/swiftlang/sourcekit-lsp), the consumer
- [SourceKit-LSP and CMake](https://forums.swift.org/t/sourcekit-lsp-and-cmake/67956), the forum thread on CMake's Swift databases
- [sourcekit-lsp#2087](https://github.com/swiftlang/sourcekit-lsp/issues/2087), a related SourceKit-LSP issue
