## Project overview

Bear generates JSON compilation databases (`compile_commands.json`) for Clang tooling.
It intercepts compiler invocations during a build (preload library or wrapper),
keeps the ones it recognizes as compiler calls, and writes them out.

### Workspace crates

| Crate | Purpose |
|---|---|
| `crates/bear` | Application layer: CLI, configuration, modes, output |
| `crates/semantic` | Compiler recognition and flag parsing; owns the `compilers/*.yaml` definitions |
| `crates/bear-driver` | Driver/supervisor binary; orchestrates interception and output |
| `crates/intercept` | Shared/agent-side interception runtime (Execution, reporter, wire, env helpers) |
| `crates/intercept-supervisor` | Driver-side interception (supervise, TCP collector, build environment) |
| `crates/intercept-preload` | `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES` shared library |
| `crates/bear-wrapper` | Compiler wrapper binary; reports intercepted executions |
| `crates/bear-completions` | Shell-completion script generator |
| `build-support/platform-checks` | Build-time platform capability detection |
| `build-support/compilers-codegen` | Code generator for compiler flag tables |
| `tests/tools` | Shared test tooling; library + `cdb-compare` binary (package `bear-test-tools`) |
| `tests/integration` | End-to-end tests (package `integration-tests`) |

Build prerequisites (C compiler, linker) are in `INSTALL.md`.

## Routing - read before modifying

Before modifying code in a subdirectory, read the `CLAUDE.md` in that directory first.
These files contain rules, context, and constraints specific to that area.

| When you are about to... | Read first |
|---|---|
| Modify CLI arguments, configuration, or output format | `crates/bear/CLAUDE.md` |
| Touch compiler recognition or flag parsing | `crates/semantic/CLAUDE.md` |
| Edit or add compiler interpreter YAML | `crates/semantic/compilers/CLAUDE.md` |
| Edit the YAML-to-Rust code generator | `build-support/compilers-codegen/CLAUDE.md` |
| Add or change a host capability probe | `build-support/platform-checks/CLAUDE.md` |
| Touch the preload interception library | `crates/intercept-preload/CLAUDE.md` |
| Touch driver-side interception (supervise, collector, build env) | `crates/intercept-supervisor/CLAUDE.md` |
| Touch the shell-completions binary | `crates/bear-completions/CLAUDE.md` |
| Write or modify integration tests | `tests/integration/CLAUDE.md` |
| Edit the test-tooling comparator or normalization | `tests/tools/CLAUDE.md` (behavior in `tests/tools/SPEC.md`) |
| Edit or regenerate the man page | `man/CLAUDE.md` |
| Write or modify a docs site page | `site/CLAUDE.md` |
| Add, modify, or review a requirement (contract) | `docs/requirements/CLAUDE.md` |
| Record or look up a design decision, or a rejected option | `docs/rationale/CLAUDE.md` |
| Run an operational procedure (release, ...) | invocable skills in `.claude/skills/` |

Do not skip these reads. They contain constraints that prevent regressions.

Keep the roles separate. A requirement says *what*; a rationale entry
says *why*; the code says *how*. The man page is the user-facing
reference for CLI flags and configuration keys; the site is the user guide.

Each rule lives in one file. Other files link to it and never restate it.

## Workflow

1. **Start the task.** Pick it up in the task tracker.
2. **Classify the change.** A new feature, an architectural change, or a
   change to an existing contract takes every step. A
   bug fix that does not change a contract skips steps 3 and 7, and cites
   the governing requirement if one exists. Any other change (refactor,
   performance, internal) skips step 3. Every change finds its governing
   requirement and keeps its acceptance criteria passing.
3. **Requirement and approval.** Write or edit the requirement at status
   `proposed` (`docs/requirements/CLAUDE.md`, `requirements` skill).
   Present a short decision log (approach, alternatives considered,
   trade-offs) and wait for approval. Record the why in
   `docs/rationale/` (`docs/rationale/CLAUDE.md`).
4. **Test first.** Write the failing test that cites the requirement
   (`.claude/rules/testing.md`, `tests/integration/CLAUDE.md`).
5. **Implement.** Read the routing table above first. Rust conventions
   are in `.claude/rules/rust.md`.
6. **Checks.** All four must pass before committing:

   ```sh
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   typos
   ```

   Run `cargo build` before `cargo test` (`tests/integration/CLAUDE.md`).
7. **Different-model review.** Have a review subagent on a different
   model check the change (the `Agent` tool with a different `model`).
8. **Commit.** If the change implements a requirement, set it to
   `implemented` first (`docs/requirements/CLAUDE.md`). See "Commit
   messages" below.
9. **Close the task** in the task tracker, citing the commit.

## Code guidelines

Always-on, language-agnostic standards for every workspace crate:

- Maintain the existing code structure; extend before adding.
- Keep dependencies minimal; a new dependency needs a real justification,
  not convenience.
- No speculative features or over-engineering.

Rust file conventions live in `.claude/rules/rust.md`, and test
conventions in `.claude/rules/testing.md`. Both are path-scoped to `.rs`
files. The per-crate `CLAUDE.md` files add only crate-specific constraints.

## Commit messages

- Imperative subject under ~70 characters, with a conventional-commit area
  prefix (`docs:`, `fix:`, `feat:`, `chore:`, `test:`, `refactor:`, ...),
  scoped where it sharpens the scan-line (`docs(requirements): ...`).
- Blank line, then a body explaining the *why*; the diff shows the *what*.
- Reference a requirement ID when the commit implements or changes that
  contract, and an issue or PR only when it adds context the body can't.

## Output format

- ASCII only in all files (no em dashes, smart quotes, Unicode bullets)
- Use hyphens, straight quotes, three dots, asterisks/hyphens for bullets
