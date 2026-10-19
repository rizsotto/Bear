---
paths:
  - "**/*.rs"
---

# Test conventions

Where tests live and how to write them.

- **Placement**: unit tests live next to the code they exercise
  (`#[cfg(test)] mod tests` at the bottom of the `.rs` file); behavioural
  contracts get integration tests under `tests/integration/`, each tagged
  `// Requirements: <id>` (see `tests/integration/CLAUDE.md` for the
  linkage rules and the test harness). A unit test that protects a
  requirement carries the same tag.

## Style

- **Name the subject under test `sut`.** Whatever the test exercises - the
  function result, the struct instance, the parsed value - binds to a
  variable named `sut`, so every test reads the same way.
- **Separate prepare from execute and verify.** Structure each test as
  arrange / act / assert: set up inputs first, then produce `sut`, then
  assert on it. Keep the three phases visually distinct (blank lines, or
  `// arrange` / `// act` / `// assert` comments when it aids reading).
- **Drive input-only variations from a table.** When several tests differ
  only in their inputs and expected outputs, do not copy-paste the test
  body. Collect `(input, expected)` tuples in a list and loop over them,
  asserting once per case. Include the case input in the assertion message
  so a failure says which row failed.
- **Prefer value factories.** Build test inputs and expected values with
  small named helper functions rather than inline literals, so each case
  reads as intent ("a relative source file in a subdir") rather than as
  raw data.
- **Fixtures.** When three or more tests in a file share on-disk
  scaffolding (temp dirs, fake binaries, wired-up structs), extract a
  `mod fixture` builder struct inside the `#[cfg(test)]` block that owns
  every `TempDir` so paths stay valid for the test's lifetime.
- **Mock doubles.** Annotate a trait that needs a test double with
  `#[cfg_attr(test, mockall::automock)]` on the trait definition; hand-write
  a mock only for a composed trait that `automock` cannot express.
