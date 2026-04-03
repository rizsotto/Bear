// SPDX-License-Identifier: GPL-3.0-or-later

//! YAML schema validation tests.
//!
//! These tests validate all YAML compiler definitions at test time,
//! providing better error messages than build-time panics.

use bear_codegen::codegen::{pattern_to_rust, result_to_rust};
use bear_codegen::load_tables;
use bear_codegen::resolve::resolve_environment;
use bear_codegen::tables::TABLES;

/// Every YAML file parses successfully.
#[test]
fn all_yaml_files_parse() {
    // load_tables panics on parse failure, so just calling it is the test
    let tables = load_tables();
    assert_eq!(tables.len(), TABLES.len());
}

/// Every `extends` reference points to an existing table.
#[test]
fn extends_references_are_valid() {
    let tables = load_tables();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        if let Some(ref base_name) = table.extends {
            assert!(
                tables.contains_key(base_name.as_str()),
                "{} extends '{}', which does not exist",
                config.yaml_file,
                base_name
            );
        }
    }
}

/// Every flag entry uses a known result string.
#[test]
fn all_flag_results_are_valid() {
    let tables = load_tables();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        for entry in &table.flags {
            // result_to_rust panics on unknown values
            let _ = result_to_rust(&entry.result);
        }
    }
}

/// Every flag pattern produces valid codegen output.
#[test]
fn all_flag_patterns_produce_valid_codegen() {
    let tables = load_tables();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        for entry in &table.flags {
            let output = pattern_to_rust(&entry.match_.pattern, entry.match_.count);
            assert!(
                output.starts_with("FlagPattern::"),
                "{}: pattern '{}' produced unexpected output: {}",
                config.yaml_file,
                entry.match_.pattern,
                output
            );
        }
    }
}

/// Every environment entry in every YAML file passes validation.
#[test]
fn all_env_entries_are_valid() {
    let tables = load_tables();
    let mut errors = Vec::new();

    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let entries = resolve_environment(key, &tables);
        for entry in &entries {
            if entry.effect == "none" {
                continue;
            }
            if let Err(e) = entry.validate(config.yaml_file) {
                errors.push(e);
            }
        }
    }

    assert!(errors.is_empty(), "Environment validation errors:\n{}", errors.join("\n"));
}

/// Every environment variable name is a valid C identifier.
#[test]
fn env_variable_names_are_c_identifiers() {
    let var_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    let tables = load_tables();

    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        if let Some(ref env) = tables[key].environment {
            for entry in env {
                assert!(
                    var_re.is_match(&entry.variable),
                    "{}: '{}' is not a valid C identifier",
                    config.yaml_file,
                    entry.variable
                );
            }
        }
    }
}

/// No two YAML files extend into a cycle.
#[test]
fn no_circular_extends() {
    let tables = load_tables();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let mut visited = std::collections::HashSet::new();
        let mut current = Some(key.to_string());
        while let Some(k) = current {
            assert!(
                visited.insert(k.clone()),
                "{}: circular extends chain detected at '{}'",
                config.yaml_file,
                k
            );
            current = tables.get(k.as_str()).and_then(|t| t.extends.clone());
        }
    }
}

/// Every table with a `type` field has at least one `recognize` entry.
#[test]
fn typed_tables_have_recognition_entries() {
    let tables = load_tables();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        if table.type_.is_some() {
            assert!(
                table.recognize.as_ref().is_some_and(|r| !r.is_empty()),
                "{}: has type but no recognize entries",
                config.yaml_file
            );
        }
    }
}

/// Every table has at least one flag entry (own or inherited).
#[test]
fn all_tables_have_flags() {
    let tables = load_tables();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        let has_own = !table.flags.is_empty();
        let has_inherited = table
            .extends
            .as_ref()
            .and_then(|base| tables.get(base.as_str()))
            .is_some_and(|base| !base.flags.is_empty());
        assert!(has_own || has_inherited, "{}: no flags defined (own or inherited)", config.yaml_file);
    }
}
