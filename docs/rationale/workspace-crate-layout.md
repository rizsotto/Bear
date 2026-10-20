# The analysis core is its own crate, not a module

## Context

Bear does two separate things. It intercepts a build, and it decides
what an intercepted execution means: is it a compiler call, and what are
its arguments. The second half, compiler recognition and flag parsing,
used to be a module in the same library crate as the command-line
arguments, the configuration schema, the modes, and the output writers.

The module had the right shape but no boundary anyone could rely on.
Inside one crate every `pub` item is visible everywhere, and
`use crate::config` from the analysis code compiles without complaint.
It did: the analysis code imported four configuration types, including
the whole top-level config struct. A module boundary is only a
convention, and only review attention enforces it.

The hard part was the configuration edge. An `as:` entry in the
configuration file names a compiler family, so a user-written string
must become a compiler identity somewhere. The set of valid identities
is generated from the compiler definition YAML. Before the split, the
schema's serde deserializer did that, so the schema layer had to include
generated code to know the family set.

Options for that edge, each of which kept the coupling in another shape:

- Keep the compiler-family type in the schema, with its serde derive.
  The smallest diff, but the schema then includes codegen output and is
  rebuilt whenever a compiler YAML file changes. That is the dependency
  the split removes.
- Have the schema's validator call a parse function in the analysis
  core. This keeps validation at load time, but it is the same edge
  pointing the other way. A schema describes a file format and should
  not need the analysis core linked in to be understood.
- Mirror the type in the schema with `serde(remote)`. This needs a
  hand-written shadow copy that tracks the original. The family set is
  generated, so there is no fixed variant list to mirror, and the copy
  still needs the generated ids to validate.

The split also raises a test problem. A dependency's `cfg(test)` code is
not built when a dependent's tests build, and the application's output
tests use the analysis crate's command factories. An always-public
module would ship test factories in release builds. A second copy in
the dependent would drift from the original.

## Decision

Compiler recognition and flag parsing live in their own library crate.
It owns the compiler YAML and its codegen, and depends on the
interception primitives only: no configuration types, no `clap`, no
command-line arguments, no serde. The schema stores `as:` as the string
the user wrote; the analysis crate resolves it, and the application
layer, the only code that sees both, is the single conversion point.
Test factories sit behind an off-by-default `testing` feature that
dependents enable only as a dev-dependency. The module is gated on
`any(test, feature = "testing")`, not `cfg(test)`, so a dependent's
tests can see it.

## Consequences

- The compiler enforces the dependency direction. A back-edge from the
  analysis core to the application is a dependency cycle and a build
  error, not a review finding.
- The public API is deliberate. `lib.rs` states what dependents may
  touch, and widening it is a visible diff.
- The manifest lists what analysis needs, and that turned out to
  exclude serde. A new dependency is a manifest line a reviewer sees.
- Editing application code no longer reruns compiler codegen or
  rebuilds the analysis core, and the analysis tests build no
  application code.
- A change that crosses the boundary touches two crates. In one
  workspace with no published versions that is a wider diff, not a
  compatibility problem.
- `as:` is no longer checked during deserialization. An unknown
  spelling is reported after the configuration echo, not before it, and
  a launcher spelling such as `ccache` echoes back as written instead of
  as `wrapper`. Both were judged neutral or better.
- The split changed no user-visible contract beyond those two, so it
  supports no requirement.

## References

- [compiler-family-definition](compiler-family-definition.md), why the family set is generated data and when `as:` is resolved
- [CLAUDE.md](../../CLAUDE.md), the crate roles and per-area routing
