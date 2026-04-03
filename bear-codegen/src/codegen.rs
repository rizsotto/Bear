// SPDX-License-Identifier: GPL-3.0-or-later

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
