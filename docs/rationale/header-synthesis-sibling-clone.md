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

Ship two selectable strategies. The default is same-directory sibling cloning
(option 4): zero prerequisites, at the cost of accepting approximate flags when
a header's true flags differ from its directory siblings'. An opt-in strategy
reads the dependency files the build emitted (option 2), for users who need
accurate, cross-directory header lists and whose build produces those files.
Running the compiler ourselves (option 1) and parsing `#include` directives
(option 3) stay rejected: parsing artifacts the build already produced is not
the same as executing compilers during analysis, and a hand-rolled
preprocessor is a correctness trap.

## Consequences

Sibling cloning alone misses headers in directories that hold no compiled
source, which is common in a split `include/`+`src/` layout; the
dependency-files strategy covers that case, reading the exact set of headers a
translation unit included regardless of where they live. The dependency-file
strategy adds a make-syntax parser and logic to locate each translation unit's
dependency file, and scopes discovered headers to the compilation's own
working directory, because duplicate detection and validation do not filter out
real system headers on their own - without the scope, a dependency file emitted
with `-MD` (which lists system headers) would flood the database with
system-header noise. Anchoring the scope to each compilation's working
directory rather than a global project root keeps the frame the same one the
compiler used to resolve relative paths.

## Rejected: scanning include directories

A third strategy was implemented and then removed: scanning a translation
unit's own `-I`/`-iquote` directories for headers to clone flags onto. It was
meant to reach the split `include/`+`src/` layout without dependency files.
Two mechanisms were on the table - a flat scan of each include directory
(misses headers nested under an include root, which is the norm), and a
source-to-header path heuristic (take the source file's path, strip leading
components, and probe each include directory for a same-stem header).

To judge whether such a heuristic could work in general, we surveyed how four
real projects lay their exposed headers out relative to the implementation
files, working backwards from a nested header to its `.c`/`.cpp`:

- **zlib** (flat): a header is a same-directory sibling of its source
  (`deflate.c` <-> `deflate.h`); `-I` is the source directory itself. The
  public umbrella `zlib.h` has no paired source. The sibling strategy already
  covers this; no include-directory logic is needed.
- **Firefox**: exposed headers are copied into a flat `dist/include` staging
  tree under author-chosen namespaces (`mozilla/dom/Element.h`) that bear no
  relation to the on-disk path (`dom/base/Element.h`). The `-I` dir is the
  staging tree, not the source tree, so no source-relative path probe can
  reconstruct the include path. But the header is a same-directory sibling of
  its `.cpp` on disk (`dom/base/Element.{h,cpp}`), so the sibling strategy
  finds it.
- **Linux**: exposed headers live in an entirely different subtree
  (`include/linux/slab.h`) from the implementation (`mm/slub.c`); the stem
  often differs (`slab` vs `slub`, `sched.h` vs `core.c`), and a large
  majority of headers have no single implementing `.c` (interface headers, the
  UAPI, multi-file subsystems). No path or name heuristic reconstructs the
  pairing.
- **LLVM**: a clean `include/<project>/...` <-> `lib/...` mirror, but the
  include tree inserts a project-namespace directory equal to the `-I` dir's
  own trailing segment, so a source-suffix probe (`IR/Function.h` under
  `-I .../include`) misses the real `include/llvm/IR/Function.h`; much of
  ADT/Support is header-only with no `.cpp` at all.

Three findings follow. First, where a header pairs cleanly with a source it is
almost always a same-directory sibling on disk (Firefox, zlib) - already
handled by `siblings`, with no path guessing. Second, in the split layouts the
include-directory strategy was meant to serve (Linux, LLVM), the source-to-
header relationship is either absent (Linux: different subtree, mismatched
stem) or project-specific (LLVM: an inserted namespace directory), so a generic
path or name heuristic is unreliable exactly where it was needed. Third, a
large fraction of exposed headers have no implementation file at all (interface,
umbrella, UAPI, header-only templates), which no source-to-header pairing can
ever reach.

Because `dependency-files` reads the exact included header paths from the
compiler's own output, it handles all of these - cross-subtree headers,
namespaced include trees, and header-only files - accurately and without
project-specific assumptions. Include-directory scanning added configuration
surface and code (include-flag extraction, a working-directory scope rule, and
an unresolved recursive-scan-versus-path-pairing choice) while covering, at
best, cases `siblings` already covers and failing on the split layouts it was
meant to solve. It was removed; `siblings` plus `dependency-files` span the
space.

## References

- Requirement: `output-header-entries`
- Related: `swift-whole-module-entries`, `source-filter-last-match-wins`
