# Compiler Flag Definitions

This directory contains YAML files that define how Bear recognizes and categorizes
compiler command-line flags. Each file corresponds to one compiler (or compiler family).

At build time, `bear/build.rs` reads these files and generates static Rust arrays
of `FlagRule` values and ignore filter arrays. The generated code is included in
the `flag_based` interpreter module via `include!()`.

## File structure

```yaml
# Optional: inherit all flags from another file (by filename stem)
extends: gcc

# Optional: conditions under which a recognized invocation should be ignored
ignore_when:
  # Ignore if the executable filename matches any of these
  executables: ["cc1", "cc1plus", "f951"]
  # Ignore if any argument matches any of these flags
  flags: ["-cc1"]

flags:
  - match: {pattern: "-o{ }*"}
    result: output
  - match: {pattern: "-c"}
    result: stops_at_compiling
  - match: {pattern: "-I{ }*"}
    result: configures_preprocessing
```

## Pattern syntax

The `pattern` string encodes both the flag name and how it consumes arguments:

| Syntax        | Example         | Meaning                                        |
|---------------|-----------------|------------------------------------------------|
| `-flag`       | `-c`            | Exact match, no additional arguments           |
| `-flag` + count | `-x` count: 1 | Exact match with N separate arguments          |
| `-flag*`      | `-W*`           | Prefix match (anything starting with `-W`)     |
| `-flag*` + count | `-Xarch*` count: 1 | Prefix match with N separate arguments   |
| `-flag{ }*`   | `-D{ }*`        | Exact match, value glued or as separate arg    |
| `-flag=*`     | `-specs=*`      | Exact match, value after `=`                   |
| `-flag{=}*`   | `--std{=}*`     | Exact match, value after `=` or as separate arg|

The `{}` pair means the separator is optional:
- `{ }` -- the space between flag and value is optional (value can be glued: `-Dfoo` or separate: `-D foo`)
- `{=}` -- the `=` between flag and value is optional (value can follow `=`: `--std=c99` or be separate: `--std c99`)

## Result values

The `result` field describes what the flag means semantically:

| Value                       | Meaning                                     |
|-----------------------------|---------------------------------------------|
| `output`                    | Output file specification                   |
| `configures_preprocessing`  | Affects the preprocessing pass              |
| `configures_compiling`      | Affects the compilation pass                |
| `configures_assembling`     | Affects the assembly pass                   |
| `configures_linking`        | Affects the linking pass                    |
| `stops_at_preprocessing`    | Stop compilation after preprocessing        |
| `stops_at_compiling`        | Stop compilation after compiling            |
| `stops_at_assembling`       | Stop compilation after assembling           |
| `info_and_exit`             | Print info and exit (e.g. `--version`)      |
| `driver_option`             | Driver/toolchain behavior flag              |
| `none`                      | No specific semantic effect                 |

## Ignore filters

The optional `ignore_when` section specifies conditions under which a recognized
compiler invocation should be treated as an internal/ignored command rather than
a user-facing compilation:

- `executables` -- list of executable filenames (not paths). If the invoked
  executable's filename matches any entry, the command is ignored. Used by GCC
  to skip internal executables like `cc1`, `collect2`, etc.
- `flags` -- list of argument strings. If any argument in the invocation matches
  any entry, the command is ignored. Used by Clang to skip `-cc1` frontend
  invocations.

Both fields are optional and default to empty. When a file uses `extends`, the
ignore filters are inherited from the base file only if the extending file does
not define its own list for that field (i.e., own values take precedence per field,
not per entry).

## Inheritance

Files with `extends: gcc` inherit all GCC flags and (unless overridden) ignore
filters. The build script concatenates own flags before base flags, then sorts
all entries by flag name length (longest first) so more specific flags match
before shorter prefixes. The sort is stable, so own flags take priority over
base flags of the same length.

## Adding a new compiler

1. Create a new YAML file in this directory (e.g., `mycompiler.yaml`)
2. Add `flags:` entries and optionally `extends:`, `ignore_when:`
3. Add a `TableConfig` entry in `bear/build.rs`
4. Add a `CompilerType` variant in `config.rs`
5. Register the `FlagBasedInterpreter` in `CompilerInterpreter::new_with_config`
6. Add recognition patterns in `compiler_recognition.rs`
7. Run `cargo build && cargo test`

## Adding a new flag

1. Find the right YAML file for the compiler
2. Add an entry under `flags:` with the appropriate `match` pattern and `result`
3. Run `cargo build` -- the build script regenerates the flag tables automatically
4. Run `cargo test` -- invariant tests verify sorting, no invalid kinds, etc.
