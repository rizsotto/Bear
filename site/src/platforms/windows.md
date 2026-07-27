<!-- Diataxis type: how-to -->

# Bear on Windows

Wrapper interception is the default on Windows, and the only method
available there: Bear puts wrapper executables ahead of the real
compilers on `PATH` (see [Configure
Bear](../reference/configuration.md#intercept) for the `intercept.mode`
key). That covers only the compilers the build picks up from the
wrapper directory, so the build has to look there in the first place: a
configure step that discovers compilers must itself run under Bear.
Forcing preload mode on Windows is a startup error that names wrapper
mode as the alternative, reported before anything runs - see [Exit
status](../reference/exit-status.md#notes); the preload library is not
even built for Windows, since the platform has no
`LD_PRELOAD`/`DYLD_INSERT_LIBRARIES` equivalent.

## MSYS2 and MinGW64 environments

Bear builds under MSYS2's Unix-like environments (MINGW64, UCRT64,
CLANG64, CLANGARM64, and similar), where a POSIX shell and the usual
build tools (`make`, `cmake`) are available. `$MINGW_PREFIX` points at
whichever of those environments you launched (`/mingw64`, `/ucrt64`,
`/clang64`, and so on), which is the natural value for `PREFIX` when
installing into it.

Install with `scripts/install.sh`, the same script as on every other
platform, staged with `DESTDIR` and pointed at the MSYS2 prefix.
Copying the built executables into a `bin` directory by hand does not
work: `bear-driver` finds `bear-wrapper` at a fixed path relative to
itself, so the layout the script creates is load-bearing.

```sh
DESTDIR="$pkgdir" PREFIX="$MINGW_PREFIX" ./scripts/install.sh
```

Only `bear-driver` and `bear-wrapper` are installed in this
configuration; there is no preload library to package.

Related: [how Bear works](../understanding/how-it-works.md) for the wrapper
mechanism, [Troubleshooting](../guides/troubleshooting.md) for output that
comes out wrong, and the [Recipes](../guides/recipes/index.md) index for other
tasks.
