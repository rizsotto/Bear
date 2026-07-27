<!-- Diataxis type: how-to -->

# Bear on BSD

Bear runs on FreeBSD, OpenBSD, NetBSD, and DragonFly BSD with the same
defaults as Linux: preload is the default interception method (see [how
Bear works](../understanding/how-it-works.md) and [Configure
Bear](../reference/configuration.md#intercept)), and it cannot see into
a statically linked build tool. What differs is packaging and the
default compiler and linker names.

## `cc` and `c++` mean Clang here

On FreeBSD, OpenBSD, NetBSD, and DragonFly BSD (and on macOS), the base
system's `cc`/`c++` is Clang, not GCC as on most Linux distributions.
Bear resolves this by probing `--version`; no configuration is needed
for the common case. See [Supported
compilers](../reference/supported-compilers.md#ambiguous-names) to
override when the probe cannot classify a compiler.

## Installing

FreeBSD packages Bear (`pkg install bear`). On OpenBSD, NetBSD, and
DragonFly BSD, build from source per
[`INSTALL.md`](https://github.com/rizsotto/Bear/blob/master/INSTALL.md);
check your ports/pkgsrc tree first.

Related: [how Bear works](../understanding/how-it-works.md) for the
preload mechanism, [Troubleshooting](../guides/troubleshooting.md) for
output that comes out wrong, and the
[Recipes](../guides/recipes/index.md) index for other tasks.
