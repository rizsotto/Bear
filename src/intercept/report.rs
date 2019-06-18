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

use super::*;

pub fn c_compiler(_args: &[String]) -> Result<ExitCode> {

//    let target = environment::target_directory()?;
//    let mut protocol = Protocol::new(target.as_path())?;
//
//    let mut supervisor = Supervisor::new(|event| protocol.send(event));
//
//    match environment::c_compiler_path() {
//        Ok(wrapper) => {
//            args[0] = wrapper;
//            supervisor.run(&args[..])
//        },
//        Err(_) => {
//            supervisor.fake(&args[..])
//        },
//    }

    unimplemented!()
}

pub fn cxx_compiler(_args: &[String]) -> Result<ExitCode> {

//    let target = environment::target_directory()?;
//    let mut protocol = Protocol::new(target.as_path())?;
//
//    let mut supervisor = Supervisor::new(|event| protocol.send(event));
//
//    match environment::cxx_compiler_path() {
//        Ok(wrapper) => {
//            args[0] = wrapper;
//            supervisor.run(&args[..])
//        },
//        Err(_) => {
//            supervisor.fake(&args[..])
//        },
//    }

    unimplemented!()
}

pub fn wrapper(_execution: &ExecutionRequest, _session: &Session) -> Result<ExitCode> {
    unimplemented!()
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub executable: Executable,
    pub arguments: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Executable {
    WithFilename(std::path::PathBuf),
    WithPath(String),
    WithSearchPath(String, Vec<std::path::PathBuf>),
}

impl Executable {
    pub fn resolve(&self) -> Result<std::path::PathBuf> {
        inner::resolve_executable(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub destination: std::path::PathBuf,
    pub verbose: bool,
    pub modes: InterceptModes,
}

mod inner {
    use super::*;

    pub fn resolve_executable(executable: &Executable) -> Result<std::path::PathBuf> {
        match executable {
            Executable::WithFilename(ref path) if path.is_absolute() => {
                Ok(path.clone())
            },
            Executable::WithFilename(ref path) => {
                let cwd = std::env::current_dir()?;
                find_executable_in(path, vec!(cwd).as_ref())
            },
            Executable::WithPath(ref string) => {
                let path = std::env::var("PATH")?;
                let paths = std::env::split_paths(&path)
                    .collect::<Vec<_>>();
                find_executable_in(string, &paths)
            }
            Executable::WithSearchPath(ref string, ref paths) => {
                find_executable_in(string, &paths)
            },
        }
    }

    fn find_executable_in<P: AsRef<std::path::Path>>(path: P, paths: &[std::path::PathBuf]) -> Result<std::path::PathBuf> {
        paths.iter()
            .filter_map(|prefix| prefix.join(&path).canonicalize().ok())
            .filter(|candidate| is_executable(candidate))
            .next()
            .ok_or("File is not found nor executable.".into())
    }

    fn is_executable(path: &std::path::Path) -> bool {
        path.exists() && path.is_file()
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use tempfile;

        #[test]
        fn when_absolute_path() -> Result<()> {
            let dir = tempfile::tempdir()?;
            let executable = create_executable(dir.path())
                .map(|candidate| to_absolute_path(dir.path(),candidate.as_path()))?;

            let sut = Executable::WithFilename(executable.clone());
            let result = sut.resolve()?;

            assert_eq!(executable, result);

            Ok(())
        }

        fn to_absolute_path(prefix: &std::path::Path, file: &std::path::Path) -> std::path::PathBuf {
            prefix.to_path_buf().join(file)
        }

        fn create_executable(path: &std::path::Path) -> Result<std::path::PathBuf> {
            use std::io::Write;

            let file_name = std::path::PathBuf::from("this_very_unique.exe");

            let abs_file_name = to_absolute_path(path, &file_name.as_path());
            let mut f = std::fs::File::create(abs_file_name)?;
            f.write_all(b"content")?;

            Ok(file_name)
        }
    }
}
