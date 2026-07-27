<!-- Diataxis type: how-to -->

# Bear on macOS

Wrapper is the default interception method on macOS: Apple-signed Xcode
compilers block preload, because System Integrity Protection strips
`DYLD_INSERT_LIBRARIES` before it can take effect (see [how Bear
works](../understanding/how-it-works.md)). Preload can be forced with
[`intercept.mode: preload`](../reference/configuration.md#intercept),
but only works with SIP disabled.

## Building through Xcode or `xcodebuild`

Xcode passes many flags through an `@file` response file, and Bear's
default recording keeps that argument literal. Turn on
[`format.arguments.from_response_files`](../reference/configuration.md#formatarguments)
so an entry built through `xcodebuild` carries the actual flags.

## Homebrew-installed compilers

Homebrew's GCC formula installs only versioned binaries (`gcc-14`, not
`gcc`); Bear recognizes these by filename automatically (see [Supported
compilers](../reference/supported-compilers.md)).

Related: [how Bear works](../understanding/how-it-works.md) for the
preload/wrapper mechanism, [Troubleshooting](../guides/troubleshooting.md)
for output that comes out wrong, and the
[Recipes](../guides/recipes/index.md) index for other tasks.
