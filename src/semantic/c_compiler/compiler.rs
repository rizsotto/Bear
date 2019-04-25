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

use regex;
use shellwords;
use std::path;
use std::process;
use std::str;

use super::Result;

pub struct Classifier {
    ignore: bool,
    c_compilers: Vec<String>,
    cxx_compilers: Vec<String>,
}

impl Default for Classifier {
    /// Default value constructor.
    ///
    /// Only the pre-set compilers will classify as compiler.
    fn default() -> Self {
        Classifier { ignore: false, c_compilers: vec![], cxx_compilers: vec![] }
    }
}

impl Classifier {
    /// Create a new Category object.
    ///
    /// # Arguments
    /// `only_use` - use only the given compiler names for classification,
    /// `c_compilers` - list of C compiler names,
    /// `cxx_compilers` - list of C++ compiler names.
    pub fn new(only_use: bool, c_compilers: &[String], cxx_compilers: &[String]) -> Self {
        let c_compiler_names: Vec<_> = c_compilers
            .iter()
            .map(|path| basename(&path))
            .collect();
        let cxx_compiler_names: Vec<_> = cxx_compilers
            .iter()
            .map(|path| basename(&path))
            .collect();

        Self {
            ignore: only_use,
            c_compilers: c_compiler_names,
            cxx_compilers: cxx_compiler_names,
        }
    }

    /// A predicate to decide whether the command is a compiler call.
    ///
    /// # Arguments
    /// `command` - the command to classify
    ///
    /// # Returns
    /// None if the command is not a compilation, or a tuple (compiler, arguments) otherwise.
    pub fn split(&self, command: &[String]) -> Option<(String, Vec<String>)> {
        match command.split_first() {
            Some((executable, parameters)) => {
                // 'wrapper' 'parameters' and
                // 'wrapper' 'compiler' 'parameters' are valid.
                // Additionally, a wrapper can wrap another wrapper.
                if self.is_wrapper(&executable) {
                    let result = self.split(parameters);
                    // Compiler wrapper without compiler is a 'C' compiler.
                    if result.is_some() {
                        result
                    } else {
                        Some((executable.clone(), parameters.to_vec()))
                    }
                // MPI compiler wrappers add extra parameters
                } else if self.is_mpi_wrapper(executable) {
                    match get_mpi_call(executable) {
                        Ok(mut mpi_call) => {
                            mpi_call.extend_from_slice(parameters);
                            self.split(mpi_call.as_ref())
                        }
                        _ => None,
                    }
                // and 'compiler' 'parameters' is valid.
                } else if self.is_c_compiler(&executable) || self.is_cxx_compiler(&executable) {
                    Some((executable.clone(), parameters.to_vec()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Match against known compiler wrappers.
    fn is_wrapper(&self, executable: &str) -> bool {
        let program = basename(executable);
        COMPILER_PATTERN_WRAPPER.is_match(&program)
    }

    /// Match against known MPI compiler wrappers.
    fn is_mpi_wrapper(&self, executable: &str) -> bool {
        let program = basename(executable);
        COMPILER_PATTERNS_MPI_WRAPPER.is_match(&program)
    }

    /// Match against known C compiler names.
    fn is_c_compiler(&self, executable: &str) -> bool {
        let program = basename(executable);
        let use_match = self.c_compilers.contains(&program);
        if self.ignore {
            use_match
        } else {
            use_match || is_pattern_match(&program, &COMPILER_PATTERNS_CC)
        }
    }

    /// Match against known C++ compiler names.
    fn is_cxx_compiler(&self, executable: &str) -> bool {
        let program = basename(executable);
        let use_match = self.cxx_compilers.contains(&program);
        if self.ignore {
            use_match
        } else {
            use_match || is_pattern_match(&program, &COMPILER_PATTERNS_CXX)
        }
    }
}

/// Takes a command string and returns as a list.
fn shell_split(string: &str) -> Result<Vec<String>> {
    match shellwords::split(string) {
        Ok(value) => Ok(value),
        _ => Err("Can't parse shell command".into()),
    }
}

/// Provide information on how the underlying compiler would have been
/// invoked without the MPI compiler wrapper.
fn get_mpi_call(wrapper: &str) -> Result<Vec<String>> {
    fn run_mpi_wrapper(wrapper: &str, flag: &str) -> Result<Vec<String>> {
        let child = process::Command::new(wrapper)
            .arg(flag)
            .stdout(process::Stdio::piped())
            .spawn()?;
        let output = child.wait_with_output()?;
        // Take the stdout if the process was successful.
        if output.status.success() {
            // Take only the first line and treat as it would be a shell command.
            let output_string = str::from_utf8(output.stdout.as_slice())?;
            match output_string.lines().next() {
                Some(first_line) => shell_split(first_line),
                _ => Err("Empty output of wrapper".into()),
            }
        } else {
            Err("Process failed.".into())
        }
    }

    // Try both flags with the wrapper and return the first successful result.
    ["--show", "--showme"]
        .iter()
        .map(|&query_flatg| run_mpi_wrapper(wrapper, &query_flatg))
        .find(Result::is_ok)
        .unwrap_or_else(|| Err("Could not determinate MPI flags.".into()))
}

/// Match against a list of regex and return true if any of those were match.
fn is_pattern_match(candidate: &str, patterns: &[regex::Regex]) -> bool {
    patterns.iter().any(|pattern| pattern.is_match(candidate))
}

/// Returns the filename of the given path (rendered as String).
fn basename(file: &str) -> String {
    let path = path::PathBuf::from(file);
    match path.file_name().map(std::ffi::OsStr::to_str) {
        Some(Some(str)) => str.to_string(),
        _ => file.to_string(),
    }
}

lazy_static! {
    /// Known C/C++ compiler wrapper name patterns.
    static ref COMPILER_PATTERN_WRAPPER: regex::Regex =
        regex::Regex::new(r"^(distcc|ccache)$").unwrap();

    /// Known MPI compiler wrapper name patterns.
    static ref COMPILER_PATTERNS_MPI_WRAPPER: regex::Regex =
        regex::Regex::new(r"^mpi(cc|cxx|CC|c\+\+)$").unwrap();

    /// Known C compiler executable name patterns.
    static ref COMPILER_PATTERNS_CC: Vec<regex::Regex> = vec![
        regex::Regex::new(r"^([^-]*-)*[mg]cc(-?\d+(\.\d+){0,2})?$").unwrap(),
        regex::Regex::new(r"^([^-]*-)*clang(-\d+(\.\d+){0,2})?$").unwrap(),
        regex::Regex::new(r"^(|i)cc$").unwrap(),
        regex::Regex::new(r"^(g|)xlc$").unwrap(),
    ];

    /// Known C++ compiler executable name patterns.
    static ref COMPILER_PATTERNS_CXX: Vec<regex::Regex> = vec![
        regex::Regex::new(r"^(c\+\+|cxx|CC)$").unwrap(),
        regex::Regex::new(r"^([^-]*-)*[mg]\+\+(-?\d+(\.\d+){0,2})?$").unwrap(),
        regex::Regex::new(r"^([^-]*-)*clang\+\+(-\d+(\.\d+){0,2})?$").unwrap(),
        regex::Regex::new(r"^icpc$").unwrap(),
        regex::Regex::new(r"^(g|)xl(C|c\+\+)$").unwrap(),
    ];
}
