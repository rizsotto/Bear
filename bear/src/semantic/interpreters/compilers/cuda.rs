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
use super::gcc::{GCC_FLAGS, parse_arguments_and_environment};
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

/// CUDA (nvcc) flag definitions using pattern matching for argument parsing
///
/// Based on NVIDIA CUDA Compiler documentation:
/// https://docs.nvidia.com/cuda/cuda-compiler-driver-nvcc/
pub static CUDA_FLAGS: std::sync::LazyLock<Vec<FlagRule>> = std::sync::LazyLock::new(|| {
    let mut flags = Vec::new();

    // Add CUDA-specific flags first
    flags.extend([
        // GPU Architecture and Code Generation
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--gpu-architecture"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-arch"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--gpu-code"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-code"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--generate-code"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-gencode"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Compilation Modes
        FlagRule::new(
            FlagPattern::Exactly("--compile", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-c", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--device-c", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-dc", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--device-w", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-dw", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--device-link", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-dlink", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--link", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--lib", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(FlagPattern::Exactly("--run", 0), ArgumentKind::Other(PassEffect::None)),
        // CUDA Runtime and Toolkit
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--cudart"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--cuda-path"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Host Compiler Options
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--compiler-bindir"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-ccbin"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-Xcompiler", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-Xlinker", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        // Device-Specific Options
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--maxrregcount"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-maxrregcount"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--use_fast_math", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-use_fast_math", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--ftz", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--prec-div", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--prec-sqrt", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--fmad", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // PTX and SASS Options
        FlagRule::new(
            FlagPattern::Exactly("--ptx", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--cubin", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--fatbin", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Debug and Profiling
        FlagRule::new(
            FlagPattern::Exactly("--device-debug", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-G", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--generate-line-info", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-lineinfo", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--profile", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Optimization
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--optimize"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-O"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Language Standard
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--std"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Relocatable Device Code
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--relocatable-device-code"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-rdc", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Extended Lambda
        FlagRule::new(
            FlagPattern::Exactly("--extended-lambda", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-extended-lambda", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Experimental Features
        FlagRule::new(
            FlagPattern::Exactly("--expt-extended-lambda", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--expt-relaxed-constexpr", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        // Output Options
        FlagRule::new(FlagPattern::ExactlyWithGluedOrSep("--output-file"), ArgumentKind::Output),
        FlagRule::new(FlagPattern::ExactlyWithGluedOrSep("-o"), ArgumentKind::Output),
        // Preprocessing
        FlagRule::new(
            FlagPattern::Exactly("--preprocess", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-E", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)),
        ),
        // Verbose and Information
        FlagRule::new(FlagPattern::Exactly("--verbose", 0), ArgumentKind::Other(PassEffect::DriverOption)),
        FlagRule::new(FlagPattern::Exactly("-v", 0), ArgumentKind::Other(PassEffect::DriverOption)),
        FlagRule::new(FlagPattern::Exactly("--version", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(FlagPattern::Exactly("--help", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        // Warnings
        FlagRule::new(FlagPattern::Prefix("-W", 0), ArgumentKind::Other(PassEffect::None)),
        FlagRule::new(FlagPattern::Prefix("--disable-warnings", 0), ArgumentKind::Other(PassEffect::None)),
        // CUDA-specific include paths
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--include-path"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        // Keep intermediate files
        FlagRule::new(FlagPattern::Exactly("--keep", 0), ArgumentKind::Other(PassEffect::DriverOption)),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--keep-dir"),
            ArgumentKind::Other(PassEffect::DriverOption),
        ),
        // Machine and Target Options
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--machine"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-m"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
    ]);

    // Add GCC flags after CUDA flags (CUDA flags have priority)
    flags.extend(GCC_FLAGS.iter().cloned());
    flags
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intercept::Execution;
    use crate::semantic::PassEffect;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_execution(program: &str, arguments: Vec<&str>) -> Execution {
        Execution {
            executable: PathBuf::from(program),
            arguments: arguments.into_iter().map(|s| s.to_string()).collect(),
            working_dir: PathBuf::from("/test"),
            environment: HashMap::new(),
        }
    }

    #[test]
    fn test_basic_cuda_compilation() {
        let interpreter = CudaInterpreter::new();
        let execution =
            create_execution("/usr/local/cuda/bin/nvcc", vec!["nvcc", "-c", "kernel.cu", "-o", "kernel.o"]);

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 4);

            // Index 0: nvcc (compiler)
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            // Index 1: -c (compilation flag)
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );

            // Index 2: kernel.cu (source file)
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source { binary: false });

            // Index 3: -o kernel.o (output file)
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Output);
            assert_eq!(
                cmd.arguments[3].as_arguments(&|p| std::borrow::Cow::Borrowed(p)),
                vec!["-o", "kernel.o"]
            );
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_gpu_architecture_flags() {
        let interpreter = CudaInterpreter::new();
        let execution = create_execution(
            "nvcc",
            vec![
                "nvcc",
                "--gpu-architecture=sm_80",
                "-arch=sm_70",
                "--gpu-code=sm_80,compute_80",
                "-c",
                "kernel.cu",
            ],
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 6);

            // Check that GPU architecture flags are recognized as compilation flags
            // Index 1: --gpu-architecture=sm_80
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            // Index 2: -arch=sm_70
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            // Index 3: --gpu-code=sm_80,compute_80
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            // Index 4: -c
            assert_eq!(
                cmd.arguments[4].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
            // Index 5: kernel.cu (source file)
            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_device_compilation_modes() {
        let interpreter = CudaInterpreter::new();
        let execution = create_execution(
            "nvcc",
            vec!["nvcc", "--device-c", "--relocatable-device-code=true", "kernel.cu", "-o", "kernel.o"],
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 5);

            // Index 1: --device-c
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );

            // Index 2: --relocatable-device-code=true
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            // Index 3: kernel.cu (source file)
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source { binary: false });

            // Index 4: -o kernel.o (output)
            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Output);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_host_compiler_passthrough() {
        let interpreter = CudaInterpreter::new();
        let execution = create_execution(
            "nvcc",
            vec!["nvcc", "-Xcompiler", "-Wall", "-Xlinker", "-rpath=/usr/lib", "main.cu"],
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 6);

            // Index 0: nvcc (compiler)
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            // Index 1: -Xcompiler
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            // Index 2: -Wall
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Other(PassEffect::None));

            // Index 3: -Xlinker
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );

            // Index 4: -rpath=/usr/lib
            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Other(PassEffect::None));

            // Index 5: main.cu (source file)
            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_debug_and_optimization() {
        let interpreter = CudaInterpreter::new();
        let execution = create_execution(
            "nvcc",
            vec!["nvcc", "-G", "--generate-line-info", "-O2", "--use_fast_math", "kernel.cu"],
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 6);

            // Index 0: nvcc (compiler)
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            // Index 1-4: All compilation flags
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }

            // Index 5: Source file
            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_flag_formats() {
        let interpreter = CudaInterpreter::new();

        // Test equals format
        let execution = create_execution("nvcc", vec!["nvcc", "--gpu-architecture=sm_80", "-c", "kernel.cu"]);
        let result = interpreter.recognize(&execution);
        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 4);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
        }

        // Test separate format
        let execution =
            create_execution("nvcc", vec!["nvcc", "--gpu-architecture", "sm_80", "-c", "kernel.cu"]);
        let result = interpreter.recognize(&execution);
        if let Some(Command::Compiler(cmd)) = result {
            // Separate format creates one argument that consumes both the flag and value
            assert_eq!(cmd.arguments.len(), 4);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
        }
    }

    #[test]
    fn test_cuda_specific_flags() {
        let interpreter = CudaInterpreter::new();

        // Test CUDA-specific --compile flag
        let execution = create_execution("nvcc", vec!["nvcc", "--compile", "kernel.cu"]);

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 3);
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }
}
