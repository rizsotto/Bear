# Header entries: clone a sibling by default, read dependency files on request

## Context

Headers need compile flags too, but the build never compiles them on
their own. Five ways to get their flags were considered:

1. Bear runs the compiler with a make-dependencies flag for every
   translation unit. Accurate, but it runs compilers during analysis and
   ties Bear to the compiler being there. Rejected.
2. Read the dependency files the build already wrote. Accurate, works
   across directories, and runs no compiler. But the build must have
   written those files, and Bear needs a make-syntax parser and a way to
   find each translation unit's dependency file.
3. Parse `#include` directives. That is a hand-made preprocessor, and
   it gets conditional includes wrong. Rejected.
4. Clone a sibling, as the `compdb` tool does and clangd does in a
   variant inside: a header gets the entry of a translation unit
   compiled in the same directory. No compiler run, no parsing, no
   prerequisites. But it misses headers in directories with no compiled
   source, which is common in a split `include/` + `src/` layout.
5. Scan a translation unit's own `-I`/`-iquote` directories for headers
   to clone flags onto. Meant to reach the split layout without
   dependency files. It was built and then removed. A flat scan misses
   headers nested under an include root, which is the norm. A
   source-to-header path guess (strip leading parts of the source path
   and look for a same-stem header in each include directory) needs the
   layout to pair headers with sources.

To judge option 5, four real projects were surveyed, working back from
a nested header to its source:

- zlib (flat): a header sits next to its source (`deflate.c`,
  `deflate.h`), and `-I` is the source directory. The umbrella `zlib.h`
  has no source. Sibling cloning already covers it.
- Firefox: public headers are copied into a flat `dist/include` tree
  under chosen names (`mozilla/dom/Element.h`) unrelated to the source
  path (`dom/base/Element.h`). `-I` points at that copy, so no
  source-relative guess can rebuild the include path. On disk the header
  sits next to its `.cpp`, so sibling cloning finds it.
- Linux: public headers live in a separate tree (`include/linux/slab.h`)
  from the code (`mm/slub.c`), the stem often differs (`slab` and
  `slub`, `sched.h` and `core.c`), and most headers have no single
  implementing `.c`. No path or name guess pairs them.
- LLVM: a clean `include/<project>/...` and `lib/...` mirror, but the
  include tree adds a project directory, so a source-suffix guess
  (`IR/Function.h` under `-I .../include`) misses
  `include/llvm/IR/Function.h`. Much of ADT and Support is header-only.

So where a header pairs cleanly with a source, it is almost always a
sibling on disk, which option 4 handles. In the split layouts option 5
was for, the pairing is either missing (Linux) or project-specific
(LLVM). And many headers have no source at all (interface, umbrella,
UAPI, header-only templates), which no pairing can reach. Option 5 added
configuration and code while covering at best what option 4 already
covers. Option 2 handles all of these cases from the compiler's own
output.

## Decision

Bear ships two strategies. The default clones a same-directory sibling
(option 4) and accepts approximate flags; an opt-in strategy reads the
build's dependency files (option 2) for exact, cross-directory headers.

## Consequences

- The default needs nothing from the build, but its flags are wrong
  when a header's real flags differ from its siblings', and it finds no
  header in a directory without compiled sources.
- The dependency-file strategy needs a build that writes those files,
  and it brings a make-syntax parser and the logic to find each file.
- Dependency-file headers are limited to the compilation's own working
  directory. Duplicate detection and entry checks do not drop system
  headers, so without the limit a file written with `-MD` (which lists
  system headers) would flood the database. The limit uses each
  compilation's working directory, not a global project root, because
  that is the frame the compiler used to resolve relative paths.

## References

- [output-header-entries](../requirements/output-header-entries.md), the requirement this supports
- [source-filter-last-match-wins](source-filter-last-match-wins.md), the source filter, which runs before header synthesis
- [compdb](https://github.com/Sarcasm/compdb), the sibling-cloning prior art
