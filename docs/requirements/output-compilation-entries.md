---
title: Compilation entries from an intercepted invocation
status: implemented
---

## Intent

Build systems often compile several sources in one compiler
invocation, or combine compiling and linking in a single command.
Tools that consume `compile_commands.json` (clangd, clang-tidy, and
similar) expect one entry per translation unit -- each entry
describing how to compile exactly one source file on its own, without
the noise of linking or of siblings compiled in the same invocation.

Bear's job is to turn each intercepted compiler invocation into zero,
one, or many entries, so that downstream tools see a clean,
per-source compile command regardless of how the build system phrased
the invocation.

The rules below describe the user-visible transformation. The JSON
shape of an individual entry is covered separately by
`output-json-compilation-database`. Per-compiler details (which flags
each compiler recognizes, which extensions identify its sources, how
MSVC-style flags differ from GCC/Clang-style) are defined per
compiler family; which executables belong to which family is owned by
the recognition requirements (`recognition-compiler-names` and its
siblings).

## Acceptance criteria

### One entry per compilable source (separable sources)

For compilers whose sources are separable translation units -- each
source compiles on its own and contributes an independent object
(GCC, Clang, the Fortran and CUDA families, MSVC, and the like):

- An invocation that names N source files produces exactly N entries
- Sources are identified by per-family extension lists: each compiler
  family defines which file extensions name its sources
- Each entry's `file` field is one of those sources
- In each entry, the other sources from the same invocation are
  removed from the argument list -- each entry looks like a command
  that compiles only that one source

### One entry per invocation (single-translation-unit compilers)

Some compilers treat all of an invocation's sources as a single
translation unit: they parse the sources together and produce one
combined output rather than one object per source. The Vala compiler
is the motivating example -- it compiles every `.vala`/`.gs` source of
a target together and produces one library/binding. For such a
compiler:

- An invocation that names N source files produces exactly ONE entry,
  regardless of N
- That entry's `file` field is the first source in argument order
- All N sources are retained in the entry's argument list (they are
  not stripped as siblings, because they are not separate units)

Because the entire invocation collapses to one entry keyed on its
first source, duplicate detection (`output-duplicate-detection`) sees
one producer per invocation rather than N near-identical producers;
this is the intended behaviour for a single-translation-unit compiler
and avoids emitting several entries that all claim to produce the same
combined output.

### One entry per source, with the whole invocation's arguments (whole-module compilers)

Some compilers analyze every source of an invocation together as a
single "whole module" but still need one entry per source file,
because per-file consuming tooling looks up a compile command by file
path. Swift's `swiftc` in whole-module mode is the motivating example:
a single invocation lists several `.swift` sources, and each file's
semantics depend on the whole module, so no entry can be reduced to
"this file only" the way a separable-sources entry is. For such a
compiler:

- An invocation that names N source files produces exactly N entries
- Each entry's `file` field is a distinct source from the invocation
- Every entry's argument list is the complete invocation -- every
  source is retained in every entry, not stripped to that entry's own
  file

This is a third, distinct shape: unlike the separable case, no source
is stripped from any entry's arguments; unlike the single-invocation
case, N entries are produced (one per source, not one for the whole
invocation).

### Zero entries for invocations that do not compile a source

Regardless of entry shape, an invocation produces an entry only when a
recognized source is compiled -- never a best-effort guess. In
particular, an invocation produces no entries when any of the
following holds:

- Every positional file argument is an object file, archive, or
  shared library (`.o`, `.obj`, `.a`, `.so`, `.lib`, `.dylib`, ...)
  -- this is a pure link step
- The invocation requests information only (`--version`, `--help`,
  `-###`, `-dumpversion`, ...)
- The invocation requests preprocessing only (`-E`) or dependency
  generation only (`-M` or `-MM` without a compile step)
- Argument parsing finds no recognized source

`-fsyntax-only`, `-MD`, and `-MMD` do compile the source (the last
two emit dependency files as a side effect) and therefore produce an
entry.

### Link-only flags are stripped from compile entries

When a single invocation both compiles and links
(`cc -o a.out -lsomething src.c`), the resulting entry describes only
the compile step. Flags whose effect is limited to the link stage are
removed; each compiler family defines its link-stage flag set, with
GCC-style `-l<name>` as the representative example. Stripping applies
to all three entry shapes.

Flags affecting preprocessing, compiling, or assembling are kept
(`-O2` as the representative example), together with driver-level
options that affect compilation.

### Argument order is preserved

Within a given entry, the remaining arguments appear in the same
relative order as in the original command. Downstream consumers are
order-sensitive (search paths, macro definitions, and warning options
all depend on their relative order), and `-x <lang>` overrides must
keep their position relative to the source files they apply to. The
compiler executable stays at index 0 of `arguments`.

### The `output` field

The per-entry `output` field (see `output-json-compilation-database`)
is optional and off by default. When enabled via configuration, each
entry's `output` is the value of the invocation's output flag, copied
verbatim into every entry produced from that invocation (see Known
limitations); when the invocation has no output flag, the field is
absent.

## Non-functional constraints

- The same rules apply to every compiler family Bear recognizes
  (GCC/Clang, MSVC, Fortran, CUDA, and others); the flag names and
  source extensions named in this file are indicative, not exhaustive
- Source-extension recognition follows the file system's rules: on
  Linux and BSD a file named `foo.C` is treated as a C++ source
  because the extension is `.C`, while on Windows and typical macOS
  configurations the same file also matches extension lists written
  in lowercase
- Response-file (`@file`) tokens pass through untouched by default;
  the default and the opt-in inlining are owned by
  `output-response-file-inlining`

## Known limitations

- For a multi-source invocation that shares a single output flag,
  the output value is copied verbatim into every entry; per-source
  object names (`src1.o`, `src2.o`) are not inferred.

## Testing

Given a build that runs `cc -c src1.c src2.c src3.c` in one
invocation:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains three entries, with `file`
> set to `src1.c`, `src2.c`, and `src3.c` respectively,
> each entry's `arguments` contain `-c` and only its own source
> file,
> and the other two source files do not appear in that entry's
> `arguments`.

Given a build that compiles a single source file:

> When the user runs `bear -- cc -c src.c`,
> then `compile_commands.json` contains exactly one entry,
> with `file` set to `src.c` and the compiler at `arguments[0]`.

Given a build that runs a single-translation-unit compiler over two or
more sources in one invocation (for example `valac src1.vala
src2.vala`):

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains exactly one entry for that
> invocation,
> with `file` set to the first source (`src1.vala`),
> and all of the sources appearing in that entry's `arguments`.

Given a build that runs a whole-module compiler over two or more
sources in one invocation (for example
`swiftc -module-name App a.swift b.swift`):

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains two entries, one for `a.swift`
> and one for `b.swift`,
> and both entries' `arguments` contain both `a.swift` and `b.swift`
> (the whole invocation, not stripped to one source each).

Given a build that runs `cc -o a.out src1.c src2.c src3.c`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains three entries, one per
> source file,
> each entry describes a pure compile step (no link-only flags),
> and no entry's `file` is `a.out`.

Given a build that runs `cc -o a.out obj1.o obj2.o obj3.o`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains no entries for this
> invocation.

Given a build that runs `cc -o a.out -lsomething -O2 src.c`:

> When the user runs Bear wrapping that build,
> then the resulting entry contains `-O2`,
> and it does not contain `-lsomething`.

Given a build that runs `cc -I first -I second -DFOO -DBAR -c src.c`:

> When the user runs Bear wrapping that build,
> then the entry lists `-I first` before `-I second`,
> and `-DFOO` before `-DBAR`, matching the original order.

Given a build that runs `cc --version` or `cc -###`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains no entries for this
> invocation.

Given a build that runs `cc -E src.c` and `cc -M src.c`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains no entries for either
> invocation (preprocess-only and dependency-generation-only steps
> compile nothing).

Given a build that runs `cc -MD -c src.c`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains exactly one entry, with `file`
> set to `src.c` (the dependency file is a side effect of a real
> compile step).

Given a build that runs `cc -o a.out src1.c src2.c` with the
`output` field enabled via configuration:

> When the user runs Bear wrapping that build,
> then every entry's `output` is `a.out` (see Known limitations).

Given a build that runs `cc -c src.c` (no output flag) with the
`output` field enabled via configuration:

> When the user runs Bear wrapping that build,
> then the resulting entry has no `output` key.

## Notes

- Related: `output-json-compilation-database` -- per-entry JSON
  shape.
- Related: `output-env-derived-flags` -- environment variables the
  compiler reads (`CPATH`, `CL`, ...) folded into entry arguments.
- Related: `output-append`, `output-duplicate-detection`,
  `output-source-directory-filter` -- stages that run on the entries
  produced by this step.

## Rationale

- [Recording Vala (valac) builds](../rationale/vala-transpiler-database.md) -
  transpiler/internal-cc handling and Vala source extensions.
- [Swift whole-module entries](../rationale/swift-whole-module-entries.md) -
  why whole-module Swift compiles need one entry per source with the
  full invocation's arguments, rather than the stripped-per-source or
  combined shapes above.
