<!-- Diataxis type: reference -->

# Exit status

Modes that run a build report the build's own outcome; modes that only
transform data report whether Bear understood its input.

| Mode | Exit status |
|---|---|
| `bear -- <build>` (combined) | The build's exit code, byte for byte: `0` on success, the same non-zero code otherwise. |
| `bear intercept -- <build>` | Same as combined: the build's exit code. |
| `bear semantic` | `0` when at least one input event was understood, or the input was empty. Non-zero when the input was non-empty and every event was rejected. |
| `bear parse-sh` | `0` when at least one line parsed as a shell command, even when none of them named a recognized compiler, or when the input was empty. Non-zero when the input was non-empty and every line was skipped. |
| `bear semantic --print-compilers` | Always `0`. |
| `bear --help` (or a subcommand's `--help`) | Always `0`. |
| A malformed invocation (unknown flag, missing build command) | Non-zero (`2`), reported before anything runs. |

See [command-line options](command-line.md) for what each mode does.

## Notes

**A build killed by a signal never exits `0`, but the signal number is not
encoded.** `bear -- sh -c 'kill -TERM $$'` exits `1`, and so does a build
killed with `SIGKILL`. Bear does not follow the `128 + signal` shell
convention: a caller that only inspects Bear's own exit code cannot tell a
signalled build from an ordinary failing one. The signal itself still
reaches the build unchanged.

**Empty input is not an error.** An empty `semantic` or `parse-sh` run
writes an empty database, exits `0`, and prints a notice on standard
error so an accidentally empty pipeline does not pass unnoticed. The
same holds for `parse-sh` input that parses cleanly but names no
compiler invocation, such as a lone `mv` command: no line is skipped, no
diagnostic is printed, and the empty database is a success.

**Bear can fail on its own, independent of the build or the input.** A
configuration file that fails to parse, an invalid combination of flags
(for example directing the compilation database to standard output
while also running a build), or a named input file that does not exist
are all reported before any build runs or any input is read, and all
exit non-zero. An output path that cannot be written (a directory sitting
where the database file should go) is also non-zero, and this one can
surface after the build already succeeded: the build's own outcome does
not change, but Bear's exit code still reports non-zero because it could
not deliver its result.

A missing preload library is not one of these failures, and the
consequence is worth knowing: the dynamic linker ignores a preload target
it cannot open, prints its own warning on standard error, and lets the
build run. The build's own commands are still observed, but the processes
it spawns are not, so a build whose compiler runs under `make` yields an
empty database and an exit status of 0. Success here means the build
succeeded, never that the database is complete. See [Bear produces an
empty compile_commands.json](../guides/recipes/empty-compilation-database.md).

For what to do about a non-zero exit, see
[troubleshooting](../guides/troubleshooting.md).
