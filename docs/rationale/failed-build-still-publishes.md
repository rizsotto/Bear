# A failed build still publishes its output

## Context

Bear writes each output file all or nothing: it keeps the previous file
unless the run succeeds. A build that runs and exits non-zero raises the
question of what counts as success.

One option was to treat a failed build like any other failure and keep
the previous output. That throws away the entries the build did produce,
so a build that fails in one file would give the user nothing.

## Decision

A build that runs and fails still publishes. Its exit status is the
build's result, not a failure of Bear. Only an error of Bear's own keeps
the previous output.

## Consequences

The database reflects what the build compiled, even when the build
failed. A build that fails before it compiles anything replaces a good
database with an empty one, unless the run is appending.

## References

- [output-atomic-write](../requirements/output-atomic-write.md), the requirement this supports
- [append-default-migration](append-default-migration.md), the related risk of an empty run replacing a good database
