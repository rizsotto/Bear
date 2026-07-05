# Generated-file filtering uses user-supplied globs

## Context

Users repeatedly ask to keep machine-generated sources - Qt `moc_*.cpp`,
protobuf `*.pb.cc`, and the output of other code generators - out of
`compile_commands.json`, so that clang-tidy and editor tooling run only over
hand-written code. Two shapes were considered:

1. Bear ships built-in knowledge of common generator outputs and filters them
   automatically. This is wrong the day a project uses a generator Bear has
   never heard of, and it silently drops files a user may want kept.
2. The user supplies filename patterns in configuration, mirroring the existing
   directory-rule mechanism.

Glob matching needs a dependency. The candidates were `glob` (rust-lang owned,
no transitive dependencies) and `globset` (faster, but pulls in
`regex-automata` and its dependencies). Bear's source filtering runs once per
entry over a set that is small relative to a whole build; matching speed is not
a bottleneck.

## Decision

User-supplied glob rules, evaluated with the same last-match-wins semantics as
the directory rules. Use the `glob` crate for its zero-transitive-dependency
footprint. Split basename-vs-full-path matching on whether the pattern contains
a path separator, following the gitignore convention so that users' intuition
carries over.

## Consequences

There is no out-of-the-box filtering: a user writes two lines of configuration
to exclude a generator's output. The man page carries the `moc`/protobuf recipes
so the common cases are copy-paste. Directory rules and file-pattern rules
compose (both must accept an entry), so the two mechanisms mix without
surprising interactions. Revisit the `globset` choice only if profiling ever
shows glob matching to be a real cost.

## References

- Requirement: `output-generated-file-filter`
- Related: `source-filter-last-match-wins`
