# A failed build still publishes its output

## Context

Bear writes each output file all or nothing: it keeps the previous file
unless the run succeeds. A build that runs and exits non-zero raises the
question of what counts as success.

One option was to treat a failed build like any other failure and keep
the previous output. That throws away the entries the build did produce,
so a build that fails in one file would give the user nothing.

A failed build and an interrupted build are different. An interrupted
build is one where Bear itself got a termination signal while the build
ran. That rule is in
[interrupted-build-and-output-mode](interrupted-build-and-output-mode.md).
A build killed by something else (the OOM killer, a `kill` on make's
pid) never touched Bear. It is a failed build, and it publishes. Splitting
"failed" in two would make an OOM-killed make lose its database where a
make that fails keeps it.

## Decision

A build that runs and fails still publishes. Its exit status is the
build's result, not a failure of Bear. Only an error of Bear's own keeps
the previous output. A build that dies without a signal to Bear is a
failed build.

## Consequences

The database reflects what the build compiled, even when the build
failed. A build that fails before it compiles anything replaces a good
database with an empty one, unless the run is appending.

## References

- [output-atomic-write](../requirements/output-atomic-write.md), the requirement this supports
- [interrupted-build-and-output-mode](interrupted-build-and-output-mode.md), where an interrupted build differs
- [append-default-migration](append-default-migration.md), the related risk of an empty run replacing a good database
