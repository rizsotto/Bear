---
title: Include compiler environment flags in compilation entries
status: implemented
---

## Intent

Compilers read part of their configuration from the environment, not
from argv. GCC and Clang take extra header search paths from `CPATH`
and its siblings. MSVC reads `CL` and `_CL_` as extra leading and
trailing options. A build that relies on these compiles fine, but a
tool that reads the database sees only argv and misses what the
compiler really used.

Bear turns these variables into the explicit flags they stand for and
adds them to each entry's `arguments`, so the entry stands on its own.
This is on by default. A configuration option turns it off for users
who want argv alone.

Flags carried inside a compiler variable such as `CC="gcc -std=c11"`,
seen by the wrapper mechanism, are a separate contract, owned by
`interception-cc-env-var-flags`.

## Acceptance criteria

- **Given** a build that runs `cc -c src.c -o src.o` with `CPATH` set to
  `/opt/first:/opt/second`, **when** Bear runs with the default
  configuration, **then** the entry's `arguments` hold an include flag
  for `/opt/first` followed by one for `/opt/second`, in the order
  `CPATH` lists them.
- **Given** the same build and environment, **when** Bear runs with
  environment-derived flags turned off, **then** the entry's `arguments`
  hold exactly the command-line arguments, after any response-file
  inlining (owned by `output-response-file-inlining`), and neither path
  appears.
- **Given** an MSVC or clang-cl compilation with `CL` and `_CL_` set,
  **when** Bear runs with the default configuration, **then** the
  compiler stays first in `arguments`, the words of `CL` go after the
  compiler and before the command-line flags, and the words of `_CL_`
  go after the command-line flags.
- The compiler YAML of each family owns which variables are recognized,
  the flag each one becomes, and whether its words go before or after
  the command-line flags. This file owns the toggle, its default, and
  the rule that a variable's values keep their order.

## Non-functional constraints

- The toggle changes only the database. The interception layer and the
  recorded event environment (`interception-events-format`) are not
  affected.
