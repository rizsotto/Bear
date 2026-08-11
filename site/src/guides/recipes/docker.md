<!-- Diataxis type: how-to -->

# Run Bear inside a Docker container

Run Bear inside the container, as part of the build it observes, the same
way you would on a bare host:

```dockerfile
RUN bear -- make -j4
```

or against an already-running container:

```sh
docker exec my-container sh -c "bear -- make -j4"
```

This is the working answer for the common case: a normal, dynamically
linked build toolchain, building for the container's own architecture.
One problem is specific to containers: the paths Bear records belong to
the container's filesystem, not the host's.

## Installing Bear in the image

Add Bear to the image the same way you would install it anywhere else:
your distribution's package if it carries a current-enough one, or a
source build otherwise. [Install Bear](../../installation.md) covers both;
a source build inside a `Dockerfile` follows the same three commands as
[`INSTALL.md`][install]:

```dockerfile
# Install the Rust toolchain, a C compiler, and lld or mold first
# (see INSTALL.md for the exact prerequisites for your base image).
COPY . /src
WORKDIR /src
RUN cargo build --release && ./scripts/install.sh
```

Build Bear in the same image (or one with the same libc) it will run in.
The preload library is a compiled shared object, so a copy built
elsewhere only works if that build's libc is no newer than the image's,
the same ABI constraint [Troubleshooting](../troubleshooting.md)
describes for cross-compilation.

## Why the build has to run inside the container

`bear -- docker exec ...` from the host does not work: `docker exec`
hands the command to the Docker daemon, which runs it in the container's
own process tree, one the host-side Bear process never sees, so it never
observes the compiler invocations. [Bear on Linux, WSL2, and
Docker](../../platforms/linux.md) explains this in full.

## What the container needs for preload interception

[Preload mode](../../reference/configuration.md#intercept) works inside
a container with no extra privileges: `LD_PRELOAD` is a plain
dynamic-linker feature, not a ptrace-based interception, so it needs no
`--privileged` flag and no added capabilities. It does need:

- a dynamic linker in the image. A `scratch` or fully static final image
  has none, so a build that runs in such an image cannot use preload
  mode there; build in a normal base image instead, or use wrapper mode
  (below).
- the container's libc to match what Bear's preload library was built
  against, per the ABI note above.
- a build tool that is itself dynamically linked. A statically linked
  build tool never loads the dynamic linker, so its own `exec()` calls
  are invisible to preload the same way they are on a bare host; this is
  not a Docker-specific limitation.
- working loopback networking, for the TCP connection intercepted
  processes use to report back to `bear-driver`. Bear binds this on
  `127.0.0.1` (or `[::1]`), and the driver and the processes it observes
  are in the same network namespace, so the connection stays inside the
  container whichever `--network` mode started it.

## When to fall back to wrapper mode

If the build tool inside the container is statically linked (a Go build
using cgo is the common case; see [How Bear
works](../../understanding/how-it-works.md)), set wrapper mode in
the configuration file the build reads:

```yaml
schema: "4.2"
intercept:
  mode: wrapper
```

As on any platform, a "configure" step that discovers compilers before
the real build runs must itself run under Bear so it discovers the
wrapper; see [Bear produces an empty
compile_commands.json](empty-compilation-database.md#wrapper-mode-the-build-never-picks-up-the-wrapper).

## The path-mapping problem

By default Bear records `directory` and `file` exactly as the build used
them, with no transformation
([`format.paths`](../../reference/configuration.md#formatpaths)). Inside
a container that path is almost always absolute under the container's
own filesystem root, for example `/src/main.c`. clangd, clang-tidy, and
similar tools normally run on the host, where `/src` does not exist; a
`compile_commands.json` copied out of the container as-is points at
files the host-side tool cannot find.

Three ways to deal with it, in order of how little they change:

- **Bind-mount the project at the same absolute path inside the
  container as on the host**, so the recorded paths are already correct
  on both sides:

  ```sh
  docker run -v "$(pwd):$(pwd)" -w "$(pwd)" myimage bear -- make
  ```

  This needs no post-processing and is the simplest fix when the host
  and container can agree on a path.
- **Rewrite the path prefix after copying the database out**, when the
  layouts cannot match (a different OS, or a host path the container
  cannot mount at the same location): substitute the container's mount
  point for the host's project root in `directory` and `file` with
  `sed` or `jq` before pointing clangd at the file.
- **Run the editor and language server inside the container too** (a
  dev-container setup), so nothing outside the container ever reads the
  database, and the mismatch does not arise.

Related: [Bear on Linux, WSL2, and Docker](../../platforms/linux.md) for
the platform-level notes, [Troubleshooting](../troubleshooting.md) for a
database that comes out wrong, and the [Recipes](index.md) index for
other tasks.

  [install]: https://github.com/rizsotto/Bear/blob/master/INSTALL.md
