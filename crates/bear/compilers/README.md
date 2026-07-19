# Compiler Definitions

This directory contains YAML files that define how Bear recognizes compiler
executables, categorizes their command-line flags, and filters internal
invocations. Each file corresponds to one compiler (or compiler family).

At build time, `crates/bear/build.rs` reads these files and generates static Rust
arrays for flag tables, ignore filters, and recognition patterns. The generated
code is included in the interpreter and recognition modules via `include!()`.

## File structure

```yaml
# Optional: inherit all flags from another file (by filename stem)
extends: gcc

# Required: maps to a CompilerType variant (gcc, clang, flang, cuda, intel_fortran, cray_fortran)
type: gcc

# Executable names this compiler is known by
recognize:
  - description: "GCC" # required: short human label, shown by --print-compilers
    references:                 # required: non-empty list of http(s) doc URLs (validate-only)
      - "https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html"
    executables: ["gcc", "g++", "gfortran"]
    versioned: true             # match with version suffix (e.g., gcc-11, gcc11)
    cross_compilation: true     # match with cross-compilation prefix (e.g., arm-linux-gnu-gcc)

# Optional: treat '/'-prefixed arguments as flags (default: false)
# When true, arguments like /Fo, /c, /I are treated as compiler flags.
# When false (default), only '-'-prefixed arguments are treated as flags.
# Inherited from base file via `extends` if not specified.
slash_prefix: false

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
| `-flag:*`     | `/std:*`        | Exact match, value after `:`                   |
| `-flag{:}*`   | `/Fe{:}*`       | Exact match, value after `:` or as separate arg|

The `{}` pair means the separator is optional:
- `{ }` -- the space between flag and value is optional (value can be glued: `-Dfoo` or separate: `-D foo`)
- `{=}` -- the `=` between flag and value is optional (value can follow `=`: `--std=c99` or be separate: `--std c99`)
- `{:}` -- the `:` between flag and value is optional (value can follow `:`: `/std:c++20` or be separate: `/std c++20`)

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
| `pass_through`              | Stop parsing; remaining args go to linker   |
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

## Recognition patterns

The `recognize` section defines which executable names this compiler is known by.
Each entry specifies:

- `executables` -- list of base executable names (e.g., `["gcc", "g++"]`)
- `cross_compilation` -- if `true`, also matches names with a cross-compilation
  prefix (e.g., `arm-linux-gnueabihf-gcc`)
- `versioned` -- if `true`, also matches names with a version suffix
  (e.g., `gcc-11`, `gcc11`, `gcc-11.2`)
- `description` -- required short human label (e.g., `"GCC"`), emitted
  into the generated recognition table and shown by `bear semantic --print-compilers`
- `references` -- required non-empty list of http(s) documentation URLs; validated
  at codegen time but never emitted into generated code

Both `description` and `references` are mandatory: the build fails if either is
missing, `description` is blank, or `references` holds a non-http(s) entry.

All patterns automatically handle `.exe` extensions on Windows.

Executables listed in `ignore_when.executables` are automatically added as
recognition entries with `cross_compilation: false, versioned: false`. This
ensures the recognizer routes them to the right compiler type, where the
interpreter then ignores them. You do not need to list them under `recognize`.

## Environment variables

The optional `environment` section declares environment variables that the
compiler binary reads and how their values map to command-line arguments.

```yaml
environment:
  - variable: CPATH
    effect: configures_preprocessing
    mapping:
      flag: "-I"
      separator: path

  - variable: CL
    effect: configures_compiling
    mapping:
      expand: prepend
      separator: space
```

Each entry has:

- `variable` -- the environment variable name (must match `[A-Za-z_][A-Za-z0-9_]*`)
- `effect` -- semantic effect (same vocabulary as `result` in flags)
- `mapping` -- how the value translates to arguments

### Mapping types

| Type | Fields | Behavior |
|------|--------|----------|
| Flag | `flag` + `separator` | Split value by separator, emit `<flag> <entry>` per element |
| Expand | `expand` + `separator: space` | Shell-split value, insert as raw arguments |

### Separators

| Value | Meaning |
|-------|---------|
| `path` | Platform path separator (`:` on Unix, `;` on Windows) |
| `";"` | Fixed semicolon separator |
| `space` | POSIX shell-word splitting (used with `expand`) |

### Expand positions

| Value | Meaning |
|-------|---------|
| `prepend` | Insert before command-line arguments (e.g., MSVC `CL`) |
| `append` | Insert after command-line arguments (e.g., MSVC `_CL_`) |

### Documentary entries

Variables the compiler reads but Bear cannot parse (e.g., config file paths)
can be listed with `effect: none`:

```yaml
  - variable: ICXCFG
    effect: none
    note: "Config file - not parsed"
    mapping:
      separator: space
```

These are skipped during code generation but document the variable for
future contributors.

### Environment inheritance

Environment variables follow the `extends` chain transitively. If
`armclang.yaml` extends `clang.yaml` which extends `gcc.yaml`, armclang
inherits all GCC and Clang environment entries. Own entries override
inherited ones matched by variable name.

Compilers that do not read GCC variables (e.g., NVIDIA HPC SDK) must not
extend GCC and will have an empty environment table.

## Per-interpreter properties set outside the YAML

Most compiler behaviour is data-driven from these YAML files, but a few
properties are consumed *after* parsing (at the converter) rather than at
parse time, so they live in the interpreter factory functions in
`crates/bear/src/semantic/interpreters/compilers/flag_based.rs`, not in
the YAML schema:

- `source_mode` (a `SourceMode` enum: `PerSourceStripped` / `PerSourceFull`
  / `Combined`) -- how an invocation's sources map to compilation database
  entries. `PerSourceStripped` (the default) is for GCC/Clang/etc.: each
  source is a separable translation unit, and the converter emits one
  entry per source, stripping sibling sources from each entry's arguments.
  `Combined` is for a single-translation-unit compiler like `valac`, which
  compiles all of a target's sources together and produces one output, so
  the converter emits exactly one combined entry per invocation (`file` is
  the first source, every source retained). `PerSourceFull` is for a
  whole-module compiler like `swiftc`: every source is analyzed together,
  but per-file consuming tooling (SourceKit-LSP) looks up a compile command
  by file path, so the converter emits one entry per source while every
  entry keeps the complete invocation (no sibling stripping). Promote this
  to the YAML/codegen path only if a compiler needs a mode that varies by
  configuration rather than being fixed per family.

## Adding a new compiler

1. Create a new YAML file in this directory (e.g., `mycompiler.yaml`)
2. Add `type:`, `recognize:`, `flags:` entries and optionally `extends:`, `ignore_when:`, `environment:`
3. Add a `TableConfig` entry in `build-support/compilers-codegen/src/tables.rs`
4. Add a `CompilerType` variant in `crates/bear/src/config/types.rs` and a mapping in
   `crates/bear/src/semantic/interpreters/compilers/compiler_recognition.rs::parse_compiler_type`
5. Add a constructor in `flag_based.rs` and register it in
   `CompilerInterpreter::new_with_config`
   (`crates/bear/src/semantic/interpreters/compilers/mod.rs`)
6. Run `cargo build && cargo test`

## Adding a new flag

1. Find the right YAML file for the compiler
2. Add an entry under `flags:` with the appropriate `match` pattern and `result`
3. Run `cargo build` -- the build script regenerates the flag tables automatically
4. Run `cargo test` -- invariant tests verify sorting, no invalid kinds, etc.
