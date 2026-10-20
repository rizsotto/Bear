---
title: Recognize compiler names across toolchains
status: implemented
---

## Intent

Builds that invoke a recognized compiler record their compilations in
the compilation database without any configuration. Recognized
compilers include the GNU and LLVM drivers and the vendor toolchains,
standalone assemblers, MPI wrappers, and Swift and Fortran compilers
that Bear's compiler definitions describe. The recorded compiler is the
name the build invoked, exactly as executed.

## Acceptance criteria

- The tool lists the recognized compilers in its own output; the man
  page names the command. Each row gives a human-readable description,
  and each compiler row gives the family name a configuration uses to
  select it. The criteria below apply to whatever that list contains.
- **Given** an event file with a `<name> -c hello.c` execution, where
  `<name>` is a classic GNU or LLVM driver in any recognized spelling
  (`gcc`, `clang++`, `arm-linux-gnueabihf-gcc`, `gcc-12`, `clang-15`),
  **when** Bear runs in semantic mode, **then** the database holds one
  entry for `hello.c`, its arguments start with the name as invoked,
  and it is parsed with the base name's flag semantics.
- The classic GNU and LLVM driver names are also recognized with a
  cross-compilation target prefix (`aarch64-linux-gnu-clang`), a version
  suffix (`gcc-12`, `gcc12`, `gcc-11.2`), or both
  (`arm-linux-gnueabi-gcc-12`). Some other families accept one of these
  shapes; which ones is a property of each compiler, not a blanket rule.
- **Given** an event file with a `<name> -c hello.c` execution for any
  other recognized C-family name (for example a vendor driver, `emcc`,
  `qcc`, or an MPI wrapper), **when** Bear runs in semantic mode,
  **then** the database holds one entry for `hello.c` whose arguments
  start with `<name>`, parsed with that family's flag semantics.
- **Given** an event file with a `gfortran -c hello.f90` execution,
  **when** Bear runs in semantic mode, **then** the database holds one
  entry for `hello.f90` whose arguments start with `gfortran`, parsed
  with GCC flag semantics. The assembler, Fortran, and Swift families
  follow the same shape with their own sources and flags.
- Each entry records the compiler and arguments as invoked. A wrapper or
  driver is never expanded to the underlying compiler command. Compiler
  launchers are the exception; they are owned by
  `recognition-compiler-launchers`.
- **Given** an event file with an info-only execution, **when** Bear
  runs in semantic mode, **then** it yields no entry. The general
  info-only contract (`--version`, `--help`, and the like) is owned by
  `output-compilation-entries`. The MPI wrappers' own wrapper-info
  options (for example `mpicc -showme`) are this requirement's addition
  to it.
- A value-carrying wrapper or driver option never swallows a following
  source file, and its token is kept verbatim in the arguments.
- **Given** an event file with an `mpicc -cc=gcc -c hello.c` execution,
  **when** Bear runs in semantic mode, **then** the database holds one
  entry for `hello.c`, and `-cc=gcc` is kept in the arguments and does
  not swallow the source file.
- **Given** an event file with a `qcc -Vgcc_ntoaarch64le -c hello.c`
  execution, **when** Bear runs in semantic mode, **then** the database
  holds one entry for `hello.c`, and `-Vgcc_ntoaarch64le` is kept in the
  arguments.
- **Given** a compiler configured at `/opt/toolchain/f95` with the Flang
  family, and an event file with an `f95 -c hello.f90` execution (or the
  same with the full path, or with a path that resolves to the same
  file), **when** Bear runs in semantic mode,
  **then** the database holds one entry for `hello.f90`, parsed with
  Flang flag semantics, not GCC's. A configured family wins over the
  family the name would otherwise get.
- **Given** an event file with a
  `swiftc -module-name App -emit-object a.swift b.swift` execution,
  **when** Bear runs in semantic mode, **then** the database holds two
  entries, one for `a.swift` and one for `b.swift`, and both entries'
  arguments contain `a.swift`, `b.swift`, and `-module-name App`. The
  whole-module shape is owned by `output-compilation-entries`.
- **Given** an event file with a `gcc -c foo.s` execution, **when** Bear
  runs in semantic mode, **then** the database holds one entry for
  `foo.s`, recorded as a GCC compilation (regression guard for issue
  #146).
- **Given** an event file with a `fasm hello.asm output.bin` execution,
  **when** Bear runs in semantic mode, **then** the database holds one
  entry for `hello.asm`, and `output.bin` is a plain argument, not the
  entry's `output` field.
- **Given** an event file with an execution of a name below, **when**
  Bear runs in semantic mode, **then** it yields no entry:
  - `as` (the GNU assembler)
  - `mpirun`, `mpiexec`
  - `swift`, `zig`
  - `swift-frontend`, and `swiftc -frontend`
  - `amdgpu-arch`
  - `ml`, `ml64` (MASM)
- The ambiguous names `cc`, `c++`, and `CC` are owned by
  `recognition-ambiguous-name-probe`.
- When the preload mechanism reports both a driver or wrapper and the
  child compiler it runs for one source, the pair collapses to one entry
  that keeps the driver invocation. That is owned by
  `output-duplicate-detection`.

## Known limitations

**Xcode's `swiftc` on macOS**: it is Apple-signed, so System Integrity
Protection keeps the preload mechanism out of it. The wrapper mechanism
records it. What Bear does when the preload mechanism is selected there
is owned by `interception-mechanism-selection`.

## Rationale

- [MPI wrappers as compilers](../rationale/mpi-wrappers-as-compilers.md)
- [Swift whole-module entries](../rationale/swift-whole-module-entries.md)
- [Compiler family definition](../rationale/compiler-family-definition.md)
- [Names deliberately not recognized](../rationale/names-deliberately-not-recognized.md)
