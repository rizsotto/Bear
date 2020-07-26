/*  Copyright (C) 2012-2020 by László Nagy
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

#include "Tool.h"
#include "libresult/Result.h"
#include "libsys/Path.h"

#include <iterator>
#include <regex>
#include <utility>

#include <fmt/format.h>
#include <spdlog/spdlog.h>


namespace {

    enum class CompilerFlagType {
        // https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html
        KIND_OF_OUTPUT,
        LANGUAGE_DIALECT,
        DIAGNOSTIC,
        WARNING,
        ANALYZER,
        OPTIMIZATION,
        INSTRUMENTATION,
        PREPROCESSOR,
        ASSEMBLER,
        LINKER,
        DIRECTORY_SEARCH,
        CODE_GENERATION,
        DEVELOPER,
        MACHINE_DEPENDENT,
        // for other types
        SOURCE,
        UNKNOWN,
    };

    struct CompilerFlag {
        std::list<std::string> arguments;
        CompilerFlagType type;
    };

    using CompilerFlags = std::list<CompilerFlag>;

    namespace parser {

        using Arguemnts = std::list<std::string>;

        struct Input {
            Arguemnts::const_iterator begin;
            Arguemnts::const_iterator end;
        };

        struct Parser {
            [[nodiscard]]
            rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const
            {
                return rust::Err(input);
            }
        };

        template <typename ... Parsers>
        struct Any {
            using container_type = typename std::tuple<Parsers...>;
            container_type const parsers;

            explicit constexpr Any(Parsers const& ...p)
            : parsers(p...)
            { }

            [[nodiscard]]
            rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const
            {
                rust::Result<std::pair<CompilerFlag, Input>, Input> result = rust::Err(input);
                const bool valid =
                        std::apply([&input, &result](auto &&... parser) {
                            bool success = ((
                                    result = parser.parse(input),
                                    result.is_ok())
                                    || ...);
                            return success;
                        }, parsers);

                return (valid) ? result : rust::Err(input);
            }
        };

        struct FlagMatcher {

            struct FlagDefinition {
                const char* name;
                size_t count;
                CompilerFlagType type;
            };

            constexpr static const FlagDefinition FLAGS[] = {
                    { "-x", 1, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-c", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-S", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-E", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-o", 1, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-dumpbase", 1, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-dumpbase-ext", 1, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-dumpdir", 1, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-v", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-###", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "--help", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "--target-help", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "--version", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-pass-exit-codes", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-pipe", 0, CompilerFlagType::KIND_OF_OUTPUT },
                    { "-wrapper", 1, CompilerFlagType::KIND_OF_OUTPUT },
            };

            [[nodiscard]]
            rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const
            {
                const std::string front = *input.begin;
                for (auto flag : FLAGS) {
                    if (front == flag.name) {

                        auto begin = input.begin;
                        auto end = std::next(begin, flag.count + 1);

                        CompilerFlag compiler_flag = { std::list(begin, end), flag.type };
                        Input remainder = { end, input.end };
                        return rust::Ok(std::make_pair(compiler_flag, remainder));
                    }
                }
                return rust::Err(input);
            }
        };

        struct SourceMatcher {
            constexpr static const char* EXTENSIONS[] {
                    // header files
                    ".h", ".hh", ".H", ".hp", ".hxx", ".hpp", ".HPP", ".h++", ".tcc",
                    // C
                    ".c", ".C",
                    // C++
                    ".cc", ".CC", ".c++", ".C++", ".cxx", ".cpp", ".cp",
                    // ObjectiveC
                    ".m", ".mi", ".mm", ".M", ".mii",
                    // Preprocessed
                    ".i", ".ii",
                    // Assembly
                    ".s", ".S", ".sx", ".asm",
                    // Fortran
                    ".f", ".for", ".ftn",
                    ".F", ".FOR", ".fpp", ".FPP", ".FTN",
                    ".f90", ".f95", ".f03", ".f08",
                    ".F90", ".F95", ".F03", ".F08",
                    // go
                    ".go",
                    // brig
                    ".brig",
                    // D
                    ".d", ".di", ".dd",
                    // Ada
                    ".ads", ".abd"
            };

            [[nodiscard]]
            rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const {
                const std::string candidate = take_extension(*input.begin);
                for (auto extension : EXTENSIONS) {
                    if (candidate == extension) {
                        auto begin = input.begin;
                        auto end = std::next(begin, 1);

                        CompilerFlag compiler_flag = { std::list(begin, end), CompilerFlagType::SOURCE };
                        Input remainder = { end, input.end };
                        return rust::Ok(std::make_pair(compiler_flag, remainder));
                    }
                }
                return rust::Err(input);
            }

            [[nodiscard]]
            static std::string take_extension(const std::string& file) {
                auto pos = file.rfind('.');
                return (pos == std::string::npos) ? file : file.substr(pos);
            }
        };
    }

    rust::Result<CompilerFlags> parse(const report::Execution::Command &command)
    {
        auto parser = parser::Any(parser::FlagMatcher(), parser::SourceMatcher());

        CompilerFlags flags;
        for (parser::Input input { std::next(command.arguments.begin()), command.arguments.end() };
            input.begin != input.end;) {

            auto result = parser.parse(input);
            if (result.is_err()) {
                return result
                        .map<CompilerFlags>([](auto ignore) {
                            return CompilerFlags();
                        })
                        .map_err<std::runtime_error>([](auto remainder) {
                            return std::runtime_error(
                                    fmt::format("Failed to recognize: {}",
                                            fmt::join(remainder.begin, remainder.end, ", ")));
                        });
            } else {
                result.on_success([&flags, &input](auto tuple) {
                    const auto& [flag, remainder] = tuple;
                    flags.push_back(flag);
                    input = remainder;
                });
            }
        }
        for (auto env : { "CPATH", "C_INCLUDE_PATH", "CPLUS_INCLUDE_PATH" }) {
            if (auto it = command.environment.find(env); it != command.environment.end()) {
                for (const auto& path : sys::path::split(it->second)) {
                    auto directory = (path.empty()) ? "." : path;
                    CompilerFlag flag = { {"-I", directory }, CompilerFlagType::DIRECTORY_SEARCH };
                    flags.emplace_back(flag);
                }
            }
        }
        if (auto it = command.environment.find("OBJC_INCLUDE_PATH"); it != command.environment.end()) {
            for (const auto& path : sys::path::split(it->second)) {
                auto directory = (path.empty()) ? "." : path;
                CompilerFlag flag = { {"-isystem", directory }, CompilerFlagType::DIRECTORY_SEARCH };
                flags.emplace_back(flag);
            }
        }
        return rust::Ok(flags);
    }

    bool runs_compilation_pass(const CompilerFlags& flags)
    {
        constexpr static const char* NO_COMPILATION_FLAG[] {
            "--help",
            "--version",
            "-cc1",
            "-cc1as"
        };
        constexpr static size_t NO_COMPILATION_FLAG_SIZE = sizeof(NO_COMPILATION_FLAG) / sizeof(const char*);

        for (const auto& flag : flags) {
            if ((flag.type == CompilerFlagType::KIND_OF_OUTPUT) && (flag.arguments.size() == 1)) {
                std::string candidate = flag.arguments.front();

                const auto begin = NO_COMPILATION_FLAG;
                const auto end = NO_COMPILATION_FLAG + NO_COMPILATION_FLAG_SIZE;
                return std::find_if(begin, end, [&candidate](const char *it) { return candidate == it; }) == end;
            }
        }
        return true;
    }

    std::optional<std::string> source_file(const CompilerFlag& flag, const std::string& working_dir)
    {
        // TODO: check if source file is exists
        return (flag.type != CompilerFlagType::SOURCE)
               ? std::optional<std::string>()
               : (sys::path::is_absolute(flag.arguments.front()))
                       ? std::make_optional(flag.arguments.front())
                       : std::make_optional(sys::path::concat(working_dir, flag.arguments.front()));
    }

    std::list<std::string> source_files(const CompilerFlags& flags, const std::string& working_dir)
    {
        std::list<std::string> result;
        for (const auto& flag : flags) {
            if (auto source = source_file(flag, working_dir); source) {
                result.push_back(source.value());
            }
        }
        return result;
    }

    std::optional<std::string> output_file(const CompilerFlag& flag, const std::string& working_dir)
    {
        if ((flag.type == CompilerFlagType::KIND_OF_OUTPUT) && (flag.arguments.size() == 2) && (flag.arguments.front() == "-o")) {
            auto output = flag.arguments.back();
            return (sys::path::is_absolute(output))
                   ? std::make_optional(output)
                   : std::make_optional(sys::path::concat(working_dir, output));
        }
        return std::optional<std::string>();
    }

    std::optional<std::string> output_files(const CompilerFlags& flags, const std::string& working_dir)
    {
        std::list<std::string> result;
        for (const auto& flag : flags) {
            if (auto output = output_file(flag, working_dir); output) {
                return output;
            }
        }
        return std::optional<std::string>();
    }

    std::list<std::string> filter_arguments(const CompilerFlags& flags)
    {
        std::list<std::string> result;
        for (const auto& flag : flags) {
            if (flag.type != CompilerFlagType::LINKER) {
                std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(result));
            }
        }
        return result;
    }

    bool match_gcc_name(const std::string& program)
    {
        std::list<std::string> patterns = {
                R"(^(cc|c\+\+|cxx|CC)$)",
                R"(^([^-]*-)*[mg]cc(-?\d+(\.\d+){0,2})?$)",
                R"(^([^-]*-)*[mg]\+\+(-?\d+(\.\d+){0,2})?$)",
        };
        auto pattern =
                fmt::format("({})", fmt::join(patterns.begin(), patterns.end(), "|"));

        std::cmatch m;
        return std::regex_match(sys::path::basename(program).c_str(), m, std::regex(pattern));
    }
}

namespace cs {

    GnuCompilerCollection::GnuCompilerCollection(std::optional<std::string> exact_name)
            : Tool()
            , exact_name(std::move(exact_name))
    { }

    rust::Result<output::Entries> GnuCompilerCollection::recognize(const report::Execution::Command &command) const {
        const bool match_compiler_name =
                (exact_name && (exact_name.value() == command.program)) || match_gcc_name(command.program);

        if (!match_compiler_name) {
            return rust::Err(std::runtime_error("Not recognized program name."));
        }

        return parse(command)
                .map<output::Entries>([&command](auto flags) {
                    if (!runs_compilation_pass(flags)) {
                        spdlog::debug("Compiler call does not run compilation pass.");
                        return output::Entries();
                    }
                    auto output = output_files(flags, command.working_dir);
                    auto sources = source_files(flags, command.working_dir);
                    if (sources.empty()) {
                        spdlog::debug("Source files not found for compilation.");
                        return output::Entries();
                    }

                    output::Entries result;
                    for (const auto &source : sources) {
                        auto arguments = filter_arguments(flags);
                        arguments.push_front(command.program);
                        cs::output::Entry entry = {source, command.working_dir, output, arguments};
                        result.emplace_back(std::move(entry));
                    }
                    return result;
                });
    }
}
