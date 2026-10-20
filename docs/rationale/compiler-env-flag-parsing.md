# Compiler env var parsing: whitespace split, no shell parser

## Context

Compiler environment variables often carry a trailing flag or two
(`CC="gcc -std=c11"`). To register a wrapper, Bear must separate the
program token from the flags.

The supported shape is narrow. The common Make/Autoconf convention is
effectively `$(firstword $(CC))` plus the rest as flags. Anything more
elaborate (flags with embedded whitespace, quoting, metacharacters,
command substitution) is explicitly redirected to `CFLAGS` /
`CXXFLAGS` / `LDFLAGS`, so the parser does not need to handle it.

Options considered:

- **Plain whitespace split**: first token is the program, the rest are
  flags. Chosen.
- **POSIX shell-word parsing (`shell_words::split`)**: handles quoted
  program names and paths containing spaces. Rejected: it adds
  quote-aware edge cases (malformed quotes, backslash-as-escape on
  Windows paths) and test surface with no matching user need.
- **Full shell expansion (`sh -c 'command -v "$CC"'`)**: resolves the
  value exactly as the build system's shell would. Rejected: it spawns a
  shell for a result that whitespace splitting already gives.

## Decision

Split the value on whitespace and go no further. No shell parser, no
subprocess.

## Consequences

Whitespace splitting gives exactly the shape the contract supports, with
no parser dependency and no subprocess. A value with quoting or embedded
whitespace is not split the way a shell would split it. If a real need
for richer parsing appears, this is the decision to revisit.

## References

- [interception-cc-env-var-flags](../requirements/interception-cc-env-var-flags.md), the requirement this supports
- [#686](https://github.com/rizsotto/Bear/issues/686), bare-name CC resolution
