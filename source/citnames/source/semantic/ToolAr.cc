/*  Copyright (C) 2012-2023 by László Nagy
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

#include "ToolAr.h"
#include "Parsers.h"

#include <regex>
#include <utility>
#include <set>

using namespace cs::semantic;

namespace {

    bool is_compiler_query(const CompilerFlags& flags)
    {
        // no flag is a no compilation
        if (flags.empty()) {
            return true;
        }
        // otherwise check if this was a version query of a help
        return std::any_of(flags.begin(), flags.end(), [](const auto& flag) {
            return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_INFO);
        });
    }

    std::tuple<
            Arguments,
            std::list<fs::path>,
            std::optional<fs::path>
    > split(const CompilerFlags &flags) {
        Arguments arguments;
        std::list<fs::path> files;
        std::optional<fs::path> output;

        for (const auto &flag : flags) {
            switch (flag.type) {
                case CompilerFlagType::LIBRARY: {
                    if (!output.has_value()) {
                        output = std::make_optional(flag.arguments.front());
                        break;
                    }
                    [[fallthrough]];
                }
                case CompilerFlagType::SOURCE:
                case CompilerFlagType::OBJECT_FILE: {
                    files.emplace_back(flag.arguments.front());
                    break;
                }
                default: {
                    break;
                }
            }
            std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(arguments));
        }
        return std::make_tuple(arguments, files, output);
    }

    auto get_parser(const FlagsByName &flags) {
        return Repeat(
                    OneOf(
                        FlagParser(flags),
                        SourceMatcher(),
                        ObjectFileMatcher(),
                        LibraryMatcher(),
                        EverythingElseFlagMatcher()
                    )
        );
    }
}

namespace cs::semantic {

    const FlagsByName ToolAr::FLAG_DEFINITION = {
        {"--help",             {MatchInstruction::PREFIX,                                    CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"--version",          {MatchInstruction::EXACTLY,                                   CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"-X32_64",            {MatchInstruction::EXACTLY,                                   CompilerFlagType::OTHER}},
        {"--plugin",           {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP,   CompilerFlagType::OTHER}},
        {"--target",           {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP,   CompilerFlagType::OTHER}},
        {"--output",           {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP,   CompilerFlagType::OTHER}},
        {"--record-libdeps",   {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP,   CompilerFlagType::OTHER}},
        {"--thin",             {MatchInstruction::EXACTLY,                                   CompilerFlagType::OTHER}},
    };

    rust::Result<SemanticPtr> ToolAr::recognize(const Execution &execution, const BuildTarget target) const {
        switch (target) {
            case BuildTarget::LINKER: {
                if (is_linker_call(execution.executable)) {
                    return linking(FLAG_DEFINITION, execution);
                }
                break;
            }
            case BuildTarget::COMPILER: {
                break;
            }
        }
        return rust::Ok(SemanticPtr());
    }

    bool ToolAr::is_linker_call(const fs::path& program) {
        static const auto pattern = std::regex(R"(^(ar)\S*$)");
        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }

    rust::Result<SemanticPtr> ToolAr::linking(const FlagsByName &flags, const Execution &execution) {
        return parse(get_parser(flags), execution.arguments)
                .and_then<SemanticPtr>([&execution](auto flags) -> rust::Result<SemanticPtr> {
                    if (is_compiler_query(flags)) {
                        SemanticPtr result = std::make_shared<QueryCompiler>();
                        return rust::Ok(std::move(result));
                    }

                    // arguments contains everything
                    auto[arguments, files, output] = split(flags);

                    SemanticPtr result = std::make_shared<Link>(
                        execution.working_dir,
                        execution.executable,
                        std::move(arguments),
                        std::move(files),
                        std::move(output)
                    );
                    return rust::Ok(std::move(result));
                });
    }
}
