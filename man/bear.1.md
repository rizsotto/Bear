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

**bear parse-sh** [*OPTIONS*]


# DESCRIPTION

Bear is a tool that generates a JSON compilation database for Clang tooling by intercepting command executions during the build process. The JSON compilation database is used in the Clang project to provide information about how individual compilation units were processed, enabling tools like clang-tidy, clangd, and other Clang-based analysis tools to understand your project's build configuration.

Bear operates by intercepting system calls during the build process to capture compilation commands. It supports two main interception methods: dynamic library preloading (on Unix-like systems) and wrapper executables (cross-platform). The captured commands are then filtered through semantic analysis to identify actual compiler invocations and generate the final compilation database.

Bear can operate in four modes:

- **Combined mode** (default): Runs both interception and semantic analysis in sequence
- **Intercept mode**: Only captures build events to an intermediate file
- **Semantic mode**: Processes previously captured events to generate the compilation database
- **Parse-sh mode**: Reconstructs the event stream from shell command text (for example, `make -n` dry-run output), without running a build at all

## OPTIONS

**-c, \-\-config** *FILE*
: Specify a configuration file path. The configuration file controls output formatting, compiler recognition, source filtering, and duplicate handling. It applies to every mode except `bear parse-sh`, which only emits an event stream and never consults it; passing `--config` to `bear parse-sh` is an error rather than a silent no-op.

**-o, \-\-output** *FILE*
: Specify the output file path (default: `compile_commands.json`). The output is a JSON compilation database. This option runs the build (combined mode, and `bear intercept`'s own `--output`), so it does not accept `-` for standard output: the build's own stdout shares that stream, and a non-atomic write could corrupt it. Use a file path, or split into `bear intercept` followed by `bear semantic --input -` (see below).

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

**-o, \-\-output** *FILE*
: Path of the event file (default: `events.json`). Rejects `-`: `bear intercept` runs the build, so its stdout is the build's stdout, and writing events there would corrupt the stream.

## bear semantic

Processes previously captured events to generate a compilation database through semantic analysis.

**bear semantic** [*OPTIONS*]

**-i, \-\-input** *FILE*
: Path of the event file to read (default: `events.json`). Pass `-` to read the event stream from standard input instead of a file, so the events format is pipeable: any non-executing producer of a conforming event stream can feed `bear semantic` directly, for example:

      <producer> | bear semantic --input -

  Since `bear semantic` does not run the build, it has no conflicting use for its own stdout; diagnostics still go to stderr, keeping stdout machine-readable.

**-o, \-\-output** *FILE*
: Path of the compilation database to write (default: `compile_commands.json`). Pass `-` to write it to standard output instead of a file -- again safe because `bear semantic` runs no build -- so the whole flow can stream, for example `<producer> | bear semantic --input - --output -`. Standard-output writing is not atomic and cannot be appended to, so `--output -` together with `--append` is rejected. (This differs from `bear intercept`'s and combined mode's `--output`, which reject `-` because their stdout is shared with the build.)

## bear parse-sh

Parses shell command text -- typically the output of a build system's dry-run mode, such as `make -n` (or `make -n -w` for recursive builds), or a saved build log -- into the same event stream `bear intercept` produces, without running anything. Feed that stream to `bear semantic` to get a compilation database:

      make -n | bear parse-sh | bear semantic --input -

**bear parse-sh** [*OPTIONS*]

**-i, \-\-input** *FILE*
: Path of the shell text to parse (default: `-`, reads from standard input).

**-o, \-\-output** *FILE*
: Path of the event file to write (default: `-`, writes to standard output). Because `bear parse-sh` runs no build, it has no conflicting use for its own stdout, so `-` is the default here (unlike `bear intercept`'s `--output`, which rejects it).

**-C, \-\-directory** *DIR*
: Sets the initial working directory for the parsed commands, for input captured elsewhere (a CI log, or a dry run from another checkout) whose paths would otherwise be interpreted relative to `bear`'s own working directory. Give an absolute path: a foreign log's build directory bears no relation to `bear`'s own working directory, and a relative value would be recorded verbatim. Not validated: the directory need not exist on this machine.

This is a best-effort front end over a documented subset of shell syntax (word splitting and quoting, `;`/`&&`/`||`/`&`/`|` separators, comments, redirections, `cd`, and recursive make's `Entering directory`/`Leaving directory` markers). Anything outside that subset -- subshells, command substitution, parameter expansion, glob in the executable position, here-documents, unterminated quotes, and shell keywords (`if`, `for`, `while`, `case`) -- causes that one line to be skipped, reported on standard error with its line number and reason; a here-document's body lines are consumed along with its opening line. The run still succeeds as long as at least one line produced an event.

**Interception remains the higher-fidelity, recommended default.** It observes the real `exec()` calls a build makes; `bear parse-sh` only reconstructs approximate events from text the build system chose to print during a dry run. In particular:

- A dry run can omit commands entirely: recursive `make` does not always propagate `-n` to sub-makes, commands behind not-yet-generated sources never print because the generator never ran, and `$(shell ...)` output captured at parse time can differ from a real build's.
- The build system must both support a dry-run mode and print real commands in it; silent rules, custom launchers, and response files reduce what `bear parse-sh` can see.
- The environment and `PATH` used to resolve bare executable names are `bear parse-sh`'s own at parse time, which may differ from the real build's -- especially when parsing a log captured on another machine, where `--directory` fixes the working directory but not the environment.
- Only POSIX `sh` command text is supported; non-POSIX shells and Windows `cmd` are out of scope, and interleaved non-command output (compiler banners, warnings) in a saved log is skipped loudly like any other unsupported line.

`bear parse-sh` itself takes no configuration file -- it rejects `--config` -- because it only emits an event stream. Configuration such as `format.paths` is applied by the `bear semantic` step that consumes the stream, so the settings below are set there, not on `bear parse-sh`.

The source files named in the parsed commands need not exist on the machine running `bear parse-sh`: entries are reconstructed from the text alone, and the default path format (`format.paths: as-is`) never touches the filesystem. The one exception is `format.paths: canonical`, which resolves symlinks and so requires every path to exist on disk; when parsing a log whose sources are absent (a CI log, another checkout) use `format.paths: absolute` for existence-free normalization instead.

Prefer `bear -- <build command>` (or `bear intercept`) whenever the build can actually be run; reach for `bear parse-sh` when it cannot -- for example, reconstructing a compilation database from a CI log after the fact.


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
  files:
    - pattern: "moc_*.cpp"
      action: exclude
    - pattern: "*.pb.cc"
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
headers:
  enabled: true
  strategy: siblings
```

This example configuration file:
 sets the interception mode to `wrapper`,
 hints the `/usr/bin/cc` to be the main compiler in this project, which is the GNU compiler,
 hints to ignore the `/usr/local/bin/gcc` compilers from the project,
 instructs to ignore files from `/project/tests`,
 instructs to drop generated Qt moc output (`moc_*.cpp`) and protobuf stubs (`*.pb.cc`) by filename,
 instructs to detect duplicates based on the `file` and `arguments` fields of the output file,
 instructs to format the output to use canonical path for the `file` and `directory` fields of the output file,
 instructs to use the `arguments` over the `command` field in the output file,
 instructs to include the `output` field in the output file,
 instructs to synthesize compilation entries for header files using the same-directory sibling strategy.

## Configuration Sections

The configuration file uses schema version `4.1` and has the following structure:

### intercept

Controls the command interception method:

- **mode**: `preload` (Unix) or `wrapper` (cross-platform)

### compilers

Contains hints about what compiler needs to be recognized and what that compiler is.

- **path**: Path to the compiler executable
- **as**: Compiler type hint for semantic analysis. Valid values are: `gcc`, `clang`, `flang`, `intel-fortran`, `cray-fortran`, `cuda`, `msvc`, `clang-cl`, `intel_cc`, `nvidia-hpc`, `armclang`, `ibm_xl`, `vala`, `mpi`, `cray-cc`, `qnx`, `nasm`, `fasm`, `swift`.
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

Emscripten's driver names `emcc` and `em++` (including the `emcc.py`/`em++.py` spellings) and Texas Instruments' `tiarmclang` are recognized automatically, using Clang flag semantics. In preload mode the underlying `clang` child process that `emcc`/`em++` spawn may be intercepted too; the default duplicate detection collapses the pair to a single entry recording the driver invocation. Microchip's XC8 driver names `xc8-cc` and `xc8` are recognized automatically, using GCC flag semantics; the `xc16-gcc` and `xc32-gcc` names were already covered by the existing cross-compiler prefix recognition.

C++20 module-interface units (`.cppm`, `.ixx`, `.mxx`, `.ccm`, `.cxxm`, `.c++m`) are recognized as sources, so their compilations are captured the same way ordinary `.cpp` translation units are -- for example `clang++ --precompile foo.cppm -o foo.pcm`, or a consumer built with GCC's `-fmodules-ts` or with `-fmodule-file=`. Precompiled module artifacts (`.pcm`) are never recorded as sources: they appear on the command line but never as an entry's `file`.

Compiler launchers (`ccache`, `distcc`, `sccache`, `icecc`) that carry the real compiler in their arguments are recorded as the real compiler's compilation: `ccache gcc -c main.c` produces an entry for `gcc -c main.c`, with the launcher token dropped. A launcher invocation that does not name a recognized compiler, or a launcher wrapping another launcher, produces no entry. icecream's `icerun` is not a compiler launcher (it runs arbitrary commands on the cluster) and is not recognized.

The standalone assemblers `nasm`, `yasm`, and `fasm` are recognized automatically, so assembly language servers (for example asm-lsp) can read per-file assembler flags from `compile_commands.json`. Assembly compiled through a C/C++ compiler driver (for example `gcc -c foo.s`) was already recorded via that driver's own entry before this support existed; both paths now produce entries. The GNU assembler `as` is deliberately not recognized: gcc and clang spawn it internally on a temporary `.s` file for every ordinary C compile, and recognizing it would pollute the database with one throwaway entry per compilation, using a temporary filename that duplicate detection cannot collapse. MASM (`ml`, `ml64`) is out of scope (Windows-only, no recorded demand).

The Swift compiler `swiftc` is recognized automatically; see "Swift Projects" below for its whole-module entry shape. The `swift` subcommand driver (`swift build`, `swift run`, `swift package`) is not recognized -- it is a subcommand dispatcher, not a compiler invocation.

### sources

Filtering functionality based on the source file location and filename.

- **directories**: List of directory-based inclusion/exclusion rules

Directory rules are evaluated in order, with the last matching rule determining inclusion/exclusion. Empty directories list means include everything.

- **files**: List of filename-glob inclusion/exclusion rules, each with a **pattern** and an **action** (`include` or `exclude`)

Filename-pattern rules exist to drop machine-generated sources -- Qt `moc` output, protobuf stubs, and the output of other code generators -- from the compilation database, so that linters and editors act only on hand-written code. This is off by default: with no `files` rules configured, the output is unchanged. A pattern with no path separator (`/`, or `\` on Windows) matches the source file's basename; a pattern containing a separator matches the full source path as it appears in the entry, so patterns should use the same path format as configured in `format.paths.file`. File-pattern rules are evaluated in order, with the last matching rule determining inclusion/exclusion, the same precedence the directory rules use; a file matched by no pattern rule is included. Directory rules and file-pattern rules compose: an entry is emitted only when both accept it. An invalid pattern is rejected during configuration validation with an error identifying the offending rule.

A common recipe for a Qt/protobuf project:

```yaml
sources:
  files:
    - pattern: "moc_*.cpp"
      action: exclude
    - pattern: "*.pb.cc"
      action: exclude
    - pattern: "*.pb.h"
      action: exclude
```

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

### headers

Editors and linters often need compile flags for header files, not only for the translation units that were compiled. When enabled, Bear synthesizes a compilation-database entry for a header file by cloning the arguments of a compiled C/C++/Objective-C translation unit, with the source path replaced by the header path and the output flag removed (the synthesized entry has no `output` field). This is off by default: with `enabled: false` the output is unchanged.

- **enabled**: Turn header-entry synthesis on or off. Default `false`.
- **strategy**: How header files are discovered and which translation unit donates the flags. One of:
  - `siblings` (default): for each directory that contains a compiled source, header files in that same directory receive an entry cloned from a same-directory source. Zero prerequisites, but the flags are approximate (a header gets its directory sibling's flags), and it produces nothing for headers in directories that hold no compiled source -- notably the split `include/` + `src/` layout, for which `dependency-files` is the answer.
  - `dependency-files`: reads the make-style dependency file (`.d`) the build already emitted (for example via `-MMD`/`-MF`) and synthesizes an entry for each header prerequisite it lists that resolves inside the compilation's working directory. This is the most accurate option for the headers a compilation actually included -- it reaches headers in other directories (a split `include/`+`src/` layout) precisely -- but it requires the build to have emitted dependency files and for them to still be on disk.

Which file extensions count as headers is fixed (the built-in C-family header set) and is not configurable. Only C, C++, and Objective-C translation units are eligible donors. Synthesized entries pass through duplicate detection and validation like any other entry, so a header that already has a real compilation entry is not duplicated. Entries recorded in `command`-string form (rather than the default `arguments` array) cannot donate and are skipped.

A recipe for a project that keeps headers under `include/` and sources under `src/`, whose build emits dependency files:

```yaml
headers:
  enabled: true
  strategy: dependency-files
```

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

## Swift Projects

Bear records `swiftc` invocations. Unlike `valac`, a whole-module
`swiftc` invocation that names several `.swift` sources produces one
entry PER source, not one combined entry -- and every one of those
entries carries the COMPLETE invocation's arguments (every source in
the module, not just its own). This matches the shape CMake's own Swift
support emits and that SourceKit-LSP already consumes: per-file tooling
looks up a compile command by file path, and whole-module compilation
means each file's semantics genuinely depend on every other source in
the invocation, so no entry can be reduced to "this file only". A
larger whole-module invocation therefore produces a database with more
duplicated argument data than a comparable GCC/Clang build; this is
expected, not a bug.

The internal per-file `swift-frontend` jobs that `swiftc` spawns (and a
legacy toolchain's `swiftc -frontend` self-invocation) are filtered out
automatically and produce no entries -- only the user-facing `swiftc`
driver invocation is recorded.

`-index-store-path` is never injected by Bear; if the build already
passes it, it is recorded like any other flag, which benefits
SourceKit-LSP's cross-file indexing, but Bear itself does not add it.

On macOS, Xcode's `swiftc` is Apple-signed, so System Integrity
Protection blocks `DYLD_INSERT_LIBRARIES`; Bear's wrapper interception
mode applies there, the same as for any other Apple-signed compiler.

## Getting Help

There could be many reasons for any of these failures. When seeking help:

1. **Always include debug logs** (`RUST_LOG=debug`) in your report
2. Consult the project wiki page for known problems
3. Search existing issues before opening a new bug report
4. Follow the bug report template, provide the requested fields

# COPYRIGHT

Copyright (C) 2012-2026 by László Nagy
<https://github.com/rizsotto/Bear>
