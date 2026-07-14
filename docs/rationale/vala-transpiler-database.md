# Recording Vala (valac) builds in the compilation database

## Context

`valac` is not a conventional compiler but a transpiler-driver: it
parses Vala (`.vala`) and Genie (`.gs`) sources, emits C, and -- unless
given `-C`/`--ccode` -- spawns a real C compiler (default command `cc`,
overridable by `--cc=` or `$CC`) on the generated C. Under interception
Bear therefore sees two processes: the `valac` invocation and the
internal `cc` on the generated C.

The consumer that motivated the request (issue #709) is
vala-language-server (VLS). Two facts about how it reads the database
drove the decisions below: it recognizes a Vala entry only when
`command[0]` contains `valac`, and it parses only the `command` *string*
form -- an entry carrying just an `arguments` array is silently skipped.
Bear defaults to the array form, so the two collide.

Two valac specifics also surfaced during implementation: `valac`'s
source files (`.vala`, `.gs`) were not in Bear's source-extension
allowlist, so without adding them the `.vala` argument was classified as
a non-compilable input and produced no entry; and `-X`/`--Xcc` forward
one arbitrary token to the C compiler, a token that is ambiguous between
a compile flag (`-X -fPIC`) and a link flag (`-X -lm`).

## Decision

We emit one entry per `valac` invocation, not one per `.vala`/`.gs`
source. `valac` compiles all of a target's sources together as a single
translation unit and produces one output (a `--library`/`--vapi`), so the
universal "one entry per source" rule was wrong for it: it fanned a single
invocation into N entries that each still carried the producing flags, and
VLS keys each command on the produced `.vapi`, so N entries became N
duplicate producers of the same binding and VLS refused to build. Meson
emits exactly one entry per `valac` invocation, with `file` set to the
first source; we match that. The combined entry keeps all sources in its
arguments (they are not separable siblings to strip) and its `file` is the
first source in argument order; the per-source C compilers are unaffected.

We record the `valac` invocation itself, rather than its generated-C
`cc` child, because VLS keys on `command[0]` containing `valac`; the
child invocation would never match and so would be useless to the
consumer that asked for this. The string-vs-array mismatch is left to the
user instead of auto-switching the format for valac: special-casing a
single compiler's output shape would break the invariant that every entry
in a database looks the same, so the choice is documented in the man page
(point VLS at a database built with `format.entries.use_array_format:
false`) rather than baked into the tool.

We keep the internal generated-C `cc` entries rather than filtering them
out. Filtering would need a reliable way to attribute a `cc` invocation
to a parent valac, which interception does not give us, and there is no
clean suppression hook since entry validation does not check file
existence. They are also valid entries in their own right -- a C language
server can consume the generated-C compile commands -- so dropping them
would discard usable data. The cost of keeping them -- clangd
background-indexing `valac` output it cannot parse -- is real but better
addressed on the clangd side (a `.clangd` `PathMatch` excluding `*.vala`)
than by teaching the database to drop valid entries; and VLS ignores them,
so the alternative buys nothing for the motivating consumer.

We extend the source-extension allowlist with `.vala` and `.gs` only.
`.vapi` and `.gir` are tempting to add alongside them, but they are
bindings that valac consumes, not translation units; treating them as
sources would emit entries for inputs nothing compiles.

We classify `-X`/`--Xcc` as compile-affecting rather than linking.
Linking arguments are stripped from the combined compile entry, so the
two readings diverge: the forwarded token is ambiguous, and dropping a
compile-relevant flag (`-X -fPIC`) corrupts the entry, whereas keeping a
stray link flag (`-X -lm`) is harmless noise. The cheaper failure wins.

## Consequences

- A default Bear database (array form) yields no usable entries for VLS;
  this is a documentation burden, not a silent failure -- the man page
  calls it out, and the format knob already exists.
- Vala source recognition is extension-gated, so a future Vala-family
  extension must be added to the allowlist explicitly (the same as every
  other language).
- The generated-C entries reference files that, for a non-`-C` build,
  live in a temporary directory and may not persist after the build.
  They are noise for clangd but are never wrong for VLS; revisit only if
  a concrete consumer needs them suppressed.
- `valac` is recognized with `versioned` and `cross_compilation` enabled:
  the former matches `valac-0.56`; the latter matches Debian/Ubuntu's
  triplet-prefixed `x86_64-linux-gnu-valac`, which is what their primary
  `valac` package installs.

## References

- Requirement: `output-compilation-entries`
- Issue #709 -- the feature request (valac support for vala-language-server)
- vala-language-server `src/projects/ccproject.vala` -- the `valac`
  command filter and command-string requirement
