---
title: Include compiler environment flags in compilation entries
status: implemented
---

## Intent

Compilers read part of their effective configuration from the
environment rather than from argv. The GCC/Clang family treats
`CPATH`, `C_INCLUDE_PATH`, `CPLUS_INCLUDE_PATH`, and
`OBJC_INCLUDE_PATH` as additional header search paths; MSVC reads
`CL` and `_CL_` as implicit leading and trailing options. A build
that relies on these variables compiles correctly, but a tool reading
`compile_commands.json` sees only the literal argv and misses the
search paths the compiler actually used.

To keep entries self-contained, Bear translates these environment
variables into the equivalent explicit flags and folds them into each
entry's `arguments`. This is on by default, matching what the compiler
did. Some users prefer entries that reflect argv alone; a single
configuration option turns the translation off.

This behaviour is observable in `compile_commands.json` only. The
intercepted event record already captures the full environment (see
`interception-events-format`); this requirement does not change it.

## Acceptance criteria

### A configuration option toggles environment-derived flags

- A single configuration option enables or disables folding compiler
  environment variables into the generated entries.
- The default is enabled.
- When disabled, recognized environment variables contribute nothing:
  each entry's `arguments` contain only the flags that came from argv
  (after any response-file inlining; see
  `output-response-file-inlining`).

### Enabled translation follows the compiler family

- When enabled, the environment variables Bear recognizes for the
  entry's compiler family are translated into the equivalent flags and
  added to the entry. The header-search variables above become include
  flags (an include flag per path, in the order the variable lists
  them); the MSVC implicit-option variables are spliced before and
  after the command-line options respectively.
- The set of variables recognized for each family, and the flag each
  maps to, is defined per compiler family. This requirement governs
  only the on/off switch, not the mapping.

## Non-functional constraints

- The toggle changes only the contents of `compile_commands.json`.
  The interception layer and the recorded event environment are not
  affected.

## Testing

Given a build that runs `cc -c src.c -o src.o` with `CPATH` set to a
directory in the environment:

> When the user runs Bear with environment flags enabled (the
> default),
> then the entry for `src.c` has `arguments` that include the
> directory from `CPATH` as an explicit include flag.

Given the same build and environment:

> When the user runs Bear with environment flags disabled,
> then the entry for `src.c` has `arguments` that do not contain the
> directory from `CPATH`,
> and the arguments are exactly those that appeared on the command
> line.

## Notes

- Related: `output-compilation-entries` -- the per-source
  transformation these flags participate in.
- Related: `output-response-file-inlining` -- the sibling option.
- Related: `interception-cc-env-var-flags` -- wrapper-mode splitting
  of a `CC="gcc -std=c11"` value, a separate mechanism.
