---
title: Recognize embedded and specialty toolchain compiler names
status: implemented
---

## Intent

Builds using Emscripten's C/C++ drivers, QNX's C/C++ drivers, Texas
Instruments' Arm compiler, or Microchip's XC8 compilers record their
compilations in the compilation database without any configuration. The
recorded compiler is the driver name the build invoked, exactly as
executed.

## Acceptance criteria

- Executions of the Emscripten drivers `emcc` and `em++` with a source
  file yield database entries, parsed with Clang flag semantics.
- Executions of the QNX drivers `qcc` and `q++` with a source file yield
  database entries, parsed with GCC flag semantics (QNX's toolchain is
  GCC-backed).
- QNX's attached-value variant selector (for example
  `-Vgcc_ntoaarch64le`) is always treated as a driver option; it is
  never mistaken for an input file, and it is retained verbatim in the
  recorded arguments.
- Executions of Texas Instruments' `tiarmclang` with a source file yield
  a database entry, parsed with Clang flag semantics.
- Executions of Microchip's XC8 drivers `xc8-cc` and `xc8` with a source
  file yield database entries, parsed with GCC flag semantics.
- In preload interception mode, Emscripten's `emcc`/`em++` may also
  intercept the underlying `clang` child process they spawn; the default
  duplicate filter collapses the pair to a single entry, with the
  user-facing driver's invocation surviving.

## Testing

Given an event file with an `emcc -c hello.c` (or `em++`) execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`.

Given an event file with a `qcc -c hello.c` (or `q++`) execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`.

Given an event file with a `qcc -Vgcc_ntoaarch64le -c hello.c` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`,
> and the `-Vgcc_ntoaarch64le` token is retained in the recorded
> arguments.

Given an event file with a `tiarmclang -c hello.c` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`.

Given an event file with an `xc8-cc -c hello.c` (or `xc8`) execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`.

Given an event file with an `emcc -c hello.c` event followed by the
underlying `clang -c hello.c` event it spawns in preload mode (both
processes intercepted):

> When `bear semantic` runs with default duplicate detection,
> then exactly one entry survives for `hello.c`,
> and it records the `emcc` invocation (the driver event comes first in
> the event stream, so it wins under first-seen duplicate detection).

## Notes

- Demand: Bear issues #580, #560 (Emscripten); #544, #579 (QNX); #540
  (TI); #286, #403 (Microchip XC8).
- QNX 8 ships a GCC 12.2-based toolchain, so `qcc`/`q++` parse with GCC
  flag semantics, not Clang.
- The legacy TI compiler `armcl` is out of scope: it is a proprietary
  dialect, and Code Composer Studio generates its own compilation
  database for it.
- `xc16-gcc`/`xc32-gcc` already match the existing cross-compilation
  prefix rule for GCC names; they are not part of this requirement's
  scope beyond a regression test locking that behavior in.
