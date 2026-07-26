<!-- Diataxis type: tutorial -->

# Getting started with Bear

By the end of this page you will have built a small two-file C project
under Bear, looked at the `compile_commands.json` it wrote, and pointed
clangd at it so your editor gets working code navigation and
diagnostics.

## Install Bear

Install Bear with your platform's package manager; see
[Install Bear](installation.md) for the commands. Then confirm it is on
your `PATH`:

```sh
bear --version
```

## Create a sample project

Make a new directory and add two source files and a header:

```sh
mkdir hello-bear
cd hello-bear
```

`main.c`:

```c
#include "greet.h"

int main(void) {
    greet();
    return 0;
}
```

`greet.c`:

```c
#include <stdio.h>
#include "greet.h"

void greet(void) {
    printf("Hello from Bear!\n");
}
```

`greet.h`:

```c
#ifndef GREET_H
#define GREET_H

void greet(void);

#endif
```

And a hand-written `Makefile` that compiles the two sources separately
and links them, the way most real Make projects do:

```makefile
CC = cc
CFLAGS = -Wall

hello: main.o greet.o
	$(CC) $(CFLAGS) -o hello main.o greet.o

main.o: main.c greet.h
	$(CC) $(CFLAGS) -c main.c

greet.o: greet.c greet.h
	$(CC) $(CFLAGS) -c greet.c

clean:
	rm -f hello main.o greet.o
```

Building it directly with `make` works, but produces no record of how
each file was compiled:

```sh
make
```

## Build it under Bear

Run the same build again, this time prefixed with `bear --`:

```sh
make clean
bear -- make
```

Bear runs the build exactly as before, forwarding its output, and adds
one thing: a `compile_commands.json` file next to your sources, one
entry per compiled source file.

## Look at the result

```sh
cat compile_commands.json
```

```json
[
  {
    "file": "main.c",
    "arguments": [
      "/usr/bin/cc",
      "-Wall",
      "-c",
      "main.c"
    ],
    "directory": "/home/you/hello-bear"
  },
  {
    "file": "greet.c",
    "arguments": [
      "/usr/bin/cc",
      "-Wall",
      "-c",
      "greet.c"
    ],
    "directory": "/home/you/hello-bear"
  }
]
```

Your own output will show your project's actual directory and the
resolved path of your `cc`, but the shape is always this: one entry per
translation unit, `arguments` holding the exact compiler invocation as
an array, and `file` given relative to `directory`. There is no entry
for `greet.h`, because Bear only records commands the build actually
ran, and the build never invokes the compiler on a header directly. For
what Bear does and does not capture, and why, see
[How Bear works](how-it-works.md).

## Point clangd at it

clangd looks for `compile_commands.json` in the directory of the file
you are editing and then in each parent directory, so nothing further
needs configuring: open `main.c` in an editor with clangd support (the
`clangd` extension for VS Code, `coc-clangd` for Neovim, and similar),
and it picks up the database from the project root automatically. You
now have working "go to definition", completion, and diagnostics for
`hello-bear`, including across the `main.c` to `greet.h` include.

## Next steps

- [Generate compile_commands.json for a Makefile
  project](recipes/compile-commands-for-makefile.md) for the real-project
  version of what you just did: autotools, incremental and parallel
  builds, recursive Makefiles.
- [Set up clangd for a project without
  CMake](recipes/clangd-setup.md) if the editor side needs more than the
  default lookup.
- [Recipes](recipes/index.md), the index of task pages for build
  systems and situations beyond a plain Makefile.
- [Bear produces an empty
  `compile_commands.json`](recipes/empty-compilation-database.md) if a
  real project's database comes out empty, and
  [Troubleshooting](troubleshooting.md) if it comes out wrong.
- [How Bear works](how-it-works.md) for the interception mechanism
  behind this page.
