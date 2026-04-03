// SPDX-License-Identifier: GPL-3.0-or-later

use crate::yaml_types::{EnvEntry, EnvMappingYaml, FlagMatch};

/// Compute the flag name length as `FlagPattern::flag()` would return it.
pub fn flag_name_len(m: &FlagMatch) -> usize {
    let pattern = &m.pattern;
    if let Some(flag) = pattern.strip_suffix("{ }*") {
        flag.len()
    } else if let Some(flag) = pattern.strip_suffix("{=}*") {
        flag.len()
    } else if let Some(flag) = pattern.strip_suffix("{:}*") {
        flag.len()
    } else if let Some(flag) = pattern.strip_suffix(":*") {
        flag.len()
    } else if let Some(flag) = pattern.strip_suffix("=*") {
        if m.count.is_some() {
            flag.len() + 1 // "=" is part of the flag name
        } else {
            flag.len()
        }
    } else if let Some(flag) = pattern.strip_suffix('*') {
        flag.len()
    } else {
        pattern.len()
    }
}

/// Parse a pattern string into a FlagPattern Rust expression.
pub fn pattern_to_rust(pattern: &str, count: Option<u32>) -> String {
    if let Some(flag) = pattern.strip_suffix("{ }*") {
        format!("FlagPattern::ExactlyWithGluedOrSep(\"{}\")", flag)
    } else if let Some(flag) = pattern.strip_suffix("{=}*") {
        format!("FlagPattern::ExactlyWithEqOrSep(\"{}\")", flag)
    } else if let Some(flag) = pattern.strip_suffix("{:}*") {
        format!("FlagPattern::ExactlyWithColonOrSep(\"{}\")", flag)
    } else if let Some(flag) = pattern.strip_suffix(":*") {
        format!("FlagPattern::ExactlyWithColon(\"{}\")", flag)
    } else if let Some(flag) = pattern.strip_suffix("=*") {
        if let Some(n) = count {
            // "=*" with count means Prefix where "=" is part of the flag name
            format!("FlagPattern::Prefix(\"{}=\", {})", flag, n)
        } else {
            format!("FlagPattern::ExactlyWithEq(\"{}\")", flag)
        }
    } else if let Some(flag) = pattern.strip_suffix('*') {
        format!("FlagPattern::Prefix(\"{}\", {})", flag, count.unwrap_or(0))
    } else {
        format!("FlagPattern::Exactly(\"{}\", {})", pattern, count.unwrap_or(0))
    }
}

/// Map a result string to its Rust ArgumentKind expression.
pub fn result_to_rust(result: &str) -> &'static str {
    match result {
        "output" => "ArgumentKind::Output",
        "configures_preprocessing" => {
            "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))"
        }
        "configures_compiling" => "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))",
        "configures_assembling" => "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Assembling))",
        "configures_linking" => "ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))",
        "stops_at_preprocessing" => "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing))",
        "stops_at_compiling" => "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))",
        "stops_at_assembling" => "ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Assembling))",
        "info_and_exit" => "ArgumentKind::Other(PassEffect::InfoAndExit)",
        "driver_option" => "ArgumentKind::Other(PassEffect::DriverOption)",
        "pass_through" => "ArgumentKind::Other(PassEffect::PassThrough)",
        "none" => "ArgumentKind::Other(PassEffect::None)",
        other => panic!("Unknown result value: '{}'", other),
    }
}

/// Validate an environment entry at build time.
pub fn validate_env_entry(entry: &EnvEntry, yaml_file: &str) {
    let var_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    assert!(
        var_re.is_match(&entry.variable),
        "{}: invalid environment variable name: '{}'",
        yaml_file,
        entry.variable
    );

    // Validate effect is a known value
    match entry.effect.as_str() {
        "configures_preprocessing"
        | "configures_compiling"
        | "configures_assembling"
        | "configures_linking"
        | "stops_at_preprocessing"
        | "stops_at_compiling"
        | "stops_at_assembling"
        | "info_and_exit"
        | "driver_option"
        | "none" => {}
        other => panic!("{}: unknown effect value: '{}'", yaml_file, other),
    }

    // Validate mapping
    let mapping = &entry.mapping;
    if mapping.flag.is_some() && mapping.expand.is_some() {
        panic!("{}: environment entry '{}' has both 'flag' and 'expand'", yaml_file, entry.variable);
    }
    if mapping.flag.is_none() && mapping.expand.is_none() && entry.effect != "none" {
        panic!(
            "{}: environment entry '{}' has neither 'flag' nor 'expand' (and effect is not 'none')",
            yaml_file, entry.variable
        );
    }

    // Validate separator
    match mapping.separator.as_str() {
        "path" | "space" | ";" => {}
        other => {
            panic!("{}: environment entry '{}' has unknown separator: '{}'", yaml_file, entry.variable, other)
        }
    }

    // Validate expand position
    if let Some(ref expand) = mapping.expand {
        match expand.as_str() {
            "prepend" | "append" => {}
            other => panic!(
                "{}: environment entry '{}' has unknown expand position: '{}'",
                yaml_file, entry.variable, other
            ),
        }
    }
}

/// Map an environment entry's mapping to a Rust EnvMapping expression.
pub fn env_mapping_to_rust(mapping: &EnvMappingYaml) -> String {
    if let Some(ref flag) = mapping.flag {
        let sep = match mapping.separator.as_str() {
            "path" => "EnvSeparator::Path".to_string(),
            ";" => "EnvSeparator::Fixed(\";\")".to_string(),
            other => format!("EnvSeparator::Fixed(\"{}\")", other),
        };
        format!("EnvMapping::Flag {{ flag: \"{}\", separator: {} }}", flag, sep)
    } else if let Some(ref expand) = mapping.expand {
        let pos = match expand.as_str() {
            "prepend" => "EnvPosition::Prepend",
            "append" => "EnvPosition::Append",
            _ => unreachable!(),
        };
        format!("EnvMapping::Expand {{ position: {} }}", pos)
    } else {
        unreachable!()
    }
}
