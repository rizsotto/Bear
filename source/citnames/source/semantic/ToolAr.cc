#include "ToolAr.h"
#include "Semantic.h"
#include "Common.h"
#include "libshell/Command.h"
#include "Parsers.h"

#include <algorithm>
#include <filesystem>
#include <regex>
#include <map>
#include <iostream>

namespace {
    const std::regex AR_PATTERN(R"(^(ar|llvm-ar)$)");
}

namespace cs::semantic {

    const FlagsByName ToolAr::FLAG_DEFINITION = {
            // Main operation flags - all operations should be part of arguments
            {"r", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Replace or insert files
            {"q", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Quick append
            {"t", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // List contents
            {"x", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Extract
            {"d", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Delete
            {"m", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Move
            {"p", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Print
            
            // Operation modifiers
            {"a", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Put files after [member-name]
            {"b", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Put files before [member-name]
            {"i", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Same as [b]
            {"D", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Use zero for timestamps and uids/gids
            {"U", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Use actual timestamps and uids/gids
            {"N", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Use instance [count] of name
            {"f", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Truncate inserted file names
            {"P", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Use full path names when matching
            {"o", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Preserve original dates
            {"O", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Display offsets of files
            {"u", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Only replace newer files
            
            // Generic modifiers
            {"c", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Don't warn if library had to be created
            {"s", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Create an archive index
            {"S", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Don't build a symbol table
            {"T", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Deprecated, use --thin instead
            {"v", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Be verbose
            {"V", {MatchInstruction::EXACTLY, CompilerFlagType::KIND_OF_OUTPUT_INFO}},  // Display version number
            
            // Long options
            {"--thin", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},  // Make a thin archive
            {"--plugin", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},  // Load specified plugin
            {"--target", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},  // Specify target object format
            {"--output", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::KIND_OF_OUTPUT_OUTPUT}},  // Specify output directory
            {"--record-libdeps", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},  // Specify library dependencies
    };

    bool ToolAr::is_ar_call(const fs::path& program) const {
        const auto name = program.filename().string();
        return std::regex_match(name, AR_PATTERN);
    }

    rust::Result<SemanticPtr> ToolAr::recognize(const Execution &execution) const {
        if (is_ar_call(execution.executable)) {
            return archiving(execution);
        }
        return rust::Ok(SemanticPtr());
    }

    rust::Result<SemanticPtr> ToolAr::archiving(const Execution &execution) const {
        return archiving(FLAG_DEFINITION, execution);
    }

    rust::Result<SemanticPtr> ToolAr::archiving(const FlagsByName &flags, const Execution &execution) {
        return archiving_impl(flags, execution);
    }
} 