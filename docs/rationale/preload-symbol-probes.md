# How the build finds the functions the preload library wraps

## Context

The preload library wraps a fixed list of libc functions (the `exec`
family, `posix_spawn*`, `popen`, `pclose`, `system`), but not every libc
has all of them: `execvpe` is missing on macOS, `execvP` is BSD only,
and bionic adds `posix_spawn` at API level 28. The build has to find
out which ones the target libc exports, and only wrap those.

The first version ran its probes in the build script of a
build-dependency crate. Cargo builds such a crate for the host, so the
probes saw the host libc, and a cross build for Android got glibc's
answers. The probes also only compiled a reference to the name, so they
checked that a header declares it, not that libc exports it.

Other tools solve the same problem. Autoconf `AC_CHECK_FUNC` and Meson
`has_function` link a test program that declares the function itself,
with no header, and they use the cross compiler when cross compiling.
CMake `CheckSymbolExists` includes the header and links. Gentoo's
sandbox, another preload library, filters a fixed list against the
symbol table of the real libc. No Rust crate probes C symbols; the
`libc` crate hardcodes its bindings per target.

Options considered:

- Probe once in a build-dependency and share the answers. Wrong target
  when cross compiling, as above.
- Probe once in the preload crate and hand the answers to the
  integration tests through `links` metadata. The tests would need the
  preload crate as a normal dependency, with the risk that its exported
  wrappers get linked into the test binary.
- Let the tests read the exports of the built library at run time. That
  turns compile-time test gates into run-time skips, and the tests still
  need to know which functions the host C compiler can call.
- Header and link probe, as in CMake. It also rejects a function libc
  exports but the header hides, such as `posix_spawn` below API 28 on
  Android. The shim defines the functions and never calls them, so the
  declaration does not matter to it.
- Pick by platform macro, such as `__APPLE__` for `_NSGetEnviron`. It
  gives the same answers on today's targets, but the build then holds
  two ways to decide the same kind of question.
- Wrap `posix_spawn*` on Android below API 28 anyway and look the real
  function up at run time. It keeps the interception on newer devices,
  but it overrides the probe for one platform.
- Wrap musl's `__execvpe`. musl has hidden the symbol since 2018, so no
  program outside libc can call it, and calls inside libc bind locally.

## Decision

Each crate that needs the answers runs the probes from its own build
script, against the target it is built for, from one shared probe list.
A function counts as present when a test program that declares it
itself links against the target libc. Optional headers are probed by
including them. The build fails when `execve` is not found, because
that means the toolchain is broken. The extension macros that autoconf's
`AC_USE_SYSTEM_EXTENSIONS` defines are passed as compiler flags on
every target.

## Consequences

The answers follow the target, so a cross build wraps what the target
libc exports. The probes run twice per build, once for the preload crate
and once for the integration tests; with the same list and the same
compiler they agree.

On Android the answer follows the API level the build targets: a build
for API 24 does not wrap `posix_spawn*`, and a build for API 28 or later
does. On musl, calls between functions inside libc (for example `execvp`
calling `__execvpe`, or `popen` calling `posix_spawn`) cannot be
intercepted; only the public entry points can.

Revisit when a target needs a function that a link probe cannot see,
such as a macro or an inline function.

## References

- [interception-preload-mechanism](../requirements/interception-preload-mechanism.md), the requirement this supports
- [preload-linker-selection](preload-linker-selection.md), how the wrapped functions are exported
- [Autoconf AC_CHECK_FUNC](https://github.com/autotools-mirror/autoconf/blob/master/lib/autoconf/c.m4)
- [Meson has_function](https://mesonbuild.com/Reference-manual_returned_compiler.html)
- [CMake CheckSymbolExists](https://cmake.org/cmake/help/latest/module/CheckSymbolExists.html)
- [Using newer APIs on Android](https://developer.android.com/ndk/guides/using-newer-apis)
- [musl commit 0676c3a3](https://git.musl-libc.org/cgit/musl/commit/?id=0676c3a34c7bf12b33f8f5efb92476f4ffc7f20e), hides `__execvpe`
- [#705](https://github.com/rizsotto/Bear/issues/705), the s390x report that led to the `__execvpe` wrapper
