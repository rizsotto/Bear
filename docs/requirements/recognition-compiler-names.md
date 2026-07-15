---
title: Recognize compiler names across toolchains
status: implemented
---

## Intent

Builds using the classic GNU and LLVM compiler drivers, AMD ROCm,
standalone assemblers, the Cray Compiling Environment, Emscripten, QNX,
Texas Instruments, Microchip, MPI wrapper commands, or the Swift
compiler record their compilations in the compilation database without
any configuration. The recorded compiler is the name the build invoked,
exactly as executed.

## Acceptance criteria

The shared contract for every recognized name below:

- An execution of a recognized name (see the Recognized names table)
  with a source file yields database entries, parsed with the flag
  semantics of the family named in that row.
- The recorded arguments are the invocation as executed. A wrapper or
  driver is never expanded to the underlying compiler command; the name
  the build invoked is what the entry records.
- An info-only invocation yields no entry. The general
  information-only contract (`--version`, `--help`, and the like) is
  owned by `output-compilation-entries`; the MPI wrapper-info options
  (`mpicc -showme`, `-show`, `-compile_info`, `-link_info`) are this
  requirement's own addition to that contract.
- The classic GNU and LLVM driver names (the first two table rows) are
  also recognized when spelled with a cross-compilation target prefix
  (`arm-linux-gnueabihf-gcc`, `aarch64-linux-gnu-clang`), a version
  suffix (`gcc-12`, `gcc12`, `gcc-11.2`, `clang-15`), or both at once
  (`arm-linux-gnueabi-gcc-12`); every such spelling carries the base
  name's flag semantics. Other rows note where these shapes also apply.
- A value-carrying wrapper or driver option never swallows a following
  source file, and the option token is retained verbatim in the
  recorded arguments. MPICH's compiler-override options (for example
  `-cc=gcc`) and QNX's variant selector (for example
  `-Vgcc_ntoaarch64le`) are options, never inputs.
- The names in the Deliberately not recognized table yield no entry.
- Ambiguous names (`cc`, `c++`, `CC`) are in neither table here; their
  classification is owned by `recognition-ambiguous-name-probe`.
- When preload interception records both a driver or wrapper and the
  child compiler it execs for one source, the pair collapses to a
  single entry and the driver invocation survives; this behavior is
  owned by `output-duplicate-detection`.

### Recognized names

| Names | Flag semantics | Notes |
|---|---|---|
| `gcc`, `g++`, `gfortran`, `f95`, `egfortran` | GCC | classic GNU driver names; the cross-prefix and version-suffix shapes apply |
| `clang`, `clang++` | Clang | classic LLVM driver names; the cross-prefix and version-suffix shapes apply |
| `amdclang`, `amdclang++`, `hipcc` | Clang | AMD ROCm; `hipcc` is a driver, recorded like `nvcc` |
| `amdflang` | Flang | AMD ROCm |
| `nasm`, `yasm`, `fasm` | own assembler, recorded as executed | standalone assemblers |
| CCE C/C++ compiler names, including versioned variants | Clang | Cray Compiling Environment; CCE C/C++ is Clang-based |
| `crayftn`, `ftn` | Cray Fortran | Cray Compiling Environment Fortran; the PrgEnv `ftn` wrapper is statically recognized, not probed |
| `emcc`, `em++` | Clang | Emscripten |
| `qcc`, `q++` | GCC | QNX; `-V<variant>` is an attached-value driver option |
| `tiarmclang` | Clang | Texas Instruments Arm |
| `xc8-cc`, `xc8` | GCC | Microchip XC8 |
| `mpicc`, `mpicxx`, `mpic++`, `mpiCC`, `mpifort`, `mpif77`, `mpif90` | GCC | MPI wrappers (Open MPI, MPICH), recorded verbatim |
| `mpiicc`, `mpiicpc`, `mpiicx`, `mpiicpx`, `mpiifort`, `mpiifx` | Intel | Intel MPI wrappers |
| `swiftc` | Swift | one entry per source, whole-module shape (see `output-compilation-entries`) |

### Deliberately not recognized

| Names | Why |
|---|---|
| `as` (GNU assembler) | GCC and Clang spawn it on a temporary `.s` file for every ordinary compile; recognizing it would add one throwaway entry per compilation, keyed on a name that differs every run (`/tmp/cc<random>.s`), which duplicate detection cannot collapse. Likely to be proposed again -- this reasoning is why it stays out. |
| `mpirun`, `mpiexec` | Launchers: they execute programs, they do not compile. No acceptance path. |
| `swift` | Subcommand dispatcher (`swift build`, `swift run`, ...), not a compiler invocation; the `zig cc` class of model mismatch. Revisit only with subcommand-aware recognition. |
| `amdgpu-arch` | Shares the `amd` prefix but is not a compiler driver; it reports target GPU architectures. |
| `armcl` (legacy TI) | Proprietary dialect; Code Composer Studio generates its own compilation database for it. |
| `ml`, `ml64` (MASM) | Windows-only, no recorded demand. |
| `swift-frontend`, `swiftc -frontend` | The Swift driver's internal per-file frontend jobs, not user-facing invocations. |

## Testing

Given an event file with a `<name> -c hello.c` execution for a classic
GNU or LLVM driver name in any recognized spelling (`gcc`, `clang++`,
`arm-linux-gnueabihf-gcc`, `gcc-12`, `clang-15`):

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`
> whose arguments start with the name as invoked,
> parsed with the base name's flag semantics.

Given an event file with a `<name> -c hello.c` execution for any of
the C-family driver names in the table (the AMD ROCm C/C++ drivers,
the CCE C/C++ names, `emcc`/`em++`, `qcc`/`q++`, `tiarmclang`, the
Microchip XC8 drivers, and the C/C++ MPI wrappers):

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`
> whose arguments start with `<name>`.

The assembler, Fortran, and Swift names in the table follow the same
pattern with a source file and flags from their own toolchain rather
than a C source and `-c`; the concrete `fasm`, `gfortran`, and
`swiftc` scenarios below exercise those shapes.

Given an event file with a `gfortran -c hello.f90` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.f90`,
> whose arguments start with `gfortran`,
> parsed with GCC flag semantics.
> This scenario is representative of the Fortran rows in the table;
> each row's names follow the same shape with that row's own flag
> semantics.

Given an event file with a `qcc -Vgcc_ntoaarch64le -c hello.c`
execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`,
> and the `-Vgcc_ntoaarch64le` token is retained in the recorded
> arguments.

Given an event file with an `mpicc -showme` (or `-show`,
`-compile_info`) execution:

> When `bear semantic` runs,
> then the database contains no entry for that execution.

Given an event file with an `mpicc -cc=gcc -c hello.c` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`,
> and the `-cc=gcc` token is retained in the arguments and does not
> swallow the source file.

Given an event file with a
`swiftc -module-name App -emit-object a.swift b.swift` execution:

> When `bear semantic` runs,
> then the database contains two entries, one for `a.swift` and one
> for `b.swift`,
> and both entries' arguments contain `a.swift`, `b.swift`, and
> `-module-name App` (see `output-compilation-entries` for the
> whole-module shape).

Given an event file with a `swift-frontend` execution (or a
`swiftc -frontend ...` execution on a legacy toolchain that re-invokes
itself):

> When `bear semantic` runs,
> then the database contains no entry for that execution.

Given an event file with an `as -o foo.o foo.s` execution:

> When `bear semantic` runs,
> then the database contains no entry for that execution.

The `as` and `swift-frontend` scenarios are representative of the
whole Deliberately not recognized table: every name in it yields no
entry.

Given an event file with a `gcc -c foo.s` execution:

> When `bear semantic` runs,
> then the database contains one entry for `foo.s`, recorded as a GCC
> compilation (the regression guard for issue #146, where
> compile-then-assemble produced an empty database).

Given an event file with a `fasm hello.asm output.bin` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.asm`,
> and the optional output positional is not recorded as an `output`
> field -- it is not a recognized source extension, so it is a plain
> argument.

## Notes

- Demand: AMD ROCm (nvcc-style driver, condensed below); assemblers
  #146; Emscripten #580, #560; QNX #544, #579; TI #540; Microchip XC8
  #286, #403.
- `hipcc` is a compiler driver (it calls clang or nvcc and passes
  include/library options), but being a driver does not disqualify it:
  gcc, clang, and the already-recognized `nvcc` are drivers too. Bear
  records the user-facing driver invocation, as it does for `nvcc`;
  `hipcc` accepts clang-style options, so it dispatches with Clang
  semantics.
- AOCC (AMD's Clang-based CPU compiler) installs plain
  `clang`/`clang++`/`flang` names, already recognized; no
  AMD-prefixed name is added for it.
- QNX 8 ships a GCC 12.2-based toolchain, so `qcc`/`q++` parse with GCC
  semantics, not Clang.
- `xc16-gcc`/`xc32-gcc` already match the GCC cross-compilation prefix
  rule in the Acceptance criteria; they are covered only by a
  regression test, not by this requirement's name list.
- The wrapper or driver invocation is recorded verbatim. Clang tooling
  users who need a wrapper's baked-in include paths can point their
  tool at the wrapper (for example clangd's `--query-driver`).
- `-index-store-path` is a documented `swiftc` flag but Bear never
  injects it; SourceKit-LSP's cross-file indexing benefits when the
  build already passes it, but that is the build's choice.
- Xcode's `swiftc` is Apple-signed, so on macOS SIP blocks preload
  interception for it and wrapper mode applies (see
  `interception-preload-mechanism` for the general SIP contract).
- Consumers: asm-lsp reads `compile_commands.json` for assembly
  (https://github.com/bergercookie/asm-lsp); clangd's refusal to handle
  `.s` files (https://github.com/clangd/vscode-clangd/issues/310) is a
  second demand signal alongside issue #146. SourceKit-LSP reads it for
  Swift (https://github.com/swiftlang/sourcekit-lsp).

## Rationale

- [MPI wrappers as compilers](../rationale/mpi-wrappers-as-compilers.md) -
  why the wrapper is recorded verbatim instead of expanded.
- [Swift whole-module entries](../rationale/swift-whole-module-entries.md) -
  why Swift needs one entry per source with the full invocation's
  arguments.
