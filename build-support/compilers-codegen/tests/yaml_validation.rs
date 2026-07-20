// SPDX-License-Identifier: GPL-3.0-or-later

//! YAML schema validation tests.
//!
//! These tests validate all YAML compiler definitions at test time,
//! providing better error messages than build-time panics.

use compilers_codegen::codegen::{pattern_to_rust, result_to_rust};
use compilers_codegen::resolve::resolve_environment;
use compilers_codegen::tables::TABLES;
use compilers_codegen::{insert_by_id, load_tables, parse_table, parse_wrapper_table, validate_extends};

/// Every YAML file parses successfully.
#[test]
fn all_yaml_files_parse() {
    let tables = load_tables().unwrap();
    assert_eq!(tables.len(), TABLES.len());
}

/// Every `extends` reference points to an existing table.
#[test]
fn extends_references_are_valid() {
    let tables = load_tables().unwrap();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        if let Some(ref base_name) = table.compiler.extends {
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
    let tables = load_tables().unwrap();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        for entry in &table.flags {
            result_to_rust(&entry.result)
                .unwrap_or_else(|e| panic!("{}: flag '{}': {}", config.yaml_file, entry.match_.pattern, e));
        }
    }
}

/// Every flag pattern produces valid codegen output.
#[test]
fn all_flag_patterns_produce_valid_codegen() {
    let tables = load_tables().unwrap();
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
    let tables = load_tables().unwrap();
    let mut errors = Vec::new();

    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let entries = resolve_environment(key, &tables);
        for entry in &entries {
            if entry.effect == "none" {
                continue;
            }
            if let Err(e) = entry.validate() {
                errors.push(format!("{}: {}", config.yaml_file, e));
            }
        }
    }

    assert!(errors.is_empty(), "Environment validation errors:\n{}", errors.join("\n"));
}

/// Every environment variable name is a valid C identifier.
#[test]
fn env_variable_names_are_c_identifiers() {
    fn is_valid_var_name(s: &str) -> bool {
        let mut chars = s.chars();
        matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    let tables = load_tables().unwrap();

    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        if let Some(ref env) = tables[key].environment {
            for entry in env {
                assert!(
                    is_valid_var_name(&entry.variable),
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
    let tables = load_tables().unwrap();
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
            current = tables.get(k.as_str()).and_then(|t| t.compiler.extends.clone());
        }
    }
}

/// Every table has at least one `recognize` entry.
///
/// `type:` is mandatory now, so every loaded table is typed by construction;
/// this asserts the invariant unconditionally instead of gating on
/// `Kind::Compiler` (the only kind any table uses today).
#[test]
fn typed_tables_have_recognition_entries() {
    let tables = load_tables().unwrap();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        assert!(
            table.recognize.as_ref().is_some_and(|r| !r.is_empty()),
            "{}: has type but no recognize entries",
            config.yaml_file
        );
    }
}

/// Every table has at least one flag entry (own or inherited).
#[test]
fn all_tables_have_flags() {
    let tables = load_tables().unwrap();
    for config in TABLES {
        let key = config.yaml_file.strip_suffix(".yaml").unwrap();
        let table = &tables[key];
        let has_own = !table.flags.is_empty();
        let has_inherited = table
            .compiler
            .extends
            .as_ref()
            .and_then(|base| tables.get(base.as_str()))
            .is_some_and(|base| !base.flags.is_empty());
        assert!(has_own || has_inherited, "{}: no flags defined (own or inherited)", config.yaml_file);
    }
}

// -- Negative-case tests for the `type:`/`compiler:` schema split --
//
// These exercise the loader against small inline YAML fixtures rather than
// mutating the real compiler YAML files on disk.

const MINIMAL_FLAGS: &str = "flags: []\n";

/// Two tables declaring the same `compiler.id` fail to load, and the error
/// names the file that redefines the id.
#[test]
fn duplicate_ids_fail_naming_the_file() {
    let yaml = format!("type: compiler\ncompiler:\n  id: dup\n{}", MINIMAL_FLAGS);
    let first = parse_table("first.yaml", &yaml).unwrap();
    let second = parse_table("second.yaml", &yaml).unwrap();

    let mut tables = std::collections::HashMap::new();
    insert_by_id(&mut tables, "first.yaml", first).unwrap();
    let err = insert_by_id(&mut tables, "second.yaml", second).unwrap_err();

    assert!(err.to_string().contains("dup"), "{}", err);
    assert!(err.to_string().contains("second.yaml"), "{}", err);
}

/// An `extends:` target that no loaded table declares as its id fails
/// validation, and the error names the file with the dangling reference.
#[test]
fn dangling_extends_target_is_detected() {
    let yaml = format!("type: compiler\ncompiler:\n  id: leaf\n  extends: nonexistent\n{}", MINIMAL_FLAGS);
    let table = parse_table("leaf.yaml", &yaml).unwrap();

    let mut tables = std::collections::HashMap::new();
    let mut file_to_id = std::collections::HashMap::new();
    let id = insert_by_id(&mut tables, "leaf.yaml", table).unwrap();
    file_to_id.insert("leaf.yaml", id);

    let err = validate_extends(&tables, &file_to_id).unwrap_err();
    assert!(err.to_string().contains("leaf.yaml"), "{}", err);
    assert!(err.to_string().contains("nonexistent"), "{}", err);
}

/// A `compiler:` block missing the mandatory `id:` fails to parse, and the
/// error names the offending file.
#[test]
fn missing_id_on_compiler_table_fails() {
    let yaml = format!("type: compiler\ncompiler:\n  extends: base\n{}", MINIMAL_FLAGS);
    let err = parse_table("no_id.yaml", &yaml).err().unwrap();
    assert!(err.to_string().contains("no_id.yaml"), "{}", err);
}

/// An unrecognized `type:` value fails to parse, and the error names the
/// offending file.
#[test]
fn unknown_type_value_fails() {
    let yaml = format!("type: bogus\ncompiler:\n  id: x\n{}", MINIMAL_FLAGS);
    let err = parse_table("bad_type.yaml", &yaml).err().unwrap();
    assert!(err.to_string().contains("bad_type.yaml"), "{}", err);
}

// -- Negative-case tests for `WrapperTable` --
//
// These exercise `WrapperTable::validate` against small inline YAML
// fixtures rather than mutating the real launcher YAML files on disk.

const MINIMAL_WRAPPER_RECOGNIZE: &str = concat!(
    "recognize:\n",
    "  - description: \"Compiler cache\"\n",
    "    references:\n",
    "      - \"https://example.com/docs\"\n",
    "    executables: [\"fake-wrapper\"]\n",
);

/// A minimal, valid wrapper table (no options) passes validation.
#[test]
fn valid_wrapper_table_passes_validation() {
    let yaml = format!("type: wrapper\n{}", MINIMAL_WRAPPER_RECOGNIZE);
    let table = parse_wrapper_table("fake_wrapper.yaml", &yaml).unwrap();
    assert!(table.validate("fake_wrapper.yaml").is_ok());
}

/// A wrapper `recognize` entry with `versioned: true` fails validation,
/// naming the offending file.
#[test]
fn wrapper_recognize_versioned_true_fails() {
    let yaml = "type: wrapper\nrecognize:\n  - description: \"Compiler cache\"\n    references:\n      - \"https://example.com/docs\"\n    executables: [\"fake-wrapper\"]\n    versioned: true\n";
    let table = parse_wrapper_table("fake_wrapper.yaml", yaml).unwrap();
    let err = table.validate("fake_wrapper.yaml").unwrap_err();
    assert!(err.to_string().contains("fake_wrapper.yaml"), "{}", err);
    assert!(err.to_string().contains("versioned"), "{}", err);
}

/// A wrapper `recognize` entry with `cross_compilation: true` fails
/// validation, naming the offending file.
#[test]
fn wrapper_recognize_cross_compilation_true_fails() {
    let yaml = "type: wrapper\nrecognize:\n  - description: \"Compiler cache\"\n    references:\n      - \"https://example.com/docs\"\n    executables: [\"fake-wrapper\"]\n    cross_compilation: true\n";
    let table = parse_wrapper_table("fake_wrapper.yaml", yaml).unwrap();
    let err = table.validate("fake_wrapper.yaml").unwrap_err();
    assert!(err.to_string().contains("fake_wrapper.yaml"), "{}", err);
    assert!(err.to_string().contains("cross_compilation"), "{}", err);
}

/// A wrapper `options` entry with a glued/prefix pattern (`-j{ }*`) fails
/// validation, naming the offending file and pattern -- the skip loop only
/// compares argv tokens for equality, so the schema must reject anything
/// that implies glued/prefix/eq/colon matching.
#[test]
fn wrapper_option_glued_pattern_fails() {
    let yaml = format!(
        "type: wrapper\n{}options:\n  - match: {{pattern: \"-j{{ }}*\"}}\n",
        MINIMAL_WRAPPER_RECOGNIZE
    );
    let table = parse_wrapper_table("fake_wrapper.yaml", &yaml).unwrap();
    let err = table.validate("fake_wrapper.yaml").unwrap_err();
    assert!(err.to_string().contains("fake_wrapper.yaml"), "{}", err);
    assert!(err.to_string().contains("-j{ }*"), "{}", err);
}

/// A wrapper `options` entry with a prefix-star pattern (`-j*`) fails
/// validation for the same reason.
#[test]
fn wrapper_option_prefix_star_pattern_fails() {
    let yaml =
        format!("type: wrapper\n{}options:\n  - match: {{pattern: \"-j*\"}}\n", MINIMAL_WRAPPER_RECOGNIZE);
    let table = parse_wrapper_table("fake_wrapper.yaml", &yaml).unwrap();
    let err = table.validate("fake_wrapper.yaml").unwrap_err();
    assert!(err.to_string().contains("fake_wrapper.yaml"), "{}", err);
}

/// An exact-token `options` pattern (no `*` or `{ }`) passes validation.
#[test]
fn wrapper_option_exact_token_passes() {
    let yaml = format!(
        "type: wrapper\n{}options:\n  - match: {{pattern: \"-j\", count: 1}}\n",
        MINIMAL_WRAPPER_RECOGNIZE
    );
    let table = parse_wrapper_table("fake_wrapper.yaml", &yaml).unwrap();
    assert!(table.validate("fake_wrapper.yaml").is_ok());
}
