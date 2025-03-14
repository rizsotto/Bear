/*  Copyright (C) 2012-2024 by László Nagy
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

#include "ToolCrayFtnfe.h"
#include "Common.h"

#include <regex>
#include <set>

using namespace cs::semantic;

namespace {

    Arguments create_argument_list(const Execution& execution)
    {
        Arguments input_arguments;
        std::copy(execution.arguments.begin(), execution.arguments.end(), std::back_inserter(input_arguments));
        return input_arguments;
    }

    bool is_preprocessor(const CompilerFlags& flags)
    {
        return std::any_of(flags.begin(), flags.end(), [](const auto& flag) {
            const std::string& candidate = flag.arguments.front();
            static const std::set<std::string_view> NO_COMPILATION_FLAG = { "-E", "-eZ", "-e Z", "-eP", "-e P" };
            return ((flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING) && (NO_COMPILATION_FLAG.find(candidate) != NO_COMPILATION_FLAG.end()))
                || ((flag.type == CompilerFlagType::PREPROCESSOR_MAKE));
        });
        return false;
    }

}

namespace cs::semantic {

    const FlagsByName ToolCrayFtnfe::FLAG_DEFINITION = {
        { "-add-rpath", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-add-rpath-shared", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-add-runpath", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-as-needed", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "--as-needed", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-A", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-b", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::KIND_OF_OUTPUT_OUTPUT } },
        { "-c", { MatchInstruction::EXACTLY, CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING } },
        { "--custom-ld-script=", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED, CompilerFlagType::LINKER } },
        { "-d", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-D", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::PREPROCESSOR } },
        { "-e", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-E", { MatchInstruction::EXACTLY, CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING } },
        { "-f", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-F", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-g", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-gcc-rpath", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-G", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-h", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-I", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::DIRECTORY_SEARCH } },
        { "-J", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::DIRECTORY_SEARCH } },
        { "-K", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-l", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::LINKER } },
        { "-L", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::DIRECTORY_SEARCH_LINKER } },
        { "-m", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-M", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-no-add-rpath", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-no-add-rpath-shared", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-no-add-runpath", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-no-as-needed", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "--no-as-needed", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "--no-custom-ld-script", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-no-gcc-rpath", { MatchInstruction::EXACTLY, CompilerFlagType::LINKER } },
        { "-N", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-O", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED, CompilerFlagType::OTHER } },
        { "-o", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::KIND_OF_OUTPUT_OUTPUT } },
        { "-p", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::DIRECTORY_SEARCH } },
        { "-Q", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::DIRECTORY_SEARCH } },
        { "-r", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::KIND_OF_OUTPUT } },
        { "-R", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-s", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-S", { MatchInstruction::EXACTLY, CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING } },
        { "-T", { MatchInstruction::EXACTLY, CompilerFlagType::KIND_OF_OUTPUT_INFO } },
        { "-target-accel=", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED, CompilerFlagType::OTHER } },
        { "-target-cpu=", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED, CompilerFlagType::OTHER } },
        { "-target-network=", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED, CompilerFlagType::OTHER } },
        { "-U", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::PREPROCESSOR } },
        { "-v", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-V", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-W", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-x", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-Y", { MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::OTHER } },
        { "-openmp", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-noopenmp", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-mp", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-Mnoopenmp", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-qno-openmp", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-dynamic", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-shared", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-static", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-default64", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-VV", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-VVV", { MatchInstruction::EXACTLY, CompilerFlagType::OTHER } },
        { "-cray", { MatchInstruction::PREFIX, CompilerFlagType::OTHER } },
        { "--cray", { MatchInstruction::PREFIX, CompilerFlagType::OTHER } },
    };

    rust::Result<SemanticPtr> ToolCrayFtnfe::recognize(const Execution& execution) const
    {
        if (is_compiler_call(execution.executable)) {
            return compilation_impl(FLAG_DEFINITION, execution, create_argument_list, is_preprocessor);
        }
        return rust::Ok(SemanticPtr());
    }

    bool ToolCrayFtnfe::is_compiler_call(const fs::path& program) const
    {
        static const auto pattern = std::regex(R"(^([^-]*-)*(ftnfe)(-?\w+(\.\d+){0,2})?$)");
        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }
}
