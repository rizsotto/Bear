---
title: Inline response files in compilation entries
status: implemented
---

## Intent

Some build systems, notably Xcode, store part of a compiler's flags
in a separate "response file" and pass it to the compiler with the
`@file` syntax. Tools that consume `compile_commands.json` usually
do not understand this indirection: they see `@/path/to/args.resp`,
treat it as a literal argument, and miss every include path, define,
and warning flag the response file actually contained. Because these
files often live inside the build directory and are routinely
cleaned, downstream tools may also encounter entries whose `@file`
references no longer resolve.

Bear should be able to inline the contents of `@file` references
into the `arguments` of each compilation entry, so that the entry
stands on its own and downstream tools see the full set of flags the
compiler was given.

This behaviour is opt-in. With the default configuration Bear
continues to record argv exactly as the build wrote it.

## Acceptance criteria

### A configuration option toggles inlining

- A single configuration option enables or disables response-file
  inlining for the generated database.
- The default is disabled. With the default, an `@file` argument
  appears literally in the entry's `arguments`, unchanged from the
  intercepted invocation.
- When the option is enabled, every `@file` argument in every entry
  is replaced as described below.

### `@file` is replaced by the file's tokenized contents

When inlining is enabled and an entry's `arguments` contain an
`@file` token:

- The `@file` token is removed from `arguments`.
- The tokens read from the file are inserted at the same position,
  in the order they appear in the file.
- Tokens that followed `@file` in the original argument list keep
  their relative position after the inserted tokens.
- The compiler executable remains at index 0 of `arguments`.

### Tokenization follows the compiler

The text inside a response file is tokenized using the conventions
of the compiler that produced the entry:

- GCC, Clang, and other GCC/Clang-family compilers: whitespace
  separates tokens; single or double quotes group whitespace into a
  single token; a backslash quotes the next character.
- MSVC and clang-cl: Windows command-line rules; only double quotes
  group tokens; backslash escaping is positional and only meaningful
  next to quote characters.

### Nested `@file` references are expanded recursively

If a response file itself contains `@file` tokens, those are
expanded with the same rules. The build Bear observed already
accepted these nested references for compilation to succeed, so
Bear follows them regardless of whether the underlying compiler
documents recursion.

A depth limit guards against accidental cycles. When the limit is
reached, the offending `@file` token is left literally in the
entry's `arguments` and a warning is reported.

### Missing or unreadable response files are kept literal

If a referenced response file cannot be opened or read, the `@file`
token is left in the entry's `arguments` unchanged and a warning is
reported. Bear does not fail the whole database because a single
response file is missing: builds that succeeded had the file present
at the time, and an absent file at analysis time is treated as a
stale build artefact rather than a fatal error.

### Path resolution

`@file` paths are resolved relative to the working directory of the
intercepted compiler invocation, matching how the compiler itself
opens the file. Absolute paths are used as-is. Nested `@file`
references inside a response file are resolved the same way,
relative to the original invocation's working directory.

### Per-source semantics are preserved

- Inlined tokens participate in flag classification, link-only
  stripping, and the one-entry-per-source rule (see
  `output-compilation-entries`) exactly like any other argument.

## Non-functional constraints

- Inlining is observable in `compile_commands.json` only. The
  intercepted event record (see `interception-events-format`)
  continues to capture argv as the build wrote it, including any
  literal `@file` tokens. The interception layer is not affected.
- Reading the response file happens on the host where Bear runs.
  The path the build recorded must be reachable from that host.

## Known limitations

- Compiler-specific response-file flags that use a different syntax
  than `@file` are not inlined: nvcc's `--options-file` / `-optf`
  and IBM XL's `-qoptfile` arguments pass through literally.

## Testing

Given a build that runs
`cc -DBASE=1 @flags.resp -c src.c -o src.o`, where `flags.resp`
contains `-I/opt/include -DEXTRA=2`:

> When the user runs Bear with response-file inlining enabled,
> then the entry for `src.c` has `arguments` that contain
> `-DBASE=1`, `-I/opt/include`, `-DEXTRA=2`, `-c`, `src.c`, `-o`,
> `src.o` in that order,
> and no element of `arguments` starts with `@`.

Given the same build and response file:

> When the user runs Bear with response-file inlining disabled (the
> default),
> then the entry for `src.c` has `@flags.resp` literally in its
> `arguments`,
> and the contents of `flags.resp` are not present in the entry.

Given a build whose response file contains quoted tokens such as
`'-std=gnu++20' -fmodules`:

> When the user runs Bear with response-file inlining enabled,
> then the entry's `arguments` include `-std=gnu++20` and
> `-fmodules` as two separate tokens with the surrounding quotes
> removed.

Given a build with `cl.exe @args.rsp src.cpp` on Windows, where
`args.rsp` contains `/I "C:\Program Files\inc" /DFOO=1`:

> When the user runs Bear with response-file inlining enabled,
> then the entry's `arguments` include `/I`,
> `C:\Program Files\inc`, and `/DFOO=1` as separate tokens, applying
> MSVC quoting rules.

Given a build whose response file references another response file
(`@outer.resp` referencing `@inner.resp`):

> When the user runs Bear with response-file inlining enabled,
> then the entry's `arguments` contain the tokens from `inner.resp`
> spliced through `outer.resp`,
> and no element of `arguments` starts with `@`.

Given an intercepted invocation `cc @rel.resp -c src.c` whose working
directory is `/proj`, where `/proj/rel.resp` contains `-DFROM_RESP=1`:

> When the user runs Bear with response-file inlining enabled,
> then `rel.resp` is read from `/proj` -- resolved relative to the
> invocation's working directory, even when Bear itself runs from a
> different directory --
> and the entry's `arguments` contain `-DFROM_RESP=1`;
> an absolute reference such as `@/opt/flags.resp` is read from that
> absolute path as-is.

Given a build that runs `cc -c src.c @flags.resp`, where `flags.resp`
contains `extra.c`:

> When the user runs Bear with response-file inlining enabled,
> then the database contains an entry for `extra.c` in addition to the
> entry for `src.c`: inlined tokens participate in per-source entry
> shaping like any other argument.

Given a build whose response file has been removed between the
build and Bear's analysis step:

> When the user runs Bear with response-file inlining enabled,
> then the entry's `arguments` keep `@flags.resp` literally,
> a warning is emitted that names the missing file,
> and other entries in the database are produced as usual.

Given a build whose response file references itself, directly or
through a cycle:

> When the user runs Bear with response-file inlining enabled,
> then expansion stops at the configured depth limit,
> the offending `@file` token is left literal in the entry,
> and a warning is emitted.

## Notes

- Source: feature request #701 -- Xcode's response files cause
  important flags to be lost when downstream tools read
  `compile_commands.json`.
- Related: `output-compilation-entries` -- the entry-shaping rules
  the inlined tokens participate in.
- Related: `output-json-compilation-database` defines the JSON shape
  of an entry. This requirement does not change it.
- Related: `interception-events-format` records argv as observed by
  interception. This requirement does not affect it.
