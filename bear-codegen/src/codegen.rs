// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Result, bail};

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
pub fn result_to_rust(result: &str) -> Result<&'static str> {
    match result {
        "output" => Ok("ArgumentKind::Output"),
        "configures_preprocessing" => {
            Ok("ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))")
        }
        "configures_compiling" => Ok("ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))"),
        "configures_assembling" => {
            Ok("ArgumentKind::Other(PassEffect::Configures(CompilerPass::Assembling))")
        }
        "configures_linking" => Ok("ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))"),
        "stops_at_preprocessing" => {
            Ok("ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing))")
        }
        "stops_at_compiling" => Ok("ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))"),
        "stops_at_assembling" => Ok("ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Assembling))"),
        "info_and_exit" => Ok("ArgumentKind::Other(PassEffect::InfoAndExit)"),
        "driver_option" => Ok("ArgumentKind::Other(PassEffect::DriverOption)"),
        "pass_through" => Ok("ArgumentKind::Other(PassEffect::PassThrough)"),
        "none" => Ok("ArgumentKind::Other(PassEffect::None)"),
        other => bail!("unknown result value '{}'", other),
    }
}
