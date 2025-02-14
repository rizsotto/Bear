#include "ToolLinker.h"
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
    const std::regex LINKER_PATTERN(R"(^(ld|ld\.gold|ld\.lld|gold|lld)$)");
}

namespace cs::semantic {

    const FlagsByName ToolLinker::FLAG_DEFINITION = {
            // Output flags
            {"-o", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::KIND_OF_OUTPUT_OUTPUT}},
            {"--output", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::KIND_OF_OUTPUT_OUTPUT}},
            
            // Library flags
            {"-l", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"-L", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::DIRECTORY_SEARCH_LINKER}},
            {"--library", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"--library-path", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::DIRECTORY_SEARCH_LINKER}},
            
            // Runtime path flags
            {"-rpath", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"--rpath", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            
            // Shared library flags
            {"-soname", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"--soname", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            
            // Version script flags
            {"-version-script", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"--version-script", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            
            // Dynamic linker flags
            {"-dynamic-linker", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"--dynamic-linker", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            
            // Other common linker flags
            {"-z", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"-m", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"--hash-style", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::LINKER}},
            {"--build-id", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},
            {"--eh-frame-hdr", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},
            {"--as-needed", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},
            {"--no-as-needed", {MatchInstruction::EXACTLY, CompilerFlagType::LINKER}},
    };

    bool ToolLinker::is_linker_call(const fs::path& program) const {
        const auto name = program.filename().string();
        return std::regex_match(name, LINKER_PATTERN);
    }

    rust::Result<SemanticPtr> ToolLinker::recognize(const Execution &execution) const {
        if (is_linker_call(execution.executable)) {
            return linking(execution);
        }
        return rust::Ok(SemanticPtr());
    }

    rust::Result<SemanticPtr> ToolLinker::linking(const Execution &execution) const {
        return linking(FLAG_DEFINITION, execution);
    }

    rust::Result<SemanticPtr> ToolLinker::linking(const FlagsByName &flags, const Execution &execution) {
        return linking_impl(flags, execution);
    }
} 