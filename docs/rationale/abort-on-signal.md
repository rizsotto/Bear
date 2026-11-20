# Abort on a signal when no build runs

## Context

Bear creates temporary artifacts: the output temp file, the wrapper
directory `.bear/`, and the directory of the report socket. A termination signal that arrives when no build runs
would, with the default action, kill Bear and leave them behind. Their
drops never run. This is also true for `SIGHUP` on a closed terminal,
if Bear does not watch it.

"No build runs" covers the semantic and parse-sh modes, the start of a
build run up to Bear's last check before the build is spawned, and the
end of it while Bear writes. A signal that comes after that check and
before the spawn is already queued when the build starts. Bear relays it
to the build, whatever its cause. A terminal key that lands in that
instant may reach the build twice.

Options:

- Leave the default action when no build runs. Rejected: it leaves the
  temp file.
- Cooperative cancel: a flag, `poll` on every read, channel timeouts.
  Rejected: it touches every producer and consumer, and still cannot
  cancel a slow write.
- The main thread as an event loop that owns the artifacts. Rejected: it
  needs the writer stack reshaped so the coordinator owns the file. A big
  refactor for the same result.
- A clean unwind. Rejected: a blocked read is not interrupted by a
  handled signal (the handlers restart system calls), so `bear semantic`
  waiting on stdin cannot be told to stop. And the temp file belongs to
  the consumer thread, deep in the writer stack.

## Decision

The driver registers the termination signals once, before anything
creates an artifact. When no build runs, a small watcher thread reads
the signal source. On a signal it removes the temporary artifacts and
ends like a killed child would
([end-like-the-build](end-like-the-build.md)): it dies by that signal.
Before it dies it prints one warning line, so the user sees why Bear
stopped.

The artifacts are known through a small process-wide list, the cleanup
registry. The file writer and the `.bear/` directory add their path when
they create it, and remove it when they are done.

During the build, `supervise` reads the signal source. When the build
ends, the source moves into the watcher thread. A signal that arrived
in between is kept, so nothing is lost and no flag is shared.

The wrapper registers on its own schedule
([supervise-a-compiler-call](supervise-a-compiler-call.md)).

## Consequences

- Every mode has the same rules. One registration, in every mode.
- `SIGKILL` and `SIGSTOP` cannot be caught. The temp file, `.bear/` and
  the socket directory may stay. `.bear/` is wiped by the next run. The
  other two are not.
- A new place that creates an artifact must use the registry.

## References

- [output-atomic-write](../requirements/output-atomic-write.md), the requirement this supports
- [interception-report-channel](../requirements/interception-report-channel.md), the socket directory it removes
- [cli-signal-forwarding](../requirements/cli-signal-forwarding.md), the signal rules
- [interrupted-build-and-output-mode](interrupted-build-and-output-mode.md), the interrupt during the build
- [end-like-the-build](end-like-the-build.md), how Bear dies
