<!-- Diataxis type: how-to -->

# Bear produces an empty compile_commands.json

The most common cause is that the build compiled nothing: an incremental
build with everything already up to date runs no compiler at all, and
Bear only records commands the build actually executes. Rebuild from
clean:

```
make clean
bear -- make
```

If a clean rebuild still leaves `[]`, work through the other causes
below, most common first. Each one shows the command that confirms it
and the command that fixes it.

## The build had nothing to compile

An incremental build that finds every target up to date runs no
compiler, so there is nothing for Bear to intercept. This is not a bug:
Bear observes execution, it does not read your build files to guess what
"should" happen.

**Confirm**: check whether the build itself said it did nothing, for
example make's

```
make: Nothing to be done for 'all'.
```

**Fix**: clean first, or build into an empty directory:

```
make clean
bear -- make
```

When you only rebuilt part of a larger project and want to keep the
rest of a previous run's entries instead of doing a full clean, use
`--append` (see the [`bear(1)` man page][manpage] for the exact
semantics, and [Troubleshooting](../troubleshooting.md) for the
duplicate-handling rules that apply when you do).

## The compiler is not recognized

Bear decides whether a command is a compilation by matching the
executable's filename against a table of known names; a name outside
that table produces no entry, even though the compiler ran and built
your code successfully.

**Confirm**: check whether your compiler's name is one Bear knows, on
[Supported compilers](../supported-compilers.md), or ask the installed
build directly:

```
bear semantic --print-compilers
```

A custom or renamed executable will not appear here. The
[Diagnostics](#diagnostics) section below shows a more precise way to
confirm the exact command that went unrecognized.

**Fix**: add a `compilers:` hint mapping the executable's path to the
family it behaves like:

```yaml
schema: "4.2"
compilers:
  - path: /usr/local/bin/my-custom-cc
    as: gcc
```

A Makefile rule invoking `./my-totally-custom-cc`, itself a plain copy
of `gcc`, produces `[]` on its own, and a working entry once the hint
above names its path. The
accepted `as:` values are listed on [Supported
compilers](../supported-compilers.md#as-and-ignore-hints).

## Preload interception is blocked or unavailable

Bear has two interception methods, chosen once per invocation: preload
(the default on Linux and the BSDs) injects a library into every process
the build starts; wrapper (the default on macOS and Windows) substitutes
the known compilers on `PATH` instead. See [How Bear
works](../how-it-works.md) for the mechanism. A few situations keep
preload from seeing a compilation at all:

**A statically linked build tool.** Preload works by hooking library
calls inside the process that makes them; a statically linked executable
never loads the dynamic linker, so no hook is ever installed in it, and
any compiler it execs directly is invisible to Bear. This is a
fundamental limit of the approach, not a bug.

Confirm with:

```
file "$(command -v your-build-tool)"
```

A report of "statically linked" (rather than "dynamically linked")
means preload cannot see what that tool executes. A statically linked
launcher that directly `exec`s a compiler produces `[]` under the
default (preload) mode on Linux, and a correct entry once wrapper mode
is forced:

```yaml
schema: "4.2"
intercept:
  mode: wrapper
```

**macOS with System Integrity Protection enabled.** SIP strips the
preload environment variable from protected executables before it can
take effect, so preload cannot run there; it is wrapper mode's reason
for being the macOS default in the first place. See [Bear on
macOS](../platforms/macos.md) for the detail. Forcing preload mode
where it is unavailable (SIP-protected macOS, or Windows, which has no
preload library at all) is a startup error naming wrapper mode as the
alternative - Bear does not silently fall back, and the build does not
run, so this case shows up as an explicit error rather than an empty
`compile_commands.json`.

**Running against a container from the outside.** `bear -- docker exec
...` on the host does not work: the daemon runs the command inside the
container's own process tree, which the host-side Bear process never
observes, so the database comes out empty with no other symptom. Bear
must run **inside** the container as part of the build. See [Run Bear
inside a Docker container](docker.md) for the correct invocation.

In every one of these cases, wrapper mode is the general fallback, with
one condition of its own: the build has to actually pick the wrapper up,
covered next.

## Wrapper mode: the build never picks up the wrapper

Wrapper mode works by putting reporting executables ahead of the real
compilers on `PATH`. A build step that discovers the compiler on its
own, before the wrapper directory is in place, bakes in the real
compiler's path directly, and every later invocation goes straight to
the real compiler with no wrapper in between: a `./configure` script run
outside Bear, or a CMake configure step that resolves `cc` to an
absolute path, are the common versions of this.

**Confirm**: look for the real compiler's absolute path already baked
into a generated build file (`config.mk`, `CMakeCache.txt`, or similar)
from before the build ran under Bear.

**Fix**: run the configure step under Bear too, discarding its output,
then run the real build under Bear:

```
bear --config bear.yml -- ./configure
bear --config bear.yml -- make
```

A `configure.sh` that resolves `cc` to an absolute path and writes it
into a makefile fragment produces `[]` when only the subsequent `make`
runs under Bear in wrapper mode. Running `configure.sh` under Bear as
well, with the same config and discarding that run's own output, makes
the following build's database come out populated.
This matches the two-step CMake example in the [`bear(1)` man
page][manpage]'s EXAMPLES section.

## The build failed before it reached a compiler

Bear records the compiler invocations that actually happen; if the
build fails before starting any of them - a missing include in a
Makefile, a dependency that does not exist, a configure step that
aborts - there is nothing to record.

Note the distinction: a compile that fails *after* the compiler was
invoked (a syntax error in the source) still produces an entry, because
Bear records the attempted command, not its outcome. An empty database
from a failed build specifically means the failure happened before any
compiler ran at all.

**Confirm**: check the build's own error output and Bear's exit code;
a non-zero exit with no compiler messages at all points here.

**Fix**: fix whatever stops the build before it reaches a compile step,
then rebuild under Bear.

## Source filtering removed everything

A `sources:` rule in the configuration file that is broader than
intended can exclude every compiled file, leaving a `[]` even though the
build compiled normally. (A `duplicates:` rule cannot do this by
itself: it always keeps the first occurrence of a group of duplicates,
so it can shrink a database but never empty it out on its own; see
[Troubleshooting](../troubleshooting.md#the-output-has-duplicate-entries)
for that case.)

**Confirm**: temporarily remove the `sources:` section from your
configuration and rebuild; if entries appear, a rule there is the cause.

**Fix**: narrow the rule. For example, excluding every `.c` file:

```yaml
schema: "4.2"
sources:
  files:
    - pattern: "*.c"
      action: exclude
```

produces `[]` for a project with only `.c` sources, by design. See the
`bear(1)` man page's CONFIGURATION section for the full `sources:`
syntax.

## Diagnostics

Before anything else, re-run with logging turned on:

```
RUST_LOG=info bear -- <build command>
```

Near the end of the output is a summary that tells you which side of
the pipeline lost your entries:

```
Output pipeline:
  semantic events: 5
  current entries: 2
  previous entries: 0
  filtered entries by duplicate: 0
  filtered entries by source: 0
  synthesized header entries: 0
  dropped entries (invalid): 0
  total entries written: 1
```

- `semantic events` near the minimum (as low as 1, just the build
  command itself) means the build never executed a compiler - the
  "nothing to compile" or "build failed early" cases above.
- `semantic events` clearly greater than zero but `current entries: 0`
  means commands ran but none were recognized as a compiler - the
  "compiler not recognized" case.
- `filtered entries by source` or `filtered entries by duplicate`
  greater than zero, with `total entries written` at zero, means
  recognized entries were filtered out by your configuration.

Set `RUST_LOG=debug` instead for more detail: every intercepted command
is tagged `Recognized(...)` or `NotRecognized(...)`, which pins down
exactly which invocation Bear saw and how it classified it. Without
`RUST_LOG` set, Bear prints only warnings and errors; see the
`bear(1)` man page's ENVIRONMENT section for the exact levels.

Related: [Troubleshooting](../troubleshooting.md) for a database that
is short or wrong rather than empty, [Supported
compilers](../supported-compilers.md) for the recognized names,
[Frequently asked questions](../faq.md), [How Bear
works](../how-it-works.md) for the interception and analysis mechanism,
the [Recipes](index.md) index, and the platform notes for
[Linux](../platforms/linux.md), [macOS](../platforms/macos.md),
[Windows](../platforms/windows.md), and [BSD](../platforms/bsd.md).

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md
