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
use std::collections;
use std::mem;
use std::path;
use std::process;
use std::str;

use trace;
use Error;
use Result;

#[derive(Debug,PartialEq)]
pub enum CompilerPass {
    Preprocessor,
    Compilation,
    Linking,
    Internal,
}

impl CompilerPass {
    fn is_compiling(&self) -> bool {
        self == &CompilerPass::Compilation || self == &CompilerPass::Linking
    }

    fn update(&mut self, new_state: &CompilerPass) {
        match self {
            CompilerPass::Linking => match new_state {
                CompilerPass::Internal => { mem::replace(self, CompilerPass::Internal); },
                CompilerPass::Compilation => { mem::replace(self, CompilerPass::Compilation); },
                CompilerPass::Preprocessor => { mem::replace(self, CompilerPass::Preprocessor); },
                _ => (),
            },
            CompilerPass::Compilation => match new_state {
                CompilerPass::Internal => { mem::replace(self, CompilerPass::Internal); },
                CompilerPass::Preprocessor => { mem::replace(self, CompilerPass::Preprocessor); },
                _ => (),
            },
            CompilerPass::Preprocessor => match new_state {
                CompilerPass::Internal => { mem::replace(self, CompilerPass::Internal); },
                _ => (),
            },
            _ => (),
        }
    }
}

#[derive(Debug)]
pub struct CompilerExecution {
    compiler: path::PathBuf,
    phase: CompilerPass,
    flags: Vec<String>,
    inputs: Vec<path::PathBuf>,
    output: Option<path::PathBuf>,
}

impl CompilerExecution {
    pub fn from(_trace: &trace::Trace) -> Option<CompilerExecution> {
        unimplemented!()
    }

    /// A predicate to decide whether the command is a compiler call.
    ///
    /// # Arguments
    /// `command` - the command to classify
    /// `category` - helper object to detect compiler
    ///
    /// # Returns
    /// None if the command is not a compilation, or a tuple (compiler, arguments) otherwise.
    fn split_compiler(command: &[String], category: &Category) -> Option<(String, Vec<String>)> {
        match command.split_first() {
            Some((executable, parameters)) => {
                // 'wrapper' 'parameters' and
                // 'wrapper' 'compiler' 'parameters' are valid.
                // Additionally, a wrapper can wrap another wrapper.
                if category.is_wrapper(&executable) {
                    let result = CompilerExecution::split_compiler(parameters, category);
                    // Compiler wrapper without compiler is a 'C' compiler.
                    if result.is_some() {
                        result
                    } else {
                        Some((executable.clone(), parameters.to_vec()))
                    }
                // MPI compiler wrappers add extra parameters
                } else if category.is_mpi_wrapper(executable) {
                    match get_mpi_call(executable) {
                        Ok(mut mpi_call) => {
                            mpi_call.extend_from_slice(parameters);
                            CompilerExecution::split_compiler(mpi_call.as_ref(), category)
                        },
                        _ => None,
                    }
                // and 'compiler' 'parameters' is valid.
                } else if category.is_c_compiler(&executable) {
                    Some((executable.clone(), parameters.to_vec()))
                } else if category.is_cxx_compiler(&executable) {
                    Some((executable.clone(), parameters.to_vec()))
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Returns a value when the command is a compilation, None otherwise.
    ///
    /// # Arguments
    /// `command` - the command to classify
    /// `category` - helper object to detect compiler
    ///
    /// Returns a CompilationCommand objects optionally.
    fn parse_command(command: &[String], category: &Category) -> Option<CompilerExecution> {
        debug!("input was: {:?}", command);
        match CompilerExecution::split_compiler(command, category) {
            Some(compiler_and_parameters) => {
                let mut result = CompilerExecution {
                    compiler: path::PathBuf::from(compiler_and_parameters.0),
                    phase: CompilerPass::Linking,
                    flags: vec![],
                    inputs: vec![],
                    output: None,
                };
                for arg in compiler_and_parameters.1.iter() {
                    // if it's a phase modifier flag, update it and move on.
                    if let Some(phase) = PHASE_FLAGS.get(arg.as_str()) {
                        result.phase.update(phase);
                        continue
                    }
                    // check shall we ignore this flag.
                    let _count_opt = IGNORED_FLAGS.get(arg.as_str());
                }
                if result.phase.is_compiling() {
                    debug!("output is {:?}", result);
                    Some(result)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

//    def _split_command(cls, command, category):
//        # iterate on the compile options
//        args = iter(compiler_and_arguments[2])
//        for arg in args:
//            # ignore some flags
//            elif arg in IGNORED_FLAGS:
//                count = IGNORED_FLAGS[arg]
//                for _ in range(count):
//                    next(args)
//            elif re.match(r'^-(l|L|Wl,).+', arg):
//                pass
//            # some parameters look like a filename, take those explicitly
//            elif arg in {'-D', '-I'}:
//                result.flags.extend([arg, next(args)])
//            # get the output file separately
//            elif arg == '-o':
//                result.output.append(next(args))
//            # parameter which looks source file is taken...
//            elif re.match(r'^[^-].+', arg) and classify_source(arg):
//                result.files.append(arg)
//            # and consider everything else as compile option.
//            else:
//                result.flags.append(arg)
//        logging.debug('output is: %s', result)
//        # do extra check on number of source files
//        return result if result.files else None

}

lazy_static! {
    /// Map of ignored compiler option for the creation of a compilation database.
    /// This map is used in split_command method, which classifies the parameters
    /// and ignores the selected ones. Please note that other parameters might be
    /// ignored as well.
    ///
    /// Option names are mapped to the number of following arguments which should
    /// be skipped.
    static ref IGNORED_FLAGS: collections::BTreeMap<&'static str, u8> = {
        let mut m = collections::BTreeMap::new();
        // preprocessor macros, ignored because would cause duplicate entries in
        // the output (the only difference would be these flags). this is actual
        // finding from users, who suffered longer execution time caused by the
        // duplicates.
        m.insert("-MD",         0u8);
        m.insert("-MMD",        0u8);
        m.insert("-MG",         0u8);
        m.insert("-MP",         0u8);
        m.insert("-MF",         1u8);
        m.insert("-MT",         1u8);
        m.insert("-MQ",         1u8);
        // linker options, ignored because for compilation database will contain
        // compilation commands only. so, the compiler would ignore these flags
        // anyway. the benefit to get rid of them is to make the output more
        // readable.
        m.insert("-static",     0u8);
        m.insert("-shared",     0u8);
        m.insert("-s",          0u8);
        m.insert("-rdynamic",   0u8);
        m.insert("-l",          1u8);
        m.insert("-L",          1u8);
        m.insert("-u",          1u8);
        m.insert("-z",          1u8);
        m.insert("-T",          1u8);
        m.insert("-Xlinker",    1u8);
        // clang-cl / msvc cl specific flags
        // consider moving visual studio specific warning flags also
        m.insert("-nologo",     0u8);
        m.insert("-EHsc",       0u8);
        m.insert("-EHa",        0u8);
        m
    };

    static ref PHASE_FLAGS: collections::BTreeMap<&'static str, CompilerPass> = {
        let mut m = collections::BTreeMap::new();
        m.insert("-v",      CompilerPass::Internal);
        m.insert("-###",    CompilerPass::Internal);
        m.insert("-cc1",    CompilerPass::Internal);
        m.insert("-cc1as",  CompilerPass::Internal);
        m.insert("-E",      CompilerPass::Preprocessor);
        m.insert("-M",      CompilerPass::Preprocessor);
        m.insert("-MM",     CompilerPass::Preprocessor);
        m.insert("-c",      CompilerPass::Compilation);
        m.insert("-S",      CompilerPass::Compilation);
        m
    };

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

struct Category {
    ignore: bool,
    c_compilers: Vec<String>,
    cxx_compilers: Vec<String>,
}

impl Category {
    fn new(only_use: bool, c_compilers: &[String], cxx_compilers: &[String]) -> Result<Category> {
        let c_compiler_names: Vec<_> = c_compilers
            .into_iter()
            .map(|path| Category::_basename(&path))
            .collect();
        let cxx_compiler_names: Vec<_> = cxx_compilers
            .into_iter()
            .map(|path| Category::_basename(&path))
            .collect();

        Ok(Category {
            ignore: only_use,
            c_compilers: c_compiler_names,
            cxx_compilers: cxx_compiler_names,
        })
    }

    fn is_wrapper(&self, executable: &String) -> bool {
        let program = Category::_basename(executable);
        COMPILER_PATTERN_WRAPPER.is_match(&program)
    }

    fn is_mpi_wrapper(&self, executable: &String) -> bool {
        let program = Category::_basename(executable);
        COMPILER_PATTERNS_MPI_WRAPPER.is_match(&program)
    }

    fn is_c_compiler(&self, executable: &String) -> bool {
        let program = Category::_basename(executable);
        let use_match = self.c_compilers.contains(&program);
        if self.ignore {
            use_match
        } else {
            use_match || Category::_is_pattern_match(&program, &COMPILER_PATTERNS_CC)
        }
    }

    fn is_cxx_compiler(&self, executable: &String) -> bool {
        let program = Category::_basename(executable);
        let use_match = self.cxx_compilers.contains(&program);
        if self.ignore {
            use_match
        } else {
            use_match || Category::_is_pattern_match(&program, &COMPILER_PATTERNS_CXX)
        }
    }

    fn _is_pattern_match(candidate: &String, patterns: &Vec<regex::Regex>) -> bool {
        patterns.iter().any(|pattern| pattern.is_match(candidate))
    }

    /// Returns the filename of the given path (rendered as String).
    fn _basename(file: &String) -> String {
        let path = path::PathBuf::from(file);
        match path.file_name().map(|path| path.to_str()) {
            Some(Some(str)) => str.to_string(),
            _ => file.clone(),
        }
    }
}

/// Takes a command string and returns as a list.
fn shell_split(string: &str) -> Result<Vec<String>> {
    match shellwords::split(string) {
        Ok(value) => Ok(value),
        _ => Err(Error::RuntimeError("Can't parse shell command")),
    }
}

/// Provide information on how the underlying compiler would have been
/// invoked without the MPI compiler wrapper.
fn get_mpi_call(wrapper: &String) -> Result<Vec<String>> {
    fn run_mpi_wrapper(wrapper: &String, flag: &str) -> Result<Vec<String>> {
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
                _ => Err(Error::RuntimeError("Empty output of wrapper")),
            }
        } else {
            Err(Error::RuntimeError("Process failed."))
        }
    }

    // Try both flags with the wrapper and return the first successful result.
    ["--show", "--showme"]
        .iter()
        .map(|&query_flatg| run_mpi_wrapper(wrapper, &query_flatg))
        .find(Result::is_ok)
        .unwrap_or(Err(Error::RuntimeError("Could not determinate MPI flags.")))
}
