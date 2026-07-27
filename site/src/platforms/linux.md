<!-- Diataxis type: how-to -->

# Bear on Linux, WSL2, and Docker

Preload is the default interception method on Linux: Bear injects a
small library (`libexec.so`) into every process the build starts, and
it is transparent to the build - no wrapper directory, no `PATH`
changes. See [how Bear works](../understanding/how-it-works.md) for the
mechanism. It cannot see into a statically linked build tool, because a
static binary's `exec()` calls never reach the dynamic linker; force
wrapper mode instead with `intercept.mode: wrapper` (see [Configure
Bear](../reference/configuration.md#intercept)).

## WSL2

Bear works in WSL2. One networking mode is known to break it: if
`.wslconfig` sets `networkingMode=mirrored`, the loopback TCP
connection that intercepted processes use to report executions back to
`bear-driver` can fail, and the symptom is an empty or short
`compile_commands.json` with no other error. If you build under WSL2
and see that symptom, check `%USERPROFILE%\.wslconfig` for this
setting:

```ini
[wsl2]
# networkingMode=mirrored   # comment out or remove this line
```

Then restart WSL2 for the change to take effect:

```sh
wsl --shutdown
```

## Docker

Bear must run **inside** the container, as part of the build it
observes. Running `bear -- docker exec ...` from the host does not
work: `docker exec` hands the command to the Docker daemon, which runs
it in the container's own process tree, a tree the host-side Bear
process never sees.

Run Bear as part of the container build instead:

```dockerfile
RUN bear -- make -j4
```

or against an already-running container:

```sh
docker exec my-container sh -c "bear -- make -j4"
```

## Installing the preload library to a non-standard lib directory

`bear-driver` locates `libexec.so` at a fixed path relative to itself
(`../$INTERCEPT_LIBDIR/libexec.so`, `lib` by default). On distributions
that use `lib64` or another name, build and install with the matching
value so the two agree:

```sh
INTERCEPT_LIBDIR=lib64 cargo build --release
INTERCEPT_LIBDIR=lib64 ./scripts/install.sh
```

A mismatch here does not fail the build or even print an error most
people notice: the dynamic linker just skips a preload target it cannot
open, prints its own warning, and lets the build run, so the usual
symptom is an `ld.so` error naming `libexec.so` at startup followed by a
database that is empty or short despite Bear itself exiting `0` - see
[Exit status](../reference/exit-status.md#notes) for why that counts as
success. [Troubleshooting](../guides/troubleshooting.md) covers that
error and the related glibc-version mismatch seen in
cross-compilation.

Related: [Troubleshooting](../guides/troubleshooting.md) for output that comes
out wrong, and the [Recipes](../guides/recipes/index.md) index for other
tasks.
