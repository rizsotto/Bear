// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;

pub const KEY_DESTINATION: &str = "INTERCEPT_COLLECTOR_ADDRESS";

// man page for `ld.so` (Linux dynamic linker/loader)
pub const KEY_OS__PRELOAD_PATH: &str = "LD_PRELOAD";
// man page for `exec` (Linux system call)
pub const KEY_OS__PATH: &str = "PATH";

// https://gcc.gnu.org/onlinedocs/cpp/Environment-Variables.html
pub const KEY_GCC__C_INCLUDE_1: &str = "CPATH";
pub const KEY_GCC__C_INCLUDE_2: &str = "C_INCLUDE_PATH";
pub const KEY_GCC__C_INCLUDE_3: &str = "CPLUS_INCLUDE_PATH";
pub const KEY_GCC__OBJC_INCLUDE: &str = "OBJC_INCLUDE_PATH";

// https://www.gnu.org/software/make/manual/html_node/Implicit-Variables.html
pub const KEY_MAKE__C_COMPILER: &str = "CC";
pub const KEY_MAKE__CXX_COMPILER: &str = "CXX";
pub const KEY_MAKE__C_PREPROCESSOR: &str = "CPP";
pub const KEY_MAKE__FORTRAN_COMPILER: &str = "FC";
pub const KEY_MAKE__ARCHIVE: &str = "AR";
pub const KEY_MAKE__ASSEMBLER: &str = "AS";
pub const KEY_MAKE__MODULA_COMPILER: &str = "M2C";
pub const KEY_MAKE__PASCAL_COMPILER: &str = "PC";
pub const KEY_MAKE__LEX: &str = "LEX";
pub const KEY_MAKE__YACC: &str = "YACC";
pub const KEY_MAKE__LINT: &str = "LINT";

pub const KEY_MAKE__AR_FLAGS: &str = "ARFLAGS";
pub const KEY_MAKE__AS_FLAGS: &str = "ASFLAGS";
pub const KEY_MAKE__C_FLAGS: &str = "CFLAGS";
pub const KEY_MAKE__CXX_FLAGS: &str = "CXXFLAGS";
pub const KEY_MAKE__C_PREPROCESSOR_FLAGS: &str = "CPPFLAGS";
pub const KEY_MAKE__FORTRAN_FLAGS: &str = "FFLAGS";
pub const KEY_MAKE__LINKER_FLAGS: &str = "LDFLAGS";
pub const KEY_MAKE__LINKER_LIBS: &str = "LDLIBS";
pub const KEY_MAKE__LEX_FLAGS: &str = "LFLAGS";
pub const KEY_MAKE__YACC_FLAGS: &str = "YFLAGS";
pub const KEY_MAKE__PASCAL_FLAGS: &str = "PFLAGS";
pub const KEY_MAKE__LINT_FLAGS: &str = "LINTFLAGS";

// https://doc.rust-lang.org/cargo/reference/environment-variables.html
pub const KEY_CARGO__CARGO: &str = "CARGO";
pub const KEY_CARGO__RUSTC: &str = "RUSTC";
pub const KEY_CARGO__RUSTC_WRAPPER: &str = "RUSTC_WRAPPER";

pub const KEY_CARGO__RUSTFLAGS: &str = "RUSTFLAGS";

static MAKE_PROGRAM_KEYS: std::sync::LazyLock<HashSet<&'static str>> =
    std::sync::LazyLock::new(|| {
        [
            KEY_MAKE__C_COMPILER,
            KEY_MAKE__CXX_COMPILER,
            KEY_MAKE__C_PREPROCESSOR,
            KEY_MAKE__FORTRAN_COMPILER,
            KEY_MAKE__ARCHIVE,
            KEY_MAKE__ASSEMBLER,
            KEY_MAKE__MODULA_COMPILER,
            KEY_MAKE__PASCAL_COMPILER,
            KEY_MAKE__LEX,
            KEY_MAKE__YACC,
            KEY_MAKE__LINT,
        ]
        .iter()
        .cloned()
        .collect()
    });

static MAKE_FLAGS_KEYS: std::sync::LazyLock<HashSet<&'static str>> =
    std::sync::LazyLock::new(|| {
        [
            KEY_MAKE__AR_FLAGS,
            KEY_MAKE__AS_FLAGS,
            KEY_MAKE__C_FLAGS,
            KEY_MAKE__CXX_FLAGS,
            KEY_MAKE__C_PREPROCESSOR_FLAGS,
            KEY_MAKE__FORTRAN_FLAGS,
            KEY_MAKE__LINKER_FLAGS,
            KEY_MAKE__LINKER_LIBS,
            KEY_MAKE__LEX_FLAGS,
            KEY_MAKE__YACC_FLAGS,
            KEY_MAKE__PASCAL_FLAGS,
            KEY_MAKE__LINT_FLAGS,
        ]
        .iter()
        .cloned()
        .collect()
    });

static CARGO_PROGRAM_KEYS: std::sync::LazyLock<HashSet<&'static str>> =
    std::sync::LazyLock::new(|| {
        [KEY_CARGO__CARGO, KEY_CARGO__RUSTC, KEY_CARGO__RUSTC_WRAPPER]
            .iter()
            .cloned()
            .collect()
    });

static CARGO_FLAGS_KEYS: std::sync::LazyLock<HashSet<&'static str>> =
    std::sync::LazyLock::new(|| [KEY_CARGO__RUSTFLAGS].iter().cloned().collect());

static GCC_INCLUDE_KEYS: std::sync::LazyLock<HashSet<&'static str>> =
    std::sync::LazyLock::new(|| {
        [
            KEY_GCC__C_INCLUDE_1,
            KEY_GCC__C_INCLUDE_2,
            KEY_GCC__C_INCLUDE_3,
            KEY_GCC__OBJC_INCLUDE,
        ]
        .iter()
        .cloned()
        .collect()
    });

pub fn relevant_env(key: &str) -> bool {
    matches!(key, KEY_DESTINATION | KEY_OS__PRELOAD_PATH)
        || MAKE_PROGRAM_KEYS.contains(key)
        || MAKE_FLAGS_KEYS.contains(key)
        || CARGO_PROGRAM_KEYS.contains(key)
        || CARGO_FLAGS_KEYS.contains(key)
        || GCC_INCLUDE_KEYS.contains(key)
        // Windows PATH variable is case sensitive and not always capitalized
        || key.to_uppercase() == KEY_OS__PATH
}

pub fn program_env(key: &str) -> bool {
    MAKE_PROGRAM_KEYS.contains(key) || CARGO_PROGRAM_KEYS.contains(key)
}
