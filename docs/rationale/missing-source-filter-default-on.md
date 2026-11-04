# Missing-source filter is on by default and checks once

## Context

Bear used to drop entries whose source file did not exist, and did so by
default. The option was removed in 4.0.3 with no recorded reason, but
configurations kept the key. Bear ignores unknown keys, so since then
those configurations parse and the filter silently does nothing.

Off by default was considered, to keep this out of the release that
makes append the default. It was not chosen. An entry for a file that
does not exist is of no use to any tool, so dropping it is a fix, not a
break. Off would also keep the silent regression for every user who
never set the key. The default of this filter does not depend on the
append default: it is useful with overwrite too, and with appending as
it works today.

Parse-sh mode is the one place where on hurts: a log copied from
another machine names sources that are not here, so its database comes
out empty. A default that depends on the mode was considered and not
chosen, since the same key would mean different things per run. The
cost is that such users must turn the option off, and the run reports
an error when every entry is dropped, so the empty output is never
silent.

In combined mode Bear writes entries while the build runs, so each
entry is checked shortly after its compiler call starts. A file that
the build deletes later, such as a `configure` probe, usually passes
the check. Two ways to close that gap were considered. One was to write
the entries to a temporary file and check them all after the build.
The other was a three-value option that lets the user pick checking
during or after the build. Both were rejected. The first adds a second
pass and a second pipeline shape. The second makes the user choose a
result that depends on timing, and that result cannot be tested
reliably. A run that is appending with a build that runs no compiler
already checks every existing entry after the fact, so a user who
needs a clean database has a simple way to get one.

## Decision

Restore the option, on by default. Check each entry once, as it passes
through the output, and document a no-op appending run as the way to
trim a database.

## Consequences

The filter stays one step in the existing output pipeline. Results for
files a build deletes differ between combined mode and semantic mode,
and the documentation has to say so.

Revisit when Bear gains a run-wide way to say the input came from
another machine: that switch should turn this check off. Also revisit
if users report leftover entries that the trim run does not
fix, or if the check turns out to be costly on slow file systems.

## References

- [output-missing-source-filter](../requirements/output-missing-source-filter.md), the requirement this supports
- [output-append](../requirements/output-append.md), the behavior the trim run relies on
