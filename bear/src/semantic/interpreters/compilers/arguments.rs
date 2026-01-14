// SPDX-License-Identifier: GPL-3.0-or-later

//! Specialized Arguments implementations for different compiler argument types.
//!
//! This module provides concrete implementations of the [`Arguments`] trait
//! for various types of compiler arguments, enabling more sophisticated
//! argument parsing than the basic [`BasicArguments`] implementation.

use crate::semantic::{ArgumentKind, Arguments};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Represents a generic argument that holds an ArgumentKind and a vector of strings.
///
/// This handles all argument types except source and output files:
/// - Simple flags: `-c`, `-Wall`
/// - Flags with values: `-I /usr/include`, `-D MACRO=value`
/// - Combined flags: `-Ipath`, `-DMACRO=value`
/// - Compiler executable names
/// - Response files: `@file`
/// - Any other arguments
#[derive(Debug, Clone, PartialEq)]
pub struct OtherArguments {
    /// The argument strings (e.g., ["-I", "/usr/include"] or ["-Wall"])
    arguments: Vec<String>,
    /// The semantic meaning of this argument
    kind: ArgumentKind,
}

impl OtherArguments {
    /// Creates a new argument with the given strings and kind.
    pub fn new(arguments: Vec<String>, kind: ArgumentKind) -> Self {
        Self { arguments, kind }
    }
}

impl Arguments for OtherArguments {
    fn kind(&self) -> ArgumentKind {
        self.kind
    }

    fn as_arguments(&self, _path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        self.arguments.clone()
    }

    fn as_file(&self, _path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        // Other arguments don't represent files directly
        None
    }
}

/// Represents a source file argument.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceArgument {
    /// Path to the source file
    path: String,
}

impl SourceArgument {
    /// Creates a new source file argument.
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

impl Arguments for SourceArgument {
    fn kind(&self) -> ArgumentKind {
        ArgumentKind::Source
    }

    fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        let path = Path::new(&self.path);
        let updated_path = path_updater(path);
        vec![updated_path.to_string_lossy().to_string()]
    }

    fn as_file(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        let path = Path::new(&self.path);
        let updated_path = path_updater(path);
        Some(updated_path.to_path_buf())
    }
}

/// Represents an output file argument.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputArgument {
    /// The output flag (usually "-o")
    flag: String,
    /// Path to the output file
    path: String,
}

impl OutputArgument {
    /// Creates a new output argument.
    pub fn new(flag: String, path: String) -> Self {
        Self { flag, path }
    }
}

impl Arguments for OutputArgument {
    fn kind(&self) -> ArgumentKind {
        ArgumentKind::Output
    }

    fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        let path = Path::new(&self.path);
        let updated_path = path_updater(path);
        vec![self.flag.clone(), updated_path.to_string_lossy().to_string()]
    }

    fn as_file(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        let path = Path::new(&self.path);
        let updated_path = path_updater(path);
        Some(updated_path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{CompilerPass, PassEffect};

    #[test]
    fn test_other_arguments_new_with_simple_flag() {
        let arg = OtherArguments::new(
            vec!["-c".to_string()],
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        );

        assert_eq!(arg.kind(), ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)));
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["-c"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), None);
    }

    #[test]
    fn test_other_arguments_new_with_flag_and_value() {
        let arg = OtherArguments::new(
            vec!["-I".to_string(), "/usr/include".to_string()],
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        );

        assert_eq!(arg.kind(), ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)));
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["-I", "/usr/include"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), None);
    }

    #[test]
    fn test_other_arguments_new_with_combined_flag() {
        let arg = OtherArguments::new(
            vec!["-I/usr/include".to_string()],
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        );

        assert_eq!(arg.kind(), ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)));
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["-I/usr/include"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), None);
    }

    #[test]
    fn test_other_arguments_new_with_compiler() {
        let arg = OtherArguments::new(vec!["gcc".to_string()], ArgumentKind::Compiler);

        assert_eq!(arg.kind(), ArgumentKind::Compiler);
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["gcc"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), None);
    }

    #[test]
    fn test_other_arguments_new_with_response_file() {
        let arg =
            OtherArguments::new(vec!["@response.txt".to_string()], ArgumentKind::Other(PassEffect::None));

        assert_eq!(arg.kind(), ArgumentKind::Other(PassEffect::None));
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["@response.txt"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), None);
    }

    #[test]
    fn test_source_argument() {
        let arg = SourceArgument::new("main.c".to_string());

        assert_eq!(arg.kind(), ArgumentKind::Source);
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["main.c"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), Some(PathBuf::from("main.c")));
    }

    #[test]
    fn test_output_argument() {
        let arg = OutputArgument::new("-o".to_string(), "main.o".to_string());

        assert_eq!(arg.kind(), ArgumentKind::Output);
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["-o", "main.o"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), Some(PathBuf::from("main.o")));
    }

    #[test]
    fn test_path_updater_functionality() {
        let arg = SourceArgument::new("src/main.c".to_string());

        // Test with identity path updater (no change)
        assert_eq!(arg.as_arguments(&|p| std::borrow::Cow::Borrowed(p)), vec!["src/main.c"]);
        assert_eq!(arg.as_file(&|p| std::borrow::Cow::Borrowed(p)), Some(PathBuf::from("src/main.c")));
    }

    #[test]
    fn test_other_arguments_new() {
        let arg = OtherArguments::new(
            vec!["-D".to_string(), "MACRO=value".to_string()],
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        );

        assert_eq!(arg.kind(), ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)));
        assert_eq!(arg.as_arguments(&|p| Cow::Borrowed(p)), vec!["-D", "MACRO=value"]);
        assert_eq!(arg.as_file(&|p| Cow::Borrowed(p)), None);
    }
}
