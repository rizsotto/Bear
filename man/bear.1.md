% BEAR(1) Bear User Manuals
% László Nagy
% Jan 03, 2026
<!-- to generate the final `bear.1` file, run `pandoc -s -t man bear.1.md -o bear.1` -->

# NAME

Bear - a tool to generate compilation database for Clang tooling.

# SYNOPSIS

**bear** [*OPTIONS*] [--] [*BUILD_COMMAND*...]

**bear intercept** [*OPTIONS*] [--] *BUILD_COMMAND*...

**bear semantic** [*OPTIONS*]


# DESCRIPTION

Bear is a tool that generates a JSON compilation database for Clang tooling by intercepting command executions during the build process. The JSON compilation database is used in the Clang project to provide information about how individual compilation units were processed, enabling tools like clang-tidy, clangd, and other Clang-based analysis tools to understand your project's build configuration.

Bear operates by intercepting system calls during the build process to capture compilation commands. It supports two main interception methods: dynamic library preloading (on Unix-like systems) and wrapper executables (cross-platform). The captured commands are then filtered through semantic analysis to identify actual compiler invocations and generate the final compilation database.

Bear can operate in three modes:

- **Combined mode** (default): Runs both interception and semantic analysis in sequence
- **Intercept mode**: Only captures build events to an intermediate file
- **Semantic mode**: Processes previously captured events to generate the compilation database

## OPTIONS

**-c, --config** *FILE*
: Specify a configuration file path. The configuration file controls output formatting, compiler recognition, source filtering, and duplicate handling.

**-o, --output** *FILE*
: Specify the output file path (default: `compile_commands.json`). The output is a JSON compilation database.

**-a, --append**
: Append results to an existing output file instead of overwriting it. This allows incremental updates to the compilation database.

**-h, --help**
: Print help information.

**-V, --version**
: Print version information.


# COMMANDS

Calling bear without commands will execute the combined mode, and will intercept the
compiler calls and generate a compilation database as output.

## bear intercept

Intercepts command execution events during the build process and saves them to an events file for later processing.

**bear intercept** [*OPTIONS*] [--] *BUILD_COMMAND*...

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
schema: "4.0"
intercept:
  mode: wrapper
compilers:
  - path: /usr/bin/cc
    as: gcc
  - path: /usr/local/bin/gcc
    ignore: true
sources:
  only_existing_files: true
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
```

This example configuration file:
 sets the interception mode to `wrapper`,
 hints the `/usr/bin/cc` to be the main compiler in this project, which is the GNU compiler,
 hints to ignore the `/usr/local/bin/gcc` compilers from the project,
 disallow to include files which are not available on the filesystem,
 instructs to ignore files from `/project/tests`,
 instructs to detect duplicates based on the `file` and `arguments` fields of the output file,
 instructs to format the output to use canonical path for the `file` and `directory` fields of the output file,
 instructs to use the `arguments` over the `command` field in the output file,
 instructs to include the `output` field in the output file.

## Configuration Sections

The configuration file uses schema version `4.0` and has the following structure:

### intercept

Controls the command interception method:

- **mode**: `preload` (Unix) or `wrapper` (cross-platform)
- **path**: Path to the preload library or wrapper executable (depending on the mode)

### compilers

Contains hints about what compiler needs to be recognized and what that compiler is.

- **path**: Path to the compiler executable
- **as**: Compiler type hint for semantic analysis. Valid values are: `gcc`, `clang`, `flang`, `intel-fortran`, `cray-fortran`, `cuda`.
- **ignore**: Whether to ignore this compiler.

### sources

Filtering functionality based on the source file location.

- **only_existing_files**: Filter out non-existent source files
- **directories**: List of directory-based inclusion/exclusion rules

Directory rules are evaluated in order, with the last matching rule determining inclusion/exclusion. Empty directories list means include everything.

### duplicates

Filtering functionality based on duplicate detection. Here you can define which fields of the output file should be used in the duplicate detection.

- **match_on**: List of fields to use for duplicate detection (file, arguments, directory, command, output)

### format

Output formatting configuration:

- **paths.directory** and **paths.file**: How to format paths of these fields. The allowed values are:
  - **as-is**: No transformation,
  - **canonical**: Resolve to canonical path,
  - **relative**: Make relative to directory field,
  - **absolute**: Convert to absolute path,
- **entries.use_array_format**: Use arguments array instead of command string
- **entries.include_output_field**: Include output field in entries

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

## Getting Help

There could be many reasons for any of these failures. When seeking help:

1. **Always include debug logs** (`RUST_LOG=debug`) in your report
2. Consult the project wiki page for known problems
3. Search existing issues before opening a new bug report
4. Follow the bug report template, provide the requested fields

# COPYRIGHT

Copyright (C) 2012-2026 by László Nagy
<https://github.com/rizsotto/Bear>
