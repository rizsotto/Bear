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

#include <spdlog/spdlog.h>
#include <fmt/format.h>

#include "Common.h"

#include <string_view>
#include <tuple>
#include <ranges>

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

    std::tuple<
        Arguments,
        std::vector<fs::path>,
        std::optional<fs::path>>
    split_linker_flags(const CompilerFlags& flags)
    {
        Arguments arguments;
        std::vector<fs::path> inputs;
        std::optional<fs::path> output;

        for (const auto& flag : flags) {
            switch (flag.type) {
            case CompilerFlagType::LINKER_OBJECT_FILE:
            case CompilerFlagType::LINKER_STATIC_LIBRARY:
            case CompilerFlagType::LINKER_SHARED_LIBRARY: {
                // For linking, we consider object files, and libraries as inputs
                auto candidate = fs::path(flag.arguments.front());
                inputs.emplace_back(std::move(candidate));
                break;
            }
            case CompilerFlagType::KIND_OF_OUTPUT_OUTPUT: {
                auto candidate = fs::path(flag.arguments.back());
                output = std::make_optional(std::move(candidate));
                break;
            }
            default: {
                std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(arguments));
                break;
            }
            }
        }
        return std::make_tuple(arguments, inputs, output);
    }

    std::tuple<
        Arguments,
        std::vector<fs::path>,
        std::optional<fs::path>>
    split_archiving_flags(const CompilerFlags& flags)
    {
        Arguments arguments;
        std::vector<fs::path> inputs;
        std::optional<fs::path> output;

        // Find the operation flag first
        std::string operation;
        for (const auto& flag : flags) {
            const auto& arg = flag.arguments.front();
            if (arg == "r" || arg == "q" || arg == "t" || arg == "x" || 
                arg == "d" || arg == "m" || arg == "p") {
                operation = arg;
                break;
            }
        }

        // Then process all flags
        for (const auto& flag : flags) {
            switch (flag.type) {
            case CompilerFlagType::LINKER_STATIC_LIBRARY: {
                auto candidate = fs::path(flag.arguments.front());
                output = std::make_optional(std::move(candidate));
                break;
            }
            case CompilerFlagType::LINKER_OBJECT_FILE: {
                auto candidate = fs::path(flag.arguments.front());
                inputs.emplace_back(std::move(candidate));
                break;
            }
            default: {
                std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(arguments));
                break;
            }
            }
        }

        // Construct final arguments in correct order: operation, modifiers
        Arguments final_arguments;
        std::copy(arguments.begin(), arguments.end(), std::back_inserter(final_arguments));
        return std::make_tuple(final_arguments, inputs, output);
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

rust::Result<SemanticPtr> cs::semantic::linking_impl(const FlagsByName& flags, const Execution& execution)
{
    const auto& parser = Repeat(
        OneOf(
            FlagParser(flags),
            SourceMatcher(),
            ObjectAndLibraryMatcher(),
            EverythingElseFlagMatcher()));

    const Arguments input_arguments(execution.arguments.begin(), execution.arguments.end());
    return parse(parser, input_arguments)
        .and_then<SemanticPtr>([&execution](auto flags) -> rust::Result<SemanticPtr> {
            // Add debug logging for parsed flags
            spdlog::debug("Parsed {} flags for linking", flags.size());
            for (const auto& flag : flags) {
                spdlog::debug("Flag type: {}, arguments: {}", 
                    static_cast<int>(flag.type),
                    fmt::join(flag.arguments.begin(), flag.arguments.end(), " "));
            }

            auto [arguments, inputs, output] = split_linker_flags(flags);
            
            // Add debug logging for split results
            spdlog::debug("Split linker flags:");
            spdlog::debug("Arguments: {}", fmt::join(arguments, " "));
            
            // Convert paths to strings for logging
            std::vector<std::string> input_strings;
            input_strings.reserve(inputs.size());
            std::transform(inputs.begin(), inputs.end(), 
                         std::back_inserter(input_strings),
                         [](const fs::path& p) { return p.string(); });
            spdlog::debug("Input files: {}", fmt::join(input_strings, ", "));
            
            spdlog::debug("Output file: {}", 
                output.has_value() ? output.value().string() : "not specified");

            // Validate: must have input files
            if (inputs.empty()) {
                spdlog::error("No input files found for linking");
                return rust::Err(std::runtime_error("Input files not found for linking."));
            }

            std::list<fs::path> input_files(inputs.begin(), inputs.end());

            SemanticPtr result = std::make_shared<Link>(
                execution.working_dir,
                execution.executable,
                std::move(arguments),
                std::move(input_files),
                std::move(output));
            return rust::Ok(std::move(result));
        });
}

rust::Result<SemanticPtr> cs::semantic::archiving_impl(const FlagsByName& flags, const Execution& execution)
{
    const auto& parser = Repeat(
        OneOf(
            FlagParser(flags),
            SourceMatcher(),
            ObjectAndLibraryMatcher(),
            EverythingElseFlagMatcher()));

    const Arguments input_arguments(execution.arguments.begin(), execution.arguments.end());
    return parse(parser, input_arguments)
        .and_then<SemanticPtr>([&execution](auto flags) -> rust::Result<SemanticPtr> {
            // Find the operation flag
            std::string operation;
            for (const auto& flag : flags) {
                const auto& arg = flag.arguments.front();
                if (arg == "r" || arg == "q" || arg == "t" || arg == "x" || 
                    arg == "d" || arg == "m" || arg == "p") {
                    operation = arg;
                    break;
                }
            }

            if (operation.empty()) {
                return rust::Err(std::runtime_error("No valid ar operation found."));
            }

            auto [arguments, inputs, output] = split_archiving_flags(flags);

            // Validate based on operation requirements
            if (operation == "r" || operation == "q" || operation == "m") {
                if (inputs.empty()) {
                    return rust::Err(std::runtime_error("Input files required for this ar operation."));
                }
            }
            if (!output) {
                return rust::Err(std::runtime_error("Archive file required for ar operation."));
            }

            SemanticPtr result = std::make_shared<Ar>(
                execution.working_dir,
                execution.executable,
                operation,
                std::move(arguments),
                std::list<fs::path>(inputs.begin(), inputs.end()),
                std::move(output));
            return rust::Ok(std::move(result));
        });
}
