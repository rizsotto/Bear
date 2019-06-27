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
        inner::resolve_executable(&inner::NativeContext, self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub destination: std::path::PathBuf,
    pub verbose: bool,
    pub modes: InterceptModes,
}

#[cfg(test)]
#[cfg(unix)]
mod test {
    use super::*;

    #[test]
    fn resolve_success_on_existing_command() {
        let sut = Executable::WithPath("true".to_string());
        let result = sut.resolve();

        assert_eq!(true, result.is_ok())
    }

    #[test]
    fn resolve_fail_on_not_existing_command() {
        let sut = Executable::WithPath("sure-this-is-not-there".to_string());
        let result = sut.resolve();

        assert_eq!(true, result.is_err())
    }

    #[test]
    fn resolve_success_with_full_path() -> Result<()> {
        let sut1 = Executable::WithPath("true".to_string());
        let result1 = sut1.resolve()?;

        let sut2 = Executable::WithFilename(result1.clone());
        let result2 = sut2.resolve()?;

        assert_eq!(result1, result2);

        Ok(())
    }

    #[test]
    fn resolve_success_from_path() -> Result<()> {
        let sut1 = Executable::WithPath("true".to_string());
        let result1 = sut1.resolve()?;
        let path1 = result1.parent().map(std::path::Path::to_path_buf).unwrap();

        let sut2 = Executable::WithSearchPath("true".to_string(), vec!(path1));
        let result2 = sut2.resolve()?;

        assert_eq!(result1, result2);

        Ok(())
    }
}

mod inner {
    use super::*;

    #[cfg(test)]
    use mockiato::mockable;

    #[cfg_attr(test, mockable)]
    pub(super) trait Context {
        fn get_cwd(&self) -> Result<std::path::PathBuf>;
        fn get_paths(&self) -> Result<Vec<std::path::PathBuf>>;
        fn is_executable(&self, file: &std::path::Path) -> bool;
    }

    pub(super) struct NativeContext;

    impl Context for NativeContext {
        fn get_cwd(&self) -> Result<std::path::PathBuf> {
            let result = std::env::current_dir()?;
            Ok(result)
        }

        fn get_paths(&self) -> Result<Vec<std::path::PathBuf>> {
            let path = std::env::var("PATH")?;
            let paths = std::env::split_paths(&path)
                .collect::<Vec<_>>();
            Ok(paths)
        }

        fn is_executable(&self, file: &std::path::Path) -> bool {
            file.exists() && file.is_file()
        }
    }

    pub(super) fn resolve_executable(context: &Context, executable: &Executable) -> Result<std::path::PathBuf> {
        match executable {
            Executable::WithFilename(ref path) if path.is_absolute() => {
                Ok(path.clone())
            },
            Executable::WithFilename(ref path) => {
                let cwd = context.get_cwd()?;
                find_executable_in(context, path, vec!(cwd).as_ref())
            },
            Executable::WithPath(ref string) => {
                let paths = context.get_paths()?;
                find_executable_in(context, string, &paths)
            }
            Executable::WithSearchPath(ref string, ref paths) => {
                find_executable_in(context, string, &paths)
            },
        }
    }

    fn find_executable_in<P: AsRef<std::path::Path>>(context: &Context, path: P, paths: &[std::path::PathBuf]) -> Result<std::path::PathBuf> {
        paths.iter()
            .filter_map(|prefix| prefix.join(&path).canonicalize().ok())
            .filter(|candidate| context.is_executable(candidate))
            .next()
            .ok_or("File is not found nor executable.".into())
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        #[cfg(unix)]
        fn when_absolute_path() -> Result<()> {
            let context = ContextMock::new();

            let sut = Executable::WithFilename(std::path::PathBuf::from("/path/to/executable"));
            let result = resolve_executable(&context, &sut);

            assert_eq!(true, result.is_ok());

            Ok(())
        }

//        #[test]
//        fn when_relative_path() -> Result<()> {
//            let mut context = ContextMock::new();
//
//            context.expect_get_cwd()
//                .returns(Ok(std::path::PathBuf::from("/path/to")));
//
//            context.expect_is_executable(|p| p.partial_eq(std::path::PathBuf::from("/path/to/executable")))
//                .returns(true);
//
//            let sut = Executable::WithFilename(std::path::PathBuf::from("executable"));
//            let result = resolve_executable(&context, &sut);
//
//            assert_eq!(true, result.is_ok());
//
//            Ok(())
//        }

    }
}
