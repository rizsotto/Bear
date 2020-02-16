# Intercept Library

This sub project implements intercept logic which is using the operating
system [dynamic loader][DYN_LOADER] pre-load functionality.

  [DYN_LOADER]: https://en.wikipedia.org/wiki/Dynamic_linker

## When to use this way of intercepting?

This method of intercepting compiler calls can work only if the following
conditions are met:

- The operating system is supported by this code.
  (Linux, FreeBSD, OSX were tried and tested)
- The operating system do support (or enable) library preload.
  (Notable example for enabled security modes is the [SIP][OSX_SIP] in
  recent OSX versions.)
- Executables which are linked dynamically.
  (Few distribution does ship statically linked compilers, which will be
  not working this method.)

  [OSX_SIP]: https://support.apple.com/en-us/HT204899

## How it works?

This project implements a shared library and a statically linked executable.

The library can be pre-loaded by the dynamic linker of the Operating System.
It implements a few function related to process creation. By pre-load this
library the executed process uses these functions instead of those from the
standard library.

The idea here is to hijack the process creation methods: do not execute
the requested file, but execute another one. The another process is a
supervisor process, which executes the requested file, but it also
reports the lifecycle related events, like start, stop or signal received.

## Limitations

* If a process can be executed it will execute it. But if the execution request
would fail for some reason, it might report a successful execution because
the statically linked executable will start (but the child process might
fail).

  The type of errors that are might not detected by the library are: E2BIG,
EAGAIN, EINVAL, EIO, ELIBAD, ELOOP, ENFILE, ENAMETOOLONG, ENOEXEC, EPERM.

* The IO redirection might not working properly. Since the requested execution
is not a direct child process (indirect child process relationship) the standard
input/output might not be closed/forwarded as requested.

* `posix_spawn` and `posix_spawn` some of the attributes might not making
effect, because the indirect child process relationship.

* There are still a few POSIX system call that are not covered by the library.
(Like `execveat` or `fexecve` which are using file descriptor to identify
the file to execute.)

With these limitation, an average build process can still be intercepted.

## Implementation details

### `libexec`

It's a shared library.

- It's written in C++ 14.
- It's using symbols only from the `libc` and `libdl`.
- Memory handling:
   - It does not allocates heap memory. (no malloc, no new)
   - It allocates static memory and uses the stack.
- Error handling:
   - Any error is fatal.
   - Errors are reported on `stderr` only if it was requested.

## `intercept`

It's statically linked executable.

- It's written in C++ 17.
- It's using the standard library and some 3rd party libraries.
- Memory handling:
   - It does allocates heap memory.
- Error handling:
   - Any error is fatal.
   - Errors are reported on `stderr` only if it was requested.
