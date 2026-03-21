# How to contribute

Thank you for taking the time to contribute!

## Development setup

Bear is a Cargo workspace with the following crates:

- **bear** — the main driver and semantic analysis tool
- **intercept-preload** — the `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES` shared library
- **platform-checks** — compile-time platform feature detection
- **integration-tests** — end-to-end tests that exercise the installed tool

Build, lint, and test with:

   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

Integration tests require a debug build (`cargo build`) to be present before
running `cargo test`.

## Reporting bugs

- Please read the documentation — it may be a known limitation of the current
  release. This might also help to clear false expectations about the tool, or
  help you classify your request not as a bug but as an enhancement.

- Ensure that the bug was not already reported by searching on GitHub under
  [Issues](https://github.com/rizsotto/Bear/issues).

- If you have not found an open issue addressing the problem, open a new one.
  Be sure to include a title and clear description, with as much relevant
  information as possible. Attach the output of the tool — try running in
  verbose mode.

## Suggesting enhancements

- Enhancement suggestions are tracked as GitHub issues.

- Use a clear and descriptive title for the issue to identify the suggestion.

- Describe the current behavior and explain which behavior you expected to see
  instead and why.

## Pull Requests

- Open a new GitHub pull request with the patch.

- Ensure the PR description clearly describes the problem and solution. Include
  the relevant issue number if applicable.

- Think about testability. Please write test(s) for the problem you have fixed.
  Make sure that existing tests are not broken.

**PR checklist:**

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes (including integration tests)
- [ ] Documentation updated if behavior changed
