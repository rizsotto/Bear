# Compiler passes are worked out per input

## Context

A compiler driver decides what to do for each input file. A `.c` file
goes through preprocessing, compiling, and assembling. A `.s` file only
goes through assembling. A `.o` file only goes to the linker. A stop
flag cuts the run short for all inputs.

The first pass comes from the file's type. The last pass comes from the
earliest stop flag, and the flag names the last pass that runs: `-E`
preprocessing, `-S` compiling, `-c` assembling. The flag tables had `-c`
and `-S` swapped; nothing showed while both were treated the same.

An input that starts after the last pass is unused. In `cc -S foo.s`
gcc does nothing for `foo.s` (checked with gcc 16, no file is written).

Assembly entries are used. [asm-lsp](https://github.com/bergercookie/asm-lsp)
reads the database and runs the entry's command to get diagnostics. The
Linux kernel adds `.S` files to its database, see
[this commit](https://lab.nexedi.com/kirr/linux/-/commit/1c67921444bf68107f7901d5bcfce954efaa8754).
So `gcc -c foo.s` keeps its entry. But `gcc -S foo.s` does nothing, so an
entry for it would give no diagnostics.

Options considered:

- One pass set per call. It cannot give per-input entries or tell which
  inputs go to the link. It also gets `cc -S foo.s` wrong.
- Map the stop labels in code and leave the data. `-E` was right while
  `-c` and `-S` were swapped, so no single mapping fixes all three.
- Keep the entry for `cc -S foo.s`. It matches no real build, and a tool
  that runs it learns nothing.

## Decision

Passes are worked out per input, as a driver does. File types come from
one extension table shared by every compiler family, as the requirement
says. An unused input gets no entry. The flag tables now say which pass
is last.

## Consequences

Per-family file typing and `-x` are left for later. They would change the
shared-table rule in the requirement.

## References

- [output-compilation-entries](../requirements/output-compilation-entries.md), the requirement this supports
- [asm-lsp](https://github.com/bergercookie/asm-lsp), a tool that reads assembly entries
- [Linux kernel commit](https://lab.nexedi.com/kirr/linux/-/commit/1c67921444bf68107f7901d5bcfce954efaa8754), adds `.S` files to the database
