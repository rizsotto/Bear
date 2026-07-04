% BEAR(1) Bear User Manuals
% László Nagy
% July 3, 2026
<!-- to generate the final `bear.1` file, run `pandoc -s -t man bear.1.md -o bear.1` -->

# NAME

Bear - a tool to generate compilation database for Clang tooling.

# SYNOPSIS

**bear** [*OPTIONS*] [\-\-] [*BUILD_COMMAND*...]

**bear intercept** [*OPTIONS*] [\-\-] *BUILD_COMMAND*...

**bear semantic** [*OPTIONS*]


# DESCRIPTION

Bear is a tool that generates a JSON compilation database for Clang tooling by intercepting command executions during the build process. The JSON compilation database is used in the Clang project to provide information about how individual compilation units were processed, enabling tools like clang-tidy, clangd, and other Clang-based analysis tools to understand your project's build configuration.

Bear operates by intercepting system calls during the build process to capture compilation commands. It supports two main interception methods: dynamic library preloading (on Unix-like systems) and wrapper executables (cross-platform). The captured commands are then filtered through semantic analysis to identify actual compiler invocations and generate the final compilation database.

Bear can operate in three modes:

- **Combined mode** (default): Runs both interception and semantic analysis in sequence
- **Intercept mode**: Only captures build events to an intermediate file
- **Semantic mode**: Processes previously captured events to generate the compilation database

## OPTIONS

**-c, \-\-config** *FILE*
: Specify a configuration file path. The configuration file controls output formatting, compiler recognition, source filtering, and duplicate handling.

**-o, \-\-output** *FILE*
: Specify the output file path (default: `compile_commands.json`). The output is a JSON compilation database.

**-a, \-\-append**
: Append results to an existing output file instead of overwriting it. This allows incremental updates to the compilation database. New entries are placed before the existing ones, so when a source file is rebuilt its newest invocation survives duplicate filtering and replaces the stale entry (see the `duplicates` section).

**-h, \-\-help**
: Print help information.

**-V, \-\-version**
: Print version information.


# COMMANDS

Calling bear without commands will execute the combined mode, and will intercept the
compiler calls and generate a compilation database as output.

## bear intercept

Intercepts command execution events during the build process and saves them to an events file for later processing.

**bear intercept** [*OPTIONS*] [\-\-] *BUILD_COMMAND*...

## bear semantic

Processes previously captured events to generate a compilation database through semantic analysis.

**bear semantic** [*OPTIONS*]


# OUTPUT

Bear generates a JSON compilation database conforming to the [Clang JSON Compilation Database](https://clang.llvm.org/docs/JSONCompilationDatabase.html) specification. The output is a JSON array of compilation entry objects.

## Entry Format

Each compilation database entry contains the following fields:

**directory**
: The working directory of the compilation (absolute path)

**file**
: The main translation unit source file (absolute path)

**arguments**
: The compilation command as an array of strings (preferred format)

**command**
: The compilation command as a single shell-escaped string (alternative to arguments)

**output**
: The output file produced by compilation (optional, absolute path)

## Output Formatting

The output format can be controlled through the configuration file:

- **Path resolution**: Paths can be formatted as absolute, relative, canonical, or as-is
- **Entry format**: Choose between arguments array (preferred) or command string
- **Field inclusion**: Control whether the output field is included
- **Source filtering**: Include/exclude files based on directory rules
- **Duplicate filtering**: Remove duplicate entries based on configurable field matching

Bear generates entries where all paths are absolute by default, and uses the `arguments` field instead of `command` to avoid shell escaping issues.


# CONFIG FILE

Bear uses a YAML configuration file to control its behavior. The configuration file follows a structured schema with several main sections.

## Configuration Schema

```yaml
schema: "4.1"
intercept:
  mode: wrapper
compilers:
  - path: /usr/bin/cc
    as: gcc
  - path: /usr/local/bin/gcc
    ignore: true
sources:
  directories:
    - path: /project/tests
      action: exclude
duplicates:
  match_on:
    - file
    - arguments
format:
  paths:
    directory: canonical
    file: canonical
  entries:
    use_array_format: true
    include_output_field: true
  arguments:
    from_response_files: false
    from_environment: true
```

This example configuration file:
 sets the interception mode to `wrapper`,
 hints the `/usr/bin/cc` to be the main compiler in this project, which is the GNU compiler,
 hints to ignore the `/usr/local/bin/gcc` compilers from the project,
 instructs to ignore files from `/project/tests`,
 instructs to detect duplicates based on the `file` and `arguments` fields of the output file,
 instructs to format the output to use canonical path for the `file` and `directory` fields of the output file,
 instructs to use the `arguments` over the `command` field in the output file,
 instructs to include the `output` field in the output file.

## Configuration Sections

The configuration file uses schema version `4.1` and has the following structure:

### intercept

Controls the command interception method:

- **mode**: `preload` (Unix) or `wrapper` (cross-platform)

### compilers

Contains hints about what compiler needs to be recognized and what that compiler is.

- **path**: Path to the compiler executable
- **as**: Compiler type hint for semantic analysis. Valid values are: `gcc`, `clang`, `flang`, `intel-fortran`, `cray-fortran`, `cuda`, `msvc`, `clang-cl`, `intel_cc`, `nvidia-hpc`, `armclang`, `ibm_xl`, `vala`, `mpi`, `cray-cc`, `qnx`.
- **ignore**: Whether to ignore this compiler.

The generic compiler names `cc`, `c++`, and the HPE Cray PrgEnv wrapper `CC` default to GCC/Clang semantics chosen by probing the executable's `--version` output (since the same basename can be a different compiler depending on the platform or, for `CC`, the loaded Cray programming environment). On platforms where the probe cannot classify the executable, use the `as` field to override:

```yaml
compilers:
  - path: /usr/bin/cc
    as: clang
  - path: /usr/bin/c++
    as: clang
```

MPI compiler wrappers (Open MPI/MPICH's `mpicc`, `mpicxx`, `mpic++`, `mpiCC`, `mpifort`, `mpif77`, `mpif90`) are recognized automatically, without any configuration. The wrapper is recorded as the compiler exactly as invoked -- Bear does not expand it to the underlying compiler command it wraps. Clang tooling that needs the wrapper's baked-in include paths can point at the wrapper directly (e.g. clangd's `--query-driver`). Intel MPI's wrappers (`mpiicc`, `mpiicpc`, `mpiicx`, `mpiicpx`, `mpiifort`, `mpiifx`) are recognized as the Intel compilers they front, using Intel's flag semantics. The launchers `mpirun` and `mpiexec` are not recognized: they execute programs, they do not compile.

The Cray Compiling Environment (CCE) C/C++ compiler names `craycc`, `crayCC`, and `craycxx` are recognized automatically, using Clang flag semantics (CCE C/C++ is Clang-based). The HPE Cray PrgEnv wrapper `CC` is classified by the same version probe as `cc`/`c++`: it resolves to CCE Clang under PrgEnv-cray, GCC under PrgEnv-gnu, and so on, matching whatever compiler module is currently loaded. A programming environment whose compiler prints a version banner the probe does not recognize (for example `nvc++` under PrgEnv-nvidia) is not classified; use the `as` field on that path to override.

AMD's ROCm compiler names `amdclang`, `amdclang++`, and `hipcc` are recognized automatically, using Clang flag semantics; `amdflang` is recognized automatically, using Flang flag semantics. `hipcc` is a compiler driver (it calls clang or nvcc depending on target), the same way `nvcc` is; Bear records the driver invocation as executed. AOCC's plain `clang`/`clang++`/`flang` names were already covered by the existing Clang/Flang recognition.

QNX's compiler driver names `qcc` and `q++` are recognized automatically, using GCC flag semantics (QNX's toolchain is GCC-backed). QNX's variant selector, `-V` (e.g. `-Vgcc_ntoaarch64le`, or bare `-V` to list available variants), is always treated as a driver option and is never mistaken for a source file.

### sources

Filtering functionality based on the source file location.

- **directories**: List of directory-based inclusion/exclusion rules

Directory rules are evaluated in order, with the last matching rule determining inclusion/exclusion. Empty directories list means include everything.

### duplicates

Filtering functionality based on duplicate detection. Here you can define which fields of the output file should be used in the duplicate detection. Two entries are duplicates when all of the configured fields match; the first occurrence is kept and later duplicates are dropped.

- **match_on**: List of fields to use for duplicate detection (file, arguments, directory, command, output). The default is `directory` and `file`, so by default one entry is kept per source file per directory, regardless of the compiler arguments. A consequence is that when a single build compiles the same file from the same directory with different flags, only one entry is kept (the first seen); add `arguments` to `match_on` to keep a separate entry per configuration. Combined with `--append` (new entries first), the default means a rebuilt file's newest invocation replaces its stale entry.

### format

Output formatting configuration:

- **paths.directory** and **paths.file**: How to format paths of these fields. The allowed values are:
  - **as-is**: No transformation,
  - **canonical**: Resolve to canonical path,
  - **relative**: Make relative to directory field,
  - **absolute**: Convert to absolute path,
- **entries.use_array_format**: Use the arguments array (default) instead of the command string. Set to `false` for consumers that read only the `command` field.
- **entries.include_output_field**: Include output field in entries
- **arguments.from_response_files**: Replace `@file` response-file references in each entry's arguments with the file's tokenized contents (resolved relative to the compiler's working directory, expanded recursively, MSVC/clang-cl using Windows quoting and other compilers using GCC/Clang quoting). Disabled by default, in which case an `@file` argument is recorded verbatim. Missing or unreadable files are left literal with a warning.
- **arguments.from_environment**: Fold compiler environment variables that act as implicit flags into each entry's arguments -- GCC/Clang header-search paths (`CPATH`, `C_INCLUDE_PATH`, `CPLUS_INCLUDE_PATH`, `OBJC_INCLUDE_PATH`) become include flags, and MSVC's `CL` / `_CL_` become leading / trailing options. Enabled by default. Set to `false` to record only the flags that appeared on the command line. (This is unrelated to the `CC="gcc -std=c11"` convention handled during interception.)

## Default Configuration

If no configuration file is specified, Bear uses built-in defaults optimized for most use cases.


# ENVIRONMENT

**RUST_LOG**
: Controls the logging level for Bear's internal operations. This environment variable is essential for troubleshooting and debugging Bear's behavior.

    Supported log levels (in order of verbosity):
    
    - `error` - Only show critical errors
    - `warn` - Show warnings and errors  
    - `info` - Show informational messages, warnings, and errors
    - `debug` - Show detailed debugging information

    Examples:
    ```
    RUST_LOG=debug bear -- make all
    RUST_LOG=info bear intercept -- cmake --build .
    ```

# FILES

The configuration file `bear.yml` is searched in the following locations, in order:

**`./bear.yml`**
: The current working directory.

**`$XDG_CONFIG_HOME/bear.yml`**, **`$XDG_CONFIG_HOME/Bear/bear.yml`** (Unix)
: When `$XDG_CONFIG_HOME` is set.

**`$HOME/.config/bear.yml`**, **`$HOME/.config/Bear/bear.yml`** (Unix)
: When `$XDG_CONFIG_HOME` is unset.

**`%LOCALAPPDATA%\bear.yml`**, **`%LOCALAPPDATA%\Bear\bear.yml`** (Windows)
: When `%LOCALAPPDATA%` is set.

**`%APPDATA%\bear.yml`**, **`%APPDATA%\Bear\bear.yml`** (Windows)
: When `%APPDATA%` is set.

The first file found is loaded; remaining locations are not consulted.


# EXIT STATUS

Bear returns the exit status of the executed build command when running in combined or intercept mode. When the build command succeeds, Bear returns 0. When the build command fails, Bear returns the same non-zero exit code.

In semantic mode, Bear returns 0 on success and a non-zero exit code if semantic analysis fails.

If Bear itself encounters an internal error or crashes, it returns a non-zero exit code regardless of the build command's status.


# TROUBLESHOOTING

The potential problems you can face with are: the build with and without Bear
behaves differently or the output is empty.

## Debug Logging

**Before reporting any issues**, always run Bear with debug logging enabled:

```
RUST_LOG=debug bear -- your-build-command
```

This will provide detailed information about Bear's internal operations. And the
debug output is essential for diagnosing problems and **must be included** in any
bug reports.

## Common Issues

The most common cause for empty outputs is that the build command did not
execute any commands. The reason for that could be, because incremental builds
not running the compilers if everything is up-to-date. Remember, Bear does not
understand the build file (eg.: makefile), but intercepts the executed
commands.

The other common cause for empty output is that the build has a "configure"
step, which captures the compiler to build the project. In case of Bear is
using the _wrapper_ mode, it needs to run the configure step with Bear too
(and discard that output), before run the build with Bear.

## GLIBC Version Errors in Cross-Compilation

When the build runs compilers from a cross-compilation SDK and you see an
error like `version 'GLIBC_2.33' not found (required by .../libexec.so)`,
Bear's preload library was built against a newer glibc than the SDK toolchain
provides. The library must be ABI-compatible not only with the host system
but also with the libc the intercepted compiler process loads from the SDK
sysroot. Build (or obtain) a Bear linked against a glibc no newer than the
SDK's. See the project wiki Troubleshooting page (LD_PRELOAD errors) for
diagnostic commands.

## Compiler Env Vars With Flags

In wrapper mode, Bear accepts compiler environment variables that carry a
trailing flag or two, matching the GNU Make convention:

```
CC="gcc -std=c11" make
CXX="clang++ -stdlib=libc++" make
CC="/usr/local/bin/gcc -m32" make
```

Bear splits the value on whitespace, resolves the first token as the
compiler, and rewrites the variable so the build still sees the flags
(`CC=<wrapper_path> -std=c11`).

This convention is a Unix / GNU Make inheritance; it applies when
`bear -- make` runs under sh/bash (including MSYS2, Git Bash, WSL on
Windows), not under native Windows build systems (MSBuild, `nmake`,
`cmd`, PowerShell), which do not consume `CC`/`CXX` from the
environment.

For anything beyond simple whitespace-separated tokens (flags containing
spaces, shell quoting, metacharacters, command substitutions), use
`CFLAGS`, `CXXFLAGS`, or `LDFLAGS` instead of packing it into `CC`. Those
variables are the portable channel for compilation flags and every build
system expects them. Bear does not parse or rewrite them; they reach the
compiler untouched.

## Vala Projects

Bear records `valac` invocations, producing one entry per `valac`
invocation (valac compiles all of a target's `.vala`/`.gs` sources
together as one translation unit, so the entry's `file` is the first
source and every source is kept in the command). Two things are worth
knowing:

- **vala-language-server might require the command-string form.** It
  reads the `command` field and ignores the `arguments` array, so build
  the database with the array format disabled:

    ```yaml
    format:
      entries:
        use_array_format: false
    ```

- **Mixed C and Vala projects.** `valac` transpiles to C and then invokes a
  C compiler on the generated C, so the database also contains entries for
  that generated C. A C language server such as clangd indexes every entry
  and will emit unknown-argument noise on the `valac` entries. Exclude the
  Vala sources on the clangd side with a `.clangd` file:

    ```yaml
    If:
      PathMatch: .*\.vala
    Diagnostics:
      Suppress: '*'
    ```

## Getting Help

There could be many reasons for any of these failures. When seeking help:

1. **Always include debug logs** (`RUST_LOG=debug`) in your report
2. Consult the project wiki page for known problems
3. Search existing issues before opening a new bug report
4. Follow the bug report template, provide the requested fields

# COPYRIGHT

Copyright (C) 2012-2026 by László Nagy
<https://github.com/rizsotto/Bear>
