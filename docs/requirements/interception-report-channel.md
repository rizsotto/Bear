---
title: Report channel between the build and Bear
status: accepted
---

## Intent

Both interception mechanisms report each execution to the Bear process
that runs the build. On Linux, macOS and the BSDs the reports travel
over a UNIX domain socket in the file system, so the build needs no
working network. Only the user who runs Bear can send reports, unless
Bear runs as root or a configuration option opens the socket to every
user. On Windows the reports travel over a TCP connection on the
loopback interface. The channel needs no setup. What a lost report
means for the build is owned by `interception-mechanism-selection`.

## Acceptance criteria

- **Given** a Linux, macOS or BSD host, a user other than root, and no
  socket configuration, **when** a build runs under Bear, **then** a
  socket exists for the time of the build, in a directory that only
  that user can enter.
- **Given** Bear runs as root and no socket configuration, **when** a
  build step switches to another user and runs `cc -c test.c`,
  **then** the entry for `test.c` appears in the database.
- **Given** a configuration option that limits the socket to the user
  who runs Bear, **when** Bear runs as root, **then** only root can
  connect to the socket.
- **Given** a configuration option that opens the socket to every
  user, **when** Bear runs as any user, **then** every user can
  connect to the socket.
- **Given** no socket configuration and `XDG_RUNTIME_DIR` names a
  writable directory, **when** a build runs under Bear, **then** the
  socket directory is created there. **Given** it is unset, not
  writable, or too long for a socket address, **then** `TMPDIR` is
  tried the same way, then `/tmp`, and the build is captured the same
  way.
- **Given** no socket configuration and a host where no socket can be
  created in any of these places, **when** a build runs under Bear,
  **then** Bear reports over the loopback interface instead and the
  build is captured.
- **Given** a configuration option that names an existing directory
  for the socket, **when** a build runs under Bear, **then** the socket
  directory is created inside it, and no other place is tried.
- **Given** any socket configuration and no socket can be created
  where it says, **when** Bear starts, **then** it exits with an error
  that names the path, before the build runs, and does not fall back
  to the loopback interface.
- **Given** a Linux, macOS or BSD host and a process that started a
  report but stays silent, **when** the build ends, **then** Bear
  still finishes and writes the other entries.
- **Given** a build step that runs in its own network namespace and
  shares the file system (`unshare --net`), **when** it runs
  `cc -c test.c`, **then** the entry for `test.c` appears in the
  database.
- **Given** a build under Bear, **when** Bear exits, or a termination
  signal aborts it, **then** the socket and its directory are removed.
- **Given** a socket directory left behind by a Bear run that was
  killed, **when** Bear starts again, **then** the run is not affected.
- **Given** two Bear runs at the same time, one of them possibly
  started by the build of the other, **when** both builds compile,
  **then** each run gets only the reports of its own build.
- **Given** a Windows host, **when** a build runs under Bear, **then**
  the reports travel over a TCP connection on the loopback interface.
- **Given** a Windows host and any socket configuration, **when** Bear
  reads the configuration, **then** it reports a configuration error.

## Non-functional constraints

- Platform: UNIX domain socket on Linux (Android included), macOS and
  the BSDs; loopback TCP on Windows.
- The socket path that Bear adds below the base directory is short
  enough for the 104-byte socket address of macOS and the BSDs.

## Known limitations

**Build steps that run as another user**: when Bear does not run as
root, the socket directory is private to the user who runs Bear. A
build step that switches to another user cannot report, and its
compilations are missing from the database. The configuration option
that opens the socket to every user avoids this.

**Root runs are open to all local users**: when Bear runs as root and
no socket configuration says otherwise, any local user can connect to
the socket and add entries. The configuration option that limits the
socket to the user who runs Bear closes it.

**Build steps that cannot see the socket**: a step that changes its
root directory, or hides the directory of the socket, cannot report.
Pointing `XDG_RUNTIME_DIR`, `TMPDIR`, or the socket directory option
at a directory the step can see avoids the second case.

**The socket is removed during the build**: when a build deletes the
socket or its directory (for example by cleaning `/tmp`), Bear ends
with an error when the build ends and writes no output. The empty
socket directory may stay behind.

**Bear killed by a signal it cannot catch**: the socket directory
stays behind.

**Windows**: any local user can connect to the loopback port.

## Rationale

- [Reports travel over a UNIX socket on Unix hosts](../rationale/report-channel-unix-socket.md)
- [Abort on a signal when no build runs](../rationale/abort-on-signal.md)
