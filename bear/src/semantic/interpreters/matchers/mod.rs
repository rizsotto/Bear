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

pub use source::{is_binary_file, looks_like_a_source_file};

/// Flag pattern definitions that describe HOW to consume arguments from the command line.
/// These patterns define the syntactic structure of compiler flags and their arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Comprehensive matching system - not all variants used yet
pub enum FlagPattern {
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
pub struct FlagRule {
    /// The flag pattern including name and matching instruction
    pub pattern: FlagPattern,

    /// What this flag represents semantically
    pub kind: ArgumentKind,
}

/// Result of matching a flag against command line arguments
#[derive(Debug, Clone)]
pub struct FlagMatch {
    /// The flag definition that matched
    pub rule: FlagRule,

    /// The arguments consumed from the command line
    pub consumed_args: Vec<String>,
}

impl FlagMatch {
    /// Get the number of arguments consumed (derived from consumed_args.len())
    pub fn consumed_args_count(&self) -> usize {
        self.consumed_args.len()
    }
}

/// A flag matcher that contains flag definitions for a specific compiler
pub struct FlagAnalyzer {
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
                        // No additional arguments required
                        Some(FlagMatch { rule: definition.clone(), consumed_args: vec![current_arg.clone()] })
                    } else if args.len() > required_args {
                        // Has required separate arguments
                        let mut consumed = vec![current_arg.clone()];
                        consumed.extend(args.iter().take(required_args + 1).skip(1).cloned());
                        Some(FlagMatch { rule: definition.clone(), consumed_args: consumed })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            FlagPattern::ExactlyWithEq(_) => {
                if current_arg.starts_with(&format!("{}=", flag)) {
                    // Flag with value glued with = (required)
                    Some(FlagMatch { rule: definition.clone(), consumed_args: vec![current_arg.clone()] })
                } else {
                    None
                }
            }

            FlagPattern::ExactlyWithEqOrSep(_) => {
                if current_arg == flag && args.len() > 1 {
                    // Has separate argument (required)
                    Some(FlagMatch {
                        rule: definition.clone(),
                        consumed_args: vec![current_arg.clone(), args[1].clone()],
                    })
                } else if current_arg.starts_with(&format!("{}=", flag)) {
                    // Flag with value glued with = (required)
                    Some(FlagMatch { rule: definition.clone(), consumed_args: vec![current_arg.clone()] })
                } else {
                    None
                }
            }

            FlagPattern::ExactlyWithGluedOrSep(_) => {
                if current_arg == flag && args.len() > 1 {
                    // Has separate argument (required)
                    Some(FlagMatch {
                        rule: definition.clone(),
                        consumed_args: vec![current_arg.clone(), args[1].clone()],
                    })
                } else if current_arg.starts_with(flag) && current_arg.len() > flag.len() {
                    // Flag with value glued (no separator, required)
                    Some(FlagMatch { rule: definition.clone(), consumed_args: vec![current_arg.clone()] })
                } else {
                    None
                }
            }

            FlagPattern::Prefix(_, required_count) => {
                if current_arg.starts_with(flag) {
                    let required_args = *required_count as usize;

                    if required_args == 0 {
                        // No additional arguments required
                        Some(FlagMatch { rule: definition.clone(), consumed_args: vec![current_arg.clone()] })
                    } else if args.len() > required_args {
                        // Check if we have enough additional arguments and they don't start with '-'
                        let mut all_args_valid = true;
                        for arg in args.iter().take(required_args + 1).skip(1) {
                            if arg.starts_with('-') {
                                all_args_valid = false;
                                break;
                            }
                        }

                        if all_args_valid {
                            // Consume the flag plus required additional arguments
                            let mut consumed = vec![current_arg.clone()];
                            consumed.extend(args.iter().take(required_args + 1).skip(1).cloned());
                            Some(FlagMatch { rule: definition.clone(), consumed_args: consumed })
                        } else {
                            // Fall back to consuming just the flag if additional args start with '-'
                            Some(FlagMatch {
                                rule: definition.clone(),
                                consumed_args: vec![current_arg.clone()],
                            })
                        }
                    } else {
                        // Not enough arguments for patterns requiring 2+ additional args
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

        assert_eq!(result.consumed_args_count(), 1);
        assert_eq!(result.consumed_args, vec!["-c"]);
    }

    #[test]
    fn test_equal_matching() {
        let matcher = FlagAnalyzer::new(test_flags());

        // Test glued with =
        let args = vec!["-std=c99".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_args_count(), 1);
        assert_eq!(result.consumed_args, vec!["-std=c99"]);

        // Test separate form
        let args = vec!["-std".to_string(), "c99".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_args_count(), 2);
        assert_eq!(result.consumed_args, vec!["-std", "c99"]);
    }

    #[test]
    fn test_glued_or_separate_matching() {
        let matcher = FlagAnalyzer::new(test_flags());

        // Test glued form
        let args = vec!["-I/usr/include".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_args_count(), 1);
        assert_eq!(result.consumed_args, vec!["-I/usr/include"]);

        // Test separate form
        let args = vec!["-I".to_string(), "/usr/include".to_string()];
        let result = matcher.match_flag(&args).unwrap();
        assert_eq!(result.consumed_args_count(), 2);
        assert_eq!(result.consumed_args, vec!["-I", "/usr/include"]);

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
        assert_eq!(result.consumed_args_count(), 1);
        assert_eq!(result.consumed_args, vec!["-Wall"]);
    }

    #[test]
    fn test_no_match() {
        let matcher = FlagAnalyzer::new(test_flags());

        let args = vec!["--unknown".to_string()];
        let result = matcher.match_flag(&args);
        assert!(result.is_none());
    }
}
