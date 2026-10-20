# Recording Vala (valac) builds in the compilation database

## Context

`valac` is a transpiler-driver, not a plain compiler. It parses Vala
(`.vala`) and Genie (`.gs`) sources and emits C. Unless given
`-C`/`--ccode`, it then runs a real C compiler on that C (by default
`cc`, or `--cc=` or `$CC`). Under interception Bear sees two processes:
`valac` and the internal `cc`.

The consumer behind the request
([#709](https://github.com/rizsotto/Bear/issues/709)) is
[vala-language-server](https://github.com/vala-lang/vala-language-server)
(VLS). It takes an entry as Vala only when `command[0]` contains
`valac`. It reads only the `command` string form and skips an entry
that has just an `arguments` array. Bear writes the array form by
default, so the two clash.

`valac` builds all of a target's sources as one unit with one output (a
`--library`/`--vapi`). The usual one-entry-per-source shape gave N
entries for one call, each still carrying the producing flags. VLS keys
each command on the `.vapi` it produces, so it saw N producers of the
same binding and refused to build. Meson writes one entry per `valac`
call, with `file` set to the first source.

Options considered:

- Record the internal `cc` call instead of `valac`. Rejected: VLS needs
  `valac` in `command[0]`, so the `cc` entry is useless to it.
- Switch to the string form for `valac` entries only. Rejected: every
  entry in a database should have the same shape.
- Filter out the internal `cc` entries. Rejected: interception gives no
  reliable way to tie a `cc` call to its parent `valac`, and entry
  checks do not test that a file exists, so there is no clean hook. The
  entries are also valid in their own right: a C language server can use
  them.
- Add `.vapi` and `.gir` as source extensions next to `.vala` and `.gs`.
  Rejected: they are bindings `valac` reads, not translation units, so
  they would give entries for inputs nothing compiles.
- Classify `-X`/`--Xcc` (one token passed to the C compiler) as a link
  flag. Rejected: the token can be a compile flag (`-X -fPIC`) or a link
  flag (`-X -lm`), and link flags are stripped from the entry. Dropping
  a compile flag breaks the entry; keeping a stray link flag is harmless.

## Decision

Bear writes one entry per `valac` call, as Meson does, and keeps the
internal `cc` entries. `.vala` and `.gs` are source extensions, and
`-X`/`--Xcc` count as compile flags.

## Consequences

- A default Bear database (array form) gives VLS no usable entries. The
  man page says so and points to `format.entries.use_array_format:
  false`. This is a documentation cost, not a silent failure.
- The internal `cc` entries point at C files that, without `-C`, live in
  a temporary directory and may be gone after the build. clangd may try
  to index them. That is better fixed on the clangd side (a `.clangd`
  `PathMatch` rule) than by dropping valid entries, and VLS ignores
  them. Revisit only if a real consumer needs them suppressed.
- A new Vala-family extension must be added to the source extension list
  by hand, as for every other language.
- `valac` is recognized with versioned names (`valac-0.56`) and with a
  target prefix, because Debian and Ubuntu install it as
  `x86_64-linux-gnu-valac`.

## References

- [output-compilation-entries](../requirements/output-compilation-entries.md), the requirement this supports
- [compilation-entry-shapes](compilation-entry-shapes.md), the entry shapes per compiler kind
- [#709](https://github.com/rizsotto/Bear/issues/709), the feature request
- [vala-language-server](https://github.com/vala-lang/vala-language-server), its `src/projects/ccproject.vala` holds the `valac` filter and the command-string rule
