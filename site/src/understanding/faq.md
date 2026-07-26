<!-- Diataxis type: explanation -->

# Frequently asked questions

Short answers to the questions that come up repeatedly in issues and in
chat: what Bear does and does not record, how it relates to build
systems that export a compilation database themselves, and what its
output is good for.

## Why is `compile_commands.json` empty?

The most common cause is that the build executed no compiler at all -
an incremental build with everything already up to date runs nothing,
and Bear only records commands the build actually executes. Run a clean
build (`make clean` first, or build into an empty directory). If the
build did compile and the file is still empty, work through [Bear
produces an empty
`compile_commands.json`](../guides/recipes/empty-compilation-database.md), which
covers the remaining causes in order.

## Which compilers does Bear recognize?

A large set of GCC- and Clang-compatible drivers, MSVC, several vendor
and embedded toolchains, and the common compiler launchers, each
matched by executable filename. The full list lives on [Supported
compilers](../reference/supported-compilers.md) rather than here, so that there is
only one copy of it to keep correct.

## Why does Bear not recognize my compiler?

Bear matches compiler executables by filename, including version
suffixes and cross-compilation prefixes for the families it knows. A
compiler with a name outside those patterns, or a renamed copy of a
known compiler, needs an explicit hint in the configuration file's
`compilers` section, mapping its path to the compiler family it
behaves like. See [Configure Bear](../reference/configuration.md) and the `bear(1)`
man page's CONFIGURATION section for the exact keys.

## Do I need Bear if I use CMake?

No. CMake and Meson can export a compilation database natively
(CMake's `CMAKE_EXPORT_COMPILE_COMMANDS`, for example) without
observing the build at all, and Bazel projects use the third-party
[Hedron's Compile Commands
Extractor](https://github.com/hedronvision/bazel-compile-commands-extractor)
for the same result. Reach for Bear when the build system does not
support this, or when what it exports does not match what actually ran.

## Can I use Bear with Docker?

Yes, but Bear has to run **inside** the container, as part of the build
it observes; running Bear against `docker exec` from the host does not
work. See [Bear on Linux, WSL2, and Docker](../platforms/linux.md) for why
and for the correct invocation.

## Can I use Bear with ccache / distcc / sccache?

Yes. Bear recognizes these as compiler launchers and drops them from
the recorded command, so the database's `arguments` show the real
compiler invocation, not the launcher call.

## How do I get verbose output for debugging?

Set the `RUST_LOG` environment variable to `debug` before running Bear.
See [Troubleshooting](../guides/troubleshooting.md) for the exact invocation, what
the output shows, and when to include it in a bug report.

## How do I switch between preload and wrapper mode?

Set `intercept.mode` to `preload` or `wrapper` in the configuration
file; without it, Bear picks the platform default (preload on Linux and
the BSDs, wrapper on macOS and Windows). See [Configure
Bear](../reference/configuration.md) for where Bear looks for that file, and the
`bear(1)` man page for the key's exact effect and limits.

## Can I use Bear with a build system that does not use `CC`/`CXX`?

Yes, in either interception mode, though the mechanism differs. In
wrapper mode Bear puts a `.bear/` directory ahead of the real compilers
on `PATH`, so any lookup that goes through `PATH` finds the wrapper,
whether or not the build reads `CC`/`CXX` to get there. In preload
mode Bear observes every `exec()` call a build process makes,
regardless of how the executable was found - more transparent, but it
cannot see a statically linked build tool's own executions, and it is
unavailable on Windows and on macOS while System Integrity Protection
is enabled.

## Can I filter which files appear in the output?

Yes, with the `sources` section of the configuration file: rules by
directory and by filename glob, each either including or excluding
matching entries. See [Configure Bear](../reference/configuration.md) and the
`bear(1)` man page's CONFIGURATION section.

## Can I get `command` instead of `arguments` in the output?

Yes, `format.entries.use_array_format: false` in the configuration file
writes a single shell-escaped `command` string instead of the
`arguments` array. Bear prefers the array by default because it avoids
shell-escaping ambiguity; switch only for a consumer that reads
`command` specifically.

## Where does Bear store temporary files?

In wrapper mode, Bear creates a `.bear/` directory in the build's
current working directory holding the wrapper executables and their
configuration; it is wiped at the start of each run and removed again
once the build finishes. In preload mode, Bear creates no such
directory - the preload library is loaded directly from Bear's own
installation.

Related: [Troubleshooting](../guides/troubleshooting.md) for a database that came
out wrong, [Configure Bear](../reference/configuration.md) for the configuration
file, and the [Recipes](../guides/recipes/index.md) index for task-shaped pages.
