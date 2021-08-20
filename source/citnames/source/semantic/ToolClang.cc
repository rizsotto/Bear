/*  Copyright (C) 2012-2021 by László Nagy
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

#include "ToolClang.h"
#include "ToolGcc.h"

#include <regex>

using namespace cs::semantic;

namespace {

    // https://clang.llvm.org/docs/ClangCommandLineReference.html
    const FlagsByName CLANG_FLAG_DEFINITION = {
            {"--prefix",           {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::DIRECTORY_SEARCH}},
            {"-F",                 {MatchInstruction::PREFIX,                           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-ObjC",              {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
            {"-ObjC++",            {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
            {"-Xarch",             {MatchInstruction::PREFIX_WITH_1_OPT,                CompilerFlagType::OTHER}},
            {"-Xcuda",             {MatchInstruction::PREFIX_WITH_1_OPT,                CompilerFlagType::OTHER}},
            {"-Xopenmp-target",    {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"-Xopenmp-target=",   {MatchInstruction::PREFIX_WITH_1_OPT,                CompilerFlagType::OTHER}},
            {"-Z",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::STATIC_ANALYZER}},
            {"-a",                 {MatchInstruction::PREFIX,                           CompilerFlagType::STATIC_ANALYZER}},
            {"--profile-blocks",   {MatchInstruction::EXACTLY,                          CompilerFlagType::STATIC_ANALYZER}},
            {"-all_load",          {MatchInstruction::EXACTLY,                          CompilerFlagType::STATIC_ANALYZER}},
            {"-allowable_client",  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::STATIC_ANALYZER}},
            {"--analyze",          {MatchInstruction::EXACTLY,                          CompilerFlagType::STATIC_ANALYZER}},
            {"--analyzer-no-default-checks",
                                   {MatchInstruction::EXACTLY,                          CompilerFlagType::STATIC_ANALYZER}},
            {"--analyzer-output",  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,         CompilerFlagType::STATIC_ANALYZER}},
            {"-Xanalyzer",         {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::STATIC_ANALYZER}},
            {"-arch",              {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"-arch_errors_fatal", {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
            {"-arch_only",         {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"-arcmt-migrate-emit-errors",
                                   {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
            {"-arcmt-migrate-report-output",
                                   {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"--autocomplete",     {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::OTHER}},
            {"-bind_at_load",      {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
            {"-bundle",            {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
            {"-bundle_loader",     {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"-client_name",       {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-compatibility_version",
                                   {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"--config",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"-Xclang",            {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
    };

    FlagsByName clang_flags(const FlagsByName &base) {
        FlagsByName flags(base);
        flags.insert(CLANG_FLAG_DEFINITION.begin(), CLANG_FLAG_DEFINITION.end());
        return flags;
    }
}

namespace cs::semantic {

    ToolClang::ToolClang() noexcept
            : flag_definition(clang_flags(ToolGcc::FLAG_DEFINITION))
    { }

    rust::Result<SemanticPtr> ToolClang::recognize(const Execution &execution) const {
        if (is_compiler_call(execution.executable)) {
            return ToolGcc::compilation(ToolClang::flag_definition, execution);
        }
        return rust::Ok(SemanticPtr());
    }

    bool ToolClang::is_compiler_call(const fs::path &program) const {
        static const auto pattern = std::regex(R"(^([^-]*-)*clang(|\+\+)(-?\d+(\.\d+){0,2})?$)");

        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }
}
