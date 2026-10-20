---
title: Resolve past masquerade wrappers in the wrapper mechanism
status: implemented
---

## Intent

Some distributions ship compiler masquerade wrappers: a directory of
symlinks named after compilers that point at ccache, icecream, or a
similar tool, placed on PATH. When the user runs a build under Bear
with the wrapper mechanism on such a host, Bear must not loop with the
masquerade wrapper. The database records the real compiler, and the
build completes. The user does not strip any directory from PATH to
make this work.

## Acceptance criteria

- **Given** a host where `/usr/lib64/ccache/gcc -> /usr/bin/ccache` is
  first in PATH, **when** the user runs a build under Bear with the
  wrapper mechanism, **then** the build completes without hanging, the
  database holds one entry per source file, and each recorded compiler
  is an absolute path to the real compiler, never the masquerade
  wrapper and never a wrapper in `.bear/`.
- **Given** a masquerade directory on PATH, **when** the user runs a
  build under Bear, **then** the user does not have to strip any
  directory from PATH, unset any environment variable, or set `CCACHE_*`
  variables. Bear filters only its own lookup. The build's PATH keeps
  the masquerade directory, behind the wrapper directory.
- **Given** a compiler variable whose value resolves, past every
  masquerade directory, to no real compiler, **when** Bear starts,
  **then** it logs a warning naming the compiler and the masquerade
  directories it excluded, registers no wrapper for that name, and does
  not fall back to the masquerade wrapper. The build then runs that
  compiler without interception.
- **Given** one compiler that cannot be resolved past the masquerade
  directories, **when** Bear starts, **then** Bear still runs, and the
  other compilers are still wrapped.

## Non-functional constraints

- A masquerade wrapper is detected by inspecting the filesystem only.
  No subprocess is spawned to identify it.
- Detection follows the symlink of the found path, rather than matching
  known directory locations. Any directory of compiler-named symlinks
  to a recognized wrapper binary is detected, wherever it is installed.

## Known limitations

**Fixed set of recognized wrapper names**: a masquerade wrapper whose
name is not in Bear's fixed set is not detected. The user can drop it
from PATH or use the preload mechanism.

**Symlink-only detection**: a masquerade wrapper installed as a shell
script or a hard copy, rather than a symlink, is not detected.

**Masquerade tool not run**: while Bear observes the build, the wrapped
compilers bypass the masquerade wrapper, so ccache and similar tools do
not run for them.

**Silent skip in the PATH scan**: when Bear scans PATH for compilers,
it skips masquerade links without a message. Only a compiler named in
a compiler variable gets the warning.

## Rationale

- [Filter Bear's lookup PATH, not the alternatives](../rationale/wrapper-recursion-ccache-alternatives.md)
