<!-- Diataxis type: how-to -->

# Bear on BSD

Bear runs on FreeBSD, OpenBSD, NetBSD, and DragonFly BSD, with the same
interception methods as Linux and the same default, preload: Bear
injects a small library into every process the build starts, and it
cannot see into a statically linked build tool for the same reason as
on Linux. See [how Bear works](../understanding/how-it-works.md) for
the mechanism, and force wrapper mode instead with [`intercept.mode:
wrapper`](../reference/configuration.md#intercept). What differs from
Linux here is the packaging and the default compiler and linker names a
build is likely to use.

## `cc` and `c++` mean Clang here

Bear resolves the ambiguous basenames `cc`, `c++`, and the HPE Cray
`CC` by probing the executable's `--version` output, precisely because
the same name is a different compiler depending on the platform: on
FreeBSD, OpenBSD, NetBSD, and DragonFly BSD (and on macOS) the base
system's `cc`/`c++` is Clang, while on most Linux distributions the
same names are GCC. No configuration is needed for the common case;
the probe classifies it automatically. Override the classification only
when the probe cannot - for example a locally built compiler with a
banner it does not recognize - with a `compilers:` entry (see
[Supported compilers](../reference/supported-compilers.md#ambiguous-names)):

```yaml
schema: "4.2"
compilers:
  - path: /usr/bin/cc
    as: clang
```

## Installing

FreeBSD ships Bear as a package:

```sh
pkg install bear
```

On OpenBSD, NetBSD, and DragonFly BSD, build from source with `cargo
build --release` and `./scripts/install.sh` as described in
[`INSTALL.md`](https://github.com/rizsotto/Bear/blob/master/INSTALL.md);
check your ports/pkgsrc tree for a Bear package before doing so.

Related: [how Bear works](../understanding/how-it-works.md) for the preload
mechanism, [Troubleshooting](../guides/troubleshooting.md) for output that
comes out wrong, and the [Recipes](../guides/recipes/index.md) index for other
tasks.
