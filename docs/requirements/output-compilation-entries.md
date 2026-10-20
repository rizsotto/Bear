---
title: Compilation entries from an intercepted invocation
status: implemented
---

## Intent

Bear turns each recognized compiler invocation into zero, one, or many
entries. Each entry tells a tool such as clangd or clang-tidy how to
compile one source, whatever way the build phrased the invocation. The
JSON shape of an entry is owned by `output-json-compilation-database`.
Which executables are compilers, and which family each belongs to, is
owned by `recognition-compiler-names`. Flags folded in from the
environment are owned by `output-env-derived-flags`. Later stages that
filter the entries are owned by `output-append`,
`output-duplicate-detection`, and `output-source-directory-filter`.

## Acceptance criteria

The compiler family definition sets which entry shape applies:

| Shape | Example | Entries for N sources | `file` | Sources kept in each entry's arguments |
|---|---|---|---|---|
| Separable sources (GCC, Clang, Fortran, CUDA, MSVC, and the like) | `cc -c a.c b.c` | N | each source in turn | only that entry's own source |
| Single translation unit (Vala) | `valac a.vala b.vala` | 1 | the first source in argument order | all N |
| Whole module (Swift) | `swiftc -module-name App a.swift b.swift` | N | each source in turn | all N |

Sources are identified by extension, from one built-in list shared by
every compiler family.

- **Given** a build that runs `cc -c src1.c src2.c src3.c` in one
  invocation, **when** Bear finishes, **then** the database holds three
  entries with `file` set to `src1.c`, `src2.c`, and `src3.c`, and each
  entry's arguments hold `-c` and only its own source.
- **Given** a build that runs `cc -c src.c`, **when** Bear finishes,
  **then** the database holds one entry, with `file` set to `src.c` and
  the compiler at `arguments[0]`.
- **Given** a build that runs `valac src1.vala src2.vala`, **when** Bear
  finishes, **then** the database holds one entry for that invocation,
  with `file` set to `src1.vala`, and both sources in its arguments.
- **Given** a build that runs `swiftc -module-name App a.swift b.swift`,
  **when** Bear finishes, **then** the database holds one entry for
  `a.swift` and one for `b.swift`, and the arguments of both hold both
  sources.
- **Given** a build that runs `cc -o a.out src1.c src2.c src3.c`,
  **when** Bear finishes, **then** the database holds one entry per
  source, each a pure compile step, and no entry has `a.out` as `file`.
- **Given** a build that runs `cc -o a.out obj1.o obj2.o obj3.o`, where
  every file argument is an object file, archive, or shared library,
  **when** Bear finishes, **then** the database holds no entry for that
  link step.
- **Given** a build that runs `cc --version` or `cc -###`, which only
  request information, **when** Bear finishes, **then** the database
  holds no entry for either.
- **Given** a build that runs `cc -E src.c` (preprocess only) and
  `cc -M src.c` (dependency generation only), **when** Bear finishes,
  **then** the database holds no entry for either. The same holds for
  any invocation where argument parsing finds no recognized source:
  Bear never makes a best-effort guess.
- **Given** a build that runs `cc -MD -c src.c`, **when** Bear finishes,
  **then** the database holds one entry for `src.c`. `-MD`, `-MMD`, and
  `-fsyntax-only` do compile the source, so they produce an entry.
- **Given** a build that runs `cc -o a.out -lsomething -O2 src.c`,
  **when** Bear finishes, **then** the entry holds `-O2` and not
  `-lsomething`. Flags that only affect the link stage are removed, in
  every entry shape; each compiler family defines that set. Flags that
  affect preprocessing, compiling, or assembling are kept.
- **Given** a build that runs `cc -I first -I second -DFOO -DBAR -c
  src.c`, **when** Bear finishes, **then** the entry lists `-I first`
  before `-I second`, and `-DFOO` before `-DBAR`. The kept arguments
  keep their relative order from the original command, and the compiler
  stays at `arguments[0]`.
- **Given** the `output` field is turned on in the configuration and a
  build that runs `cc -o a.out src1.c src2.c`, **when** Bear finishes,
  **then** every entry's `output` is `a.out`. The `output` field is off
  by default. Its path spelling is owned by `output-path-format`.
- **Given** the `output` field is turned on and a build that runs
  `cc -c src.c` with no output flag, **when** Bear finishes, **then**
  the entry has no `output` key.

## Non-functional constraints

- The same rules apply to every compiler family Bear recognizes. The
  flags and extensions named here are examples, not the full sets.
- Extensions match exactly, case included, on every platform. `foo.C`
  is a source because `.C` is in the list, not because `.c` is.
- Response-file (`@file`) arguments are owned by
  `output-response-file-inlining`.

## Known limitations

- A multi-source invocation with one output flag gives every entry the
  same `output` value. Per-source object names (`src1.o`, `src2.o`) are
  not inferred.

## Rationale

- [Entry shapes for clang tooling](../rationale/compilation-entry-shapes.md)
- [Recording Vala (valac) builds in the compilation database](../rationale/vala-transpiler-database.md)
- [Swift whole-module entries: one per source, full arguments](../rationale/swift-whole-module-entries.md)
