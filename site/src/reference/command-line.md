<!-- Diataxis type: reference -->

# Command-line options

`bear` runs in four modes, selected by the subcommand or its absence:
combined (`bear`, no subcommand, the default), `bear intercept`, `bear
semantic`, and `bear parse-sh`. Options given before `--` belong to Bear;
everything after `--` is the build command, passed through untouched.

- **Combined** runs the build, captures it, and writes the compilation
  database in one step. It is the default and the recommended way to run
  Bear.
- **`bear intercept`** runs the build and only captures it, writing the
  raw event stream to a file for later analysis.
- **`bear semantic`** reads an event stream and writes the compilation
  database, without running a build.
- **`bear parse-sh`** writes the compilation database from shell command
  text, such as a build's dry-run output or a saved build log, without
  running anything.

See [How Bear works](../understanding/how-it-works.md) for the mechanism
behind capture and analysis, [Configure Bear](configuration.md) for every
key the `-c`/`--config` file accepts, and [Exit status](exit-status.md)
for what Bear's own exit code means in each mode.

<!-- BEGIN GENERATED: scripts/generate-command-line-reference.py -- DO NOT EDIT. Edits inside this block are lost the next time the script runs. -->

## Global usage

```
bear [OPTIONS] -- <BUILD_COMMAND>...
bear [OPTIONS] <COMMAND>
```

Bear is a tool that generates a compilation database for clang tooling.

### Subcommands

| Command | Description |
|---|---|
| `intercept` | intercepts command execution |
| `semantic` | detect semantics of command executions |
| `parse-sh` | produces the compilation database from build-system dry-run text (e.g. `make -n` output) |
| `help` | Print this message or the help of the given subcommand(s) |

### Arguments

| Argument | Description |
|---|---|
| `<BUILD_COMMAND>...` | Build command |

### Options

| Flag | Description | Default |
|---|---|---|
| `-c, --config <FILE>` | Path of the config file | - |
| `-o, --output <FILE>` | Path of the result file | `compile_commands.json` |
| `-a, --append` | Append result to an existing output file | - |
| `-h, --help` | Print help | - |
| `-V, --version` | Print version | - |

## bear intercept

intercepts command execution

```
bear intercept --output <FILE> -- <BUILD_COMMAND>...
```

### Arguments

| Argument | Description |
|---|---|
| `<BUILD_COMMAND>...` | Build command |

### Options

| Flag | Description | Default |
|---|---|---|
| `-o, --output <FILE>` | Path of the event file to write | - |
| `-h, --help` | Print help | - |

## bear semantic

detect semantics of command executions

```
bear semantic [OPTIONS]
```

### Options

| Flag | Description | Default |
|---|---|---|
| `-i, --input <FILE>` | Path of the event file to read | `-` |
| `-o, --output <FILE>` | Path of the result file | `compile_commands.json` |
| `-a, --append` | Append result to an existing output file | - |
| `--print-compilers` | Print the compilers Bear recognizes and exit | - |
| `-h, --help` | Print help | - |

Reads the event stream that `bear intercept` (or a third-party producer) writes and produces the compilation database, e.g.:

```sh
bear semantic --input events.json
```

## bear parse-sh

produces the compilation database from build-system dry-run text (e.g. `make -n` output)

```
bear parse-sh [OPTIONS]
```

### Options

| Flag | Description | Default |
|---|---|---|
| `-i, --input <FILE>` | Path of the shell text to parse | `-` |
| `-o, --output <FILE>` | Path of the result file | `compile_commands.json` |
| `-a, --append` | Append result to an existing output file | - |
| `-C, --directory <DIR>` | Initial working directory for the parsed commands | - |
| `-h, --help` | Print help | - |

Parses the text a build system prints in dry-run mode and writes the compilation database, without running a build, e.g.:

```sh
make -n | bear parse-sh
```

<!-- END GENERATED -->
