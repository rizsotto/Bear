# Compiler env var parsing: whitespace split, no shell parser

## Context

Compiler environment variables often carry a trailing flag or two
(`CC="gcc -std=c11"`). To register a wrapper, Bear must separate the
program token from the flags. Three parsing approaches were on the table:

- **Plain whitespace split** - first token is the program, the rest are
  flags.
- **POSIX shell-word parsing (`shell_words::split`)** - handles quoted
  program names and paths containing spaces.
- **Full shell expansion (`sh -c 'command -v "$CC"'`)** - resolves the
  value exactly as the build system's shell would.

## Decision

Split the value on whitespace and go no further. No shell parser, no
subprocess.

## Consequences

Whitespace splitting matches the shape the contract actually supports:
the common Make/Autoconf convention is effectively `$(firstword $(CC))`
plus the rest as flags, which is exactly what this produces. Anything
more elaborate - flags with embedded whitespace, quoting,
metacharacters, command substitution - is explicitly redirected to
`CFLAGS` / `CXXFLAGS` / `LDFLAGS`, so the parser does not need to handle
it.

The rejected alternatives buy nothing for that contract: `shell_words`
adds quote-aware edge cases (malformed quotes, backslash-as-escape on
Windows paths) and test surface with no matching user need; full shell
expansion requires spawning a shell for a result whitespace splitting
already gives directly. If a real need for richer parsing appears, this
is the decision to revisit.

## References

- Requirement: `interception-cc-env-var-flags`
- Issue #686 - bare-name CC resolution
