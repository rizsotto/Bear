# Naming the overwrite behaviour before it stops being the default

## Context

Bear overwrites `compile_commands.json` by default and accumulates only
when asked. That default has one bad failure mode: a run that produces
no compiler invocations, because the build was already up to date or
because interception failed, replaces a good database with an empty one.
The user loses working data by running the tool successfully. The
intended fix is to make accumulation the default, so a run that captures
nothing leaves the previous database alone.

Flipping a default is a compatibility break, and the scripts it breaks
are exactly those that never named the behaviour they depend on: a
packaging script or a CI job that runs Bear plainly and expects a fresh
database each time has no flag to point at. Nothing in the current
command line lets such a script say what it means. Until one exists, the
break has no migration path at all - the advice would be "wait for the
new release, then edit your scripts", with a window in which correct
scripts are silently wrong.

So the flag has to exist before the default moves, which means shipping a
flag that changes nothing. Its whole value is time in the wild: it must
land early enough that a script can adopt it, be released, and reach
users, all before the default flips.

Three naming options were on the table. `--no-append` pairs with the
existing flag and needs no new concept, but it names the new behaviour as
the absence of the old one, which reads backwards once accumulation is
the default. `--truncate` is the most precise verb, since it describes
what happens to the file rather than to the entries, but no build tool
spells it that way and it suggests a maintenance operation on the
database. `--overwrite` is what `compiledb` and `kubectl` use for the
same idea, so it is the spelling a user is most likely to guess.

## Decision

Ship `--overwrite` one release ahead of the default change, accepted
wherever `--append` is accepted and documented as naming the current
default. Reject the two together as a usage error rather than giving one
precedence over the other.

There is no short form. `-o` is Bear's output path, so the letter
`compiledb` uses is unavailable, and inventing a different letter would
give users a short flag to misremember.

## Consequences

A script can become forward-compatible today, and it keeps working
unchanged across the default change. `--append` stays accepted
permanently afterwards, naming the new default, so a script that never
adapts also keeps working. The only invocations whose behaviour changes
are those that relied on the unnamed default.

Rejecting the combination costs a script that passes both flags an
immediate failure. That is the point: the two flags state opposite
intents, and a precedence rule would let the contradiction survive the
default change, where it would quietly start losing entries or quietly
start keeping stale ones depending on which rule was chosen.

The cost is a release in which a documented flag does nothing. A reader
of the code will find a parsed argument that is never inspected, which
looks like a bug; the parsing site carries a comment saying otherwise.

Revisit when the default flips: at that point `--overwrite` becomes
load-bearing, and this entry becomes the record of why it predates the
behaviour it selects.

## References

- Requirements: `output-append`
