// SPDX-License-Identifier: GPL-3.0-or-later

//! Property-based tests for pattern_to_rust and flag_name_len.

use bear_codegen::codegen::{flag_name_len, pattern_to_rust};
use bear_codegen::yaml_types::FlagMatch;
use proptest::prelude::*;

/// Strategy for generating flag name strings (e.g., "-I", "--std", "/Fo").
fn flag_name_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("-{1,2}[A-Za-z][A-Za-z0-9_-]{0,15}")
        .unwrap()
        .prop_filter("must not be empty after prefix", |s| s.len() > 1)
}

/// Strategy for generating optional count values.
fn count_strategy() -> impl Strategy<Value = Option<u32>> {
    prop_oneof![Just(None), (0u32..10).prop_map(Some)]
}

/// Strategy for generating pattern suffixes.
fn suffix_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just(""), Just("*"), Just("=*"), Just(":*"), Just("{ }*"), Just("{=}*"), Just("{:}*"),]
}

// pattern_to_rust always produces output starting with "FlagPattern::".
proptest! {
    #[test]
    fn output_starts_with_flag_pattern(
        flag in flag_name_strategy(),
        suffix in suffix_strategy(),
        count in count_strategy(),
    ) {
        let pattern = format!("{}{}", flag, suffix);
        let output = pattern_to_rust(&pattern, count);
        prop_assert!(
            output.starts_with("FlagPattern::"),
            "Expected FlagPattern:: prefix, got: {}",
            output
        );
    }
}

// pattern_to_rust maps each suffix to the expected variant.
proptest! {
    #[test]
    fn suffix_determines_variant(
        flag in flag_name_strategy(),
        count in count_strategy(),
    ) {
        // { }* -> ExactlyWithGluedOrSep
        let output = pattern_to_rust(&format!("{}{{ }}*", flag), count);
        prop_assert!(output.contains("ExactlyWithGluedOrSep"), "{}", output);

        // {=}* -> ExactlyWithEqOrSep
        let output = pattern_to_rust(&format!("{}{{=}}*", flag), count);
        prop_assert!(output.contains("ExactlyWithEqOrSep"), "{}", output);

        // {:}* -> ExactlyWithColonOrSep
        let output = pattern_to_rust(&format!("{}{{:}}*", flag), count);
        prop_assert!(output.contains("ExactlyWithColonOrSep"), "{}", output);

        // :* -> ExactlyWithColon
        let output = pattern_to_rust(&format!("{}:*", flag), count);
        prop_assert!(output.contains("ExactlyWithColon"), "{}", output);
    }
}

// =* without count -> ExactlyWithEq; with count -> Prefix.
proptest! {
    #[test]
    fn eq_star_variant_depends_on_count(flag in flag_name_strategy()) {
        let pattern = format!("{}=*", flag);

        let without_count = pattern_to_rust(&pattern, None);
        prop_assert!(without_count.contains("ExactlyWithEq"), "{}", without_count);

        let with_count = pattern_to_rust(&pattern, Some(1));
        prop_assert!(with_count.contains("Prefix"), "{}", with_count);
    }
}

// Bare flag (no suffix) -> Exactly.
proptest! {
    #[test]
    fn bare_flag_produces_exactly(flag in flag_name_strategy(), count in count_strategy()) {
        let output = pattern_to_rust(&flag, count);
        prop_assert!(output.contains("Exactly("), "Expected Exactly, got: {}", output);
    }
}

// flag* (star suffix, no separator) -> Prefix.
proptest! {
    #[test]
    fn star_suffix_produces_prefix(flag in flag_name_strategy(), count in count_strategy()) {
        let pattern = format!("{}*", flag);
        let output = pattern_to_rust(&pattern, count);
        prop_assert!(output.contains("Prefix("), "Expected Prefix, got: {}", output);
    }
}

// flag_name_len is always <= the pattern length.
proptest! {
    #[test]
    fn flag_name_len_bounded_by_pattern(
        flag in flag_name_strategy(),
        suffix in suffix_strategy(),
        count in count_strategy(),
    ) {
        let pattern = format!("{}{}", flag, suffix);
        let m = FlagMatch { pattern: pattern.clone(), count };
        let len = flag_name_len(&m);
        prop_assert!(
            len <= pattern.len(),
            "flag_name_len({}) = {} > pattern.len() = {}",
            pattern, len, pattern.len()
        );
    }
}

// flag_name_len is always > 0 for non-empty patterns.
proptest! {
    #[test]
    fn flag_name_len_positive(
        flag in flag_name_strategy(),
        suffix in suffix_strategy(),
        count in count_strategy(),
    ) {
        let pattern = format!("{}{}", flag, suffix);
        let m = FlagMatch { pattern, count };
        prop_assert!(flag_name_len(&m) > 0);
    }
}
