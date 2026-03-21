// SPDX-License-Identifier: GPL-3.0-or-later

//! Hardened interception tests for Bear
//!
//! These tests verify that Bear's interception mechanism survives when build
//! scripts and programs deliberately sabotage environment variables
//! (`LD_PRELOAD`, `BEAR_INTERCEPT`).
//!
//! ## Test Categories
//!
//! ### Shell-level environment sabotage (HI-1 through HI-5)
//!
//! Tests where shell scripts modify the environment before invoking compilers.
//! These work because the preload library is already loaded in the shell
//! process and intercepts the subsequent `exec*` call. The `resolve_environment`
//! function in `implementation.rs` detects that the envp is missing the session
//! variables and creates a "doctored" copy with `LD_PRELOAD` and `BEAR_INTERCEPT`
//! restored before forwarding to the real `exec*`.
//!
//! ### system()/popen() after environ modification (HI-6, HI-7)
//!
//! Simulates issue #660: a C program (standing in for a build orchestrator like
//! perl) calls `unsetenv("LD_PRELOAD")` then `system("gcc ...")`.
//!
//! The concern is that `system()` and `popen()` use the process's `environ`
//! internally and do not accept an explicit envp we could doctor. On some glibc
//! versions their internal `fork+execve` path bypasses the PLT, so our
//! LD_PRELOAD override of `execve` never fires and the child starts without
//! the preload library.
//!
//! **Why this currently passes (glibc ≥ 2.34 on x86-64):**
//!
//! glibc 2.34+ rewrote `system()` to call `posix_spawn` internally
//! (`do_system` → `posix_spawn@@GLIBC_2.15` → `__spawni` → `__spawnix`).
//! `__spawni` loads the address of the `execve` *symbol* (not `__execve`) and
//! passes it as a function pointer to `__spawnix`, which runs it in the child
//! after `clone(CLONE_VM | CLONE_VFORK)`. Because the child shares the
//! parent's virtual memory (including GOT/PLT entries patched by LD_PRELOAD),
//! the function pointer resolves to **our** overridden `execve`, not libc's
//! raw syscall wrapper. Our override doctors the environment, so the shell
//! child gets `LD_PRELOAD` and `BEAR_INTERCEPT` back.
//!
//! **When this would fail:**
//!
//! - glibc < 2.34 where `system()` uses `fork()` + a direct internal
//!   `__execve()` call that bypasses the PLT entirely.
//! - Any libc (musl, bionic) that calls the `execve` syscall directly
//!   in its `system()` implementation.
//! - Statically linked programs that don't load shared libraries at all.
//!
//! If these tests start failing on a new platform, the fix is to have
//! `rust_system` / `rust_popen` restore `LD_PRELOAD` and `BEAR_INTERCEPT`
//! in the process's `environ` (via `setenv`) before calling the real function.
//!
//! ### Competing LD_PRELOAD libraries (HI-8)
//!
//! Simulates issue #675: another LD_PRELOAD library (`libsandbox.so` on
//! Gentoo) coexists with Bear's.
//!
//! **Why this currently passes:**
//!
//! When two LD_PRELOAD libraries both override `execve`, `dlsym(RTLD_NEXT)`
//! from the first library resolves to the second library's `execve`, which
//! in turn resolves to libc's. This forms a working chain as long as each
//! link delegates correctly. Our trivial mock library does exactly that.
//!
//! **When this would fail:**
//!
//! - The competing library modifies the envp (strips `LD_PRELOAD`) before
//!   delegating — our doctoring would have already run, but the downstream
//!   library could undo it.
//! - The competing library calls the `execve` **syscall** directly (via
//!   `syscall(SYS_execve, ...)`) instead of using `dlsym(RTLD_NEXT)`.
//! - On architectures with different calling conventions for variadic
//!   functions (e.g., i686), two chained shim layers for `execle` may
//!   corrupt the stack — this is the likely cause of the SIGSEGV in
//!   issue #675 on x86 (32-bit).

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

/// Config to enforce preload mode (same as intercept_posix tests).
const CONFIG: &str = concat!(
    r#"schema: '4.1'

intercept:
  mode: preload
  path: "#,
    env!("PRELOAD_LIBRARY_PATH"),
    r#"
"#
);

// =========================================================================
// Shell-level environment sabotage tests (HI-1 through HI-5)
//
// These all work because the preload library is loaded in the shell and
// doctors the envp on every exec* call via `resolve_environment`.
// =========================================================================

/// HI-1: Build script that unsets LD_PRELOAD before compiling.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn hardened_unset_ld_preload() -> Result<()> {
    let env = TestEnvironment::new("hardened_unset_ld_preload")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let build_commands =
        ["unset LD_PRELOAD".to_string(), format!("{} -c test.c -o test.o", filename_of(COMPILER_C_PATH))]
            .join("\n");
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "test.c".to_string(),
            "-o".to_string(),
            "test.o".to_string(),
        ]
    ))?;

    Ok(())
}

/// HI-2: Build script that overwrites BEAR_INTERCEPT with garbage.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn hardened_overwrite_bear_intercept() -> Result<()> {
    let env = TestEnvironment::new("hardened_overwrite_bear_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let build_commands = [
        "export BEAR_INTERCEPT=garbage".to_string(),
        format!("{} -c test.c -o test.o", filename_of(COMPILER_C_PATH)),
    ]
    .join("\n");
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "test.c".to_string(),
            "-o".to_string(),
            "test.o".to_string(),
        ]
    ))?;

    Ok(())
}

/// HI-3: Build script that clears the entire environment with `env -i`.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell, has_executable_env))]
fn hardened_env_clear() -> Result<()> {
    let env = TestEnvironment::new("hardened_env_clear")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let build_commands = format!(
        "{env} -i PATH=/usr/bin:/bin {compiler} -c test.c -o test.o",
        env = ENV_PATH,
        compiler = COMPILER_C_PATH,
    );
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "test.c".to_string(),
            "-o".to_string(),
            "test.o".to_string(),
        ]
    ))?;

    Ok(())
}

/// HI-4: Nested make invocations where the inner make clears the environment.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell, has_executable_make))]
fn hardened_nested_make_env_cleared() -> Result<()> {
    let env = TestEnvironment::new("hardened_nested_make")?;

    env.create_source_files(&[
        ("outer.c", "int outer() { return 1; }"),
        ("inner/inner.c", "int inner() { return 2; }"),
    ])?;

    let inner_makefile =
        format!("all:\n\t{compiler} -c inner.c -o inner.o\n", compiler = filename_of(COMPILER_C_PATH),);
    env.create_makefile("inner/Makefile", &inner_makefile)?;

    // NOTE: `inner` must be .PHONY because `inner/` is a real directory;
    // without this, make considers the target up-to-date and skips the recipe.
    let outer_makefile = format!(
        concat!(
            ".PHONY: inner\n",
            "all: outer.o inner\n",
            "outer.o:\n",
            "\t{compiler} -c outer.c -o outer.o\n",
            "inner:\n",
            "\tunset LD_PRELOAD && $(MAKE) -C inner\n",
        ),
        compiler = filename_of(COMPILER_C_PATH),
    );
    env.create_makefile("Makefile", &outer_makefile)?;

    env.run_bear_success(&["--output", "compile_commands.json", "--", MAKE_PATH])?;

    let db = env.load_compilation_database("compile_commands.json")?;

    db.assert_contains(&compilation_entry!(
        file: "outer.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "outer.c".to_string(),
            "-o".to_string(),
            "outer.o".to_string(),
        ]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "inner.c".to_string(),
        directory: env.test_dir().join("inner").to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "inner.c".to_string(),
            "-o".to_string(),
            "inner.o".to_string(),
        ]
    ))?;

    Ok(())
}

/// HI-5: Wrapper script that does `exec env -i gcc "$@"`.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell, has_executable_env))]
fn hardened_wrapper_exec_env_clear() -> Result<()> {
    let env = TestEnvironment::new("hardened_wrapper_exec_env_clear")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let wrapper_commands = format!(
        "exec {env} -i PATH=/usr/bin:/bin {compiler} \"$@\"",
        env = ENV_PATH,
        compiler = COMPILER_C_PATH,
    );
    let wrapper_path = env.create_shell_script("cc-wrapper.sh", &wrapper_commands)?;

    let build_commands = format!("{} -c test.c -o test.o", wrapper_path.to_str().unwrap());
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "test.c".to_string(),
            "-o".to_string(),
            "test.o".to_string(),
        ]
    ))?;

    Ok(())
}

// =========================================================================
// system()/popen() after environ modification (simulates #660)
//
// See module-level docs for why these pass on glibc ≥ 2.34 and when
// they would fail.
// =========================================================================

/// HI-6: C program that unsets LD_PRELOAD+BEAR_INTERCEPT, then uses system() to compile.
///
/// Simulates issue #660: a build orchestrator strips Bear's environment
/// variables from its own `environ`, then spawns compilation via `system()`.
///
/// On glibc ≥ 2.34 (x86-64), `system()` calls `posix_spawn` internally.
/// The `__spawni` helper loads the address of the `execve` *symbol* and
/// passes it to the child via `clone(CLONE_VM)`. Because the child shares
/// the parent's GOT/PLT (patched by LD_PRELOAD), the call resolves to our
/// overridden `execve`, which doctors the environment. This is why the test
/// passes here.
///
/// On older glibc or other libc implementations, `system()` may call an
/// internal `__execve` that bypasses the PLT — in that case this test would
/// fail, proving the need for `rust_system` to restore the session variables
/// in `environ` before calling the real `system()`.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn hardened_system_after_unsetenv() -> Result<()> {
    let env = TestEnvironment::new("hardened_system_after_unsetenv")?;

    env.create_source_files(&[("hello.c", "int main() { return 0; }")])?;

    let c_program = format!(
        r#"#include <stdlib.h>

int main() {{
    /* Strip Bear's environment variables — simulates a build tool
       that sanitizes the environment before spawning sub-processes. */
    unsetenv("LD_PRELOAD");
    unsetenv("BEAR_INTERCEPT");

    /* Now invoke the compiler via system().
       system() uses the process's environ internally. */
    return system("{compiler} -c hello.c -o hello.o");
}}"#,
        compiler = COMPILER_C_PATH,
    );

    env.create_source_files(&[("orchestrator.c", &c_program)])?;
    env.run_c_compiler("orchestrator", &["orchestrator.c"])?;
    env.create_config(CONFIG)?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "--output",
        "compile_commands.json",
        "--",
        "./orchestrator",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;

    // The compiler invocation inside system() should be captured.
    // NOTE: not asserting exact count because ccache may produce extra entries.
    db.assert_contains(&compilation_entry!(
        file: "hello.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "hello.c".to_string(),
            "-o".to_string(),
            "hello.o".to_string(),
        ]
    ))?;

    Ok(())
}

/// HI-7: C program that unsets LD_PRELOAD+BEAR_INTERCEPT, then uses popen() to compile.
///
/// Same mechanism as HI-6 but through `popen()`. See HI-6 docs for the
/// glibc version dependency.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn hardened_popen_after_unsetenv() -> Result<()> {
    let env = TestEnvironment::new("hardened_popen_after_unsetenv")?;

    env.create_source_files(&[("hello.c", "int main() { return 0; }")])?;
    env.create_config(CONFIG)?;

    let c_program = format!(
        r#"#include <stdlib.h>
#include <stdio.h>

int main() {{
    /* Strip Bear's environment variables */
    unsetenv("LD_PRELOAD");
    unsetenv("BEAR_INTERCEPT");

    /* Use popen to invoke the compiler */
    FILE *fp = popen("{compiler} -c hello.c -o hello.o 2>&1", "r");
    if (!fp) return 1;

    /* Drain output */
    char buf[256];
    while (fgets(buf, sizeof(buf), fp)) {{}}

    return pclose(fp) == 0 ? 0 : 1;
}}"#,
        compiler = COMPILER_C_PATH,
    );

    env.create_source_files(&[("orchestrator.c", &c_program)])?;
    env.run_c_compiler("orchestrator", &["orchestrator.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "--output",
        "compile_commands.json",
        "--",
        "./orchestrator",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;

    // The compiler invocation inside popen() should be captured.
    // NOTE: not asserting exact count because ccache may produce extra entries.
    db.assert_contains(&compilation_entry!(
        file: "hello.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "hello.c".to_string(),
            "-o".to_string(),
            "hello.o".to_string(),
        ]
    ))?;

    Ok(())
}

// =========================================================================
// Competing LD_PRELOAD library (simulates #675)
// =========================================================================

/// HI-8: Another LD_PRELOAD library coexists with Bear's.
///
/// Simulates issue #675 where Gentoo's `libsandbox.so` is already in
/// LD_PRELOAD when Bear starts. We build a trivial shared library that also
/// overrides `execve` (just delegates to the real one via RTLD_NEXT), append
/// it to LD_PRELOAD, and verify that interception still works.
///
/// This works because LD_PRELOAD libraries form a chain: our `execve` calls
/// `dlsym(RTLD_NEXT, "execve")` which resolves to the competing library's
/// `execve`, which in turn calls `dlsym(RTLD_NEXT, "execve")` → libc.
///
/// The real #675 issue involves additional complexity:
/// - `libsandbox.so` may modify the environment or refuse certain exec calls.
/// - On i686 (32-bit), chained variadic function overrides (`execle`) can
///   corrupt the stack — likely the cause of the SIGSEGV reported in #675.
///   This test covers the basic "chain of RTLD_NEXT" scenario only.
///
/// See also HI-9 which tests the harder case: competing library + `env -i`.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn hardened_competing_ld_preload() -> Result<()> {
    let env = TestEnvironment::new("hardened_competing_ld_preload")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Build a trivial shared library that overrides execve.
    // It just calls the real execve via dlsym(RTLD_NEXT, ...).
    // This simulates what libsandbox.so does.
    let competing_lib_source = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <unistd.h>

typedef int (*execve_func_t)(const char *, char *const[], char *const[]);

int execve(const char *path, char *const argv[], char *const envp[]) {
    execve_func_t real_execve = (execve_func_t)dlsym(RTLD_NEXT, "execve");
    if (!real_execve) return -1;
    return real_execve(path, argv, envp);
}
"#;

    env.create_source_files(&[("competing_lib.c", competing_lib_source)])?;

    // Compile the competing shared library
    let compile_result = {
        let mut cmd = std::process::Command::new(COMPILER_C_PATH);
        cmd.current_dir(env.test_dir()).args([
            "-shared",
            "-fPIC",
            "-o",
            "libcompeting.so",
            "competing_lib.c",
            "-ldl",
        ]);
        cmd.output()?
    };
    if !compile_result.status.success() {
        anyhow::bail!(
            "Failed to compile competing library: {}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
    }

    // Build script that compiles with the competing library in LD_PRELOAD
    let build_commands = format!(
        "export LD_PRELOAD=\"$LD_PRELOAD:{}\" && {} -c test.c -o test.o",
        env.test_dir().join("libcompeting.so").to_str().unwrap(),
        filename_of(COMPILER_C_PATH),
    );
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "test.c".to_string(),
            "-o".to_string(),
            "test.o".to_string(),
        ]
    ))?;

    Ok(())
}

/// HI-9: Competing LD_PRELOAD library survives exec with empty envp.
///
/// Simulates the core #675 failure: a competing LD_PRELOAD library (like
/// Gentoo's `libsandbox.so`) coexists with Bear's, and a program calls
/// `execve` with a completely empty envp. When Bear's doctoring restores
/// `LD_PRELOAD`, it must include the competing library — not just its own.
///
/// The competing library here **enforces** its presence: its `execve`
/// override checks that its path appears in the envp's `LD_PRELOAD` and
/// rejects the exec with `EPERM` if missing. This simulates sandbox's
/// self-protection.
///
/// We use a C program that calls `execve` directly (not `env -i`) because
/// `env` uses `execvp` → `execvpe` which bypasses the competing library's
/// `execve` override entirely.
///
/// Without the fix (preserving the initial full LD_PRELOAD in INITIAL_PRELOAD),
/// doctoring after an empty-envp exec would set `LD_PRELOAD=libexec.so` only,
/// and the competing library's check would reject the exec.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn hardened_competing_ld_preload_survives_env_clear() -> Result<()> {
    let env = TestEnvironment::new("hardened_competing_env_clear")?;
    env.create_config(CONFIG)?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Build a competing library that enforces its own presence in LD_PRELOAD.
    // If a child exec's envp does NOT contain this library in LD_PRELOAD,
    // the library rejects the exec with EPERM. This simulates sandbox
    // self-protection behavior.
    let lib_path = env.test_dir().join("libcompeting.so");
    let competing_lib_source = format!(
        r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <string.h>
#include <unistd.h>

#define MY_PATH "{lib_path}"

typedef int (*execve_func_t)(const char *, char *const[], char *const[]);

int execve(const char *path, char *const argv[], char *const envp[]) {{
    execve_func_t real_execve = (execve_func_t)dlsym(RTLD_NEXT, "execve");
    if (!real_execve) return -1;

    /* Check that our library is still in LD_PRELOAD.
       If not, reject the exec — simulating sandbox self-protection. */
    int found = 0;
    for (char *const *e = envp; *e; e++) {{
        if (strstr(*e, "LD_PRELOAD=") == *e) {{
            if (strstr(*e, MY_PATH)) {{
                found = 1;
            }}
            break;
        }}
    }}
    if (!found) {{
        errno = EPERM;
        return -1;
    }}

    return real_execve(path, argv, envp);
}}
"#,
        lib_path = lib_path.to_str().unwrap(),
    );

    env.create_source_files(&[("competing_lib.c", &competing_lib_source)])?;

    let compile_result = {
        let mut cmd = std::process::Command::new(COMPILER_C_PATH);
        cmd.current_dir(env.test_dir()).args([
            "-shared",
            "-fPIC",
            "-o",
            "libcompeting.so",
            "competing_lib.c",
            "-ldl",
        ]);
        cmd.output()?
    };
    if !compile_result.status.success() {
        anyhow::bail!(
            "Failed to compile competing library: {}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
    }

    // Build a C program that calls execve with an empty envp.
    // This forces the call through execve (not execvp/execvpe), so the
    // competing library's execve override is in the RTLD_NEXT chain.
    let launcher_source = format!(
        r#"#include <unistd.h>
#include <stdio.h>

int main() {{
    char *const argv[] = {{ "{compiler}", "-c", "test.c", "-o", "test.o", 0 }};
    char *const envp[] = {{ "PATH=/usr/bin:/bin", 0 }};
    execve("{compiler}", argv, envp);
    perror("execve failed");
    return 1;
}}"#,
        compiler = COMPILER_C_PATH,
    );

    env.create_source_files(&[("launcher.c", &launcher_source)])?;
    env.run_c_compiler("launcher", &["launcher.c"])?;

    // Run bear with the competing library added to LD_PRELOAD.
    // Bear will prepend libexec.so, giving: libexec.so:libcompeting.so
    // The launcher then calls execve with an empty envp. Bear's doctoring
    // must restore LD_PRELOAD with BOTH libraries for the competing
    // library's self-check to pass.
    let build_commands =
        format!("export LD_PRELOAD=\"$LD_PRELOAD:{}\" && ./launcher", lib_path.to_str().unwrap(),);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "test.c".to_string(),
            "-o".to_string(),
            "test.o".to_string(),
        ]
    ))?;

    Ok(())
}
