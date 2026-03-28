// SPDX-License-Identifier: GPL-3.0-or-later

//! CUDA compiler (nvcc) command-line argument parser for compilation database generation.
//!
//! This module provides a specialized interpreter for parsing NVIDIA CUDA Compiler (nvcc)
//! command lines. NVCC is a meta-compiler that separates device code from host code and
//! compiles them with appropriate compilers. It supports many GCC-compatible flags since
//! it often delegates host compilation to GCC or Clang.
//!
//! The interpreter recognizes various CUDA-specific flags and categorizes them into semantic
//! groups (source files, output files, compilation options, etc.) to generate accurate
//! compilation database entries for CUDA-based projects.

use super::super::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use super::gcc::parse_arguments_and_environment;
use crate::semantic::{
    ArgumentKind, Command, CompilerCommand, CompilerPass, Execution, Interpreter, PassEffect,
};

/// CUDA compiler (nvcc) command-line argument parser that extracts semantic information from compiler invocations.
///
/// This interpreter processes NVIDIA CUDA Compiler command lines to identify:
/// - CUDA source files (.cu, .cuh) and host source files being compiled
/// - Output files and directories
/// - CUDA-specific compiler flags (GPU architecture, compilation modes, etc.)
/// - Host compiler flags that are passed through to the host compiler
/// - Include directories and preprocessor definitions
///
/// NVCC supports many GCC-compatible flags since it delegates host compilation to
/// GCC or Clang, so this interpreter extends GCC flag definitions with CUDA-specific flags.
pub struct CudaInterpreter {
    /// Flag analyzer that recognizes and categorizes CUDA command-line flags
    /// (includes GCC-compatible flags plus CUDA-specific extensions)
    matcher: FlagAnalyzer,
}

impl Default for CudaInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl CudaInterpreter {
    /// Creates a new CUDA interpreter with comprehensive CUDA flag definitions.
    ///
    /// The interpreter is configured with patterns to recognize both GCC-compatible flags
    /// and CUDA-specific extensions including GPU architecture specifications,
    /// device/host compilation modes, CUDA runtime flags, and nvcc-specific options.
    pub fn new() -> Self {
        Self { matcher: FlagAnalyzer::new(&CUDA_FLAGS) }
    }
}

impl Interpreter for CudaInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // Parse both command-line arguments and environment variables
        let annotated_args = parse_arguments_and_environment(&self.matcher, execution);

        Some(Command::Compiler(CompilerCommand::new(
            execution.working_dir.clone(),
            execution.executable.clone(),
            annotated_args,
        )))
    }
}

// CUDA flag definitions. Generated at build time from flags/cuda.yaml.
include!(concat!(env!("OUT_DIR"), "/flags_cuda.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_table_invariants() {
        crate::semantic::interpreters::compilers::gcc::assert_flag_table_invariants(&CUDA_FLAGS);
    }
}
