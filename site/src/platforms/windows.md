<!-- Diataxis type: how-to -->

# Bear on Windows

Wrapper is the only interception method on Windows: there is no
`LD_PRELOAD`/`DYLD_INSERT_LIBRARIES` equivalent, so forcing preload is a
startup error before anything runs (see [how Bear
works](../understanding/how-it-works.md) and [Configure
Bear](../reference/configuration.md#intercept)).

## MSYS2 and MinGW64 environments

Bear builds under MSYS2's Unix-like environments (MINGW64, UCRT64,
CLANG64, CLANGARM64). `$MINGW_PREFIX` points at whichever one you
launched (`/mingw64`, `/ucrt64`, `/clang64`, and so on), the natural
`PREFIX` for installing into it:

```sh
DESTDIR="$pkgdir" PREFIX="$MINGW_PREFIX" ./scripts/install.sh
```

Only `bear-driver` and `bear-wrapper` are installed in this
configuration; there is no preload library to package.

Related: [how Bear works](../understanding/how-it-works.md) for the
wrapper mechanism, [Troubleshooting](../guides/troubleshooting.md) for
output that comes out wrong, and the
[Recipes](../guides/recipes/index.md) index for other tasks.
