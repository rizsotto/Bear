// SPDX-License-Identifier: GPL-3.0-or-later

/// Table metadata: name of the static, which file to generate.
pub struct TableConfig {
    pub yaml_file: &'static str,
    pub static_name: &'static str,
    pub ignore_executables_name: &'static str,
    pub ignore_flags_name: &'static str,
    pub slash_prefix_name: &'static str,
    pub env_rules_name: &'static str,
    pub output_file: &'static str,
}

// Table order matters: it determines recognition pattern priority.
// More specific compilers (whose executable names could be mistaken for
// cross-compilation variants of general compilers) must come first.
// E.g., ibm-clang must match IbmXl before Clang's cross-compilation pattern.
pub const TABLES: &[TableConfig] = &[
    TableConfig {
        yaml_file: "gcc.yaml",
        static_name: "GCC_FLAGS",
        ignore_executables_name: "GCC_IGNORE_EXECUTABLES",
        ignore_flags_name: "GCC_IGNORE_FLAGS",
        slash_prefix_name: "GCC_SLASH_PREFIX",
        env_rules_name: "GCC_ENV_RULES",
        output_file: "flags_gcc.rs",
    },
    // IBM XL before Clang: ibm-clang looks like cross-compilation clang
    TableConfig {
        yaml_file: "ibm_xl.yaml",
        static_name: "IBM_XL_FLAGS",
        ignore_executables_name: "IBM_XL_IGNORE_EXECUTABLES",
        ignore_flags_name: "IBM_XL_IGNORE_FLAGS",
        slash_prefix_name: "IBM_XL_SLASH_PREFIX",
        env_rules_name: "IBM_XL_ENV_RULES",
        output_file: "flags_ibm_xl.rs",
    },
    // clang-cl before Clang: clang-cl is versioned and could match clang's pattern
    TableConfig {
        yaml_file: "clang_cl.yaml",
        static_name: "CLANG_CL_FLAGS",
        ignore_executables_name: "CLANG_CL_IGNORE_EXECUTABLES",
        ignore_flags_name: "CLANG_CL_IGNORE_FLAGS",
        slash_prefix_name: "CLANG_CL_SLASH_PREFIX",
        env_rules_name: "CLANG_CL_ENV_RULES",
        output_file: "flags_clang_cl.rs",
    },
    TableConfig {
        yaml_file: "clang.yaml",
        static_name: "CLANG_FLAGS",
        ignore_executables_name: "CLANG_IGNORE_EXECUTABLES",
        ignore_flags_name: "CLANG_IGNORE_FLAGS",
        slash_prefix_name: "CLANG_SLASH_PREFIX",
        env_rules_name: "CLANG_ENV_RULES",
        output_file: "flags_clang.rs",
    },
    TableConfig {
        yaml_file: "flang.yaml",
        static_name: "FLANG_FLAGS",
        ignore_executables_name: "FLANG_IGNORE_EXECUTABLES",
        ignore_flags_name: "FLANG_IGNORE_FLAGS",
        slash_prefix_name: "FLANG_SLASH_PREFIX",
        env_rules_name: "FLANG_ENV_RULES",
        output_file: "flags_flang.rs",
    },
    TableConfig {
        yaml_file: "cuda.yaml",
        static_name: "CUDA_FLAGS",
        ignore_executables_name: "CUDA_IGNORE_EXECUTABLES",
        ignore_flags_name: "CUDA_IGNORE_FLAGS",
        slash_prefix_name: "CUDA_SLASH_PREFIX",
        env_rules_name: "CUDA_ENV_RULES",
        output_file: "flags_cuda.rs",
    },
    TableConfig {
        yaml_file: "intel_fortran.yaml",
        static_name: "INTEL_FORTRAN_FLAGS",
        ignore_executables_name: "INTEL_FORTRAN_IGNORE_EXECUTABLES",
        ignore_flags_name: "INTEL_FORTRAN_IGNORE_FLAGS",
        slash_prefix_name: "INTEL_FORTRAN_SLASH_PREFIX",
        env_rules_name: "INTEL_FORTRAN_ENV_RULES",
        output_file: "flags_intel_fortran.rs",
    },
    TableConfig {
        yaml_file: "cray_fortran.yaml",
        static_name: "CRAY_FORTRAN_FLAGS",
        ignore_executables_name: "CRAY_FORTRAN_IGNORE_EXECUTABLES",
        ignore_flags_name: "CRAY_FORTRAN_IGNORE_FLAGS",
        slash_prefix_name: "CRAY_FORTRAN_SLASH_PREFIX",
        env_rules_name: "CRAY_FORTRAN_ENV_RULES",
        output_file: "flags_cray_fortran.rs",
    },
    TableConfig {
        yaml_file: "msvc.yaml",
        static_name: "MSVC_FLAGS",
        ignore_executables_name: "MSVC_IGNORE_EXECUTABLES",
        ignore_flags_name: "MSVC_IGNORE_FLAGS",
        slash_prefix_name: "MSVC_SLASH_PREFIX",
        env_rules_name: "MSVC_ENV_RULES",
        output_file: "flags_msvc.rs",
    },
    TableConfig {
        yaml_file: "intel_cc.yaml",
        static_name: "INTEL_CC_FLAGS",
        ignore_executables_name: "INTEL_CC_IGNORE_EXECUTABLES",
        ignore_flags_name: "INTEL_CC_IGNORE_FLAGS",
        slash_prefix_name: "INTEL_CC_SLASH_PREFIX",
        env_rules_name: "INTEL_CC_ENV_RULES",
        output_file: "flags_intel_cc.rs",
    },
    TableConfig {
        yaml_file: "nvidia_hpc.yaml",
        static_name: "NVIDIA_HPC_FLAGS",
        ignore_executables_name: "NVIDIA_HPC_IGNORE_EXECUTABLES",
        ignore_flags_name: "NVIDIA_HPC_IGNORE_FLAGS",
        slash_prefix_name: "NVIDIA_HPC_SLASH_PREFIX",
        env_rules_name: "NVIDIA_HPC_ENV_RULES",
        output_file: "flags_nvidia_hpc.rs",
    },
    TableConfig {
        yaml_file: "armclang.yaml",
        static_name: "ARMCLANG_FLAGS",
        ignore_executables_name: "ARMCLANG_IGNORE_EXECUTABLES",
        ignore_flags_name: "ARMCLANG_IGNORE_FLAGS",
        slash_prefix_name: "ARMCLANG_SLASH_PREFIX",
        env_rules_name: "ARMCLANG_ENV_RULES",
        output_file: "flags_armclang.rs",
    },
    TableConfig {
        yaml_file: "vala.yaml",
        static_name: "VALA_FLAGS",
        ignore_executables_name: "VALA_IGNORE_EXECUTABLES",
        ignore_flags_name: "VALA_IGNORE_FLAGS",
        slash_prefix_name: "VALA_SLASH_PREFIX",
        env_rules_name: "VALA_ENV_RULES",
        output_file: "flags_vala.rs",
    },
    // MPI wrapper names (mpicc, mpicxx, ...) do not overlap any other
    // compiler's recognition pattern, so position here does not affect
    // priority. Intel MPI wrappers are extra basenames on intel_cc/
    // intel_fortran above, not a separate table.
    TableConfig {
        yaml_file: "mpi.yaml",
        static_name: "MPI_FLAGS",
        ignore_executables_name: "MPI_IGNORE_EXECUTABLES",
        ignore_flags_name: "MPI_IGNORE_FLAGS",
        slash_prefix_name: "MPI_SLASH_PREFIX",
        env_rules_name: "MPI_ENV_RULES",
        output_file: "flags_mpi.rs",
    },
    // Cray CCE C/C++ names (craycc, crayCC, craycxx) do not overlap any
    // other compiler's recognition pattern, so position here does not
    // affect priority. The ambiguous PrgEnv wrapper "CC" is handled by the
    // version probe, not by this table (see cray_cc.yaml).
    TableConfig {
        yaml_file: "cray_cc.yaml",
        static_name: "CRAY_CC_FLAGS",
        ignore_executables_name: "CRAY_CC_IGNORE_EXECUTABLES",
        ignore_flags_name: "CRAY_CC_IGNORE_FLAGS",
        slash_prefix_name: "CRAY_CC_SLASH_PREFIX",
        env_rules_name: "CRAY_CC_ENV_RULES",
        output_file: "flags_cray_cc.rs",
    },
    // QNX driver names (qcc, q++) do not overlap any other compiler's
    // recognition pattern, so position here does not affect priority.
    TableConfig {
        yaml_file: "qnx.yaml",
        static_name: "QNX_FLAGS",
        ignore_executables_name: "QNX_IGNORE_EXECUTABLES",
        ignore_flags_name: "QNX_IGNORE_FLAGS",
        slash_prefix_name: "QNX_SLASH_PREFIX",
        env_rules_name: "QNX_ENV_RULES",
        output_file: "flags_qnx.rs",
    },
    // NASM/YASM names do not overlap any other compiler's recognition
    // pattern, so position here does not affect priority.
    TableConfig {
        yaml_file: "nasm.yaml",
        static_name: "NASM_FLAGS",
        ignore_executables_name: "NASM_IGNORE_EXECUTABLES",
        ignore_flags_name: "NASM_IGNORE_FLAGS",
        slash_prefix_name: "NASM_SLASH_PREFIX",
        env_rules_name: "NASM_ENV_RULES",
        output_file: "flags_nasm.rs",
    },
    // fasm does not overlap any other compiler's recognition pattern, so
    // position here does not affect priority.
    TableConfig {
        yaml_file: "fasm.yaml",
        static_name: "FASM_FLAGS",
        ignore_executables_name: "FASM_IGNORE_EXECUTABLES",
        ignore_flags_name: "FASM_IGNORE_FLAGS",
        slash_prefix_name: "FASM_SLASH_PREFIX",
        env_rules_name: "FASM_ENV_RULES",
        output_file: "flags_fasm.rs",
    },
    // swiftc does not overlap any other compiler's recognition pattern, so
    // position here does not affect priority. The internal swift-frontend
    // executable is filtered via ignore_when, the same mechanism gcc's cc1
    // uses, so it needs no separate table entry.
    TableConfig {
        yaml_file: "swift.yaml",
        static_name: "SWIFT_FLAGS",
        ignore_executables_name: "SWIFT_IGNORE_EXECUTABLES",
        ignore_flags_name: "SWIFT_IGNORE_FLAGS",
        slash_prefix_name: "SWIFT_SLASH_PREFIX",
        env_rules_name: "SWIFT_ENV_RULES",
        output_file: "flags_swift.rs",
    },
];
