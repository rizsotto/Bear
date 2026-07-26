<!-- Diataxis type: explanation -->

# Configure Bear

Every mode Bear ships has a built-in default for every setting, so Bear
runs with no configuration file at all. Reach for a `bear.yml` when a
default does not fit your project: forcing an interception method,
teaching Bear about a compiler at an unusual path, dropping generated
sources from the database, or changing how paths and entries are
written. This page explains what the file is for, how Bear finds it,
and which section to reach for. The literal keys, their accepted
values, and their defaults are enumerated in the [`bear(1)` man
page][manpage], which stays the single reference; this page does not
duplicate it.

## Finding the file

Without `--config`, Bear searches for `bear.yml` starting in the
current working directory, then falls back to the platform's standard
per-user configuration directory: the XDG locations on Linux, BSD, and
macOS, and `%LOCALAPPDATA%` / `%APPDATA%` on Windows. The first file
found wins; the rest are not consulted. Name a file explicitly to skip
the search, for example to keep a strict configuration alongside a
permissive default one and pick between them per invocation. The
exhaustive, ordered list of paths is the man page's FILES section.

When no file is found anywhere, Bear runs on its built-in defaults: the
interception method that fits the host platform, no source filtering,
no duplicate collapsing beyond the built-in file-and-directory match,
paths recorded as the build produced them, and no header synthesis.
Nothing in the configuration file is required; every section below is
optional, and an empty or absent file is a valid configuration.

## Seeing the effective configuration

Bear resolves the found file (or the built-in defaults, if none is
found) into one configuration before it runs the build, and logs it as
YAML when `RUST_LOG` is set to `info` or a more verbose level:

    RUST_LOG=info bear -- true

With no `bear.yml` on the search path, this prints Bear's complete
built-in defaults:

```yaml
schema: "4.2"
intercept:
  mode: preload
compilers: []
sources: {}
duplicates:
  match_on:
  - directory
  - file
format:
  paths:
    directory: as-is
    file: as-is
  entries:
    use_array_format: true
    include_output_field: true
  arguments:
    from_response_files: false
    from_environment: true
headers:
  enabled: false
  strategy: siblings
```

This is the same log line whether the values came from a `bear.yml` or
from defaults, so it is the way to check what a given file actually
changed, rather than guessing from the sections below.

## What each section is for

**intercept** chooses how Bear observes the build: preload (injecting a
library into the build's processes) or wrapper (substituting compilers
on `PATH`). The default is preload on Linux and the BSDs, wrapper on
macOS and Windows. Reach for this section when the platform default is
wrong for your case, for example forcing wrapper mode on Linux to work
around a statically linked build tool, or forcing preload on a macOS
host with System Integrity Protection disabled. The trade-offs between
the two methods, and what each one cannot see, are explained in [How
Bear works](how-it-works.md); this section only records the choice.

**compilers** gives Bear hints about specific executables: what
compiler family a path is (`as`), or that its invocations should be
dropped entirely (`ignore`). Reach for this section when a compiler at
a non-standard path or name is not recognized, or is recognized as the
wrong family; [Supported compilers](supported-compilers.md) explains
how automatic recognition works and when it needs help.

**sources** filters which entries make it into the database by the
source file's directory or filename, independently of how the compiler
was invoked. By default no rule is configured, so nothing is filtered
out. Reach for this section to drop machine-generated code (Qt `moc`
output, protobuf stubs) or a `tests/` tree you do not want a linter to
see.

**duplicates** controls which fields two entries must share to count as
the same compilation, and therefore which one survives when a source is
compiled more than once. The built-in default keeps one entry per
source file per directory regardless of arguments; reach for this
section when a build compiles the same file with different flags (for
example once per target architecture) and you want an entry for each
configuration.

**format** controls how the JSON itself is written: whether paths are
left as the build produced them or normalized, whether an entry carries
the command as an argument array or a shell string, and whether
environment variables that act as implicit flags (compiler include
paths, MSVC's `CL`) are folded into the recorded arguments. By default
paths are left as-is, entries use the arguments array with the output
field included, and environment-variable folding is on while
response-file expansion is off. Reach for this section when a specific
consumer expects one particular shape, for example a tool that only
reads the `command` string.

**headers** synthesizes entries for header files by cloning a compiled
source's flags, since headers are never compiled on their own but
editors and linters need flags for them too. It is off by default;
reach for it when clangd or a similar tool complains about a header
that has no compile command.

## A small example

A configuration touches only the sections it needs to change; this is
illustrative, not a template to copy wholesale:

```yaml
schema: "4.2"
intercept:
  mode: wrapper
sources:
  files:
    - pattern: "moc_*.cpp"
      action: exclude
```

The `schema` key names the configuration format version Bear expects;
a file whose value does not match the version this Bear release
supports is rejected rather than partially applied.

See also: [How Bear works](how-it-works.md) for the interception and
semantic-analysis mechanism this configuration shapes, [Supported
compilers](supported-compilers.md) for compiler recognition, and the
[Recipes](recipes/index.md) for task-oriented uses of these sections
(for example excluding generated sources).

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md
