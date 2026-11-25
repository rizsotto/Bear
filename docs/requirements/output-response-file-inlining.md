---
title: Inline response files in compilation entries
status: accepted
---

## Intent

Some build systems, Xcode among them, put part of a compiler's flags in
a response file and pass it as `@file`. Tools that read the database
usually do not follow this. They see `@/path/to/args.resp` as a literal
argument and miss every include path, define, and warning flag inside.
The file often lives in the build directory and may be gone by the time
a tool reads the entry.

By default Bear replaces each `@file` with the file's contents, so the
entry stands on its own, and the file's flags count when Bear decides
what the invocation does. When the user turns inlining off, Bear keeps
the `@file` token as the build wrote it and decides from the rest of
the command line, as best it can.

## Acceptance criteria

- **Given** a build that runs `cc -DBASE=1 @flags.resp -c src.c -o src.o`,
  where `flags.resp` holds `-I/opt/include -DEXTRA=2`, **when** Bear runs
  with the default configuration, **then** the entry's `arguments` hold
  `-DBASE=1`, `-I/opt/include`, `-DEXTRA=2`, `-c`, `src.c`, `-o`,
  `src.o` in that order, and no element starts with `@`. The file's
  tokens take the place of the `@file` token, in file order, and the
  compiler stays at index 0.
- **Given** the same build, **when** Bear runs with inlining off,
  **then** the entry's `arguments` hold `@flags.resp` literally and none
  of the file's contents.
- **Given** an MSVC compilation `cl.exe @args.rsp /c src.cpp`, **when**
  Bear runs with inlining off, **then** the entry's `arguments` hold
  `@args.rsp`. With inlining off the `@file` token stays in the entry in
  every compiler family.
- **Given** a GCC/Clang-family compilation whose response file holds
  `'-std=gnu++20' -fmodules`, **when** Bear inlines it, **then** the
  entry's `arguments` hold `-std=gnu++20` and `-fmodules` as two tokens,
  quotes removed. For this family, whitespace separates tokens, single
  or double quotes group them, and a backslash quotes the next
  character.
- **Given** an MSVC or clang-cl compilation `cl.exe @args.rsp src.cpp`,
  where `args.rsp` holds `/I "C:\Program Files\inc" /DFOO=1`, **when**
  Bear inlines it, **then** the entry's `arguments` hold `/I`,
  `C:\Program Files\inc`, and `/DFOO=1` as separate tokens. For this
  family, Windows command-line rules apply: only double quotes group,
  and a backslash matters only next to a quote.
- **Given** `@outer.resp` that references `@inner.resp`, **when** Bear
  inlines it, **then** the tokens of `inner.resp` appear in place, and
  no element of `arguments` starts with `@`. Nested references are
  followed whether or not the compiler documents them.
- **Given** a response file that references itself, directly or through
  a cycle, **when** Bear inlines it, **then** expansion stops at a
  fixed, built-in depth limit, the `@file` token at that depth stays
  literal, and a warning is logged.
- **Given** an invocation `cc @rel.resp -c src.c` run in `/proj`, where
  `/proj/rel.resp` holds `-DFROM_RESP=1`, **when** Bear inlines it from
  another directory, **then** `rel.resp` is read from `/proj` and the
  entry holds `-DFROM_RESP=1`. Nested references resolve against the
  same invocation directory. An absolute reference such as
  `@/opt/flags.resp` is read from that path as is.
- **Given** a build that runs `cc -c src.c @flags.resp`, where
  `flags.resp` holds `extra.c`, **when** Bear inlines it, **then** the
  database holds an entry for `extra.c` as well as one for `src.c`.
  Inlined tokens take part in flag classification, link-flag removal,
  and per-source entries (owned by `output-compilation-entries`) like
  any other argument.
- **Given** a response file that was removed after the build, **when**
  Bear tries to inline it, **then** the entry keeps `@flags.resp`
  literally, a warning names the missing file, and the other entries
  are written as usual. An unreadable file is handled the same way.
- For input declared to come from another machine, response files are
  not read. When that is the default and when it is an error is owned
  by `cli-input-from-another-machine`.

## Non-functional constraints

- Inlining changes only the database. The recorded event
  (`interception-events-format`) keeps argv as the build wrote it,
  `@file` tokens included.
- Bear reads response files on the host where it runs. The path the
  build recorded must be reachable from there.

## Known limitations

When a response file is not read (inlining is off, the file is missing
or unreadable, it is past the depth limit, or the input comes from
another machine), Bear decides from the rest of the command line. Then:

- A source named only inside the file gets no entry.
- A flag inside the file that stops the compiler early (`-E`, `-M`) or
  only requests information (`--version`) is not seen, so the
  invocation can still get an entry.
- An output flag inside the file is not seen, so the entry has no
  `output` field.
- Link-only flags inside the file are not removed. The whole file stays
  in the entry as the `@file` token.
- A flag inside the file that hands the rest of the command line to the
  linker (MSVC `/link`) is not seen, so the arguments after the `@file`
  are kept as compile flags.
- In a compiler family that does not take response files, an argument
  that starts with `@` is still treated as one.

When a response file is read:

- Bear reads the file as it is when Bear runs. A file changed after the
  build gives the new contents.

- Response-file flags with a syntax other than `@file`, such as nvcc's
  `--options-file` and `-optf` or IBM XL's `-qoptfile`, are not inlined.
  They pass through literally.

## Rationale

- [Response-file inlining is best effort](../rationale/response-file-inlining-best-effort.md)
