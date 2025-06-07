// This file exists only to ensure that the dependencies are built before running the integration tests.
// The binary is not meant to be run directly; it's used with the "required-features" field
// in Cargo.toml to ensure that bear, intercept-preload, and intercept-wrapper binaries are built
// when the "allow-integration-tests" feature is enabled.

fn main() {
    // This function intentionally does nothing.
    // The mere existence of this file and its mention in Cargo.toml with required-features="allow-integration-tests"
    // ensures that the dependencies (bear, intercept-preload, intercept-wrapper) are built
    // before the integration tests run.

    // For debugging purposes, we could print a message, but it's not necessary
    // println!("Dependencies built successfully");
}
