<!-- Diataxis type: how-to -->

# Intercept a 32-bit build on a 64-bit host

A build that runs 32-bit compilers or 32-bit build tools on a 64-bit
host records nothing from them: the build still exits `0`, no error
appears, and `compile_commands.json` just comes out short or empty. The
preload library a normal Bear install ships is built for the host's
64-bit ELF class, and the dynamic linker cannot inject a 64-bit shared
library into a 32-bit process (see [How Bear
works](../../understanding/how-it-works.md#two-ways-to-capture-a-real-build)
for what preload does and why the ELF class has to match). [Exit
status](../../reference/exit-status.md#notes) covers why a missing
preload library never turns into a non-zero exit: the dynamic linker
treats it as something to warn about and skip, not to fail on, so
nothing about the run tells you a source file went unrecorded.

On glibc Linux this is fixable: build a second, 32-bit preload library,
install it alongside the normal 64-bit one, and point `bear-driver` at
both with a single `INTERCEPT_LIBDIR` setting. `INTERCEPT_LIBDIR` names
the directory `bear-driver` looks in for the preload library, relative
to its own `bin/`, and on glibc it accepts the literal string `$LIB` as
that name. The dynamic linker itself expands `$LIB` at load time, per
process: `lib64` when loading into a 64-bit process, `lib` when loading
into a 32-bit one. One install then serves both kinds of process,
provided both libraries exist on disk.

## Get a 32-bit toolchain and Rust target

```sh
# Fedora
sudo dnf install glibc-devel.i686 libgcc.i686 libatomic.i686

# Debian / Ubuntu
sudo apt install gcc-multilib

rustup target add i686-unknown-linux-gnu
```

For any other distribution, install whatever it calls its 32-bit
development packages; the two lines above are only confirmed for
Fedora.

## Work around the missing cross driver

The preload library's build script compiles a small C shim with
`cc-rs`, which looks for a `i686-linux-gnu-gcc` cross driver. Fedora (and
any distribution that provides multilib through `gcc -m32` rather than a
separate cross-driver binary) has no such executable, so the build fails
with:

```
error occurred in cc-rs: failed to find tool "i686-linux-gnu-gcc"
```

There is no supported fix for this yet; the workaround is a two-line
shim on `PATH` that execs `gcc -m32`, pointed to by
`CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_LINKER`:

```sh
cat >/tmp/i686-linux-gnu-gcc <<'EOF'
#!/bin/sh
exec gcc -m32 "$@"
EOF
chmod +x /tmp/i686-linux-gnu-gcc
export PATH="/tmp:${PATH}"
export CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_LINKER=/tmp/i686-linux-gnu-gcc
```

## Build the 32-bit preload library

```sh
INTERCEPT_LIBDIR='$LIB' cargo build -p intercept-preload \
    --target i686-unknown-linux-gnu
```

This produces `target/i686-unknown-linux-gnu/debug/libexec.so`, a 32-bit
`libexec.so` distinct from the normal 64-bit one.

## Build and install the 64-bit Bear, then place both libraries

Build and install the regular 64-bit Bear with the same `$LIB` setting:

```sh
INTERCEPT_LIBDIR='$LIB' cargo build
SRCDIR=target/debug PREFIX=<prefix> INTERCEPT_LIBDIR='$LIB' ./scripts/install.sh
```

(See [Install Bear](../../installation.md) for what `PREFIX`, `DESTDIR`,
and `INTERCEPT_LIBDIR` each control; the `$LIB` value here is what makes
this recipe work and is on top of what that page documents.)

`install.sh` does not understand `$LIB` as a dynamic-linker token: it
creates a directory literally named `$LIB` under `<prefix>/libexec/bear/`
and installs the 64-bit library there. Finishing the layout is a manual
step, and there is currently no supported way to have the build or
install scripts do it for you:

```sh
cd <prefix>/libexec/bear
mkdir -p lib lib64
mv 'target/i686-unknown-linux-gnu/debug/libexec.so' lib/libexec.so
mv '$LIB/libexec.so' lib64/libexec.so
rmdir '$LIB'
```

## Confirm it worked

Run the build under `bear -- <build>` as usual. If it drives any 32-bit
compiler or a 32-bit build tool that execs a compiler on its own behalf,
compare the entry count in `compile_commands.json` with and without
`lib/libexec.so` in place: removing the 32-bit library drops exactly the
entries that 32-bit process would have contributed, while the build
itself still exits `0`. With the library missing, the dynamic linker also
prints the warning covered in [Troubleshooting: LD_PRELOAD
errors](../troubleshooting.md#ld_preload-errors) - `cannot be preloaded
(cannot open shared object file): ignored.` - which is the tell that this
is the missing-32-bit-library case rather than some other silent gap.

## Limitations

- This works only on glibc Linux. musl, macOS, and the BSDs have no
  `$LIB`-style token: they need a concrete library directory name
  instead, so one install cannot serve two ELF classes the same way.
- The `cc-rs` shim above is a workaround for a missing cross driver, not
  a fix; a distribution that ships `i686-linux-gnu-gcc` directly does not
  need it.
- `scripts/install.sh` has no option to produce the `lib` / `lib64` split
  itself. Repeat the manual step after every reinstall.

This is a different problem from a vendor cross-toolchain whose compiler
driver is itself a 32-bit binary; that case, and wrapper mode as its fix,
is covered in [Generate compile_commands.json when
cross-compiling](cross-compilation.md#when-preload-cannot-reach-the-compiler).

## Related pages

- [Generate compile_commands.json when cross-compiling](cross-compilation.md)
  for the general cross-prefix and wrong-ELF-class story.
- [Install Bear](../../installation.md) for `PREFIX`, `DESTDIR`, and
  `INTERCEPT_LIBDIR`.
- [Exit status](../../reference/exit-status.md) for why Bear exits `0`
  even when a preload library never loaded.
- [Troubleshooting](../troubleshooting.md) for `LD_PRELOAD` errors and
  other silent interception failures.
- [Recipes](index.md) for the rest of the task pages.
