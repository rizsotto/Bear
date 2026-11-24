# Reports travel over a UNIX socket on Unix hosts

## Context

Every intercepted execution opens one connection to the Bear process,
writes one record, and closes. Up to 4.2 that was a TCP connection on
the loopback interface on every platform. It had four problems:

- A build step in its own network namespace cannot reach the loopback
  port. Its reports fail with "Network is unreachable" and its entries
  are missing, with no warning. Bitbake runs every task in its own
  network namespace unless the task asks for the network
  ([discussion #724](https://github.com/rizsotto/Bear/discussions/724),
  [#498](https://github.com/rizsotto/Bear/issues/498)).
- Any local user can connect to the port and add entries
  ([discussion #665](https://github.com/rizsotto/Bear/discussions/665)).
- Each connection leaves a socket in `TIME_WAIT`. A measured run of
  60000 reports left 30000 of them. Linux reuses them on loopback;
  other systems may run out of ports.
- A process that outlives the build keeps the port number. Later it can
  hit another program, or another Bear run, on the same port.

On the same machine a report took 68 microseconds over TCP and 10 over
a UNIX socket.

Options considered:

- Keep TCP everywhere. Rejected for the reasons above.
- A Linux abstract socket. Rejected: it belongs to the network
  namespace, so it fails in the same builds, it has no permission
  check, and it is Linux only.
- A pipe or socket pair that the build inherits. Rejected: build tools
  close the descriptors they do not know, and many writers on one pipe
  mix records larger than `PIPE_BUF`.
- The socket in `.bear/` in the build directory. Rejected: a socket
  path holds about 104 bytes, build trees are often on file systems
  without sockets, and the preload mechanism has no `.bear/`.
- One fixed socket path per user. Rejected: it needs stale-socket and
  pid-reuse handling. A new directory per run needs none.
- `/tmp` alone as the second base. Rejected: macOS has no
  `XDG_RUNTIME_DIR` but a per-user `TMPDIR`, and some systems have no
  `/tmp`. A `TMPDIR` that is too long is skipped like any other base.
- Always private, also for root. Rejected for now: a root Bear whose
  build drops to a build user would lose those reports with no
  warning. A missing entry nobody notices is worse than a socket a
  local user could write to.
- Always open to everybody. Rejected: it gives up the access control
  for the common case, a normal user.
- A config key with raw permission bits. Rejected: the only values
  that work are "this user" and "everybody"; 0755 looks right and
  fails.
- A non-blocking `poll` loop that serves the listener and every open
  connection at once. Rejected: it handled failure modes no user
  reported (descriptor exhaustion, a stuck accept, slow writers that
  interleave), cost about 500 lines, and took several macOS CI rounds
  to get right. The blocking loop it replaced never caused a user
  problem.
- A reporter retry on a reset connection. Rejected: it cannot see a
  reset that arrives after the write returned, which is how FreeBSD
  loses reports in a burst, and its effect was never measured.
- A delivery acknowledgement: the collector answers each record and
  the reporter waits for it. It is the only real fix for a silent loss.
  Rejected: it changes the timing of every report on every platform,
  not worth it for a fallback.
- UNIX sockets on Windows too. Rejected for now: the standard library
  has them only behind a feature flag
  ([rust-lang/rust#150487](https://github.com/rust-lang/rust/issues/150487)).

## Decision

On Linux, macOS and the BSDs Bear listens on a UNIX stream socket named
`sock`, in a new directory `bear-<random>`, under the first of
`XDG_RUNTIME_DIR`, `TMPDIR` and `/tmp` that works. The directory is
private (0700) to the user who runs Bear. When Bear runs as root it is
open to everybody (0711 directory, 0777 socket), so a build that drops
to a build user still reports. `intercept.socket.access` overrides
both ways. The address reaches the reporters as text: `unix:` and the
path. `intercept.socket.dir` picks the base directory. When no socket
can be created and there is no `intercept.socket` section, and on
Windows, Bear uses loopback TCP as before. The record format is the
same on both.

One blocking loop serves both transports on every platform: accept a
connection, read one record, close it. Each read gives up on a silent
peer after a short timeout. To stop, Bear connects to its own address
and sends a zero length, which no reporter sends. On the UNIX socket
the accept queue is in connect order, so when the loop reads that
marker it has read every report that was complete before the stop.

## Consequences

- A build step in its own network namespace is captured, as long as it
  shares the file system.
- A build step that changes its root or hides the directory of the
  socket is not captured. TCP reached it. A build step that runs as
  another user is captured only when Bear runs as root or the key
  says `all`.
- A root run is as open as TCP was. Revisit this default once a failed
  report is visible to the user: then a private socket for root, plus
  a clear warning, becomes the better trade.
- A full accept queue blocks the reporter on Linux. On macOS and the
  BSDs it refuses the connection, so the reporter retries for up to
  511 ms. A stale socket also refuses, so a build that outlives a
  killed Bear pays that wait on every execution.
- On TCP, FreeBSD loses reports in a burst that overflows the accept
  queue, mostly with no error on either side. In CI, 16 threads sending
  500 reports each delivered 3033 of 8000. We believe the kernel resets
  the overflowing connections after the reporter's write returned; this
  is not verified.
- The minimum Rust version is 1.90. The preload library runs in
  processes that die on `SIGPIPE`, and the standard library's UNIX
  socket write suppresses it only from 1.90. Its vectored write still
  does not, so the reporter must not use it.
- The collector finds its own socket by path when it stops. A build
  that deletes the socket makes Bear end with an error and write no
  output. A workaround would need state the loop does not have, for a
  case no user reported.
- One connection is read at a time. A silent connection holds up the
  others for the read timeout. On a UNIX socket it is 200 ms. The rule:
  it stays below the 511 ms that a reporter retries a refused connect
  on macOS and the BSDs, so a stuck peer cannot outlast the reporters
  waiting behind it. The 511 ms budget covers two stuck peers in a row
  at 200 ms, as 255 ms did at the earlier 100 ms. On TCP it is 1 s, because Linux drops the last
  handshake packet when the accept queue is full, and that report
  comes on a resend 200 ms or more later. The kernel retries a TCP
  connect to a full queue, so there is no reporter retry budget to
  stay under. The reporter encodes the record before it connects and
  sends it in one write, so an honest reporter is rarely cut off. On a
  loaded machine it still can be: with a 100 ms timeout a Yocto build
  lost 2 of about 50000 reports. A connection cut off without a
  complete record is a lost entry, and Bear warns with how long it
  waited and how many bytes of the record came.
- The timeout is per read, so a peer that sends a byte just under
  every timeout is not bounded. Only a hostile local process does
  that; accepted.
- On TCP the order of the accept queue holds only while it has not
  overflowed. After an overflow a connection that Linux queues on a
  resend can come after the stop marker, and that report is lost. TCP
  is the fallback on Unix and the transport on Windows; the UNIX
  socket has no such case.
- The loop stays in blocking mode, so no accepted socket inherits
  `O_NONBLOCK` from the listener, as it does on macOS and the BSDs.
- A Bear killed by `SIGKILL` leaves the directory. Nothing sweeps old
  directories, because a sweep could remove the socket of a live run.
- Two transports stay in the code. Revisit Windows when the standard
  library has stable UNIX sockets there.

## References

- [interception-report-channel](../requirements/interception-report-channel.md), the requirement this supports
- [interception-mechanism-selection](../requirements/interception-mechanism-selection.md), what a lost report means for the build
- [abort-on-signal](abort-on-signal.md), how the socket directory is removed on an abort
- [#498](https://github.com/rizsotto/Bear/issues/498), [#704](https://github.com/rizsotto/Bear/issues/704), [discussion #665](https://github.com/rizsotto/Bear/discussions/665), [discussion #724](https://github.com/rizsotto/Bear/discussions/724)
