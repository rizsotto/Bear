# One option for input from another machine

## Context

Bear can read a build log (parse-sh mode) or an event file (semantic
mode) that was captured on another machine. Several steps after
interception look at this machine to decide what goes into an entry:
the missing-source filter checks that the source exists, recognition
runs `cc --version` and resolves configured compiler paths through
symlinks, the configuration loader checks that configured compiler
paths exist, and parse-sh lays Bear's own environment under each parsed
command. For input from another machine each of these gives a wrong or
empty answer, or mixes this machine into the output.

The rule we wanted to state is simple: with input from another machine,
the database depends only on the input and the configuration. The
options considered:

- **One config key per step.** Users already turn the missing-source
  filter off by hand. Rejected: the probe, path resolution, the
  existence check and the environment have no key, and adding four keys
  for one intent makes users find all of them. The intent is a property
  of the input, so it belongs on the command line next to it.
- **Accept the option in every mode.** Rejected: intercept mode runs
  nothing after interception, so the option would do nothing, and in
  combined mode the build runs here, so every check sees the file
  system the build used and gives the right answer. The option could
  only remove correct data there. It is offered in the modes that do
  not run the build, which also covers future inputs of that kind.
- **Warn and fall back for configuration that needs the build's files**
  (the symlink-resolving path format, header entries, response-file
  inlining). Rejected: Bear would run with a configuration it does not
  honor, every key needs a documented fallback, and the warning is easy
  to miss in a CI log. An error names the key once, and the user fixes
  the configuration. The cost is that a configuration file found
  automatically can block the option until the user points Bear at
  another file.
- **Error for an explicit missing-source filter too.** Rejected: the
  filter is on by default and the configuration cannot tell "not set"
  from "on", so every run with the option would fail. Turning it off is
  the main point of the option, so it is turned off without a message.
- **No entry for an unconfigured `cc`, `c++` or `CC`.** Without the
  probe, these names cannot be identified. Rejected: GNU make's default
  `CC` is `cc`, so a plain dry-run log from Linux would give an empty
  database. Instead they are parsed with Clang's flag table. In Bear's
  definitions the Clang family extends GCC's, so its table holds every
  GCC flag plus Clang's own. A GCC call parsed with it is read right
  except where a GCC argument spells a Clang-only flag. The other way
  round, a Clang call parsed as GCC, misreads `-Xclang <arg>`, the bug
  the probe exists to avoid.
- **Keep Bear's environment in parse-sh.** Rejected: a local `CPATH`
  would become include flags in the entries of another machine's
  build.
- **Name the option after the input** (`--foreign-input`). Rejected:
  "foreign database" already means a database Bear did not produce, on
  this machine.
- **Name it after file-system checks** (`--no-fs-check`,
  `--no-local-checks`). Rejected: running `cc --version` and reading
  the environment are not file-system checks. Every effect is Bear
  looking something up on this machine, so the option is
  `--no-local-lookups`. Like compiledb-go's `--no-strict`, which only
  drops its source-existence check, it says what Bear stops doing.

## Decision

One command-line option, offered in the modes that do not run the
build. With it, Bear opens none of the build's files, runs none of its
programs, and reads none of its own environment to shape an entry.
Configuration that needs the build's files is an error, except the
missing-source filter, which is turned off.

## Consequences

Users with a log from another machine pass one option and get a full
database without a configuration file. Combined and intercept mode do
not change. Path formats that only rewrite text, directory and pattern
rules, and compiler paths in the configuration keep working, and match
the paths as the other machine wrote them.

Out of scope: rebuilding the other machine's environment or `PATH`, and
parse-sh's initial working directory, which the caller sets.

Revisit when a new mode that does not run the build is added (it gets
the option), when path remapping lands (it rewrites text only, so it is
allowed), or if users report GCC calls misread by the Clang table.

## References

- [cli-input-from-another-machine](../requirements/cli-input-from-another-machine.md), the requirement this supports
- [missing-source-filter-default-on](missing-source-filter-default-on.md), the default this option turns off
- [ambiguous-cc-version-probe](ambiguous-cc-version-probe.md), the probe this option skips
- [compiledb-go](https://github.com/fcying/compiledb-go), prior art for `--no-strict`
