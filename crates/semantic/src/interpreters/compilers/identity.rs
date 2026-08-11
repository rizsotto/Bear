// SPDX-License-Identifier: GPL-3.0-or-later

//! The identity of a recognized tool: a compiler family or a launcher.
//!
//! This module also owns the resolution of a configured `as:` spelling onto
//! that identity. Resolution lives here, next to the generated id data it
//! validates against, so the [`CompilerId`] invariant -- always one of the
//! generated [`KNOWN_IDS`], as a `&'static str` -- holds by construction.
//! The configuration schema keeps the spelling the user wrote and knows
//! nothing about the compiler families.

use std::fmt;
use std::str::FromStr;

// Generated compiler-id data (KNOWN_IDS, WRAPPER_AS_NAMES) from
// compilers/*.yaml. These are the sole accepted config `as:` spellings; the
// resolution below validates against them instead of a hand-maintained
// mirror of the family set. See compiler-as-no-aliases.
include!(concat!(env!("OUT_DIR"), "/compiler_ids.rs"));

/// A compiler family's identity: the `compiler.id` declared in its YAML
/// definition, which is also its sole accepted config `as:` spelling.
///
/// Wraps a `&'static str` that is always one of the generated [`KNOWN_IDS`]:
/// [`CompilerType`]'s [`FromStr`] resolves user input to a canonical entry,
/// and internal literals are drift-guarded against that list by a test.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct CompilerId(&'static str);

impl CompilerId {
    /// Construct from a compile-time-known id. The caller guarantees the id is
    /// one of [`KNOWN_IDS`]; `compiler_id_literals_are_known` enforces this for
    /// every internal literal.
    pub(crate) const fn new(id: &'static str) -> Self {
        CompilerId(id)
    }

    /// The canonical id string, verbatim.
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for CompilerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// The kind of tool Bear recognizes: a real compiler family (carrying its
/// [`CompilerId`]) or a compiler launcher (wrapper). Mirrors the YAML `type:`
/// field; the family set lives in data ([`KNOWN_IDS`]), not in this enum.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum CompilerType {
    Compiler(CompilerId),
    Wrapper,
}

impl CompilerType {
    /// Construct a compiler-kind value from a compile-time-known id.
    pub(crate) const fn compiler(id: &'static str) -> Self {
        CompilerType::Compiler(CompilerId::new(id))
    }

    /// Map a generated `RECOGNITION_PATTERNS` id column to a value. The string
    /// is trusted generated data: `"wrapper"` names the launcher kind, every
    /// other value is a real compiler id.
    pub(crate) fn from_recognized_id(id: &'static str) -> Self {
        if id == "wrapper" { CompilerType::Wrapper } else { CompilerType::compiler(id) }
    }
}

impl fmt::Display for CompilerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerType::Compiler(id) => fmt::Display::fmt(id, f),
            CompilerType::Wrapper => f.write_str("wrapper"),
        }
    }
}

/// Failure to resolve a configured `as:` spelling onto a [`CompilerType`].
#[derive(Debug, thiserror::Error)]
pub enum SpellingError {
    // List every accepted `as:` value: the compiler ids plus the launcher
    // spellings ("wrapper" and each basename). Omitting the launcher
    // spellings would misdescribe a mistyped `ccache` as if only compiler
    // ids were valid.
    #[error(
        "unknown compiler id {spelling:?}, expected one of: {}, wrapper, {}",
        KNOWN_IDS.join(", "),
        WRAPPER_AS_NAMES.join(", ")
    )]
    Unknown { spelling: String },
}

impl FromStr for CompilerType {
    type Err = SpellingError;

    fn from_str(spelling: &str) -> Result<Self, Self::Err> {
        // The four launchers share one runtime kind: "wrapper" plus each
        // launcher basename all resolve to it (see compiler-as-no-aliases).
        if spelling == "wrapper" || WRAPPER_AS_NAMES.contains(&spelling) {
            return Ok(CompilerType::Wrapper);
        }
        // A compiler family is its id, verbatim -- no aliases. Resolve to the
        // canonical `&'static str` so the value is unconstructible unvalidated.
        if let Some(&canonical) = KNOWN_IDS.iter().find(|&&id| id == spelling) {
            return Ok(CompilerType::Compiler(CompilerId::new(canonical)));
        }
        Err(SpellingError::Unknown { spelling: spelling.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_id_literals_are_known() {
        // Every compiler id hand-written as a literal in production code must
        // be one of the generated KNOWN_IDS. If a YAML rename ever drifts one
        // of these, this fails instead of silently misclassifying at runtime.
        //
        // These are the only such literals: the ambiguous-name probe's two
        // verdicts and the recognition fallback (all `gcc`/`clang`). Family
        // registration and response-file syntax are driven entirely by
        // generated data (the FAMILIES registry), so they carry no id
        // literals to guard here; the dispatch test
        // (every_recognition_pattern_row_is_dispatched_by_a_registered_interpreter)
        // covers that path end to end.
        for id in ["gcc", "clang"] {
            assert!(KNOWN_IDS.contains(&id), "internal id literal {id:?} is not in KNOWN_IDS");
        }
    }

    #[test]
    fn unknown_as_value_error_lists_wrapper_and_a_real_id() {
        // The "expected one of" list must name every accepted `as:` value,
        // including the launcher spellings -- not just the compiler ids.
        let sut = "nope".parse::<CompilerType>().unwrap_err().to_string();

        assert!(sut.contains("unknown compiler id"), "message: {sut}");
        assert!(sut.contains(KNOWN_IDS[0]), "message should list a real id: {sut}");
        assert!(sut.contains("wrapper"), "message should list the wrapper spelling: {sut}");
        assert!(sut.contains(WRAPPER_AS_NAMES[0]), "message should list a launcher name: {sut}");
    }

    #[test]
    fn compiler_type_round_trips_through_its_spelling() {
        // Display then resolve yields the same value, for both kinds.
        for sut in [CompilerType::compiler(KNOWN_IDS[0]), CompilerType::Wrapper] {
            let spelling = sut.to_string();

            let back: CompilerType = spelling.parse().unwrap();

            assert_eq!(back, sut, "round-trip via {spelling}");
        }
    }

    #[test]
    fn every_known_id_resolves_to_its_own_family() {
        // Every canonical id resolves to its own compiler family. The set
        // is generated (KNOWN_IDS); no id is hand-listed here, so a new family
        // is covered automatically.
        for &id in KNOWN_IDS {
            let sut = id.parse::<CompilerType>().unwrap();

            assert_eq!(sut, CompilerType::compiler(id), "id: {id}");
        }
    }

    #[test]
    fn every_launcher_spelling_resolves_to_the_wrapper_kind() {
        // The wrapper kind: "wrapper" plus every launcher basename resolve to
        // the one runtime CompilerType::Wrapper (see compiler-as-no-aliases).
        for spelling in std::iter::once("wrapper").chain(WRAPPER_AS_NAMES.iter().copied()) {
            let sut = spelling.parse::<CompilerType>().unwrap();

            assert_eq!(sut, CompilerType::Wrapper, "spelling: {spelling}");
        }
    }

    #[test]
    fn dropped_aliases_are_rejected() {
        // Every spelling that used to be an accepted alias now fails: one id,
        // one spelling, verbatim.
        let dropped_spellings = ["clangcl", "llvm", "intel-cc", "craycc"];

        for spelling in dropped_spellings {
            let result = spelling.parse::<CompilerType>();

            match result {
                Err(error) => {
                    let message = error.to_string();
                    assert!(
                        message.contains("unknown compiler id"),
                        "case: {spelling}, unexpected error message: {message}"
                    );
                }
                Ok(value) => panic!("expected {spelling} to be rejected, got: {value:?}"),
            }
        }
    }

    #[test]
    fn display_is_the_id_verbatim() {
        // Display prints the id, not a curated pretty name: one spelling
        // everywhere (YAML, config `as:`, generated data, output).
        assert_eq!(CompilerType::compiler("gcc").to_string(), "gcc");
        assert_eq!(CompilerType::compiler("clang_cl").to_string(), "clang_cl");
        assert_eq!(CompilerType::compiler("intel_fortran").to_string(), "intel_fortran");
        assert_eq!(CompilerType::Wrapper.to_string(), "wrapper");
    }
}
