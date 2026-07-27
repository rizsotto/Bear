<!-- Diataxis type: how-to -->

# Bear on Linux, WSL2, and Docker

Preload is the default interception method on Linux (see [how Bear
works](../understanding/how-it-works.md) for the mechanism, and
[Configure Bear](../reference/configuration.md#intercept) for the
setting). It cannot see into a statically linked build tool; force
[`intercept.mode: wrapper`](../reference/configuration.md#intercept)
instead.

## WSL2

If `.wslconfig` sets `networkingMode=mirrored`, the loopback connection
intercepted processes use to report back to `bear-driver` breaks,
producing an empty or short `compile_commands.json` with no other
error. Remove the setting and restart WSL2 (`wsl --shutdown`).

## Docker

Bear must run inside the container, as part of the build it observes;
`bear -- docker exec ...` from the host does not work. See [Run Bear
inside a Docker container](../guides/recipes/docker.md).

## Installing the preload library to a non-standard lib directory

```sh
INTERCEPT_LIBDIR=lib64 cargo build --release
INTERCEPT_LIBDIR=lib64 ./scripts/install.sh
```

A mismatch here fails silently: preload never loads and the database
comes out empty or short. See [Installing Bear](../installation.md) for
why.

Related: [Troubleshooting](../guides/troubleshooting.md) for output that
comes out wrong, and the [Recipes](../guides/recipes/index.md) index for
other tasks.
