<!-- Diataxis type: how-to -->

# Bear on macOS

```sh
bear -- make
```

Wrapper interception is the default on macOS: Bear puts wrapper
executables ahead of the real compilers on `PATH`, so the build has to
pick them up. A configure step that discovers compilers must itself run
under Bear, or the rest of the build records nothing.

Preload interception can be forced in the configuration file, but it
works only with System Integrity Protection disabled, because SIP blocks
the library injection it relies on. With SIP enabled, forcing preload is
a startup error that names wrapper mode as the alternative: Bear does
not silently switch modes, and it does not run the build.

## What SIP blocks

System Integrity Protection is what makes wrapper mode the macOS
default in the first place: SIP strips `DYLD_INSERT_LIBRARIES` from
SIP-protected executables, and that is exactly the injection preload
mode depends on, so preload cannot intercept those executables while
SIP is enabled. Wrapper mode has no such restriction, because it
substitutes an executable on `PATH` instead of injecting into one.

## Where the wrapper directory lives

In wrapper mode Bear creates a `.bear/` directory in the build's current
working directory (not a system or per-user temporary directory), wipes
it at the start of each run, and removes it again once the build
finishes. If the build cannot find its compiler after Bear starts,
check that the working directory Bear ran from is the one the build
actually searches `PATH` from.

## Homebrew-installed compilers

Bear recognizes GCC and Clang drivers by filename, including versioned
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
