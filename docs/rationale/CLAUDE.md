## Rationale directory

Each file here is one design decision: the *why* behind a part of Bear.

## When to write an entry

Write one when a decision is not obvious and a future reader would
otherwise have to rebuild the reasoning. Typically:

- You chose one approach among several with real trade-offs, and the
  code does not make the choice self-evident.
- You rejected an alternative that someone will propose again.
- Outside research (a compiler quirk, a kernel behavior, a file format)
  shaped the decision and would otherwise be lost in a PR thread.

Does not belong here:

- A detail obvious from the code. That goes in a code comment.
- A restatement of an acceptance criterion. Link the requirement instead.
- To-dos and open work. Those go in the task tracker.

## Files

A file is named `<short-kebab-case-title>.md`. The name without `.md` is
the entry's ID. No number prefix: git history gives the order, and a
name links better than `0007`.

Fill [`TEMPLATE.md`](TEMPLATE.md) for the format.

When a decision changes, rewrite the entry in place so it states the
current decision. Do not add "Update:" notes. Keep the filename so links
still resolve. Git history keeps the old version.

## Linking

Every reference in an entry, in the References section and in prose, is a
markdown link with a relative path:
`[output-append](../requirements/output-append.md)` for a requirement,
`[parse-sh-producer](parse-sh-producer.md)` for another entry. Issues and
outside docs get a link to their URL.

An entry links each requirement it supports. Add the link back in that
requirement's Rationale section in the same change.

## Entry vs rejected requirement

An idea declined before it had a requirement file goes straight to an
entry here. It never was a contract, so it gets no requirement file.

A requirement file that already exists stays when the user declines it
or it is withdrawn. Its status becomes `rejected`, and the file is a
tombstone that may link an entry here for the full reasoning.
