// SPDX-License-Identifier: GPL-3.0-or-later

//! Lazy `--version` probe used to disambiguate compiler executables whose
//! basename does not uniquely identify the underlying toolchain (notably
//! `cc` and `c++`, which are GCC on most Linuxes and Clang on the BSDs and
//! macOS).
//!
//! The trait is cross-platform; the real implementation [`VersionProbe`]
//! is Unix-only because (a) the BSD/macOS `cc`/`c++` ambiguity is the
//! reason the probe exists in the first place, and (b) the watchdog needs
//! `setsid` + `killpg` to take down a misbehaving subprocess tree, which
//! has no portable equivalent on Windows. On non-Unix targets the
//! recognizer wires up [`NoProbe`] instead and relies on the regex layer,
//! which already classifies Windows toolchain names (`cl.exe`,
//! `clang-cl`, `gcc.exe`) unambiguously.
//!
//! Safety properties of the Unix probe:
//! - stdin is closed (`Stdio::null`) so probed binaries that read input
//!   (e.g. `bash` if it lands in `PATH`) cannot deadlock the call.
//! - stdout/stderr are captured with no size cap from the OS but with a
//!   bounded timeout, so a chatty binary cannot stall recognition.
//! - `LD_PRELOAD` and `DYLD_INSERT_LIBRARIES` are stripped from the probe
//!   environment so the probe is not itself intercepted by Bear.
//! - A watchdog thread `SIGKILL`s the child's process group if
//!   `--version` does not return within the configured timeout.
//! - A spawn that fails with `ETXTBSY` (the probed file transiently held
//!   open for writing across another fork+exec) is retried briefly
//!   before the probe declines.

use crate::config::CompilerType;
use std::path::Path;

#[cfg(unix)]
use std::cell::RefCell;
#[cfg(unix)]
use std::collections::HashMap;
#[cfg(unix)]
use std::path::PathBuf;

/// `Send` (and not `Sync`) for the same reason as [`crate::semantic::Interpreter`]:
/// the probe lives inside the interpreter that is moved into the single
/// consumer thread, and is never shared across threads.
pub trait CompilerProbe: Send {
    /// Classify `executable_path` by running `--version` on it.
    /// Returns `None` if the binary does not produce a recognizable signature
    /// (timeout, spawn failure, non-zero exit, garbage output, etc.).
    fn probe(&self, executable_path: &Path) -> Option<CompilerType>;
}

/// Decorator that memoizes a probe's verdict per executable path. Used to
/// wrap [`VersionProbe`] so a single recognizer never fork-execs the same
/// compiler twice. Keyed by the path it receives -- the recognizer
/// canonicalizes before calling, so the same compiler under different
/// argv spellings collapses to one cache entry.
///
/// Cache hits return both `Some(ty)` (a successful classification) and
/// `None` (an inconclusive probe), so a binary that doesn't understand
/// `--version` is probed once, not on every recognize() call.
///
/// The cache is a `RefCell` because `probe()` takes `&self` (it is called
/// through shared references all the way down from `Box<dyn Interpreter>`)
/// but a miss must write. Recognition runs on the single consumer thread
/// (see [`CompilerProbe`]'s `Send`-only bound), so no locking is needed;
/// the borrow is released before the inner-probe call, which forks and
/// can block for the probe timeout.
///
/// Gated to Unix because the only probe worth caching is `VersionProbe`,
/// which is itself Unix-only. On Windows `default_probe` returns
/// `NoProbe` directly -- caching a constant `None` would be pure
/// overhead.
#[cfg(unix)]
pub(crate) struct CachingProbe<P: CompilerProbe> {
    inner: P,
    cache: RefCell<HashMap<PathBuf, Option<CompilerType>>>,
}

#[cfg(unix)]
impl<P: CompilerProbe> CachingProbe<P> {
    pub(crate) fn new(inner: P) -> Self {
        Self { inner, cache: RefCell::new(HashMap::new()) }
    }
}

#[cfg(unix)]
impl<P: CompilerProbe> CompilerProbe for CachingProbe<P> {
    fn probe(&self, executable_path: &Path) -> Option<CompilerType> {
        if let Some(&hit) = self.cache.borrow().get(executable_path) {
            return hit;
        }
        let result = self.inner.probe(executable_path);
        self.cache.borrow_mut().insert(executable_path.to_path_buf(), result);
        result
    }
}

/// Probe that always declines to classify. Used in two places:
/// - on non-Unix targets, where `VersionProbe` does not exist and the
///   recognizer falls back to its regex layer for every name;
/// - in unit tests of `CompilerRecognizer` that want to exercise the
///   regex/hint layer deterministically without depending on whatever
///   `cc`/`c++` resolve to on the host.
///
/// On a Unix lib build neither use applies, so silence the dead-code
/// lint that would otherwise fire there.
#[cfg_attr(all(unix, not(test)), allow(dead_code))]
pub(crate) struct NoProbe;

impl CompilerProbe for NoProbe {
    fn probe(&self, _: &Path) -> Option<CompilerType> {
        None
    }
}

#[cfg(unix)]
pub(crate) use unix::VersionProbe;

/// The default probe for the current target. On Unix wraps a real
/// `VersionProbe` in a [`CachingProbe`] so each unique compiler path is
/// only probed once per process. On Windows where the BSD/macOS `cc`/`c++`
/// ambiguity does not exist (compiler basenames are uniquely classified
/// by the regex layer) returns a `NoProbe`; caching a no-op would just be
/// overhead.
pub(crate) fn default_probe() -> Box<dyn CompilerProbe> {
    #[cfg(unix)]
    {
        Box::new(CachingProbe::new(VersionProbe::new()))
    }
    #[cfg(not(unix))]
    {
        Box::new(NoProbe)
    }
}

#[cfg(all(unix, test))]
mod caching_tests {
    //! Requirements: recognition-ambiguous-name-probe

    use super::*;
    use std::cell::Cell;

    /// Probe that returns canned answers and counts how many times it ran.
    /// Used to verify that [`CachingProbe`] collapses repeated lookups for
    /// the same path into a single inner call.
    struct CountingProbe {
        answers: HashMap<PathBuf, CompilerType>,
        calls: Cell<usize>,
    }

    impl CountingProbe {
        fn empty() -> Self {
            Self { answers: HashMap::new(), calls: Cell::new(0) }
        }

        fn with_answer(p: &str, t: CompilerType) -> Self {
            let mut answers = HashMap::new();
            answers.insert(PathBuf::from(p), t);
            Self { answers, calls: Cell::new(0) }
        }

        fn calls(&self) -> usize {
            self.calls.get()
        }
    }

    impl CompilerProbe for CountingProbe {
        fn probe(&self, p: &Path) -> Option<CompilerType> {
            self.calls.set(self.calls.get() + 1);
            self.answers.get(p).copied()
        }
    }

    #[test]
    fn caches_successful_classification_per_path() {
        let inner = CountingProbe::with_answer("cc", CompilerType::Clang);
        let cached = CachingProbe::new(inner);

        for _ in 0..100 {
            assert_eq!(cached.probe(Path::new("cc")), Some(CompilerType::Clang));
        }

        // The inner CompilerProbe is moved into CachingProbe, so we have
        // to reach through the wrapper to read the call count.
        assert_eq!(cached.inner.calls(), 1, "inner probe must run at most once per path");
    }

    #[test]
    fn caches_inconclusive_results_too() {
        // A binary that doesn't understand --version returns None; without
        // None-caching we'd burn a fork on every recognize() call.
        let inner = CountingProbe::empty();
        let cached = CachingProbe::new(inner);

        for _ in 0..50 {
            assert_eq!(cached.probe(Path::new("cc")), None);
        }

        assert_eq!(cached.inner.calls(), 1, "None results must be cached too");
    }

    #[test]
    fn distinct_paths_are_cached_independently() {
        let mut answers = HashMap::new();
        answers.insert(PathBuf::from("/usr/bin/cc"), CompilerType::Gcc);
        answers.insert(PathBuf::from("/usr/local/bin/cc"), CompilerType::Clang);
        let inner = CountingProbe { answers, calls: Cell::new(0) };
        let cached = CachingProbe::new(inner);

        assert_eq!(cached.probe(Path::new("/usr/bin/cc")), Some(CompilerType::Gcc));
        assert_eq!(cached.probe(Path::new("/usr/local/bin/cc")), Some(CompilerType::Clang));
        // Repeat to confirm both entries are stable in the cache.
        assert_eq!(cached.probe(Path::new("/usr/bin/cc")), Some(CompilerType::Gcc));
        assert_eq!(cached.probe(Path::new("/usr/local/bin/cc")), Some(CompilerType::Clang));

        assert_eq!(cached.inner.calls(), 2, "each distinct path triggers exactly one inner call");
    }
}

#[cfg(unix)]
mod unix {
    use super::CompilerProbe;
    use crate::config::CompilerType;
    use intercept::environment::{KEY_OS_MACOS_PRELOAD_PATH, KEY_OS_PRELOAD_PATH};
    use std::os::unix::process::CommandExt;
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    /// Runs `<exe> --version`, classifies the output. Default timeout: 2 seconds.
    pub(crate) struct VersionProbe {
        timeout: Duration,
    }

    impl VersionProbe {
        pub(crate) fn new() -> Self {
            Self { timeout: Duration::from_secs(2) }
        }

        #[cfg(test)]
        pub(crate) fn with_timeout(timeout: Duration) -> Self {
            Self { timeout }
        }
    }

    impl Default for VersionProbe {
        fn default() -> Self {
            Self::new()
        }
    }

    impl CompilerProbe for VersionProbe {
        fn probe(&self, executable_path: &Path) -> Option<CompilerType> {
            let mut cmd = Command::new(executable_path);
            cmd.arg("--version")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env_remove(KEY_OS_PRELOAD_PATH)
                .env_remove(KEY_OS_MACOS_PRELOAD_PATH);

            // Put the child in its own process group so the watchdog can kill
            // not just the immediate child but the entire process tree it
            // spawns. Without this, SIGKILL on a `#!/bin/sh ; sleep 30` script
            // would kill sh but leave sleep holding the stdout/stderr pipes
            // open, blocking wait_with_output() for the script's full runtime.
            unsafe {
                cmd.pre_exec(|| {
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }

            // exec fails with ETXTBSY while any process holds a write fd to
            // the probed file. The pre_exec closure above forces the
            // fork+exec spawn path, and a child forked elsewhere (by another
            // thread here, or by a build tool that just wrote a compiler
            // wrapper script) holds every inherited fd until its own exec.
            // That window is transient, so retry briefly; a persistently
            // busy file still declines as inconclusive.
            let mut attempts = 0;
            let child = loop {
                match cmd.spawn() {
                    Ok(c) => break c,
                    Err(err) if err.raw_os_error() == Some(libc::ETXTBSY) && attempts < 10 => {
                        attempts += 1;
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(err) => {
                        log::debug!("probe: spawn failed for {}: {err}", executable_path.display());
                        return None;
                    }
                }
            };
            let pid = child.id();

            // Watchdog: SIGKILL the child's process group if --version does
            // not return in time. The cancel channel lets the main thread
            // tell the watchdog to exit cleanly when the child has already
            // finished. recv_timeout returns Err(Timeout) when nothing is
            // sent within the deadline; that's our signal to kill.
            let (cancel_tx, cancel_rx) = mpsc::channel::<()>();
            let timeout = self.timeout;
            let watchdog = thread::spawn(move || {
                if cancel_rx.recv_timeout(timeout).is_err() {
                    // SAFETY: killpg() is async-signal-safe; ESRCH on an
                    // already-reaped group is benign. We signal the whole
                    // process group (set up by setsid in pre_exec) to take
                    // down any grandchildren that would otherwise keep
                    // stdout/stderr pipes open.
                    unsafe {
                        libc::killpg(pid as libc::pid_t, libc::SIGKILL);
                    }
                }
            });

            let output = child.wait_with_output();
            let _ = cancel_tx.send(());
            let _ = watchdog.join();

            let output = match output {
                Ok(o) => o,
                Err(err) => {
                    log::debug!("probe: wait failed for {}: {err}", executable_path.display());
                    return None;
                }
            };

            // A killed-by-watchdog process exits with a signal, not status code 0.
            // Treat any non-success as inconclusive.
            if !output.status.success() {
                log::debug!("probe: {} exited with {:?}", executable_path.display(), output.status);
                return None;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            classify_version_output(&stdout, &stderr)
        }
    }

    /// Classify the combined `--version` output of a candidate compiler.
    ///
    /// The rule set is intentionally narrow: a misclassification corrupts
    /// the compilation database (wrong flag-arity table), while a non-
    /// classification just falls back to the regex layer. We accept Clang
    /// and GCC and reject everything else.
    fn classify_version_output(stdout: &str, stderr: &str) -> Option<CompilerType> {
        // Compilers print to stdout in practice, but the doc's PR #695 used
        // stderr; Apple clang has historically used stderr too. Check both.
        for haystack in [stdout, stderr] {
            // Apple clang prints "Apple clang version ...", upstream LLVM prints
            // "clang version ..."; both contain the substring "clang version".
            if haystack.contains("clang version") {
                return Some(CompilerType::Clang);
            }
            // GNU gcc prints a "Copyright (C) ... Free Software Foundation,
            // Inc." line in every build. The leading "(GCC)" vendor tag is
            // NOT reliable: distro and BSD-ports builds rewrite it to their
            // own tag ("(Alpine 15.2.0)", "(Ubuntu 13.3.0-...)", "(FreeBSD
            // Ports Collection)"), and when gcc is invoked as `cc` -- the
            // case this probe exists for -- the string "gcc" does not appear
            // at all. The FSF copyright is the only portable GCC signal.
            // Clang is excluded above (it prints LLVM license text, never the
            // FSF copyright), so this does not misclassify clang as gcc. See
            // issue #711.
            if haystack.contains("Free Software Foundation") {
                return Some(CompilerType::Gcc);
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        //! Requirements: recognition-ambiguous-name-probe
        //!
        //! Every test in this module protects the ambiguous-name probe
        //! requirement. The classify_* and reject_* tests cover the
        //! classification rule (acceptance criterion: "classify it as GCC or
        //! Clang"); the fork_exec submodule covers the runtime safety
        //! properties (timeout, stdin closed, exit codes, missing executables).

        use super::*;

        #[test]
        fn classifies_upstream_clang() {
            let stdout = "clang version 17.0.6\nTarget: x86_64-pc-linux-gnu\n";
            assert_eq!(classify_version_output(stdout, ""), Some(CompilerType::Clang));
        }

        #[test]
        fn classifies_apple_clang() {
            let stdout = "Apple clang version 15.0.0 (clang-1500.3.9.4)\n";
            assert_eq!(classify_version_output(stdout, ""), Some(CompilerType::Clang));
        }

        #[test]
        fn classifies_gcc_across_vendor_builds() {
            // The leading "(...)" vendor tag varies per distro and BSD ports;
            // when gcc is invoked as `cc` the string "gcc" is absent entirely.
            // The FSF copyright line is the only marker common to every build,
            // so every one of these must classify as GCC. See issue #711.
            let cases = [
                // Upstream FSF / Red Hat (the originally-recognized form).
                "gcc (GCC) 13.2.1 20231011 (Red Hat 13.2.1-4)\n\
                 Copyright (C) 2023 Free Software Foundation, Inc.\n",
                // Alpine Linux (the reported case).
                "gcc (Alpine 15.2.0) 15.2.0\n\
                 Copyright (C) 2025 Free Software Foundation, Inc.\n",
                // Ubuntu.
                "gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0\n\
                 Copyright (C) 2023 Free Software Foundation, Inc.\n",
                // FreeBSD ports.
                "gcc14 (FreeBSD Ports Collection) 14.2.0\n\
                 Copyright (C) 2024 Free Software Foundation, Inc.\n",
                // gcc invoked as `cc`: no "gcc" token anywhere in the output.
                "cc (Debian 12.2.0-14) 12.2.0\n\
                 Copyright (C) 2022 Free Software Foundation, Inc.\n",
            ];

            for stdout in cases {
                assert_eq!(
                    classify_version_output(stdout, ""),
                    Some(CompilerType::Gcc),
                    "expected GCC for output: {stdout:?}"
                );
            }
        }

        #[test]
        fn classifies_via_stderr() {
            // Some toolchains print --version to stderr. Apple clang has done this
            // historically.
            let stderr = "Apple clang version 12.0.0\n";
            assert_eq!(classify_version_output("", stderr), Some(CompilerType::Clang));
        }

        #[test]
        fn rejects_garbage() {
            assert_eq!(classify_version_output("I am not a compiler\n", ""), None);
        }

        #[test]
        fn rejects_partial_gcc_marker() {
            // "gcc" appears but not the FSF copyright -> conservative reject.
            assert_eq!(classify_version_output("my-gcc-wrapper version 1.0\n", ""), None);
        }

        // ----- Real fork-exec behavior tests ---------------------------

        mod fork_exec {
            use super::*;
            use std::io::Write;
            use std::os::unix::fs::PermissionsExt;
            use std::time::Instant;

            /// Write `body` to a uniquely-named executable script in `dir`.
            fn write_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
                let path = dir.join(name);
                let mut f = std::fs::File::create(&path).unwrap();
                f.write_all(body.as_bytes()).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
                path
            }

            #[test]
            fn classifies_a_script_that_emits_clang_signature() {
                let dir = tempfile::tempdir().unwrap();
                let script = write_script(
                    dir.path(),
                    "fake-clang",
                    "#!/bin/sh\necho 'clang version 17.0.0 (Fedora 17.0.0-1)'\n",
                );

                assert_eq!(VersionProbe::new().probe(&script), Some(CompilerType::Clang));
            }

            #[test]
            fn classifies_a_script_that_emits_gcc_signature() {
                // Alpine-style signature: a vendor tag instead of "(GCC)".
                // See issue #711.
                let dir = tempfile::tempdir().unwrap();
                let script = write_script(
                    dir.path(),
                    "fake-gcc",
                    "#!/bin/sh\n\
                     echo 'gcc (Alpine 15.2.0) 15.2.0'\n\
                     echo 'Copyright (C) 2025 Free Software Foundation, Inc.'\n",
                );

                assert_eq!(VersionProbe::new().probe(&script), Some(CompilerType::Gcc));
            }

            #[test]
            fn returns_none_for_garbage_output() {
                let dir = tempfile::tempdir().unwrap();
                let script = write_script(dir.path(), "garbage", "#!/bin/sh\necho 'i am not a compiler'\n");

                assert_eq!(VersionProbe::new().probe(&script), None);
            }

            #[test]
            fn returns_none_when_executable_exits_nonzero() {
                let dir = tempfile::tempdir().unwrap();
                let script =
                    write_script(dir.path(), "failing", "#!/bin/sh\necho 'clang version 1.0'\nexit 7\n");

                // Even though the output looks like clang, a non-zero exit is
                // treated as inconclusive: a real compiler returns 0 on --version.
                assert_eq!(VersionProbe::new().probe(&script), None);
            }

            #[test]
            fn returns_none_when_executable_does_not_exist() {
                let probe = VersionProbe::new();
                let result = probe.probe(std::path::Path::new("/nonexistent/path/to/cc-xxxx"));
                assert_eq!(result, None);
            }

            #[test]
            fn returns_within_budget_for_a_hung_process() {
                // The watchdog must SIGKILL a child that never completes
                // --version. We use a 200ms timeout and assert the call returns
                // in well under a second.
                let dir = tempfile::tempdir().unwrap();
                let script = write_script(dir.path(), "hangs", "#!/bin/sh\nsleep 30\n");

                let probe = VersionProbe::with_timeout(std::time::Duration::from_millis(200));
                let started = Instant::now();
                let result = probe.probe(&script);
                let elapsed = started.elapsed();

                assert_eq!(result, None);
                assert!(
                    elapsed < std::time::Duration::from_secs(2),
                    "probe took {elapsed:?}, expected <2s; watchdog did not fire"
                );
            }

            #[test]
            fn retries_exec_while_script_is_transiently_open_for_write() {
                // exec of a script fails with ETXTBSY while any process holds
                // a write fd to it. That happens transiently under parallel
                // fork+exec: a child forked by another thread inherits the
                // write fd of a script still being created and holds it until
                // its own exec. Hold the write handle open past the probe's
                // first spawn attempt and assert the retry absorbs it.
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("busy-clang");
                let mut writer = std::fs::File::create(&path).unwrap();
                writer.write_all(b"#!/bin/sh\necho 'clang version 1.0'\n").unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
                let release = std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(30));
                    drop(writer);
                });

                let sut = VersionProbe::new().probe(&path);
                release.join().unwrap();

                assert_eq!(sut, Some(CompilerType::Clang));
            }

            #[test]
            fn does_not_block_on_a_binary_that_reads_stdin() {
                // If stdin were inherited (the default), this script would block
                // forever on `read`. The probe must close stdin so `read` sees
                // EOF immediately and the script exits.
                let dir = tempfile::tempdir().unwrap();
                let script = write_script(
                    dir.path(),
                    "reads-stdin",
                    "#!/bin/sh\nread line\necho 'clang version 1.0'\n",
                );

                let probe = VersionProbe::with_timeout(std::time::Duration::from_secs(2));
                let started = Instant::now();
                let result = probe.probe(&script);
                let elapsed = started.elapsed();

                // The script exits cleanly after reading EOF, prints the clang
                // signature, exits 0. Probe should classify it as Clang well
                // before the timeout.
                assert_eq!(result, Some(CompilerType::Clang));
                assert!(
                    elapsed < std::time::Duration::from_secs(1),
                    "probe took {elapsed:?}; stdin was not closed"
                );
            }
        }
    }
}
