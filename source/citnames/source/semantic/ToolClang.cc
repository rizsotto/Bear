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
            {"--prefix",           {Instruction(1, Match::BOTH, true),     CompilerFlagType::DIRECTORY_SEARCH}},
            {"-F",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::DIRECTORY_SEARCH}},
            {"-ObjC",              {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-ObjC++",            {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-Xarch",             {Instruction(1, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-Xcuda",             {Instruction(1, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-Xopenmp-target",    {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-Xopenmp-target",    {Instruction(1, Match::EXACT, true),    CompilerFlagType::OTHER}},
            {"-Z",                 {Instruction(1, Match::EXACT, false),   CompilerFlagType::STATIC_ANALYZER}},
            {"-a",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::STATIC_ANALYZER}},
            {"--profile-blocks",   {Instruction(0, Match::EXACT, false),   CompilerFlagType::STATIC_ANALYZER}},
            {"-all_load",          {Instruction(0, Match::EXACT, false),   CompilerFlagType::STATIC_ANALYZER}},
            {"-allowable_client",  {Instruction(1, Match::EXACT, false),   CompilerFlagType::STATIC_ANALYZER}},
            {"--analyze",          {Instruction(0, Match::EXACT, false),   CompilerFlagType::STATIC_ANALYZER}},
            {"--analyzer-no-default-checks",
                                   {Instruction(0, Match::EXACT, false),   CompilerFlagType::STATIC_ANALYZER}},
            {"--analyzer-output",  {Instruction(1, Match::PARTIAL, true),  CompilerFlagType::STATIC_ANALYZER}},
            {"-Xanalyzer",         {Instruction(1, Match::EXACT, true),    CompilerFlagType::STATIC_ANALYZER}},
            {"-arch",              {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-arch_errors_fatal", {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-arch_only",         {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-arcmt-migrate-emit-errors",
                                   {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-arcmt-migrate-report-output",
                                   {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"--autocomplete",     {Instruction(1, Match::EXACT, true),    CompilerFlagType::OTHER}},
            {"-bind_at_load",      {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-bundle",            {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-bundle_loader",     {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-client_name",       {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-compatibility_version",
                                    {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"--config",            {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-Xclang",            {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
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
