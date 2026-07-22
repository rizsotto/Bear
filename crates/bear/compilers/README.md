# Compiler Definitions

This directory contains YAML files that define how Bear recognizes compiler
executables, categorizes their command-line flags, and filters internal
invocations. Each file corresponds to one compiler (or compiler family).

At build time, `crates/bear/build.rs` reads these files and generates static Rust
arrays for flag tables, ignore filters, and recognition patterns. The generated
code is included in the interpreter and recognition modules via `include!()`.

## File structure

Every file in this directory declares two things up front: `type:`, a
closed kind enum naming which of the two schemas the rest of the file
follows (`compiler` or `wrapper`), and, for `type: compiler`, a nested
`compiler:` block holding the family's identity. `type: wrapper` files
(compiler launchers such as ccache) are a different, simpler schema --
see "Compiler-launcher (wrapper) files" below.

```yaml
# Required: closed kind enum. This file follows the compiler schema.
type: compiler

# Required for type: compiler -- the family's identity.
compiler:
  id: gcc          # Required, unique across every file in this directory
                    # (checked at codegen). This is the `extends:` target,
                    # the value emitted into RECOGNITION_PATTERNS, and the
                    # ONLY accepted config `as:` spelling for this family
                    # -- no aliases.
  extends: <base>  # Optional: another file's compiler.id to inherit
                    # flags, ignore filters, slash_prefix, and environment
                    # entries from. References an id, not a filename --
                    # file names are pure packaging. By convention a
                    # file's stem still matches its own id (see the real
                    # files in this directory), but nothing enforces that.

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

# Optional: how this family's sources map to database entries (default:
# per-source-stripped). See "Source mode and response-file syntax" below.
# Not inherited via `extends`; each family states its own.
source_mode: per-source-stripped

# Optional: response-file (@file) tokenization convention (default: gnu;
# msvc for MSVC-style families). Not inherited via `extends`.
response_file_syntax: gnu

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

Both fields are optional and default to empty. When a file's `compiler.extends`
is set, the ignore filters are inherited from the base file only if the extending
file does not define its own list for that field (i.e., own values take precedence
per field, not per entry).

## Inheritance

Files with `compiler.extends: gcc` inherit all GCC flags and (unless overridden)
ignore filters. The build script concatenates own flags before base flags, then sorts
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

## Compiler-launcher (wrapper) files

A compiler launcher (`ccache`, `distcc`, `sccache`, `icecc`) is not a compiler:
it carries the real compiler in its own argv (`ccache gcc -c main.c`), and Bear
records the invocation as the real compiler's, not the launcher's. Dispatch for
every launcher is uniform (one runtime `CompilerType::Wrapper`), so a launcher
file needs no family identity of its own -- no `compiler:` block, no `id`, no
`extends:`, no `flags:`, `ignore_when:`, `slash_prefix:`, or `environment:`.
Its only job is to say what the launcher is (`recognize:`) and, optionally,
which of its own options precede the inner compiler in argv (`options:`):

```yaml
type: wrapper
recognize:
  - description: "Compiler cache"     # same description/references contract as compiler recognize
    references:
      - "https://ccache.dev/manual/latest.html"
    executables: ["ccache"]
    # versioned and cross_compilation must both be false (or omitted): a
    # launcher basename is matched exactly, never version-suffixed or
    # cross-compilation prefixed.
options:               # optional; only distcc.yaml has this today
  - match: {pattern: "-j", count: 1}
  - match: {pattern: "-v"}
```

`options:` reuses the `match`/`count` sub-language from "Pattern syntax"
above, but only its exact-token form: no `*` prefix matching and no
`{ }`/`{=}`/`{:}` glued-or-separate forms. `WrapperTable::validate` (in
`build-support/compilers-codegen/src/yaml_types.rs`) rejects any pattern
containing `*` or `{` at codegen time, because the unwrap loop that consumes
this data only compares argv tokens for equality -- it does not implement
prefix or glued-value matching. `count` is how many following argv tokens
the option's value consumes, so the skip loop advances `1 + count` slots per
matched option.

The *absence* of `options:` is itself the contract: "argv[1] is the real
compiler" (true for ccache, sccache, and icecc; distcc is the one launcher
whose own flags can precede the compiler).

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

## Source mode and response-file syntax

Two per-family selectors are consumed outside the flag classifier -- one
after parsing (at the converter), one before it (at response-file
tokenization). Their *semantics* are code; the per-family *choice* is
data, declared in these YAML files. Neither is inherited through
`extends`: each family states its own (so `clang_cl` carries
`response_file_syntax: msvc` itself, it does not inherit it from `msvc`).

`source_mode` -- how an invocation's sources map to compilation database
entries. Consumed at the converter (post-parse). Values:

- `per-source-stripped` (default) -- for GCC/Clang and most families: each
  source is a separable translation unit, and the converter emits one
  entry per source, stripping sibling sources from each entry's arguments.
- `combined` -- for a single-translation-unit compiler like `valac`, which
  compiles all of a target's sources together and produces one output: the
  converter emits exactly one combined entry per invocation (`file` is the
  first source, every source retained).
- `per-source-full` -- for a whole-module compiler like `swiftc`: every
  source is analyzed together, but per-file consuming tooling
  (SourceKit-LSP) looks up a compile command by file path, so the
  converter emits one entry per source while every entry keeps the
  complete invocation (no sibling stripping).

`response_file_syntax` -- how `@file` response files are tokenized before
flag classification (only relevant with `format.arguments.from_response_files`).
Values: `gnu` (default) for GCC/Clang whitespace-and-quote rules;
`msvc` for the Windows `CommandLineToArgv` rules used by `msvc` and
`clang-cl`.

## Adding a new compiler

Adding a compiler family is a YAML file plus accepting snapshots -- no
Rust edit. Codegen discovers the file by scanning this directory and
peeking its `type:`, derives every generated name from the file stem, and
emits the recognition row, the `KNOWN_IDS` entry (so `as: <id>` is
accepted), and the family's interpreter registration automatically.

1. Create a new YAML file in this directory (e.g., `mycompiler.yaml`).
2. Add `type: compiler`, a `compiler:` block (`id:`, and optionally
   `extends:`), `recognize:`, and `flags:` entries; optionally
   `ignore_when:`, `environment:`, `slash_prefix:`, `source_mode:`,
   `response_file_syntax:`. `id:` must be unique across every YAML file in
   this directory (checked at codegen); it is also the `extends:` target
   and the ONLY accepted config `as:` spelling for this family -- there is
   no alias step.
3. Run `cargo build`, then `cargo test`. The snapshot tests will show a
   diff for the new family (its flag table, the recognition table, and the
   family registry); run `cargo insta accept` (or update the snapshots) to
   accept it.

## Adding a new wrapper

1. Create `mywrapper.yaml` in this directory with `type: wrapper`, a
   `recognize:` entry (description + references + executables, with
   `versioned`/`cross_compilation` false or omitted), and an optional
   `options:` list of exact-token flags the launcher accepts before its
   inner compiler.
2. Run `cargo build && cargo test`, and accept the snapshots. That is the
   whole change: codegen discovers `type: wrapper` files by kind
   (`load_wrapper_tables`), the unwrap logic (`extract_real_compiler`) is
   generic over the generated `WRAPPER_NAMES`/`WRAPPER_OPTIONS`, and the
   launcher basename is emitted into `WRAPPER_AS_NAMES` so `as: mywrapper`
   is accepted -- no wrapper-specific Rust code.

## Adding a new flag

1. Find the right YAML file for the compiler
2. Add an entry under `flags:` with the appropriate `match` pattern and `result`
3. Run `cargo build` -- the build script regenerates the flag tables automatically
4. Run `cargo test` -- invariant tests verify sorting, no invalid kinds, etc.
