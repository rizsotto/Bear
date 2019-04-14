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

use super::super::InterceptMode;

pub type Environment = std::collections::HashMap<String, String>;

pub struct EnvironmentBuilder {
    state: Box<Environment>,
}

impl EnvironmentBuilder {

    fn new() -> EnvironmentBuilder {
        unimplemented!()
    }

    fn from(_environment: &Environment) -> EnvironmentBuilder {
        unimplemented!()
    }

    pub fn build(&self) -> Environment {
        unimplemented!()
    }

    pub fn with_verbose(&mut self, verbose: bool) -> &mut EnvironmentBuilder {
        if verbose {
            self.insert_str(keys::VERBOSE, "1");
        }
        self
    }

    pub fn with_destination(&mut self, destination: &std::path::Path) -> &mut EnvironmentBuilder {
        self.insert_path(keys::DESTINATION, destination);
        self
    }

    pub fn with_modes(&mut self, modes: &[InterceptMode]) -> &mut EnvironmentBuilder {
        for mode in modes {
            match mode {
                InterceptMode::WrapperPreload { library, wrapper } => {
                    self.insert_path(keys::INTERCEPT_LIBRARY, library);
                    self.insert_path(keys::INTERCEPT_REPORTER, wrapper);
                    self.insert_library(library)
                },
                InterceptMode::WrapperCC { compiler, wrapper, .. } => {
                    self.insert_path(keys::INTERCEPT_CC, compiler);
                    self.insert_path(keys::CC, wrapper);
                },
                InterceptMode::WrapperCXX { compiler, wrapper, .. } => {
                    self.insert_path(keys::INTERCEPT_CXX, compiler);
                    self.insert_path(keys::CXX, wrapper);
                },
            }
        }
        self
    }

    #[cfg(any(target_os = "android",
              target_os = "freebsd",
              target_os = "linux"))]
    fn insert_library(&mut self, library: &std::path::Path) {
        self.insert_preload(keys::GLIBC_PRELOAD, library);
    }

    #[cfg(target_os = "macos")]
    fn insert_library(&mut self, library: &std::path::Path) {
        self.insert_str(keys::OSX_NAMESPACE, "1");
        self.insert_preload(keys::OSX_PRELOAD, library);
    }

    #[cfg(not(unix))]
    fn insert_library(&mut self, library: &std::path::Path) {
        info!("preload library ignored");
    }

    fn insert_preload(&mut self, key: &str, library: &std::path::Path) {
        self.state.entry(key.to_string())
            .and_modify(|current| {
                *current = insert_into_paths(current, library);
            })
            .or_insert(library.to_string_lossy().to_string());
    }

    fn insert_path(&mut self, key: &str, value: &std::path::Path) {
        self.insert_str(key, &value.to_string_lossy());
    }

    fn insert_str(&mut self, key: &str, value: &str) {
        self.state.insert(key.to_string(), value.to_string());
    }
}

fn insert_into_paths(path_str: &str, library: &std::path::Path) -> String {
    // Split up the string into paths.
    let mut paths = std::env::split_paths(path_str)
        .into_iter()
        .collect::<Vec<_>>();
    // Make sure the library is the first one in the paths.
    if paths.len() < 1 || paths[0] != library {
        paths.insert(0, library.to_path_buf());
        paths.dedup();
    }
    // Join the paths into a string again.
    std::env::join_paths(paths)
        .map(|os| os.to_string_lossy().to_string())
        .unwrap_or_else(|err| {
            warn!("Failed to insert library into path: {}", err);
            path_str.to_string()
        })
}

pub mod get {
    use super::keys;
    use crate::intercept::Result;

    pub fn target_directory() -> Result<std::path::PathBuf> {
        let env = std::env::var(keys::DESTINATION)
            .map(std::path::PathBuf::from)?;
//            .chain_err(|| "Can't find target directory.".into())
        Ok(env)
    }

    pub fn c_compiler_path() -> Result<String> {
        let env = std::env::var(keys::INTERCEPT_CC)?;
//            .chain_err(|| "Can't find cc compiler.".into())
        Ok(env)
    }

    pub fn cxx_compiler_path() -> Result<String> {
        let env = std::env::var(keys::INTERCEPT_CXX)?;
//            .chain_err(|| "Can't find c++ compiler.".into())
        Ok(env)
    }

    pub fn parent_pid() -> Result<u32> {
        let env = std::env::var(keys::PARENT_PID)?;
//            .chain_err(|| "Can't find parent process id.".into())?;
        let num = env.parse::<u32>()?;
        Ok(num)
    }
}

mod keys {
    #[cfg(target_os = "macos")]
    pub const OSX_PRELOAD: &str = "DYLD_INSERT_LIBRARIES";

    #[cfg(target_os = "macos")]
    pub const OSX_NAMESPACE: &str = "DYLD_FORCE_FLAT_NAMESPACE";

    #[cfg(any(target_os = "android",
    target_os = "freebsd",
    target_os = "linux"))]
    pub const GLIBC_PRELOAD: &str = "LD_PRELOAD";

    pub const CC: &str = "CC";
    pub const CXX: &str = "CXX";

    pub const INTERCEPT_CC: &str = "INTERCEPT_CC";
    pub const INTERCEPT_CXX: &str = "INTERCEPT_CXX";
    pub const INTERCEPT_LIBRARY: &str = "INTERCEPT_LIBRARY";
    pub const INTERCEPT_REPORTER: &str = "INTERCEPT_REPORT_COMMAND";
    pub const DESTINATION: &str = "INTERCEPT_REPORT_DESTINATION";
    pub const VERBOSE: &str = "INTERCEPT_VERBOSE";
    pub const PARENT_PID: &str = "INTERCEPT_PARENT_PID";
}