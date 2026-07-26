<!-- Diataxis type: explanation -->

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
short, lower-case id in the `as:` column of the family tables below,
spelled verbatim. There are no aliases, so `as: gcc` is accepted while
`as: GCC` is not, and several families share one id (`icx` and `icc`
are both `intel_cc`). The complete accepted set is

    armclang, cray_cc, ibm_xl, intel_cc, clang, clang_cl, cuda, flang,
    mpi, qnx, cray_fortran, fasm, gcc, intel_fortran, msvc, nasm,
    nvidia_hpc, swift, vala

plus the compiler-launcher spellings `wrapper`, `ccache`, `distcc`,
`icecc`, and `sccache`, which all select the one launcher kind. An
unaccepted value is rejected when the configuration loads, with an
error listing every value that would have worked. `bear semantic
--print-compilers` prints the same mapping for the version you have
installed.

## Recognized families

Internal jobs that a driver spawns for itself (GCC's `cc1`, `cc1plus`,
`collect2`; MSVC's `c1`, `c2`; Swift's `swift-frontend`) are recognized
only so Bear can filter them back out: they are never user-facing
invocations and never produce a database entry. They are omitted from
the tables below; everything else Bear recognizes is expected to
produce an entry when it compiles a source.

### GCC and Clang family

| Executable names | Recognized as | `as:` value |
|---|---|---|
| `gcc`, `g++`, `gfortran`, `egfortran`, `f95` | GCC | `gcc` |
| `clang`, `clang++` | Clang | `clang` |
| `clang-cl` | Clang (MSVC mode) | `clang_cl` |

### Vendor compilers built on GCC or Clang

| Executable names | Recognized as | `as:` value |
|---|---|---|
| `armclang`, `armclang++` | Arm Compiler 6 | `armclang` |
| `craycc`, `crayCC`, `craycxx` | Cray C/C++ | `cray_cc` |
| `ibm-clang`, `ibm-clang++` | IBM Open XL C/C++ | `ibm_xl` |
| `xlclang`, `xlclang++` | IBM XL C/C++ | `ibm_xl` |
| `icx`, `icpx` | Intel oneAPI C/C++ | `intel_cc` |
| `icc`, `icpc` | Intel C++ Compiler Classic | `intel_cc` |
| `qcc`, `q++` | QNX qcc | `qnx` |
| `amdclang`, `amdclang++`, `hipcc` | AMD ROCm Clang/HIP | `clang` |
| `emcc`, `em++`, `emcc.py`, `em++.py` | Emscripten | `clang` |
| `tiarmclang` | Texas Instruments Arm Clang | `clang` |
| `xc8-cc`, `xc8` | Microchip XC8 | `gcc` |

### Fortran

| Executable names | Recognized as | `as:` value |
|---|---|---|
| `crayftn`, `ftn` | Cray Fortran | `cray_fortran` |
| `ifort`, `ifx` | Intel Fortran | `intel_fortran` |
| `flang`, `flang-new` | Flang | `flang` |
| `amdflang` | AMD ROCm Flang | `flang` |
| `nvc`, `nvc++`, `nvfortran` | NVIDIA HPC SDK | `nvidia_hpc` |
| `pgcc`, `pgc++`, `pgfortran` | PGI (legacy NVIDIA HPC) | `nvidia_hpc` |

### CUDA and MSVC

| Executable names | Recognized as | `as:` value |
|---|---|---|
| `nvcc` | NVIDIA CUDA | `cuda` |
| `cl` | Microsoft Visual C++ | `msvc` |

### Assemblers

| Executable names | Recognized as | `as:` value |
|---|---|---|
| `nasm`, `yasm` | NASM / YASM assembler | `nasm` |
| `fasm` | flat assembler | `fasm` |

### Swift and Vala

| Executable names | Recognized as | `as:` value |
|---|---|---|
| `swiftc` | Swift | `swift` |
| `valac` | Vala | `vala` |

### MPI compiler wrappers

| Executable names | Recognized as | `as:` value |
|---|---|---|
| `mpicc`, `mpicxx`, `mpic++`, `mpiCC`, `mpifort`, `mpif77`, `mpif90` | MPI compiler wrapper | `mpi` |
| `mpiicc`, `mpiicpc`, `mpiicx`, `mpiicpx` | Intel MPI C/C++ wrapper | `intel_cc` |
| `mpiifort`, `mpiifx` | Intel Fortran MPI wrapper | `intel_fortran` |

### Compiler launchers

The four launchers share one kind rather than a compiler family, so any
of `wrapper`, `ccache`, `distcc`, `icecc`, and `sccache` selects it.

| Executable names | Recognized as |
|---|---|
| `ccache` | Compiler cache |
| `sccache` | Compiler cache |
| `distcc` | Distributed compiler |
| `icecc` | Distributed compiler |

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
section in full, and [How Bear works](how-it-works.md) for where
recognition fits between interception and writing the database.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md
