# Entry shapes for clang tooling

## Context

Build systems often compile several sources in one compiler call, or
compile and link in one command. Tools that read the compilation
database (clangd, clang-tidy, and similar) look up a compile command by
file path and expect it to compile that one file on its own. Link flags
and sibling sources in that command are noise to them.

These tools are also order-sensitive. Include search paths, macro
definitions, and warning options depend on their relative order, and a
`-x <lang>` override applies to the sources after it. Reordering the
arguments would change what the command means.

Not every compiler fits the one-source-per-entry shape. Vala's `valac`
compiles all sources of a target as one translation unit and produces
one combined output. Swift's `swiftc` in whole-module mode analyzes all
sources together, but editors still need a command per file.

## Decision

By default, one entry per source, with the sibling sources and the
link-only flags removed and the rest kept in the original order. A
single-translation-unit compiler gets one entry per invocation, keyed
on its first source, with every source kept. A whole-module compiler
gets one entry per source, each with the whole invocation's sources.

## Consequences

For a single-translation-unit compiler, duplicate detection sees one
entry per invocation, not several entries that all claim the same
combined output. Only the first source of such an invocation can be
looked up by path.

A new compiler family must pick its shape from how its consumers look
up files. The Swift trade-offs are in
[swift-whole-module-entries](swift-whole-module-entries.md).

## References

- [output-compilation-entries](../requirements/output-compilation-entries.md), the requirement this supports
- [vala-transpiler-database](vala-transpiler-database.md), the Vala case
- [swift-whole-module-entries](swift-whole-module-entries.md), the Swift case
