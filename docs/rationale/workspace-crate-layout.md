# The analysis core is its own crate, not a module

## Context

Bear does two separable things. It intercepts a build, and it decides
what an intercepted execution means: whether it is a compiler
invocation, and what its arguments are. The second half, compiler
recognition and flag parsing, used to be a module inside the same
library crate as the command-line arguments, the configuration schema,
the modes of operation, and the output writers.

The module already had the right shape. What it did not have was a
boundary anyone could rely on. Inside one crate, every `pub` item is
visible to every other module, and `use crate::config` from the analysis
code compiles without complaint. It did compile: the analysis core
imported four configuration types, including the whole top-level config
struct. Nothing could have stopped that edge from
growing, because a module boundary is a convention and conventions are
enforced by review attention.

The awkward part was the configuration edge itself. An `as:` entry in
the configuration file names a compiler family, so somewhere a
user-written string has to become a compiler identity, and the set of
valid identities is generated from the compiler definition YAML. Before
the split, that resolution happened in the configuration schema's serde
deserializer, which meant the schema layer had to include generated
codegen output to know the family set.

## Decision

Compiler recognition and flag parsing live in their own library crate,
which owns the compiler definition YAML and the codegen build script
that turns it into tables. It depends on the interception primitives and
nothing from the application layer: no configuration types, no `clap`,
no command-line arguments, and no serde.

The configuration schema depends on nothing. It stores a configured
`as:` value as the string the user wrote. The analysis core owns the
resolution of that string onto a compiler identity, next to the
generated data it validates against. The application layer is the only
place that sees both, so it is the single conversion point, and it
converts once at startup before any build runs.

## Consequences

What the crate boundary buys, none of which a module boundary can
provide:

- The compiler enforces the dependency direction. A back-edge from the
  application into the analysis core is a dependency cycle and a build
  error, not a review finding.
- The public API is deliberate. The crate's `lib.rs` states what
  dependents may touch, and internals stay `pub(crate)`. Widening the
  surface is now a visible diff rather than an accident of visibility.
- Dependencies are scoped and auditable. The manifest lists what
  analysis actually needs, which turned out to exclude serde entirely.
  It cannot quietly grow a dependency on the configuration stack,
  because that is a manifest line a reviewer sees.
- Build and test units are smaller. Editing application code no longer
  reruns compiler codegen or recompiles the analysis core, and the
  analysis crate's own test loop builds no application code.
- Routing is simpler: one guide per role, instead of one file explaining
  which subdirectories belong to which layer.

Accepted costs: one more manifest, and a change that crosses the
boundary now touches two crates. Inside a single workspace with no
published versions that is a wider diff, not a compatibility problem.
The other cost is diagnostic. Because the schema no longer validates
`as:` at deserialization time, an unknown spelling is reported after the
configuration echo rather than before it, and a launcher spelling such
as `ccache` now echoes back verbatim instead of being normalized to
`wrapper`. Both were judged improvements, or at worst neutral.

The rejected alternatives for the configuration boundary each preserved
the coupling in a different shape:

- **Keep the compiler-family type in the configuration schema, with its
  serde derive.** This is the smallest diff and the reason the coupling
  existed. It requires the schema layer to include codegen output so it
  can name the family set, which is exactly the dependency the split
  removes. The schema layer would then also have to be rebuilt whenever
  a compiler YAML file changed.
- **Have the schema's validator call a parse function in the analysis
  core.** This keeps fail-fast validation at load time, but it points
  the schema layer at the analysis core, which is the same edge running
  the other way. A schema is a description of a file format; it should
  not need the analysis core to be linked in order to be understood.
- **Mirror the type in the schema with `serde(remote)`.** This puts the
  serde derive on the application side while the real type stays in the
  analysis core, but it works by hand-writing a shadow copy of the type
  that must track the original. The family set is generated data, so
  there is no fixed variant list to mirror in the first place, and the
  shadow copy still needs the generated ids to validate against.

The chosen split wins on all three counts at once: the schema depends on
nothing, the analysis core owns resolution next to the generated data
that gives it meaning, and the conversion happens in exactly one place
that is named in the code rather than hidden in a deserializer.

The analysis crate carries a `testing` cargo feature that compiles its
command factories and comparison helpers into the library. This exists
because a dependency's `cfg(test)` code is not compiled when a
dependent's tests are built, and the output tests in the application
layer are written against those factories. An unconditional public
module would ship test factories in release builds; a second copy of the
factories in the dependent would drift from the original. The feature is
off by default and enabled only as a dev-dependency, and the module is
gated on "test or feature" so the crate's own fast test loop still
covers it.

Still open, deferred rather than refused: the application library and
the driver binary are separate packages with no other consumer between
them, and the shell-completions generator is a third package that exists
only to read the command-line definitions it does not own. Folding the
library into the driver, and the generator into whichever package owns
those definitions, is a reasonable next step. Nothing in this layout
blocks it, and the analysis crate is unaffected either way.

## References

- No requirement. The split changed no user-observable contract; the two
  diagnostic deltas above are the whole visible difference.
- Related rationale: [`compiler-as-no-aliases`](compiler-as-no-aliases.md)
  (the one-spelling-per-family `as:` contract whose mechanism this
  relocated) and
  [`compiler-family-definition`](compiler-family-definition.md) (why the
  family set is generated data rather than a hand-written enum).
- Crate roles and per-area routing: the root `CLAUDE.md`.
