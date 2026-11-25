// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result, bail};

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

/// Map a pass-effect string to its Rust `PassEffect` expression.
pub fn effect_to_rust(effect: &str) -> Result<&'static str> {
    match effect {
        "configures_preprocessing" => Ok("PassEffect::Configures(CompilerPass::Preprocessing)"),
        "configures_compiling" => Ok("PassEffect::Configures(CompilerPass::Compiling)"),
        "configures_assembling" => Ok("PassEffect::Configures(CompilerPass::Assembling)"),
        "configures_linking" => Ok("PassEffect::Configures(CompilerPass::Linking)"),
        "stops_at_preprocessing" => Ok("PassEffect::StopsAt(CompilerPass::Preprocessing)"),
        "stops_at_compiling" => Ok("PassEffect::StopsAt(CompilerPass::Compiling)"),
        "stops_at_assembling" => Ok("PassEffect::StopsAt(CompilerPass::Assembling)"),
        "info_and_exit" => Ok("PassEffect::InfoAndExit"),
        "driver_option" => Ok("PassEffect::DriverOption"),
        "pass_through" => Ok("PassEffect::PassThrough"),
        "none" => Ok("PassEffect::None"),
        other => bail!("unknown effect value '{}'", other),
    }
}

/// Map a flag result string to its Rust `RuleKind` expression.
pub fn result_to_rust(result: &str) -> Result<String> {
    match result {
        "output" => Ok("RuleKind::Output".to_string()),
        other => Ok(format!(
            "RuleKind::Effect({})",
            effect_to_rust(other).with_context(|| format!("unknown result value '{other}'"))?
        )),
    }
}
