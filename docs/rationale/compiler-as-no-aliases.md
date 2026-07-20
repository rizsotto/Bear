# Config `as:` accepts one spelling per compiler, no aliases

## Context

`CompilerType` (`crates/bear/src/config/types.rs`) used to accept many
config `as:` spellings per compiler family through hand-written
`#[serde(alias = ...)]` lists, plus whatever `#[serde(rename_all =
"lowercase")]` happened to squash a Rust variant name into when no
alias was written (for example `clangcl` for `ClangCl`, with no
separator between the words). This alias surface was never designed
as a feature; it accumulated over time as each compiler family was
added, with no record of who relied on which spelling.

While building the wrapper-YAML plan (adding `ccache`/`distcc`/
`sccache`/`icecc` as YAML-defined compiler launchers), two related
ideas were drafted in full and then reversed on 2026-07-20:

- Replacing the hand-aliases with a YAML-driven `aliases:` list per
  compiler file, generating an alias map at codegen time. This was
  reviewed and discarded: the alias surface itself was not worth
  keeping. No user demand for any specific alias was on record, and
  generating a bigger version of an undesigned surface does not fix
  the underlying problem.
- Re-spelling the seven multi-word ids to a hyphenated form (for
  example `clang_cl` to `clang-cl`) for nicer `--print-compilers`
  output. This was drafted as a follow-on to the aliases idea and
  discarded for the same reason: once no aliases exist, printing
  anything other than the literal id is a cosmetic, unexplained
  mutation of the value that is supposed to be the source of truth.

## Decision

Each `CompilerType` variant accepts exactly one config `as:` spelling:
its YAML `compiler.id`, verbatim, everywhere that spelling appears
(the YAML file, the generated recognition data, `--print-compilers`
output, and config parsing). No aliases, no re-spelling. `CompilerType`
keeps a plain `#[serde(rename = "...")]` only where the derive's
default `rename_all = "lowercase"` spelling would otherwise diverge
from the id (seven of the nineteen compiler variants; the other
twelve already match by construction).

`CompilerType::Wrapper` is the one exception: wrapper YAML files
(`crates/bear/compilers/{ccache,distcc,icecc,sccache}.yaml`) carry no
`compiler.id` to draw a canonical spelling from, since dispatch for
every launcher is uniform and a launcher file declares no family
identity. It keeps a small hand-coded set of accepted `as:` spellings
(`ccache`, `distcc`, `sccache`, `icecc`) as the sole surviving
per-launcher wiring.

## Consequences

- Breaking change for any config pinning a non-canonical spelling
  (`llvm`, `gnu`, `clangcl`, `intel-cc`, and similar dropped aliases).
  There is no static changelog file in this repository; a release-notes
  line for this change is drafted from commit history at release time
  (see the `release` skill).
- Simpler code: no generated alias map, and no per-variant alias list
  to keep in sync as compiler families are added or renamed.
- `--print-compilers`'s `as` column is now always exactly the string a
  user must type in `as:` for that family; there is no display
  transform anywhere in the pipeline to keep in sync with the parser.
- A future contributor proposing to re-add aliases, or to hyphenate
  ids for display, should read this entry first: both were drafted in
  full and discarded, not merely left undone.

## References

- Requirement: `recognition-compiler-launchers` (the closest governing
  requirement; there was never a requirement formalizing the `as:`
  alias contract itself, since it was undocumented behavior that
  accumulated rather than a contract anyone shipped deliberately,
  which is part of why dropping it was acceptable).
