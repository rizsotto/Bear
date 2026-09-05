% BEAR(1) Bear User Manuals
% László Nagy
% September 5, 2026
<!-- to generate the final `bear.1` file, run `pandoc -s -t man bear.1.md -o bear.1` -->

# NAME

bear - generate a compilation database for Clang tooling

# SYNOPSIS

**bear** [*OPTIONS*] \-\- *BUILD_COMMAND*...

**bear intercept** [*OPTIONS*] \-\-output *FILE* \-\- *BUILD_COMMAND*...

**bear semantic** [*OPTIONS*]

**bear parse-sh** [*OPTIONS*]


# DESCRIPTION

Bear produces a JSON compilation database (`compile_commands.json`).
Clang-based tools such as clangd and clang-tidy read this file to learn
how each source file of a project is compiled: with which compiler,
which flags, from which directory. Build systems know this, but most do
not export it; Bear recovers it from the build itself.

Bear works in two stages. First it captures the commands a build
executes, and writes them as a stream of execution events. The capture
either observes a real build run, or parses the text of a dry run. Then
semantic analysis recognizes the compiler invocations among those
events, parses their command lines, and writes the compilation
database.

## Modes

**Combined mode** (default)
: `bear -- <build command>` runs the build, captures and analyzes in one
step, and writes the database. This is the recommended way to use Bear:
it observes the commands the build actually executes, so the result is
exact. Use it whenever you can run the build.

**Intercept mode**
: `bear intercept` runs the build but only captures, writing the raw
event stream to a file. Useful when the build is expensive and you want
to run it once and process the events several times, for example to
try different output configurations.

**Semantic mode**
: `bear semantic` runs no build: it is a filter that reads an event
stream (from `bear intercept`, or any producer of the documented event
format) and writes the compilation database. Combine it with intercept
mode to re-generate the database without rebuilding.

**Parse-sh mode**
: `bear parse-sh` produces the compilation database from shell command
text, without executing anything. Typical input is a build system's dry
run (`make -n`) or a saved build log. It is a text parser and inherently
lower fidelity than interception; reach for it when the build cannot be
run, for example to recover a database from a CI log. See its
limitations under COMMANDS.

## Interception methods

Interception itself has two methods, selectable in the configuration
file. **Preload** (the default on Linux and the BSDs) injects a small
library into every process the build starts and observes its `exec()`
calls. **Wrapper** (the default on macOS and Windows) substitutes the
known compilers with a reporting wrapper executable. Preload is
transparent to the build but cannot observe statically linked
executables, and it is unavailable on Windows and on macOS while
System Integrity Protection is enabled; forcing it there is a startup
error that names wrapper mode, not a silent fallback. Wrapper mode
works on every platform, but the build has to pick the wrapper up: a
"configure" step that discovers compilers must itself run under Bear
(see TROUBLESHOOTING).

## OPTIONS

**-c, \-\-config** *FILE*
: Path of the configuration file. It controls the interception method,
compiler recognition, source filtering, duplicate handling, and output
formatting (see CONFIGURATION). Given before a subcommand it applies to
that subcommand too.

**-o, \-\-output** *FILE*
: Path of the compilation database to write (default:
`compile_commands.json`). `-` is not accepted here, because in combined
mode standard output belongs to the build. To stream the database, use
`bear semantic --output -` (see COMMANDS).

**-a, \-\-append**
: Append to an existing output file instead of overwriting it. New
entries are placed before the existing ones, so a rebuilt file's newest
invocation survives duplicate filtering and replaces the stale entry
(see `duplicates` under CONFIGURATION).

**\-\-overwrite**
: Replace the output file with only this run's entries. This is what
Bear already does when neither flag is given; the flag exists so a
script can name the behaviour it depends on rather than rely on the
default. Giving it together with `--append` is an error. A future
release makes appending the default, and a script that passes
`--overwrite` today keeps its current behaviour across that change.

**-h, \-\-help**
: Print help.

**-V, \-\-version**
: Print version.


# COMMANDS

## bear intercept

Runs the build and captures execution events to a file, without
analyzing them.

**bear intercept** [*OPTIONS*] \-\-output *FILE* \-\- *BUILD_COMMAND*...

**-o, \-\-output** *FILE*
: Path of the event file to write. Required, and `-` is not accepted:
the build owns standard output, and there is no default event-file name.

## bear semantic

Reads an event stream and writes the compilation database. It runs no
build, so it behaves as a plain filter: diagnostics go to standard
error, and both ends can be standard streams.

**bear semantic** [*OPTIONS*]

**-i, \-\-input** *FILE*
: Path of the event file to read (default: `-`, standard input). Any
producer of a conforming event stream can pipe into it. When standard
input is a terminal, or the stream turns out to be empty, a notice is
printed on standard error.

**-o, \-\-output** *FILE*
: Path of the compilation database to write (default:
`compile_commands.json`). Pass `-` to write to standard output; that
write is neither atomic nor appendable, so `--output -` together with
`--append` is rejected.

**-a, \-\-append**
: Same as in combined mode: place new entries before the existing ones
in the output file.

**\-\-overwrite**
: Same as in combined mode: name the current default explicitly. Not
accepted together with `--append`.

## bear parse-sh

Produces the compilation database from shell command text, without
running anything. Typical input is a build system's dry-run output, such
as `make -n`, or a saved build log:

    make -n | bear parse-sh

**bear parse-sh** [*OPTIONS*]

**-i, \-\-input** *FILE*
: Path of the shell text to parse (default: `-`, standard input).

**-o, \-\-output** *FILE*
: Path of the compilation database to write (default:
`compile_commands.json`). Pass `-` to write to standard output; that
write is neither atomic nor appendable, so `--output -` together with
`--append` is rejected.

**-a, \-\-append**
: Same as in combined mode: place new entries before the existing ones
in the output file.

**\-\-overwrite**
: Same as in combined mode: name the current default explicitly. Not
accepted together with `--append`.

**-C, \-\-directory** *DIR*
: Initial working directory for the parsed commands. Use it for input
captured elsewhere (a CI log, a dry run from another checkout). Give an
absolute path; the directory need not exist on this machine.

The commands recognized in the text go through the same semantic
analysis and output stage as an intercepted build, so the configuration
applies exactly as in combined mode. Parsed commands that are not
compiler invocations (`ar`, `mv`, `mkdir`) simply produce no entries.

It understands a documented subset of POSIX shell syntax: word splitting
and quoting; the `;`, `&&`, `||`, `&`, and `|` separators; comments;
redirections; `cd`; brace groups (`{ ...; }`, whose `cd` persists past
the closing brace); and make's `Entering directory` / `Leaving
directory` markers, which track the working directory across a
recursive build. Sub-makes print these markers by default; run
`make -nw` to have the top-level make print them too, so the log itself
names the build root. A line using anything outside that subset
is skipped and reported on standard error with its line number and
reason. This covers subshells, command substitution, parameter
expansion, globs in the executable position, here-documents,
unterminated quotes, and shell keywords such as `if` or `for`. The run
succeeds as long as at least one line parsed as a command.

Interception remains the higher-fidelity default: it observes the
`exec()` calls a build really makes, while `bear parse-sh` reconstructs
approximate events from text the build system chose to print. A dry run
can omit commands (recursive make does not always propagate `-n`, and
commands behind not-yet-generated sources never print); the environment
and `PATH` used to resolve bare executable names are the parse-time
ones, not the real build's; and only POSIX `sh` text is supported.
Prefer `bear -- <build command>` whenever the build can actually be run.

The source files named in the parsed commands need not exist: entries
are reconstructed from the text alone. The one exception is the
`canonical` path format, which resolves symlinks on disk; when the
sources are absent, configure `absolute` instead (see `format.paths`
under CONFIGURATION).


# OUTPUT

Bear writes a JSON compilation database conforming to the Clang JSON
Compilation Database specification (see SEE ALSO): a JSON array of entry
objects with the following fields.

**directory**
: The working directory of the compilation.

**file**
: The main translation unit source file.

**arguments**
: The compilation command as an array of strings. This is the default;
Bear prefers it over `command` because it avoids shell-escaping issues.

**command**
: The compilation command as a single shell-escaped string (written
instead of `arguments` when `format.entries.use_array_format` is
`false`).

**output**
: The file produced by the compilation (optional).

By default Bear does not transform paths: each entry records them as
they appeared in the intercepted invocation, so `file` and `output` are
often relative to `directory`. Use `format.paths` in the configuration
to normalize them.

Two compilers produce an entry shape worth knowing about:

- **swiftc**: a whole-module invocation naming several `.swift` sources
produces one entry per source, each carrying the complete invocation's
arguments (every source of the module, not just its own). This matches
the shape CMake emits and SourceKit-LSP consumes: in whole-module
compilation each file's semantics depend on every other source. The
repeated argument data is expected, not a bug. The internal
`swift-frontend` jobs that `swiftc` spawns are filtered out
automatically.
- **valac**: one entry per `valac` invocation (valac compiles all of a
target's `.vala`/`.gs` sources together as one unit); the entry's `file`
is the first source and every source is kept in the command. Since valac
transpiles to C and compiles the result, the database also contains
entries for the generated C files (see TROUBLESHOOTING for the clangd
consequences).


# CONFIGURATION

Bear reads a YAML configuration file, found by search (see FILES) or
named with `--config`. Without one, built-in defaults apply. A file that
is written at all must name `schema`; every section below it is
optional. The example below shows every section:

```yaml
schema: "4.2"
intercept:
  mode: wrapper              # default: preload on Linux and the BSDs
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
  match_on:                  # default: directory, file
    - file
    - arguments
format:
  paths:
    directory: canonical     # default: as-is
    file: canonical          # default: as-is
  entries:
    use_array_format: true
    include_output_field: true
  arguments:
    from_response_files: false
    from_environment: true
headers:
  enabled: true               # default: false
  strategy: siblings
```

Every value above is illustrative; the non-default ones are annotated
inline. Each section below states its actual defaults.

## intercept

Controls the interception method (see DESCRIPTION for the trade-offs):

- **mode**: `preload` (default on Linux and the BSDs) or `wrapper`
(default on macOS and Windows; works everywhere).

## compilers

Hints for compiler recognition. Bear recognizes GCC- and Clang-compatible
drivers (including cross-compiler prefixed names), the common vendor
compiler drivers and standalone assemblers, `swiftc` and `valac`,
compiler launchers, and MPI compiler wrappers. Run
`bear semantic --print-compilers` for the full list of recognized
compiler names, each with the `as` value it maps to.

A few recognition behaviors are worth knowing. Compiler launchers
(`ccache`, `distcc`, `sccache`, `icecc`) are dropped from the recorded
command: `ccache gcc -c main.c` produces an entry for `gcc -c main.c`.
MPI compiler wrappers are recorded as invoked, not expanded to the
compiler they wrap; Clang tooling that needs the wrapper's baked-in
include paths can point at it directly, for example with clangd's
`--query-driver`. C++20 module-interface units (`.cppm`, `.ixx`, and
similar) are recognized as sources, while precompiled `.pcm` artifacts
never appear as an entry's `file`.

In wrapper mode this section also decides what gets wrapped. With no
entry other than `ignore` ones, Bear scans PATH at startup and wraps
every compiler it recognizes. As soon as one entry is not ignored, Bear
wraps exactly the listed paths and skips the scan, so listing one
unusual compiler stops `gcc` and `clang` from being intercepted unless
they are listed too. Compilers named in the compiler environment
variables (`CC`, `CXX`, and similar) are wrapped either way. Preload
mode is unaffected: it intercepts every executed program regardless of
this section.

Use this section when a compiler at a given path is not recognized, or
is recognized wrongly:

- **path**: Path of the compiler executable. Required.
- **as**: Compiler type hint for semantic analysis. Optional; when
omitted, Bear guesses the family from the executable's filename and
falls back to `gcc` when nothing matches. The accepted values are the
`as` names shown by `bear semantic --print-compilers`, plus `wrapper`
for a compiler launcher.
- **ignore**: Exclude this executable's invocations from the database.
Optional, default `false`.

The generic names `cc`, `c++`, and the HPE Cray PrgEnv wrapper `CC` are
classified by probing the executable's `--version` output, since the
same basename can be a different compiler depending on the platform or
the loaded environment module. When the probe cannot classify the
executable, override it:

```yaml
compilers:
  - path: /usr/bin/cc
    as: clang
```

## sources

Filters entries by source file location and filename.

- **directories**: List of directory-based rules, each with a **path**
and an **action** (`include` or `exclude`).
- **files**: List of filename-glob rules, each with a **pattern** and an
**action** (`include` or `exclude`).

Rules of each list are evaluated in order, the last matching rule wins,
and an entry matched by no rule is included; an empty list includes
everything. The two lists compose: an entry is emitted only when both
accept it. A file pattern without a path separator matches the source
file's basename; a pattern containing a separator matches the full
source path as it appears in the entry, so use the same path format as
configured in `format.paths.file`. An invalid pattern is rejected during
configuration validation with an error naming the offending rule.
Typical use for file patterns is dropping machine-generated sources (Qt
`moc` output, protobuf stubs) so that linters and editors act only on
hand-written code; see EXAMPLES.

## duplicates

Controls duplicate detection.

- **match_on**: List of entry fields to compare (`file`, `arguments`,
`directory`, `command`, `output`). Two entries are duplicates when all
of the configured fields match; the first occurrence is kept. The
default is `directory` and `file`, so one entry is kept per source file
per directory, regardless of arguments; when a build compiles the same
file with different flags, add `arguments` to keep an entry per
configuration. Combined with `--append` (new entries first), the default
means a rebuilt file's newest invocation replaces its stale entry.

## format

Controls output formatting.

- **paths.directory**, **paths.file**: How to format these fields'
paths: `as-is` (default, no transformation), `canonical` (resolve
symlinks; requires the path to exist), `relative` (to the `directory`
field), or `absolute`.
- **entries.use_array_format**: Use the `arguments` array (default)
instead of the `command` string. Set to `false` for consumers that read
only the `command` field.
- **entries.include_output_field**: Include the `output` field in
entries.
- **arguments.from_response_files**: Replace `@file` response-file
references with the file's tokenized contents (resolved relative to the
compilation's working directory, expanded recursively, using the
compiler's quoting convention). Disabled by default, in which case an
`@file` argument is recorded verbatim; a missing or unreadable file is
left literal with a warning.
- **arguments.from_environment**: Fold compiler environment variables
that act as implicit flags into the arguments. GCC/Clang header-search
paths (`CPATH`, `C_INCLUDE_PATH`, `CPLUS_INCLUDE_PATH`,
`OBJC_INCLUDE_PATH`) become include flags, and MSVC's `CL` / `_CL_`
become leading / trailing options. Enabled by default; set to `false` to
record only the command-line flags. (Unrelated to the `CC="gcc -std=c11"`
convention handled during interception; see TROUBLESHOOTING.)

## headers

Editors and linters often need compile flags for header files, not only
for the translation units that were compiled. When enabled, Bear
synthesizes an entry for a header by cloning the arguments of a compiled
C/C++/Objective-C translation unit, with the source path replaced by the
header path and the output flag removed. Off by default.

- **enabled**: Turn header-entry synthesis on or off. Default `false`.
- **strategy**: How headers are discovered and which translation unit
donates the flags:
  - `siblings` (default): headers receive an entry cloned from a
compiled source in the same directory. Zero prerequisites, but the flags
are approximate, and headers in directories without compiled sources
(the split `include/` + `src/` layout) get nothing.
  - `dependency-files`: reads the make-style `.d` dependency files the
build already emitted (for example via `-MMD`) and synthesizes an entry
for each header prerequisite that resolves inside the compilation's
working directory. Accurate, and reaches headers in other directories,
but requires the build to have left dependency files on disk.

The header extension set is fixed (the built-in C-family headers).
Synthesized entries pass through duplicate detection like any other
entry, so a header with a real compilation entry is not duplicated.


# EXIT STATUS

In combined and intercept mode, Bear exits with the build command's exit
code: 0 when the build succeeds, the same non-zero code when it fails.

In semantic mode, Bear exits 0 on success and non-zero when the analysis
fails.

In parse-sh mode, Bear exits 0 when the input could be parsed - even
when the database comes out empty because the text named no compiler
invocations - and also on empty input (with a notice on standard
error). It exits non-zero when every non-empty input line was skipped,
so a run that understood nothing cannot pass for a successful one.

If Bear itself encounters an internal error, it exits non-zero
regardless of the build command's status.


# ENVIRONMENT

**RUST_LOG**
: Selects the diagnostic format and verbosity on standard error. When
unset, Bear uses the UNIX style, `bear: message` (and `wrapper:` or
`preload:` from its helper processes), showing warnings and errors only.
When set to a level (`error`, `warn`, `info`, or `debug`, in increasing
verbosity), Bear switches to a developer format that tags every line with
a timestamp, the level, the emitting process and its pid, and the source
location. Debug output is essential for troubleshooting; see
TROUBLESHOOTING.


# FILES

The configuration file `bear.yml` is searched in the following
locations, in order; the first file found is loaded:

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


# EXAMPLES

Generate a database for a Make project:

    bear -- make

For a CMake project in preload mode, only the build needs Bear:

    cmake -B build
    bear -- cmake --build build

In wrapper mode the configure step must also run under Bear, so it
discovers the wrapper as the compiler; discard that run's output:

    bear -- cmake -B build
    bear -- cmake --build build

Capture once, analyze many times. This allows trying output
configurations without rebuilding:

    bear intercept --output events.json -- make
    bear semantic --input events.json
    bear --config strict.yml semantic --input events.json --output strict.json

Recover a database from a dry run, without building:

    make -n | bear parse-sh

For a recursive Make build, add `-w`. The top-level make then prints
`Entering directory` markers too, and parse-sh resolves every command
against the right directory:

    make -nw | bear parse-sh

Update the database after rebuilding one part of the project:

    bear --append -- make -C src/module

Drop generated sources (Qt moc output, protobuf stubs) with a `bear.yml`:

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

Synthesize entries for headers in a split `include/` + `src/` layout,
when the build emits `.d` dependency files:

```yaml
headers:
  enabled: true
  strategy: dependency-files
```


# TROUBLESHOOTING

## Debug logging

Before reporting any issue, run Bear with debug logging enabled and
include the output in the report:

    RUST_LOG=debug bear -- <build command>

## Empty output

The most common cause is that the build executed no compiler: an
incremental build with everything up to date runs nothing. Bear
intercepts executed commands only; it does not read the build files.
Run a clean build.

In wrapper mode, a "configure" step that detects compilers captures the
real compiler path before Bear can substitute the wrapper. Run the
configure step under Bear too (discarding that output), then run the
build under Bear.

## Compiler variables with flags

In wrapper mode Bear accepts the GNU Make convention of a compiler
variable carrying a trailing flag or two (`CC="gcc -std=c11" make`): it
splits the value on whitespace, resolves the first token as the
compiler, and rewrites the variable so the build still sees the flags.
This is a Unix / GNU Make convention; native Windows build systems
(MSBuild, `nmake`) do not consume `CC`/`CXX` from the environment. For
anything beyond simple whitespace-separated tokens (quoting,
metacharacters, substitutions), use `CFLAGS` / `CXXFLAGS` / `LDFLAGS`
instead; Bear passes those through untouched.

## Preload errors in cross-compilation

An error like `version 'GLIBC_2.33' not found (required by
.../libexec.so)` means Bear's preload library was built against a newer
glibc than the one the SDK's compilers load: the library must be
ABI-compatible with the libc of the intercepted process, not only with
the host's. Use a Bear build linked against a glibc no newer than the
SDK's. The project wiki's Troubleshooting page lists diagnostic
commands.

## Language servers on Vala projects

vala-language-server reads the `command` field and ignores the
`arguments` array; build the database with
`format.entries.use_array_format: false`. In a mixed C/Vala project,
clangd also indexes the `valac` entries and emits unknown-argument
noise on them; suppress it with a `.clangd` file:

```yaml
If:
  PathMatch: .*\.vala
Diagnostics:
  Suppress: '*'
```

## Getting help

Consult the documentation site for known problems and search existing
issues before opening a new report. Follow the bug report template, and
always include the `RUST_LOG=debug` output.


# SEE ALSO

**clangd**(1), **clang-tidy**(1), **make**(1)

Documentation site, with task pages for build systems, platforms, and
toolchains: <https://rizsotto.github.io/Bear/>

The Clang JSON Compilation Database specification:
<https://clang.llvm.org/docs/JSONCompilationDatabase.html>

Project homepage and issue tracker:
<https://github.com/rizsotto/Bear>

# COPYRIGHT

Copyright (C) 2012-2026 by László Nagy
<https://github.com/rizsotto/Bear>
