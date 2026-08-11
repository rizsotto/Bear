<!-- Diataxis type: reference -->

# Configure Bear

Bear reads an optional `bear.yml`. Every key is optional except `schema`,
which is required once you write a file at all; every section below
states its default, so a key you do not write keeps that value. The
canonical source for every key, accepted value, and default is the
[`bear(1)` man page][manpage] CONFIGURATION section.

## Where the file is found

Without `--config`, Bear searches for `bear.yml` in the current working
directory first, then the platform's per-user configuration directory
(the XDG locations on Linux and the BSDs, `%LOCALAPPDATA%` /
`%APPDATA%` on Windows). The first file found is loaded; the rest are
not consulted. The exhaustive, ordered list of paths is the man page's
FILES section. Pass `-c`/`--config FILE` to load a specific file
instead of searching.

## schema

```yaml
schema: "4.2"
```

Names the configuration format version this file is written for. It is
the one key with no default: a file that omits it, or names a version
this Bear release does not support, is rejected rather than partially
applied.

## intercept

```yaml
intercept:
  mode: wrapper
```

- **`intercept.mode`**: `preload` or `wrapper`. Default: `preload` on
  Linux and the BSDs, `wrapper` on macOS and Windows.

## compilers

```yaml
compilers:
  - path: /usr/bin/cc
    as: gcc
  - path: /usr/local/bin/gcc
    ignore: true
```

Default: `[]` (no overrides; every compiler is recognized automatically).

- **`compilers.path`**: Path of the compiler executable. Required. A
  `path` on its own, with no other key, is already enough to make an
  otherwise unrecognized executable be treated as a compiler.
- **`compilers.as`**: Compiler type hint. Optional. The accepted values
  are the family ids listed on [Supported
  compilers](supported-compilers.md), which is generated from Bear's own
  compiler definitions, plus `wrapper` for a compiler launcher (see [Use
  Bear with ccache, distcc, or icecc](../guides/recipes/ccache-distcc-icecc.md)).
  `bear semantic --print-compilers` prints the same set for the version
  you have installed. When `as` is omitted, Bear guesses the family from the executable's filename
  using its normal recognition patterns, falling back to `gcc` when no
  pattern matches; an MSVC-style or otherwise unusual compiler whose
  name matches nothing is then parsed with GCC's flag rules unless `as`
  is set explicitly.
- **`compilers.ignore`**: Exclude this executable's invocations from the
  database. Optional. Default: `false`.

In wrapper mode this section also decides which executables get wrapped.
With no entry other than `ignore` ones, Bear scans PATH at startup and
wraps every compiler it recognizes. As soon as one entry is not ignored,
Bear wraps exactly the listed paths and skips the scan, so listing a
single unusual compiler stops `gcc` and `clang` from being intercepted
unless you list them as well. Compilers named in the compiler
environment variables (`CC`, `CXX`, and similar) are wrapped either way.
Preload mode is unaffected, because it intercepts every executed program
regardless of this section.

## sources

```yaml
sources:
  directories:
    - path: /project/tests
      action: exclude
  files:
    - pattern: "moc_*.cpp"
      action: exclude
```

Default: `{}` (both lists empty; nothing is filtered out).

- **`sources.directories.path`**, **`sources.directories.action`**: list
  of directory rules; `action` is `include` or `exclude`.
- **`sources.files.pattern`**, **`sources.files.action`**: list of
  filename-glob rules; `action` is `include` or `exclude`.

## duplicates

```yaml
duplicates:
  match_on:
    - file
    - arguments
```

- **`duplicates.match_on`**: list of entry fields to compare: `file`,
  `arguments`, `directory`, `command`, `output`. Two entries are
  duplicates when all configured fields match; the first occurrence is
  kept. Default: `[directory, file]`.

## format

```yaml
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

### format.paths

- **`format.paths.directory`**, **`format.paths.file`**: `as-is`,
  `canonical`, `relative`, or `absolute`. Default: `as-is` for both.

### format.entries

- **`format.entries.use_array_format`**: write the `arguments` array
  instead of the `command` string. Default: `true`.
- **`format.entries.include_output_field`**: include the `output` field.
  Default: `true`.

### format.arguments

- **`format.arguments.from_response_files`**: expand `@file`
  response-file references into their tokenized contents. Default:
  `false`.
- **`format.arguments.from_environment`**: fold compiler environment
  variables (`CPATH`, `C_INCLUDE_PATH`, `CPLUS_INCLUDE_PATH`,
  `OBJC_INCLUDE_PATH`, MSVC's `CL` / `_CL_`) into the recorded arguments.
  Default: `true`.

## headers

```yaml
headers:
  enabled: true
  strategy: dependency-files
```

- **`headers.enabled`**: turn header-entry synthesis on. Default:
  `false`.
- **`headers.strategy`**: `siblings` or `dependency-files`. Default:
  `siblings`.

## Checking what is actually in effect

Bear logs its fully resolved configuration as YAML whenever `RUST_LOG`
is set to `info` or a more verbose level:

```sh
RUST_LOG=info bear -- true
```

This is the same line whether the values came from a `bear.yml` or from
built-in defaults, so it is the way to check what a given file actually
changed on this machine, rather than working it out from the sections
above.

See also: [How Bear works](../understanding/how-it-works.md) for the interception and
semantic-analysis mechanism this configuration shapes, [Supported
compilers](supported-compilers.md) for compiler recognition, and the
[Recipes](../guides/recipes/index.md) for task-oriented uses of these sections
(for example excluding generated sources).

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md
