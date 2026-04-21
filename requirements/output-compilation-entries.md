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
MSVC-style flags differ from GCC/Clang-style) are defined by Bear's
per-compiler interpreters.

## Acceptance criteria

### One entry per compilable source

- An invocation that names N source files produces exactly N entries
- Recognized source extensions include `.c`, `.cc`, `.cpp`, `.cxx`,
  `.m`, `.mm`, `.S`, `.s`, `.f`, `.f90`, `.cu`, and other language-
  specific extensions defined by the per-compiler interpreters
- Each entry's `file` field is one of those sources
- In each entry, the other sources from the same invocation are
  removed from the argument list -- each entry looks like a command
  that compiles only that one source
- If the same source appears more than once in the same invocation
  (`cc -c foo.c foo.c`), one entry is produced per positional
  occurrence; deduplication is then the responsibility of
  `output-duplicate-detection`

### Zero entries for invocations that do not compile a source

An invocation produces no entries when any of the following holds:

- Every positional file argument is an object file, archive, or
  shared library (`.o`, `.obj`, `.a`, `.so`, `.lib`, `.dylib`, ...)
  -- this is a pure link step
- The invocation requests information only (`--version`, `--help`,
  `-###`, `-dumpversion`, ...)
- The invocation requests preprocessing only (`-E`) or dependency
  generation only (`-M` or `-MM` without a compile step)
- The executable is not a recognized compiler, or argument parsing
  does not find a source; Bear emits no entry rather than a
  best-effort guess

`-fsyntax-only`, `-MD`, and `-MMD` do compile the source (the last
two emit dependency files as a side effect) and therefore produce an
entry.

### Link-only flags are stripped from compile entries

When a single invocation both compiles and links
(`cc -o a.out -lsomething src.c`), the resulting entry describes only
the compile step. Flags whose effect is limited to the link stage are
removed.

GCC/Clang-style examples: `-l<name>`, `-L<dir>`, `-Wl,...`,
`-Xlinker ...`, `-shared`, `-static`, `-rdynamic`. MSVC counterpart:
`/link` and every argument following it.

Preprocessing, compiling, and assembling flags (`-D`, `-I`,
`-isystem`, `-iquote`, `-std=...`, `-O2`, `-Wall`, `-c`, `-S`,
`-x <lang>`, and their MSVC equivalents) are kept, together with
driver-level options that affect compilation.

### Argument order is preserved

Within a given entry, the remaining arguments appear in the same
relative order as in the original command. Downstream consumers are
order-sensitive:

- Include search paths (`-I`, `-isystem`, `-iquote`) are searched in
  the order they appear
- Later `-D` definitions override earlier ones
- `-W` options can enable and then disable the same warning
- `-x <lang>` language overrides apply to subsequent source files
  and must keep their position relative to them
- The compiler executable stays at index 0 of `arguments`

### The `output` field

The per-entry `output` field (see `output-json-compilation-database`)
is optional and off by default. When enabled via configuration, Bear
records the value of the invocation's output flag (`-o`, MSVC `/Fo`,
`/Fe`) and emits it in each entry produced from that invocation.

- For a single-source invocation (`cc -c src.c -o src.o`), the entry's
  `output` is `src.o`.
- For a multi-source invocation with a single output flag
  (`cc -o a.out src1.c src2.c src3.c`), Bear copies the output value
  verbatim into **every** entry. All three entries report
  `output` = `a.out`, even though a real build would produce
  `src1.o`, `src2.o`, and `src3.o`. Per-source inference of object
  names is a known gap; see Notes.
- When the invocation has no output flag, the `output` field is
  absent.

## Non-functional constraints

- The same rules apply to every compiler family recognized by Bear's
  interpreters (GCC/Clang, MSVC, Fortran, CUDA, and others); the
  flag names and source extensions listed above are indicative, not
  exhaustive
- Source-extension recognition follows the file system's rules: on
  Linux and BSD a file named `foo.C` is treated as a C++ source
  because the extension is `.C`, while on Windows and typical macOS
  configurations the same file also matches extension lists written
  in lowercase
- Response files (`@argfile`) are observed as written by the build;
  Bear does not expand them. An entry whose original command used
  `@args.rsp` will contain `@args.rsp` in its `arguments`

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

Protected by `multiple_sources_single_command`.

Given a build that compiles multiple files via separate compiler
invocations (the typical `make -j` case):

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains one entry per source file,
> and each entry names the compiler used for that source.

Protected by `successful_build_multiple_sources`.

Given a build that compiles a single source file:

> When the user runs `bear -- cc -c src.c`,
> then `compile_commands.json` contains exactly one entry,
> with `file` set to `src.c` and the compiler at `arguments[0]`.

Protected by `simple_single_file_compilation`.

Given a build that runs `cc -o a.out src1.c src2.c src3.c`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains three entries, one per
> source file,
> each entry describes a pure compile step (no link-only flags),
> and no entry's `file` is `a.out`.

Coverage pending.

Given a build that runs `cc -o a.out obj1.o obj2.o obj3.o`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains no entries for this
> invocation.

Coverage pending.

Given a build that runs `cc -o a.out -lsomething -O2 src.c`:

> When the user runs Bear wrapping that build,
> then the resulting entry contains `-O2`,
> and it does not contain `-lsomething`.

Coverage pending.

Given a build that runs `cc -I first -I second -DFOO -DBAR -c src.c`:

> When the user runs Bear wrapping that build,
> then the entry lists `-I first` before `-I second`,
> and `-DFOO` before `-DBAR`, matching the original order.

Coverage pending.

Given a build that runs `cc --version` or `cc -###`:

> When the user runs Bear wrapping that build,
> then `compile_commands.json` contains no entries for this
> invocation.

Coverage pending.

Given a build that runs `cc -o a.out src1.c src2.c` with the
`output` field enabled via configuration:

> When the user runs Bear wrapping that build,
> then every entry's `output` is `a.out` (reflecting the known
> limitation documented above, not an ideal behaviour).

Coverage pending.

## Notes

- Per-source inference of object names (`src1.o`, `src2.o`) for
  multi-source invocations that share a single `-o` output is a
  plausible future improvement. It is not implemented today; the
  first output value is copied into every entry.
- Dedicated integration tests for the compile-and-link split, the
  pure-link case, link-flag stripping, argument-order preservation,
  info-only invocations, and the `output` field behaviour are
  pending.
- Related: `output-json-compilation-database` -- per-entry JSON
  shape.
- Related: `interception-compiler-env-with-flags` -- environment
  variables that contribute flags to entries.
- Related: `output-append`, `output-duplicate-detection`,
  `output-source-directory-filter` -- stages that run on the entries
  produced by this step.
