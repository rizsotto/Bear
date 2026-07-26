<!-- Diataxis type: how-to -->

# Bear on Windows

```sh
bear -- make
```

Wrapper interception is the default on Windows, and the only method
available there: Bear puts wrapper executables ahead of the real
compilers on `PATH`. That covers the compilers the build picks up from
the wrapper directory, so the build has to look there in the first
place: a configure step that discovers compilers must itself run under
Bear. Forcing preload mode on Windows is a startup error that names
wrapper mode as the alternative; the preload library is not even built
for Windows, since the platform has no `LD_PRELOAD`/`DYLD_INSERT_LIBRARIES`
equivalent.

## MSYS2

Bear builds under MSYS2's Unix-like environments (UCRT64, CLANG64,
CLANGARM64, and similar), where a POSIX shell and the usual build
tools (`make`, `cmake`) are available. Windows is a supported platform:
the project's CI runs the full test suite on Windows for every change,
alongside Linux and macOS. MSYS2 itself is not in that CI matrix, and
Windows has far fewer users than the other two, so report anything that
looks wrong.

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
