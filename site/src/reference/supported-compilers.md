<!-- Diataxis type: reference -->

# Supported compilers

Bear decides whether an intercepted command is a compilation by
matching the name of the executable the build ran against a table of
known compiler names. A name it does not know is not a compiler, and
Bear does not run the program to guess otherwise. The one exception is a
short list of names that are genuinely ambiguous across platforms, and
[Ambiguous names](#ambiguous-names) below says which and why. Once a
name matches, the command line is
parsed with that family's own flag rules, since GCC, MSVC, and Swift
(to pick three) do not agree on how to spell an include path or an
output file. This page explains how that matching works and lists the
names it currently covers; the configuration keys that let you extend
or correct it are in the [`bear(1)` man page][manpage] and explained in
[Configure Bear](configuration.md).

## Cross-compiler prefixes and version suffixes

A handful of core families are recognized under more than their plain
name. GCC and Clang (and, following the same pattern, CUDA's `nvcc`,
Flang, and Vala's `valac`) are also recognized with a
cross-compilation target prefix (`arm-linux-gnueabihf-gcc`,
`aarch64-linux-gnu-clang`), a version suffix (`gcc-12`, `clang-15`), or
both together (`arm-linux-gnueabi-gcc-12`). Every such spelling is
parsed with its base name's flag rules. Support for this varies by
family: it is not a blanket rule applied to every entry in the table
below.

## Compiler launchers

ccache, sccache, distcc, and icecc wrap a real compiler invocation
rather than compiling anything themselves. Bear recognizes each
launcher by name, finds the real compiler among its arguments, and
records the compilation as if the launcher were not there: `ccache gcc
-c main.c` is recorded as a `gcc` invocation, not a `ccache` one. A
launcher invocation whose argument is not a recognized compiler
(`ccache make all`), or that wraps another launcher (`ccache distcc gcc
-c main.c`), produces no entry; Bear does not chase a chain of
launchers. distcc's own options (`-j`, `--jobs`, `-v`, and similar) are
skipped while looking for the compiler, so they never get mistaken for
one.

## MPI compiler wrappers

`mpicc`, `mpicxx`, `mpifort`, and the vendor-specific MPI wrappers
(Intel's `mpiicc`, `mpiifort`, and similar) are recorded as invoked,
not expanded to the compiler they wrap. Clang tooling that needs a
wrapper's baked-in include paths can point at the wrapper directly, for
example with clangd's `--query-driver`. An information-only invocation
(`mpicc -showme`, `-show`, `-compile_info`, `-link_info`) produces no
entry, and a wrapper option that carries a value (MPICH's `-cc=gcc`)
never swallows a following source file.

## Ambiguous names

`cc`, `c++`, and the HPE Cray PrgEnv wrapper `CC` are not in the name
table at all, because the same basename is a different compiler
depending on the platform or the loaded environment module (GCC on
most Linux distributions, Clang on the BSDs and macOS, whatever the
loaded programming environment selects on a Cray system). Bear
classifies these by running the executable once with `--version` and
caching the result; a `compilers:` entry in the configuration (see
below) skips the probe and forces a classification when its output
does not match a known signature.

## `as` and `ignore` hints

A `compilers:` entry in the configuration names a path and either an
`as` value (the family to parse it with) or `ignore: true` (drop its
invocations entirely). Reach for it when a compiler sits at a path or
under a name the table below does not cover, or when a generic name is
misclassified. The generic names `cc` and `c++` are the usual reason to
add an entry, since not every custom build of GCC or Clang answers
`--version` in a way the probe recognizes.

An `as` value is a family identifier, not a display name: it is the
short, lower-case id given as each family's "Configuration `as:` value"
below, spelled verbatim. There are no aliases, so `as: gcc` is accepted while
`as: GCC` is not, and several families share one id (`icx` and `icc`
are both `intel_cc`). The accepted ids are the ones in the family tables
below, which are generated from the compiler definitions, plus `wrapper`
for a compiler launcher. This page does not repeat them here as a list:
the tables are the copy that cannot drift. An unaccepted value is
rejected when the configuration loads, with an error listing every value
that would have worked, and `bear semantic --print-compilers` prints the
same mapping for the version you have installed.

## Recognized families

Internal jobs that a driver spawns for itself are recognized only so
Bear can filter them back out: they are never user-facing invocations
and never produce a database entry. The tables below label each one as
internal, separately from the family's real, user-facing names.

The compiler launchers share one kind rather than a compiler family, so
`wrapper`, or any launcher's own name from the launcher table below,
selects it.

<!-- BEGIN GENERATED: scripts/generate-supported-compilers.py -- DO NOT EDIT. Edits inside this block are lost the next time the script runs. -->

### Compiler families

#### `armclang`

Configuration `as:` value: `armclang`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `armclang`, `armclang++` | Arm Compiler 6 | `armclang-12` | not recognized | [developer.arm.com](https://developer.arm.com/documentation/100748/latest/) |

#### `clang`

Configuration `as:` value: `clang`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `clang`, `clang++` | Clang | `clang-12` | `arm-linux-gnueabihf-clang` | [clang.llvm.org](https://clang.llvm.org/docs/ClangCommandLineReference.html) |
| `amdclang`, `amdclang++`, `hipcc` | AMD ROCm Clang/HIP | not recognized | not recognized | [rocm.docs.amd.com](https://rocm.docs.amd.com/en/latest/) |
| `emcc`, `em++`, `emcc.py`, `em++.py` | Emscripten | not recognized | not recognized | [emscripten.org](https://emscripten.org/docs/tools_reference/emcc.html) |
| `tiarmclang` | Texas Instruments Arm Clang | not recognized | not recognized | [ti.com](https://www.ti.com/tool/ARM-CGT) |

An invocation is also ignored when its arguments include `-cc1`: that is an internal frontend or codegen call, not a user-facing compile.

#### `clang_cl`

Configuration `as:` value: `clang_cl`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `clang-cl` | Clang (MSVC mode) | `clang-cl-12` | not recognized | [clang.llvm.org](https://clang.llvm.org/docs/UsersManual.html#clang-cl) |

#### `cray_cc`

Configuration `as:` value: `cray_cc`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `craycc`, `crayCC`, `craycxx` | Cray C/C++ | `craycc-12` | not recognized | [support.hpe.com](https://support.hpe.com/hpesc/public/docDisplay?docId=a00113893en_us&page=index.html) |

#### `cray_fortran`

Configuration `as:` value: `cray_fortran`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `crayftn`, `ftn` | Cray Fortran | `crayftn-12` | not recognized | [support.hpe.com](https://support.hpe.com/hpesc/public/docDisplay?docId=a00115296en_us&page=Fortran_Command-line_Options.html) |

#### `cuda`

Configuration `as:` value: `cuda`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `nvcc` | NVIDIA CUDA | `nvcc-12` | `arm-linux-gnueabihf-nvcc` | [docs.nvidia.com](https://docs.nvidia.com/cuda/cuda-compiler-driver-nvcc/index.html) |

#### `fasm`

Configuration `as:` value: `fasm`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `fasm` | flat assembler | not recognized | not recognized | [flatassembler.net](https://flatassembler.net/docs.php) |

#### `flang`

Configuration `as:` value: `flang`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `flang`, `flang-new` | Flang | `flang-12` | `arm-linux-gnueabihf-flang` | [flang.llvm.org](https://flang.llvm.org/docs/FlangDriver.html) |
| `amdflang` | AMD ROCm Flang | not recognized | not recognized | [rocm.docs.amd.com](https://rocm.docs.amd.com/en/latest/) |

#### `gcc`

Configuration `as:` value: `gcc`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `gcc`, `g++`, `gfortran`, `egfortran`, `f95` | GCC | `gcc-12` | `arm-linux-gnueabihf-gcc` | [gcc.gnu.org](https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html) |
| `xc8-cc`, `xc8` | Microchip XC8 | not recognized | not recognized | [microchip.com](https://www.microchip.com/en-us/tools-resources/develop/mplab-xc-compilers) |

Internal, not user-facing: `cc1`, `cc1plus`, `cc1obj`, `cc1objplus`, `f951`, `collect2`, `lto1`. Bear recognizes these only so it can filter them back out; they never produce a database entry.

#### `ibm_xl`

Configuration `as:` value: `ibm_xl`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `ibm-clang`, `ibm-clang++` | IBM Open XL C/C++ | `ibm-clang-12` | not recognized | [ibm.com](https://www.ibm.com/docs/en/openxl-c-and-cpp-aix) |
| `xlclang`, `xlclang++` | IBM XL C/C++ | `xlclang-12` | not recognized | [ibm.com](https://www.ibm.com/docs/en/xl-c-and-cpp-aix) |

#### `intel_cc`

Configuration `as:` value: `intel_cc`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `icx`, `icpx` | Intel oneAPI C/C++ | `icx-12` | not recognized | [intel.com](https://www.intel.com/content/www/us/en/developer/tools/oneapi/dpc-compiler.html) |
| `icc`, `icpc` | Intel C++ Compiler Classic | `icc-12` | not recognized | [intel.com](https://www.intel.com/content/www/us/en/developer/tools/oneapi/dpc-compiler.html) |
| `mpiicc`, `mpiicpc`, `mpiicx`, `mpiicpx` | Intel MPI C/C++ wrapper | not recognized | not recognized | [intel.com](https://www.intel.com/content/www/us/en/developer/tools/oneapi/mpi-library.html) |

#### `intel_fortran`

Configuration `as:` value: `intel_fortran`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `ifort`, `ifx` | Intel Fortran | `ifort-12` | not recognized | [intel.com](https://www.intel.com/content/www/us/en/developer/tools/oneapi/fortran-compiler.html) |
| `mpiifort`, `mpiifx` | Intel Fortran MPI wrapper | not recognized | not recognized | [intel.com](https://www.intel.com/content/www/us/en/developer/tools/oneapi/mpi-library.html) |

#### `mpi`

Configuration `as:` value: `mpi`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `mpicc`, `mpicxx`, `mpic++`, `mpiCC`, `mpifort`, `mpif77`, `mpif90` | MPI compiler wrapper | `mpicc-12` | not recognized | [open-mpi.org](https://www.open-mpi.org/doc/), [mpich.org](https://www.mpich.org/documentation/) |

#### `msvc`

Configuration `as:` value: `msvc`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `cl` | Microsoft Visual C++ | not recognized | not recognized | [learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/build/reference/compiler-options) |

Internal, not user-facing: `c1`, `c1xx`, `c2`. Bear recognizes these only so it can filter them back out; they never produce a database entry.

#### `nasm`

Configuration `as:` value: `nasm`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `nasm`, `yasm` | NASM / YASM assembler | not recognized | not recognized | [nasm.us](https://www.nasm.us/doc/nasmdoc2.html), [github.com](https://github.com/yasm/yasm) |

#### `nvidia_hpc`

Configuration `as:` value: `nvidia_hpc`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `nvc`, `nvc++`, `nvfortran` | NVIDIA HPC SDK | `nvc-12` | not recognized | [docs.nvidia.com](https://docs.nvidia.com/hpc-sdk/compilers/hpc-compilers-user-guide/index.html) |
| `pgcc`, `pgc++`, `pgfortran` | PGI (legacy NVIDIA HPC) | `pgcc-12` | not recognized | [docs.nvidia.com](https://docs.nvidia.com/hpc-sdk/compilers/hpc-compilers-user-guide/index.html) |

#### `qnx`

Configuration `as:` value: `qnx`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `qcc`, `q++` | QNX qcc | not recognized | not recognized | [qnx.com](https://www.qnx.com/developers/docs/) |

#### `swift`

Configuration `as:` value: `swift`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `swiftc` | Swift | not recognized | not recognized | [swift.org](https://www.swift.org/documentation/) |

Internal, not user-facing: `swift-frontend`. Bear recognizes these only so it can filter them back out; they never produce a database entry.

An invocation is also ignored when its arguments include `-frontend`: that is an internal frontend or codegen call, not a user-facing compile.

#### `vala`

Configuration `as:` value: `vala`.

| Executable names | Recognized as | Version suffix | Cross-compilation prefix | Documentation |
|---|---|---|---|---|
| `valac` | Vala | `valac-12` | `arm-linux-gnueabihf-valac` | [vala.dev](https://vala.dev/) |

### Compiler launchers

| Executable name | Recognized as | Configuration `as:` value | Documentation |
|---|---|---|---|
| `ccache` | Compiler cache | `ccache` | [ccache.dev](https://ccache.dev/manual/latest.html) |
| `distcc` | Distributed compiler | `distcc` | [distcc.org](https://www.distcc.org/man/distcc_1.html) |
| `icecc` | Distributed compiler | `icecc` | [github.com](https://github.com/icecc/icecream/blob/master/README.md) |
| `sccache` | Compiler cache | `sccache` | [github.com](https://github.com/mozilla/sccache/blob/main/README.md) |

<!-- END GENERATED -->

This list is kept in step with Bear's own compiler definitions, and grows
by request as users bring toolchains that are not covered yet; it is
not a fixed set.

## What is not recognized

A few names are excluded on purpose, not by oversight:

- **`as`** (the GNU assembler): GCC and Clang already spawn it on a
  temporary file for every ordinary compile, so recognizing it would
  add a throwaway entry per compilation, keyed on a name that differs
  every run.
- **`mpirun`, `mpiexec`**: these launch programs, they do not compile
  anything.
- **`swift`**: this is the Swift subcommand dispatcher (`swift build`,
  `swift run`); the actual compiler invocation is `swiftc`.
- **`amdgpu-arch`**: reports target GPU architectures; despite the
  shared prefix, it is not a compiler driver.
- **`ml`, `ml64`** (MASM): not currently supported.

None of these can be recovered with a `compilers:` override, because
`as` only redirects a path to one of the families already in the
tables above; it cannot teach Bear a new one. If your build uses a
toolchain that genuinely is not covered anywhere in this page, that is
a gap in Bear's compiler definitions, not a configuration problem: it
is worth filing as an issue against the project.

See also: [Configure Bear](configuration.md) for the `compilers:`
section in full, and [How Bear works](../understanding/how-it-works.md) for where
recognition fits between interception and writing the database.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md
