// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler flag pattern matching and semantic analysis system.
//!
//! This module provides a comprehensive framework for recognizing and categorizing
//! compiler command-line flags. It separates the concerns of flag pattern matching
//! (HOW to consume arguments) from semantic meaning (WHAT the flag represents).
//!
//! The system supports various flag patterns including exact matches, prefix matches,
//! and flags that require additional arguments in different forms (separate, glued, etc.).

pub(super) mod source;

use crate::semantic::ArgumentKind;

pub use source::looks_like_a_source_file;

/// Flag pattern definitions that describe HOW to consume arguments from the command line.
/// These patterns define the syntactic structure of compiler flags and their arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FlagPattern {
    /// Match the flag exactly with specified number of required separate arguments: "-c", "-o file"
    /// The u32 represents how many additional arguments are required (0 for no additional args)
    Exactly(&'static str, u32),

    /// Match the flag exactly with 1 required argument glued with '=': "-std=c99"
    ExactlyWithEq(&'static str),

    /// Match exactly with 1 required argument, glued with '=' OR separate: "-std=c99", "-std c99"
    ExactlyWithEqOrSep(&'static str),

    /// Match exactly with 1 required argument in any form: "-Dname=value", "-D name=value", "-Dname", "-D name"
    ExactlyWithGluedOrSep(&'static str),

    /// Match as prefix with specified number of additional arguments: anything starting with flag
    /// The u32 represents how many additional arguments to consume beyond the flag itself
    Prefix(&'static str, u32),
}

impl FlagPattern {
    /// Get the flag string from the pattern
    pub fn flag(&self) -> &'static str {
        match self {
            FlagPattern::Exactly(flag, _) => flag,
            FlagPattern::ExactlyWithEq(flag) => flag,
            FlagPattern::ExactlyWithEqOrSep(flag) => flag,
            FlagPattern::ExactlyWithGluedOrSep(flag) => flag,
            FlagPattern::Prefix(flag, _) => flag,
        }
    }
}

/// A flag definition combining the flag pattern and argument kind.
/// This combines syntactic pattern matching with semantic meaning for compiler flags.
#[derive(Debug, Clone)]
pub(super) struct FlagRule {
    /// The flag pattern including name and matching instruction
    pub pattern: FlagPattern,

    /// What this flag represents semantically
    pub kind: ArgumentKind,
}

/// Result of matching a flag against command line arguments.
/// Contains only metadata (which rule matched and how many args it consumed),
/// avoiding string clones in the matching phase.
#[derive(Debug, Clone)]
pub(super) struct FlagMatch {
    /// The flag definition that matched
    pub rule: FlagRule,

    /// Number of arguments consumed from the command line (including the flag itself)
    pub consumed_count: usize,
}

/// Check if `arg` starts with `prefix` followed by '='.
/// Zero-allocation alternative to `arg.starts_with(&format!("{}=", prefix))`.
fn starts_with_eq(arg: &str, prefix: &str) -> bool {
    arg.len() > prefix.len() && arg.as_bytes()[prefix.len()] == b'=' && arg.starts_with(prefix)
}

/// A flag matcher that contains flag definitions for a specific compiler
pub(super) struct FlagAnalyzer {
    /// All flag definitions, sorted by priority (longer flags first for better matching)
    rules: &'static [FlagRule],
}

impl FlagAnalyzer {
    /// Create a new flag matcher from pre-sorted flag definitions
    ///
    /// The rules slice must already be sorted by flag length in descending order
    /// for proper matching behavior (longer flags matched first).
    pub fn new(rules: &'static [FlagRule]) -> Self {
        Self { rules }
    }

    /// Try to match a flag against the remaining command line arguments
    pub fn match_flag(&self, args: &[String]) -> Option<FlagMatch> {
        if args.is_empty() {
            return None;
        }

        for definition in self.rules {
            if let Some(result) = self.try_match_definition(definition, args) {
                return Some(result);
            }
        }

        None
    }

    /// Try to match a specific flag definition against the arguments
    fn try_match_definition(&self, definition: &FlagRule, args: &[String]) -> Option<FlagMatch> {
        let current_arg = &args[0];
        let flag = definition.pattern.flag();

        match &definition.pattern {
            FlagPattern::Exactly(_, required_count) => {
                let required_args = *required_count as usize;
                if current_arg == flag {
                    if required_args == 0 {
                        Some(FlagMatch { rule: definition.clone(), consumed_count: 1 })
                    } else if args.len() > required_args {
                        Some(FlagMatch { rule: definition.clone(), consumed_count: 1 + required_args })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            FlagPattern::ExactlyWithEq(_) => {
                if starts_with_eq(current_arg, flag) {
                    Some(FlagMatch { rule: definition.clone(), consumed_count: 1 })
                } else {
                    None
                }
            }

            FlagPattern::ExactlyWithEqOrSep(_) => {
                if current_arg == flag && args.len() > 1 {
                    Some(FlagMatch { rule: definition.clone(), consumed_count: 2 })
                } else if starts_with_eq(current_arg, flag) {
                    Some(FlagMatch { rule: definition.clone(), consumed_count: 1 })
                } else {
                    None
                }
            }

            FlagPattern::ExactlyWithGluedOrSep(_) => {
                if current_arg == flag && args.len() > 1 {
                    Some(FlagMatch { rule: definition.clone(), consumed_count: 2 })
                } else if current_arg.starts_with(flag) && current_arg.len() > flag.len() {
                    Some(FlagMatch { rule: definition.clone(), consumed_count: 1 })
                } else {
                    None
                }
            }

            FlagPattern::Prefix(_, required_count) => {
                if current_arg.starts_with(flag) {
                    let required_args = *required_count as usize;

                    if required_args == 0 {
                        Some(FlagMatch { rule: definition.clone(), consumed_count: 1 })
                    } else if args.len() > required_args {
                        let all_args_valid =
                            args.iter().take(required_args + 1).skip(1).all(|arg| !arg.starts_with('-'));

                        if all_args_valid {
                            Some(FlagMatch { rule: definition.clone(), consumed_count: 1 + required_args })
                        } else {
                            Some(FlagMatch { rule: definition.clone(), consumed_count: 1 })
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
}

impl FlagRule {
    /// Create a new flag definition
    pub const fn new(pattern: FlagPattern, kind: ArgumentKind) -> Self {
        Self { pattern, kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{CompilerPass, PassEffect};
    use std::sync::LazyLock;

    // Create test-specific flags (not compiler-specific)
    static TEST_FLAGS: LazyLock<Vec<FlagRule>> = LazyLock::new(|| {
        let mut flags = vec![
            FlagRule::new(
                FlagPattern::Exactly("-c", 0),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
            ),
            FlagRule::new(
                FlagPattern::ExactlyWithEqOrSep("-std"),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
            ),
            FlagRule::new(
                FlagPattern::ExactlyWithGluedOrSep("-I"),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
            ),
            FlagRule::new(FlagPattern::Prefix("-W", 0), ArgumentKind::Other(PassEffect::None)),
            FlagRule::new(FlagPattern::Exactly("-o", 1), ArgumentKind::Output),
        ];

        // Sort by flag length descending to ensure longer matches are tried first
        flags.sort_by(|a, b| b.pattern.flag().len().cmp(&a.pattern.flag().len()));

        flags
    });

    fn test_flags() -> &'static [FlagRule] {
        &TEST_FLAGS
    }

    #[test]
    fn test_exactly_matching() {
        let matcher = FlagAnalyzer::new(test_flags());

        let args = vec!["-c".to_string()];
        let result = matcher.match_flag(&args).unwrap();

        assert_eq!(result.consumed_count, 1);
    }

    #[test]
    fn test_equal_matching() {
        let matcher = FlagAnalyzer::new(test_flags());

        // Test glued with =
        let args = vec!["-std=c99".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_count, 1);

        // Test separate form
        let args = vec!["-std".to_string(), "c99".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_count, 2);
    }

    #[test]
    fn test_glued_or_separate_matching() {
        let matcher = FlagAnalyzer::new(test_flags());

        // Test glued form
        let args = vec!["-I/usr/include".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_count, 1);

        // Test separate form
        let args = vec!["-I".to_string(), "/usr/include".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_count, 2);

        // Test flag only (should not match since -I requires an argument)
        let args = vec!["-I".to_string()];
        let result = matcher.match_flag(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_prefix_matching() {
        let matcher = FlagAnalyzer::new(test_flags());

        let args = vec!["-Wall".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_count, 1);
    }

    #[test]
    fn test_no_match() {
        let matcher = FlagAnalyzer::new(test_flags());

        let args = vec!["--unknown".to_string()];
        let result = matcher.match_flag(&args);
        assert!(result.is_none());
    }
}
