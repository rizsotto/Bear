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

#include "Common.h"

#include <string_view>
#include <tuple>

using namespace cs::semantic;

namespace {

    std::tuple<
        Arguments,
        std::vector<fs::path>,
        std::optional<fs::path>>
    split(const CompilerFlags& flags)
    {
        Arguments arguments;
        std::vector<fs::path> sources;
        std::optional<fs::path> output;

        for (const auto& flag : flags) {
            switch (flag.type) {
            case CompilerFlagType::SOURCE: {
                auto candidate = fs::path(flag.arguments.front());
                sources.emplace_back(std::move(candidate));
                break;
            }
            case CompilerFlagType::KIND_OF_OUTPUT_OUTPUT: {
                auto candidate = fs::path(flag.arguments.back());
                output = std::make_optional(std::move(candidate));
                break;
            }
            case CompilerFlagType::LINKER:
            case CompilerFlagType::PREPROCESSOR_MAKE:
            case CompilerFlagType::DIRECTORY_SEARCH_LINKER:
                break;
            default: {
                std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(arguments));
                break;
            }
            }
        }
        return std::make_tuple(arguments, sources, output);
    }

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

    bool linking(const CompilerFlags& flags)
    {
        return std::none_of(flags.begin(), flags.end(), [](auto flag) {
            return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING);
        });
    }
}

rust::Result<SemanticPtr> cs::semantic::compilation_impl(const FlagsByName& flags, const Execution& execution,
    std::function<Arguments(const Execution&)> create_argument_list_func,
    std::function<bool(const CompilerFlags&)> is_preprocessor_func)
{
    const auto& parser = Repeat(
        OneOf(
            FlagParser(flags),
            SourceMatcher(),
            EverythingElseFlagMatcher()));

    const Arguments& input_arguments = create_argument_list_func(execution);
    return parse(parser, input_arguments)
        .and_then<SemanticPtr>([&execution, is_preprocessor_func](auto flags) -> rust::Result<SemanticPtr> {
            if (is_compiler_query(flags)) {
                SemanticPtr result = std::make_shared<QueryCompiler>();
                return rust::Ok(std::move(result));
            }
            if (is_preprocessor_func(flags)) {
                SemanticPtr result = std::make_shared<Preprocess>();
                return rust::Ok(std::move(result));
            }

            auto [arguments, sources, output] = split(flags);
            // Validate: must have source files.
            if (sources.empty()) {
                return rust::Err(std::runtime_error("Source files not found for compilation."));
            }
            // TODO: introduce semantic type for linking
            if (linking(flags)) {
                arguments.insert(arguments.begin(), "-c");
            }

            SemanticPtr result = std::make_shared<Compile>(
                execution.working_dir,
                execution.executable,
                std::move(arguments),
                std::move(sources),
                std::move(output));
            return rust::Ok(std::move(result));
        });
}
