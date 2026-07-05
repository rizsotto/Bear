# Header entry synthesis by sibling cloning

## Context

Getting compile flags for header files was considered in four shapes:

1. Bear runs the compiler with a make-dependencies flag for every translation
   unit. This is accurate, but it executes compilers during analysis and
   couples Bear to compiler availability; rejected.
2. Parse the dependency files the build already emitted. This is accurate and
   works across directories, and requires no compiler execution, but it needs
   the build to have produced those files, plus a make-syntax parser and
   logic to locate each translation unit's dependency file.
3. Parse `#include` directives directly. This amounts to reimplementing a
   preprocessor and gets conditional includes wrong; rejected.
4. Compdb-style sibling cloning: synthesize a header's entry from a
   translation unit compiled in the same directory. No compiler execution, no
   parsing, no prerequisites. Prior art: the `compdb` tool takes this
   approach, and clangd does a variant of it internally.

## Decision

Ship three selectable strategies rather than one. The default is same-directory
sibling cloning (option 4): zero prerequisites, at the cost of accepting
approximate flags when a header's true flags differ from its directory
siblings'. An opt-in extension additionally follows a translation unit's own
user include directories, scoped to those that resolve inside the
compilation's own working directory - the frame the compiler itself resolves
relative include paths against - so that a split `include/`+`src/` layout is
reachable without dependency files when the build compiles from a directory
containing both. A second opt-in strategy reads the dependency files the
build emitted (option 2), for users who need accurate, cross-directory header
lists and whose build produces those files. Running the compiler ourselves
(option 1) stays rejected: parsing artifacts the build already produced is
not the same as executing compilers during analysis.

## Consequences

Sibling cloning alone misses headers in directories that hold no compiled
source, which is common in split `include/`+`src/` layouts; the
include-directories and dependency-file strategies exist to cover that case,
the first without needing dependency files, the second with better accuracy.
The dependency-file strategy adds a make-syntax parser and logic to locate
each translation unit's dependency file. Both the include-directories and
dependency-file strategies scope discovered headers to the compilation's own
working directory, because duplicate detection and validation do not filter
out real system headers on their own - without the scope, following system
include directories, or dependency files emitted with `-MD` (which lists
system headers), would flood the database with system-header noise. Anchoring
the scope to each compilation's working directory rather than a single global
project root keeps the frame the same one the compiler used.

## References

- Requirement: `output-header-entries`
- Related: `swift-whole-module-entries`, `source-filter-last-match-wins`
