# Response-file inlining is on by default

## Context

Without a response file's content Bear can misread the call. A source
inside the file gets no entry. A `-E` or `--version` inside it goes
unseen, so a call that compiles nothing can get an entry. The
requirement lists these failures for the case where inlining is off.

Reading is safe in the modes that run on the build's machine. The files
are there.

Input from another machine is different, because the files are not
there. That mode needs "not set" to be told from "set". An unset value
means off. A value the user set to on is rejected, as before.

Options considered:

- Keep inlining off by default. Most builds want the file's flags, and
  the failures above are silent.
- Let the another-machine option quietly win over an explicit on. The
  other settings that need the build's files are rejected in that case,
  so this would be inconsistent.

## Decision

Response files are read and inlined by default. The configuration keeps
"not set" apart from "set". For input from another machine, an unset
value means off, and only a value the user set to on is rejected.

## Consequences

Entries have the file's flags without any setting. Users who do not want
that turn inlining off.

## References

- [output-response-file-inlining](../requirements/output-response-file-inlining.md), the requirement this supports
- [cli-input-from-another-machine](../requirements/cli-input-from-another-machine.md), the requirement for that mode
- [response-file-inlining-best-effort](response-file-inlining-best-effort.md), how unread files are handled
- [input-from-another-machine-switch](input-from-another-machine-switch.md), the switch for that mode
