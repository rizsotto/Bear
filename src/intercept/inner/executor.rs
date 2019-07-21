/*  Copyright (C) 2012-2018 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use crate::intercept::{Error, Result, ResultExt, EventEnvelope};
use crate::intercept::{Event, ExitCode, ProcessId};
use super::protocol::sender::EventSink;
use super::env::Vars;

pub trait Executor {
    fn run(&self, program: &std::path::Path, args: &[String], envs: &Vars) -> Result<ExitCode>;
}

#[cfg(unix)]
pub fn executor(reporter: impl EventSink) -> impl Executor {
    unix::UnixExecutor::new(reporter)
}

#[cfg(not(unix))]
pub fn executor(reporter: impl EventSink) -> impl Executor {
    generic::GenericExecutor::new(reporter)
}

#[cfg(unix)]
mod unix {
    use std::str;
    use std::ffi;
    use std::os::unix::io;
    use nix::fcntl;
    use nix::unistd;
    use nix::sys::wait;

    use super::*;

    pub struct UnixExecutor<T: EventSink> {
        reporter: T,
    }

    impl<T> UnixExecutor<T> where T: EventSink {
        pub fn new(reporter: T) -> Self {
            UnixExecutor { reporter }
        }

        fn spawn(&self, program: &std::path::Path, args: &[String], envs: &Vars) -> Result<nix::unistd::Pid>
        {
            spawn(program, args, envs)
                .and_then(|pid| {
                    let id = pid.as_raw() as ProcessId;
                    let event = Event::created(program, args)?;
                    self.reporter.report(id, event);
                    Ok(pid)
                })
        }

        fn wait(&self, pid: nix::unistd::Pid) -> Result<ExitCode>
        {
            let id = pid.as_raw() as ProcessId;

            match wait::waitpid(pid, wait_flags()) {
                Ok(wait::WaitStatus::Exited(_pid, code)) => {
                    let event = Event::TerminatedNormally { code };
                    self.reporter.report(id, event);
                    Ok(code)
                },
                Ok(wait::WaitStatus::Signaled(_pid, signal, _dump)) => {
                    let event = Event::TerminatedAbnormally { signal: format!("{}", signal) };
                    self.reporter.report(id, event);
                    Ok(127)
                },
                Ok(wait::WaitStatus::Stopped(_pid, signal)) => {
                    let event = Event::Stopped { signal: format!("{}", signal) };
                    self.reporter.report(id, event);
                    Self::wait(self, pid)
                },
                Ok(wait::WaitStatus::Continued(_pid)) => {
                    let event = Event::Continued {};
                    self.reporter.report(id, event);
                    Self::wait(self, pid)
                },
                Ok(_) => {
                    info!("Wait status is ignored, continue to wait.");
                    Self::wait(self, pid)
                },
                Err(error) =>
                    Err(Error::with_chain(error, "Process creation failed.")),
            }
        }
    }

    impl<T> super::Executor for UnixExecutor<T> where T: EventSink {
        fn run(&self, program: &std::path::Path, args: &[String], envs: &Vars) -> Result<ExitCode> {
            let pid = self.spawn(program, args, envs)?;
            let exit_code = self.wait(pid)?;
            Ok(exit_code)
        }
    }

    fn spawn(program: &std::path::Path, args: &[String], envs: &Vars) -> Result<nix::unistd::Pid> {
        // Create communication channel between the child and parent processes.
        // Parent want to be notified if process execution went well or failed.
        let (read_fd, write_fd) = unistd::pipe()
            .chain_err(|| "Unable to create pipe.")?;

        match unistd::fork() {
            Ok(unistd::ForkResult::Parent { child, .. }) => {
                debug!("Parent process: waiting for pid: {}", child);
                let _ = unistd::close(write_fd);
                defer! {{ let _ = unistd::close(read_fd); }}

                let mut buffer = vec![0u8; 1024];
                match unistd::read(read_fd, buffer.as_mut()) {
                    Ok(0) => {
                        // In case of successful start the child closed the pipe,
                        // so we can't read anything from it.
                        debug!("Parent process: looks the child was done well.");
                        Ok(child)
                    },
                    Ok(_) => {
                        // If the child failed to exec the given process,
                        // it sends us a message through the pipe.
                        // Take that read value and use as error message.
                        debug!("Parent process: looks the child failed exec.");
                        Err(
                            str::from_utf8(buffer.as_ref())
                                .unwrap_or("Unknown reason.")
                                .into())
                    },
                    Err(error) =>
                        Err(Error::with_chain(error, "Read from pipe failed.")),
                }
            }
            Ok(unistd::ForkResult::Child) => {
                debug!("Child process: calling exec.");
                let _ = unistd::close(read_fd);
                set_close_on_exec(write_fd);

                match execute(program, args, envs) {
                    Ok(_) => Err("Never gonna happen".into()),
                    Err(error) => {
                        let message = error.to_string().into_bytes();
                        let _ = unistd::write(write_fd, message.as_ref());
                        debug!("Child process: exec failed, calling exit.");
                        std::process::exit(1);
                    },
                }
            },
            Err(error) =>
                Err(Error::with_chain(error, "Fork process failed.")),
        }
    }

    fn execute(program: &std::path::Path, args: &[String], envs: &Vars) -> Result<()> {
        fn str_to_cstring(str: &str) -> Result<ffi::CString> {
            ffi::CString::new(str)
                .map_err(|_e| "String contains null byte.".into())
        }
        fn path_to_str(path: &std::path::Path) -> Result<&str> {
            path.as_os_str()
                .to_str()
                .ok_or_else(|| "Path can't converted into string.".into())
        }

        let c_args = args.iter()
            .map(|arg| str_to_cstring(arg))
            .collect::<Result<Vec<ffi::CString>>>()?;
        let c_envs = envs.iter()
            .map(|(key, value)| {
                let env = key.to_string() + "=" + value;
                str_to_cstring(env.as_ref())
            })
            .collect::<Result<Vec<ffi::CString>>>()?;
        let c_program = path_to_str(program)
            .and_then(|str| str_to_cstring(str))?;

        let _ = unistd::execve(&c_program, c_args.as_ref(), c_envs.as_ref())?;

        Ok(())
    }

    fn wait_flags() -> Option<wait::WaitPidFlag> {
        let mut wait_flags = wait::WaitPidFlag::empty();
        wait_flags.insert(wait::WaitPidFlag::WCONTINUED);
        #[cfg(not(target_os = "macos"))]
            wait_flags.insert(wait::WaitPidFlag::WSTOPPED);
        wait_flags.insert(wait::WaitPidFlag::WUNTRACED);
        Some(wait_flags)
    }

    fn set_close_on_exec(fd: io::RawFd) {
        fcntl::fcntl(fd, fcntl::F_GETFD)
            .and_then(|current_bits| {
                let flags: fcntl::FdFlag = fcntl::FdFlag::from_bits(current_bits)
                    .map(|mut flag| {
                        flag.insert(fcntl::FdFlag::FD_CLOEXEC);
                        flag
                    })
                    .unwrap_or(fcntl::FdFlag::FD_CLOEXEC);
                fcntl::fcntl(fd, fcntl::F_SETFD(flags))
            })
            .expect("set close-on-exec failed.");
    }

    #[cfg(test)]
    mod test {
        use super::*;

        use mockers::Scenario;
        use mockers::matchers::{ANY, eq};

        use nix::sys::signal;
        use nix::unistd::Pid;

        use crate::intercept::inner::env;
        use crate::intercept::report::Executable;

        macro_rules! vector_of_strings {
            ($($x:expr),*) => (vec![$($x.to_string()),*]);
        }

        fn resolve_executable(cmd: &[String]) -> std::path::PathBuf {
            Executable::WithPath(cmd[0].clone()).resolve().unwrap()
        }

        #[test]
        fn success() {
            let cmd = vector_of_strings!("true", "with", "arguments");
            let program = resolve_executable(&cmd);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, eq(Event::created(&program, &cmd).unwrap()))
                    .and_return(())
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::TerminatedNormally { code: 0 }))
                    .and_return(())
            );

            let sut = super::UnixExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(true, result.is_ok());
            assert_eq!(0i32, result.unwrap());
        }

        #[test]
        fn fail() {
            let cmd = vector_of_strings!("false");
            let program = resolve_executable(&cmd);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, eq(Event::created(&program, &cmd).unwrap()))
                    .and_return(())
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::TerminatedNormally { code: 1 }))
                    .and_return(())
            );

            let sut = super::UnixExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(true, result.is_ok());
            assert_eq!(1i32, result.unwrap());
        }

        #[test]
        fn exec_failure() {
            let cmd = vector_of_strings!("sure-this-is-not-there");
            let program = std::path::PathBuf::from(&cmd[0]);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, ANY).never()
            );

            let sut = super::UnixExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(false, result.is_ok());
        }

        #[test]
        fn kill_signal() {
            let cmd = vector_of_strings!("sleep", "5");
            let program = resolve_executable(&cmd);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, ANY).and_call(|pid, _event| {
                    signal::kill(Pid::from_raw(pid as i32), signal::SIGKILL).unwrap();
                })
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::TerminatedAbnormally { signal: "SIGKILL".to_string() }))
                    .and_return(())
            );

            let sut = super::UnixExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(true, result.is_ok());
            assert_eq!(127i32, result.unwrap());
        }

        #[test]
        fn stop_signal() {
            let cmd = vector_of_strings!("sleep", "5");
            let program = resolve_executable(&cmd);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, ANY).and_call(|pid, _event| {
                    signal::kill(Pid::from_raw(pid as i32), signal::SIGSTOP).unwrap();
                })
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::Stopped { signal: "SIGSTOP".to_string() }))
                    .and_call(|pid, _event| {
                        signal::kill(Pid::from_raw(pid as i32), signal::SIGCONT).unwrap();
                    })
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::Continued {}))
                    .and_call(|pid, _event| {
                        signal::kill(Pid::from_raw(pid as i32), signal::SIGKILL).unwrap();
                    })
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::TerminatedAbnormally { signal: "SIGKILL".to_string() }))
                    .and_return(())
            );

            let sut = super::UnixExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(true, result.is_ok());
            assert_eq!(127i32, result.unwrap());
        }
    }
}

mod generic {
    use super::*;

    pub struct GenericExecutor<T: EventSink> {
        reporter: T,
    }

    impl<T> GenericExecutor<T> where T: EventSink {
        pub fn new(reporter: T) -> Self {
            GenericExecutor { reporter }
        }

        fn spawn(&self, program: &std::path::Path, args: &[String], envs: &Vars) -> Result<std::process::Child> {
            let child = std::process::Command::new(program)
                .args(&args[1..])
                .envs(envs)
                .spawn()
                .chain_err(|| format!("unable to execute process: {:?}", program))?;

            let event = Event::created(program, args)?;
            self.reporter.report(child.id() as ProcessId, event);

            Ok(child)
        }

        fn wait(&self, handle: &mut std::process::Child) -> Result<ExitCode> {
            match handle.wait() {
                Ok(status) => {
                    match status.code() {
                        Some(code) => {
                            let event = Event::TerminatedNormally { code };
                            self.reporter.report(handle.id(), event);
                            Ok(code)
                        }
                        None => {
                            let event = Event::TerminatedAbnormally { signal: "unknown".to_string() };
                            self.reporter.report(handle.id(), event);
                            Ok(127)
                        }
                    }
                }
                Err(error) => {
                    warn!("Child process was not running: {:?}", handle.id());
                    Err(Error::with_chain(error, "Waiting for process failed."))
                }
            }
        }
    }

    impl<T> super::Executor for GenericExecutor<T> where T: EventSink {
        fn run(&self, program: &std::path::Path, args: &[String], envs: &Vars) -> Result<ExitCode> {
            let mut handle = self.spawn(program, args, envs)?;
            let exit_code = self.wait(&mut handle)?;
            Ok(exit_code)
        }
    }


    #[cfg(test)]
    mod test {
        use super::*;

        use mockers::Scenario;
        use mockers::matchers::{ANY, eq};

        use crate::intercept::inner::env;
        use crate::intercept::report::Executable;

        macro_rules! vector_of_strings {
            ($($x:expr),*) => (vec![$($x.to_string()),*]);
        }

        fn resolve_executable(cmd: &[String]) -> std::path::PathBuf {
            Executable::WithPath(cmd[0].clone()).resolve().unwrap()
        }

        #[test]
        #[cfg(unix)]
        fn success() {
            let cmd = vector_of_strings!("true", "with", "arguments");
            let program = resolve_executable(&cmd);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, eq(Event::created(&program, &cmd).unwrap()))
                    .and_return(())
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::TerminatedNormally { code: 0 }))
                    .and_return(())
            );

            let sut = super::GenericExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(true, result.is_ok());
            assert_eq!(0i32, result.unwrap());
        }

        #[test]
        #[cfg(unix)]
        fn fail() {
            let cmd = vector_of_strings!("false");
            let program = resolve_executable(&cmd);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, eq(Event::created(&program, &cmd).unwrap()))
                    .and_return(())
            );
            scenario.expect(
                sink_handle.report(ANY, eq(Event::TerminatedNormally { code: 1 }))
                    .and_return(())
            );

            let sut = super::GenericExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(true, result.is_ok());
            assert_eq!(1i32, result.unwrap());
        }

        #[test]
        fn exec_failure() {
            let cmd = vector_of_strings!("sure-this-is-not-there");
            let program = std::path::PathBuf::from(&cmd[0]);

            let scenario = Scenario::new();
            let (sink, sink_handle) = scenario.create_mock_for::<dyn EventSink>();

            scenario.expect(
                sink_handle.report(ANY, ANY).never()
            );

            let sut = super::GenericExecutor::new(sink);
            let result = sut.run(program.as_path(), &cmd, &env::Builder::new().build());

            assert_eq!(false, result.is_ok());
        }
    }
}

mod fake {
    use super::*;
    use crate::semantic::c_compiler::CompilerCall;

    /// The main responsibility is to fake the program execution by making the
    /// relevant side effects.
    ///
    /// For a compiler, linker call the expected side effect by the build system
    /// is to create the output files. That will make sure that the build tool
    /// will continue the build process.
    fn fake_execution(cmd: &[String], cwd: &std::path::Path) -> Result<()> {
        let compilation = CompilerCall::from(cmd, cwd)?;
        match compilation.output() {
            // When the file is not yet exists, create one.
            Some(ref output) if !output.exists() =>
                std::fs::OpenOptions::new()
                    .create(true)
                    .open(output)
                    .map(|_| ())
                    .map_err(std::convert::Into::into),
            _ =>
                Ok(()),
        }
    }
}
