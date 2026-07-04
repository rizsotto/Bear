---
title: Recognize standalone assembler invocations
status: in-progress
---

## Intent

When a build assembles source directly through NASM, YASM, or FASM,
Bear records those invocations in the compilation database, with
per-file arguments, so assembly language servers (for example asm-lsp)
can read them from `compile_commands.json`. Assembly compiled through a
C/C++ compiler driver (for example `gcc -c foo.s`) is already recorded
by that driver's own entry; this requirement only covers the
standalone assemblers.

## Acceptance criteria

- An execution of `nasm` or `yasm` with an assembly source file yields
  a database entry whose recorded arguments are the invocation as
  executed.
- An execution of `fasm` with an assembly source file yields a database
  entry whose recorded arguments are the invocation as executed.
- The GNU assembler `as` is NOT recognized as a compiler; executions of
  `as` never yield a database entry through this requirement.
- A driver invocation that compiles an assembly source (for example
  `gcc -c foo.s`) continues to yield the driver's own entry, unaffected
  by this requirement.

## Non-functional constraints

- MASM (`ml`, `ml64`) is out of scope: it is Windows-only and no demand
  for it has been recorded.

## Testing

Given an event file with a `nasm -f elf64 -o hello.o hello.asm`
execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.asm`
> whose arguments are the invocation as executed.

Given an event file with a `fasm hello.asm` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.asm`.

Given an event file with an `as -o foo.o foo.s` execution:

> When `bear semantic` runs,
> then the database contains no entry for that execution.

Given an event file with a `gcc -c foo.s` execution:

> When `bear semantic` runs,
> then the database contains one entry for `foo.s`,
> recorded as a GCC compilation.

## Notes

- `as` is deliberately excluded, and this exclusion will likely be
  proposed again -- write down why so it survives that. GCC and Clang
  spawn `as` internally on a temporary `.s` file for every ordinary C
  compile (the compiler's own preprocess-then-assemble pipeline). If
  `as` were recognized, every normal C compilation would additionally
  produce an `as` entry for a temporary file whose name differs on
  every invocation (`/tmp/cc<random>.s` and similar), and duplicate
  detection cannot collapse those because there is nothing stable to
  match on. The result would be a database polluted with one throwaway
  entry per compilation. Direct assembly through a driver
  (`gcc -c foo.s`) is already recorded today via the driver's own
  entry, which is the actual fix for the empty-database class of bug
  reported in issue #146.
- fasm's CLI accepts an optional second positional argument, the output
  file (`fasm source [output]`). That output positional is not recorded
  as an `output` field in this requirement; it is simply not a
  recognized source extension, so it is classified as a plain argument.
- Consumer: asm-lsp reads `compile_commands.json` /
  `compile_flags.txt` for assembly language support
  (https://github.com/bergercookie/asm-lsp). Standing demand: Bear
  issue #146 (compile-then-assemble produced an empty database) and
  clangd's own refusal to handle `.s` files
  (https://github.com/clangd/vscode-clangd/issues/310).
