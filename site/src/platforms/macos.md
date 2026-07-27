<!-- Diataxis type: how-to -->

# Bear on macOS

Wrapper interception is the default on macOS, and Apple's own toolchain
is why: Xcode's compilers are Apple-signed, and System Integrity
Protection strips `DYLD_INSERT_LIBRARIES` from Apple-signed executables
before it can take effect - Bear's compiler recognition calls this out
explicitly for `swiftc` - so preload cannot observe them. Bear puts
wrapper executables ahead of the real compilers on `PATH` instead, so
the build has to pick them up: a configure step that discovers
compilers must itself run under Bear, or the rest of the build records
nothing.

Preload can be forced instead with [`intercept.mode:
preload`](../reference/configuration.md#intercept), but it only works
with SIP disabled; see [how Bear works](../understanding/how-it-works.md)
for the injection mechanism SIP blocks. With SIP enabled, forcing
preload is a startup error that names wrapper mode as the alternative:
Bear does not silently switch modes, and it does not run the build.

## Building through Xcode or `xcodebuild`

Xcode routes many flags through a response file it passes to the
compiler with `@file` syntax, and Bear's default recording keeps that
`@file` argument literal instead of the flags behind it. Turn on
[`format.arguments.from_response_files`](../reference/configuration.md#formatarguments)
so an entry built through `xcodebuild` carries the actual flags rather
than a reference a downstream tool like clangd cannot follow.

## Where the wrapper directory lives

Wrapper mode's `.bear/` directory (see [FAQ: where does Bear store
temporary files?](../understanding/faq.md#where-does-bear-store-temporary-files))
sits in the build's current working directory here, since wrapper is
the default on this platform. If the build cannot find its compiler
after Bear starts, check that the working directory Bear ran from is
the one the build actually searches `PATH` from.

## Homebrew-installed compilers

Bear recognizes GCC and Clang drivers by filename (see [Supported
compilers](../reference/supported-compilers.md)), including versioned
names such as `gcc-14` or `g++-13`. Homebrew's GCC formula installs
only versioned binaries (`gcc-14`, not `gcc`), and Bear picks those up
automatically; no configuration is needed for the common case. If a
Homebrew compiler still is not recognized - a nonstandard rename, or a
driver outside the GCC/Clang families - add it explicitly:

```yaml
schema: "4.2"
compilers:
  - path: /opt/homebrew/bin/my-cc
    as: gcc
```

Bear itself is also available through Homebrew:

```sh
brew install bear
```

Related: [how Bear works](../understanding/how-it-works.md) for the preload/wrapper
mechanism, [Troubleshooting](../guides/troubleshooting.md) for output that
comes out wrong, and the [Recipes](../guides/recipes/index.md) index for other
tasks.
