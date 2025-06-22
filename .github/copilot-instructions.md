# Bear - Rust Implementation

Bear is a tool that generates compilation databases for clang tooling, implemented in Rust.
It is intended for developers who need to integrate clang-based tools with their build systems.

## Development Guidelines

### Required Before Each Commit
- Run `cargo fmt` to ensure proper code formatting.
- Run `cargo clippy -- -D warnings` to ensure the code passes all linter checks.

### Development Workflow
- Build the project: `cargo build --verbose`
- Run unit tests: `cargo test`
- Run integration tests: `cargo test --features allow-integration-tests`
- Run the linter: `cargo clippy -- -D warnings`

## Repository Structure
- `bear/`: Library code and executable for Bear
  - `bear/src/bin/`: Entry point for the Bear executable
  - `bear/src/modes/`: Modes of operation for Bear
  - `bear/src/intercept/`: Command interception logic
  - `bear/src/output/`: Output generation logic
  - `bear/src/semantic/`: Semantic analysis logic
- `intercept-preload/`: Dynamic library for Bear interception
- `intercept-wrapper/`: Wrapper for Bear interception
- `integration-tests/`: Integration tests for Bear
- `platform-checks/`: Platform-specific checks for cargo

## Architecture Overview and Data Flow

Bear operates by intercepting system calls during the build process to capture compilation commands
and generate a JSON compilation database.

The basic flow is as follows:
1. **Interception:**
   - On Linux and other Unix-like systems, Bear uses a dynamic library loaded via `LD_PRELOAD` to intercept
     system calls that execute commands during the build.
   - On other platforms, Bear provides a wrapper executable to achieve similar command interception.
2. **Semantic Analysis:**
   - Bear applies a semantic analysis layer to filter out non-compiler commands, ensuring only relevant
     compilation commands are processed.
3. **Formatting and Output:**
   - The filtered commands are formatted according to user-provided configuration.
   - The resulting compilation database is written as a JSON file, typically named `compile_commands.json`.

This architecture allows Bear to work transparently with a wide range of build systems and platforms.

## Key Guidelines
1. Follow Rust best practices and idioms.
2. Maintain the existing code structure and organization.
3. Use dependency injection patterns where appropriate.
4. Write unit tests for all new functionality.
5. Document public APIs and complex logic using rustdoc conventions.
6. Keep dependencies up to date and minimal.

## Getting Started
For new contributors, we recommend starting with:
1. Reading this document and `CONTRIBUTING.md`.
2. Building the project and running the test suite.
3. Looking at open issues labeled "good first issue".

## Documentation
- All public APIs should have documentation comments explaining their purpose and usage.
- Complex algorithms should include explanatory comments.
- Avoid unnecessary dependencies and update them regularly.
- Use CI for formatting, linting, and testing to catch issues early.
