<!-- Diataxis type: explanation -->

# How Bear works

Producing a compilation database is two separate problems: capturing
which commands a build runs, and recognizing which of those commands
are compilations worth recording. Bear keeps the two apart. A capture
stage produces a stream of execution events (program, arguments,
working directory, environment); a semantic analysis stage turns the
events it recognizes as compiler invocations into compilation-database
entries. Combined mode runs both in one process; `bear intercept` and
`bear semantic` run them as two separate steps over a file in between,
and `bear parse-sh` replaces capture entirely by reconstructing events
from text instead of running anything. This page explains the
mechanism; for the commands themselves see [Getting started with
Bear](../tutorials/getting-started.md) and the [Recipes](../guides/recipes/index.md).

## Two ways to capture a real build

Interception observes the build as it runs. Bear has two mechanisms for
it, and uses exactly one per invocation, chosen before the build starts.
Which one is the default on your system is on its platform page -
[Linux](../platforms/linux.md), [macOS](../platforms/macos.md),
[Windows](../platforms/windows.md), [BSD](../platforms/bsd.md) - and
[Configure Bear](../reference/configuration.md) explains how to force the other one.

**Preload** injects a small shared library into every process the build
starts, using the dynamic linker's library-preloading facility
(`LD_PRELOAD` on Linux and the BSDs, `DYLD_INSERT_LIBRARIES` on macOS).
The library intercepts the `exec` family of calls (and `posix_spawn`,
`popen`, `system`) before they reach the kernel, reports the call, then
lets it proceed unchanged. Because it hooks the mechanism every process
uses to start another one, it sees every compiler invocation regardless
of how the build found it, and the build itself is unaware anything is
watching. It cannot see into a statically linked executable, because
such a binary never goes through the dynamic linker in the first place.
It cannot run at all on Windows, which has no preloading facility to
hook and for which the library is not even built, nor on a macOS host
with System Integrity Protection enabled, since SIP strips
`DYLD_INSERT_LIBRARIES` from protected executables before it can take
effect.

**Wrapper** takes the opposite approach: instead of watching every
process, it puts a reporting executable where the compiler is expected
to be. Bear creates a `.bear/` directory holding one wrapper for each
compiler it recognizes on `PATH` at startup, puts that directory first
on `PATH`, and points
compiler-selection variables (`CC`, `CXX`, and the like) at it. When the
build launches what it thinks is the compiler, it launches the wrapper
instead; the wrapper reports the call, then execs the real compiler
found at its resolved, absolute path so the build sees no difference.
This works on every platform, including Windows, but only for compilers
the build actually reaches through `PATH` or a variable Bear resolved
at startup: a build step that discovers the compiler on its own (a
`./configure` probing for `cc`) needs to run under Bear too, so the
wrapper is in place by the time the real build starts.

Neither mechanism is a fallback for the other. Where preload is
unavailable, Bear does not quietly switch to wrapper mode; forcing
preload there is a startup error that names wrapper mode as the
alternative, and the build does not run.

## From an intercepted call to the driver

Both mechanisms report through the same channel: the process running
the build (`bear-driver`, from the `bear-driver` binary) opens a TCP
listener on the loopback interface, and every preload library instance
or wrapper process connects to it and writes one length-prefixed JSON
event per execution. This keeps the reporting side dependency-free and
lets interception and semantic analysis live in separate processes when
running as `bear intercept` and `bear semantic`, with the event stream
as the file in between. The shared pieces of this runtime (the
execution type, the wire encoding, environment-variable filtering) live
in the `intercept` crate; the driver-side half that supervises the
build, runs the TCP collector, and prepares the wrapper directory or
preload environment lives in `intercept-supervisor`; the injected
library itself is `intercept-preload`, and the wrapper executable's own
code is `bear-wrapper`.

## Reconstructing events without running anything

`bear parse-sh` takes a third path that skips capture entirely: instead
of observing real execution, it parses shell command text, typically a
build system's dry-run output (`make -n`) or a saved build log, and
reconstructs the executions that text describes. It understands a
documented subset of `sh` syntax and make's recursive-directory
markers, and feeds what it reconstructs through the same semantic
analysis as a real build. This is inherently lower fidelity: a dry run
can omit commands a real build would issue, and the environment used to
resolve bare executable names is the parsing environment, not the
build's. It exists for builds that cannot be run at all, such as
recovering a database from a CI log after the fact; interception
remains the higher-fidelity default whenever the build can run.

## Semantic analysis

Whatever produced the event stream, from here the processing is the
same. Each event's executable name is checked against Bear's compiler
definitions, including cross-compiler prefixes, version suffixes, and
compiler launchers such as ccache and distcc that carry the real
compiler in their own arguments; see [Supported
compilers](../reference/supported-compilers.md) for how that recognition works. An
event that names no recognized compiler produces nothing.

A recognized invocation is then turned into zero, one, or several
entries, depending on the compiler and what the invocation asked it to
do:

- most compilers treat each source file as its own translation unit, so
  an invocation naming several sources yields one entry per source,
  each stripped down to look like a command that compiled only that
  file;
- a compiler that always compiles a whole target as one unit (Vala's
  `valac`) collapses to a single entry keyed on the first source, with
  every source kept in its arguments;
- a compiler whose sources depend on each other within one invocation
  (Swift's `swiftc` in whole-module mode) produces one entry per source,
  but every entry keeps the complete invocation's arguments;
- an invocation that only links, or only asks for information
  (`--version`, preprocessing, `--help`), produces no entry at all.

From there, the entries pass through whatever the configuration
requests: dropping generated sources by directory or filename,
collapsing duplicate entries, resolving or leaving paths as they are,
choosing between an arguments array and a command string, and
optionally synthesizing entries for header files. [Configure
Bear](../reference/configuration.md) explains each of these.

## Output

The result is written as a JSON array of entries conforming to the
[Clang JSON Compilation Database
specification][jsoncdb], the format clangd, clang-tidy, and similar
tools read to know how each source file is compiled.

See also: [Getting started with Bear](../tutorials/getting-started.md) for the
first run, the [Recipes](../guides/recipes/index.md) for specific tasks, and
[Troubleshooting](../guides/troubleshooting.md) for a database that came out
empty or wrong.

  [jsoncdb]: https://clang.llvm.org/docs/JSONCompilationDatabase.html
