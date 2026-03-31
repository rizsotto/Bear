// SPDX-License-Identifier: GPL-3.0-or-later

use super::Main;
use super::validation::Validator;
use directories::{BaseDirs, ProjectDirs};
use log::{debug, info};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub struct Loader {}

impl Loader {
    /// Loads the configuration from the specified file or the default locations.
    ///
    /// If the configuration file is specified, it will be used. Otherwise, the default locations
    /// will be searched for the configuration file. If the configuration file is not found, the
    /// default configuration will be returned.
    #[allow(clippy::result_large_err)] // Config loading runs once at startup.
    pub fn load(context: &crate::context::Context, filename: &Option<String>) -> Result<Main, ConfigError> {
        if let Some(path) = filename {
            // If the configuration file is specified, use it.
            Self::from_file(Path::new(path))
        } else {
            // Otherwise, try to find the configuration file in the default locations.
            let locations = Self::file_locations(context);
            for location in locations {
                debug!("Checking configuration file: {}", location.display());
                if location.exists() {
                    return Self::from_file(location.as_path());
                }
            }
            // If the configuration file is not found, return the default configuration.
            debug!("Configuration file not found. Using the default configuration.");
            Ok(Main::default())
        }
    }

    /// The default locations where the configuration file can be found.
    ///
    /// The locations are searched in the following order:
    /// - The current working directory.
    /// - The local configuration directory of the user.
    /// - The configuration directory of the user.
    /// - The local configuration directory of the application.
    /// - The configuration directory of the application.
    fn file_locations(context: &crate::context::Context) -> Vec<PathBuf> {
        let mut locations = Vec::new();

        locations.push(context.current_directory.clone());
        if let Some(base_dirs) = BaseDirs::new() {
            locations.push(base_dirs.config_local_dir().to_path_buf());
            locations.push(base_dirs.config_dir().to_path_buf());
        }

        if let Some(proj_dirs) = ProjectDirs::from("com.github", "rizsotto", "Bear") {
            locations.push(proj_dirs.config_local_dir().to_path_buf());
            locations.push(proj_dirs.config_dir().to_path_buf());
        }
        // filter out duplicate elements from the list
        locations.dedup();
        // append the default configuration file name to the locations
        locations.iter().map(|p| p.join("bear.yml")).collect()
    }

    /// Loads the configuration from the specified file.
    #[allow(clippy::result_large_err)] // Config loading runs once at startup.
    pub fn from_file(path: &Path) -> Result<Main, ConfigError> {
        info!("Loading configuration file: {}", path.display());

        let reader = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|source| ConfigError::FileAccess { path: path.to_path_buf(), source })?;

        let content: Main = Self::from_reader(reader)
            .map_err(|source| ConfigError::ParseError { path: path.to_path_buf(), source })?;

        // Validate the loaded configuration
        Main::validate(&content)
            .map_err(|source| ConfigError::ValidationError { path: path.to_path_buf(), source })?;

        Ok(content)
    }

    /// Define the deserialization format of the config file.
    pub(crate) fn from_reader<R, T>(rdr: R) -> Result<T, serde_saphyr::Error>
    where
        R: std::io::Read,
        T: serde::de::DeserializeOwned,
    {
        serde_saphyr::from_reader(rdr)
    }
}

/// Represents all possible configuration-related errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Error when opening or reading a configuration file.
    #[error("Failed to access configuration file '{path}': {source}")]
    FileAccess {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Error when parsing the configuration file format.
    #[error("Failed to parse configuration from file '{path}': {source}")]
    ParseError {
        path: PathBuf,
        #[source]
        source: serde_saphyr::Error,
    },
    /// Error when the schema version is not supported.
    #[error("Unsupported schema version: {found}. Expected: {expected}")]
    UnsupportedSchema { found: String, expected: String },
    /// Error when configuration validation fails.
    #[error("Configuration validation failed: {source}")]
    ValidationError {
        path: PathBuf,
        #[source]
        source: crate::config::validation::ValidationError,
    },
}

#[cfg(test)]
mod test {

    use super::super::*;
    use super::*;
    use std::fs;

    #[test]
    fn test_wrapper_config() {
        let content: &[u8] = br#"
            schema: 4.1

            intercept:
                mode: wrapper

            compilers:
              - path: /usr/local/bin/cc
                as: gcc
              - path: /usr/bin/cc
                ignore: true
              - path: /usr/bin/clang++
                flags:
                    add: ["-I/opt/MPI/include"]
                    remove: ["-Wall"]

            sources:
                directories:
                  - path: "/opt/project/sources"
                    action: include
                  - path: "/opt/project/tests"
                    action: exclude

            duplicates:
                match_on: [file, directory]

            format:
                paths:
                    directory: canonical
                    file: canonical
                entries:
                    use_array_format: true
                    include_output_field: true
            "#;

        let result = Loader::from_reader(content).unwrap();

        let expected = Main {
            schema: String::from("4.1"),
            intercept: Intercept::Wrapper,
            compilers: vec![
                Compiler {
                    path: PathBuf::from("/usr/local/bin/cc"),
                    as_: Some(CompilerType::Gcc),
                    ignore: false,
                },
                Compiler { path: PathBuf::from("/usr/bin/cc"), as_: None, ignore: true },
                Compiler { path: PathBuf::from("/usr/bin/clang++"), as_: None, ignore: false },
            ],
            sources: SourceFilter {
                directories: vec![
                    DirectoryRule {
                        path: PathBuf::from("/opt/project/sources"),
                        action: DirectoryAction::Include,
                    },
                    DirectoryRule {
                        path: PathBuf::from("/opt/project/tests"),
                        action: DirectoryAction::Exclude,
                    },
                ],
            },
            duplicates: DuplicateFilter { match_on: vec![OutputFields::File, OutputFields::Directory] },
            format: Format {
                paths: PathFormat { directory: PathResolver::Canonical, file: PathResolver::Canonical },
                entries: EntryFormat { use_array_format: true, include_output_field: true },
            },
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_incomplete_wrapper_config() {
        let content: &[u8] = br#"
            schema: 4.1

            intercept:
              mode: wrapper

            format:
              paths:
                directory: as-is
                file: as-is
            "#;

        let result = Loader::from_reader(content).unwrap();

        let expected = Main {
            schema: String::from("4.1"),
            intercept: Intercept::Wrapper,
            compilers: vec![],
            sources: SourceFilter { directories: vec![] },
            duplicates: DuplicateFilter {
                match_on: vec![OutputFields::Directory, OutputFields::File, OutputFields::Arguments],
            },
            format: Format {
                paths: PathFormat { directory: PathResolver::AsIs, file: PathResolver::AsIs },
                entries: EntryFormat { use_array_format: true, include_output_field: true },
            },
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_incomplete_preload_config() {
        let content: &[u8] = br#"
            schema: 4.1

            intercept:
              mode: preload
            format:
              paths:
                directory: absolute
                file: absolute
            "#;

        let result = Loader::from_reader(content).unwrap();

        let expected = Main {
            schema: String::from("4.1"),
            intercept: Intercept::Preload,
            compilers: vec![],
            sources: SourceFilter { directories: vec![] },
            duplicates: DuplicateFilter {
                match_on: vec![OutputFields::Directory, OutputFields::File, OutputFields::Arguments],
            },
            format: Format {
                paths: PathFormat { directory: PathResolver::Absolute, file: PathResolver::Absolute },
                entries: EntryFormat { use_array_format: true, include_output_field: true },
            },
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_default_config() {
        let result = Main::default();

        let expected = Main {
            schema: String::from("4.1"),
            intercept: Intercept::default(),
            compilers: vec![],
            sources: SourceFilter::default(),
            duplicates: DuplicateFilter::default(),
            format: Format::default(),
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_invalid_schema_version() {
        let content: &[u8] = br#"
            schema: 3.0

            intercept:
              mode: wrapper
              directory: /tmp
            "#;

        let result: Result<Main, serde_saphyr::Error> = Loader::from_reader(content);

        assert!(result.is_err());

        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("Unsupported schema version: 3.0. Expected: 4.1"),
            "unexpected error message: {message}"
        );
    }

    #[test]
    fn test_failing_config() {
        let content: &[u8] = br#"{
                "output": {
                    "format": {
                        "command_as_array": false
                    },
                    "content": {
                        "duplicates": "files"
                    }
                }
            }"#;

        let result: Result<Main, serde_saphyr::Error> = Loader::from_reader(content);

        assert!(result.is_err());
    }

    #[test]
    fn test_validation_error_on_invalid_config() {
        use tempfile;

        let temp_dir = tempfile::tempdir().unwrap();
        let config_file = temp_dir.path().join("bear.yml");

        let invalid_config = r#"
            schema: "4.1"

            intercept:
                mode: wrapper
                path: /nonexistent/wrapper/path
                directory: /nonexistent/directory

            compilers:
              - path: /nonexistent/compiler
                as: "invalid_compiler_type"
                flags:
                    add: [""]
                    remove: ["valid", ""]
            "#;

        fs::write(&config_file, invalid_config).unwrap();

        // Try to load the config - should fail validation
        let result = Loader::from_file(&config_file);
        assert!(result.is_err());

        match result.unwrap_err() {
            ConfigError::ParseError { source, .. } => {
                // Verify we got a parse error for invalid compiler type
                let error_msg = source.to_string();
                assert!(error_msg.contains("unknown variant"));
                assert!(error_msg.contains("invalid_compiler_type"));
            }
            other => panic!("Expected ParseError for invalid compiler type, got: {:?}", other),
        }
    }

    #[test]
    fn test_compiler_type_serialization() {
        fn assert_compiler_type_deserializes(json_str: &str, expected: CompilerType) {
            use serde_json;

            let result = serde_json::from_str::<CompilerType>(json_str).unwrap();
            assert_eq!(result, expected);
        }

        // Test canonical names
        assert_compiler_type_deserializes("\"gcc\"", CompilerType::Gcc);
        assert_compiler_type_deserializes("\"clang\"", CompilerType::Clang);
        assert_compiler_type_deserializes("\"fortran\"", CompilerType::Flang);
        assert_compiler_type_deserializes("\"intelfortran\"", CompilerType::IntelFortran);
        assert_compiler_type_deserializes("\"crayfortran\"", CompilerType::CrayFortran);

        // Test aliases for GCC
        assert_compiler_type_deserializes("\"gnu\"", CompilerType::Gcc);

        // Test aliases for Clang
        assert_compiler_type_deserializes("\"llvm\"", CompilerType::Clang);

        // Test aliases for Fortran
        assert_compiler_type_deserializes("\"gfortran\"", CompilerType::Flang);

        // Test aliases for Intel Fortran
        assert_compiler_type_deserializes("\"ifort\"", CompilerType::IntelFortran);
        assert_compiler_type_deserializes("\"intel-fortran\"", CompilerType::IntelFortran);
        assert_compiler_type_deserializes("\"intel_fortran\"", CompilerType::IntelFortran);

        // Test aliases for Cray Fortran
        assert_compiler_type_deserializes("\"crayftn\"", CompilerType::CrayFortran);
        assert_compiler_type_deserializes("\"cray-fortran\"", CompilerType::CrayFortran);
        assert_compiler_type_deserializes("\"cray_fortran\"", CompilerType::CrayFortran);
    }

    #[test]
    fn test_compiler_config_with_type_hints() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_file = temp_dir.path().join("bear.yml");

        // Create temporary compiler files for validation
        let gcc_wrapper = temp_dir.path().join("custom-gcc-wrapper");
        let clang_wrapper = temp_dir.path().join("custom-clang");
        let fortran_wrapper = temp_dir.path().join("my-fortran");
        let intel_wrapper = temp_dir.path().join("ifort-wrapper");
        let cray_wrapper = temp_dir.path().join("ftn-wrapper");

        fs::write(&gcc_wrapper, "#!/bin/bash\necho gcc wrapper").unwrap();
        fs::write(&clang_wrapper, "#!/bin/bash\necho clang wrapper").unwrap();
        fs::write(&fortran_wrapper, "#!/bin/bash\necho fortran wrapper").unwrap();
        fs::write(&intel_wrapper, "#!/bin/bash\necho intel wrapper").unwrap();
        fs::write(&cray_wrapper, "#!/bin/bash\necho cray wrapper").unwrap();

        // Create wrapper executable and directory for validation
        let wrapper_dir = temp_dir.path().join("wrapper");
        std::fs::create_dir(&wrapper_dir).unwrap();
        let wrapper_exe = wrapper_dir.join("wrapper");
        fs::write(&wrapper_exe, "#!/bin/bash\necho wrapper").unwrap();

        let config_with_hints = format!(
            r#"
                schema: "4.1"

                intercept:
                    mode: wrapper
                    path: {}
                    directory: {}

                compilers:
                  - path: {}
                    as: "gcc"
                  - path: {}
                    as: "llvm"
                  - path: {}
                    as: "gfortran"
                  - path: {}
                    as: "intel-fortran"
                  - path: {}
                    as: "cray_fortran"
                "#,
            wrapper_exe.display(),
            wrapper_dir.display(),
            gcc_wrapper.display(),
            clang_wrapper.display(),
            fortran_wrapper.display(),
            intel_wrapper.display(),
            cray_wrapper.display()
        );

        fs::write(&config_file, config_with_hints).unwrap();

        let result = Loader::from_file(&config_file);

        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.compilers.len(), 5);

        // Verify compiler type hints are correctly parsed
        assert_eq!(config.compilers[0].as_, Some(CompilerType::Gcc));
        assert_eq!(config.compilers[1].as_, Some(CompilerType::Clang));
        assert_eq!(config.compilers[2].as_, Some(CompilerType::Flang));
        assert_eq!(config.compilers[3].as_, Some(CompilerType::IntelFortran));
        assert_eq!(config.compilers[4].as_, Some(CompilerType::CrayFortran));
    }

    #[test]
    fn test_compiler_type_display() {
        assert_eq!(CompilerType::Gcc.to_string(), "GCC");
        assert_eq!(CompilerType::Clang.to_string(), "Clang");
        assert_eq!(CompilerType::Flang.to_string(), "Flang");
        assert_eq!(CompilerType::IntelFortran.to_string(), "Intel Fortran");
        assert_eq!(CompilerType::CrayFortran.to_string(), "Cray Fortran");
    }
}
