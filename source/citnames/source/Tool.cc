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

#include <filesystem>
#include <iterator>
#include <regex>
#include <set>
#include <utility>

#include <fmt/format.h>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace {

    enum class CompilerFlagType {
        KIND_OF_OUTPUT,
        KIND_OF_OUTPUT_INFO,
        KIND_OF_OUTPUT_OUTPUT,
        PREPROCESSOR,
        PREPROCESSOR_MAKE,
        LINKER,
        LINKER_OBJECT_FILE,
        DIRECTORY_SEARCH,
        SOURCE,
        OTHER,
    };

    using Arguemnts = std::list<std::string>;

    struct CompilerFlag {
        Arguemnts arguments;
        CompilerFlagType type;
    };

    using CompilerFlags = std::list<CompilerFlag>;

    namespace parser {

        struct Input {
            Arguemnts::const_iterator begin;
            Arguemnts::const_iterator end;
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

        // Represents compiler flag definition.
        //
        // Can match by exact name definition, or by regex like pattern matching.
        // Polymorphic behaviour achieved not by inheritance (because that would
        // stop us using static array of these object).
        class FlagDefinition {
        private:
            const char* name_;
            const char* pattern_;
            size_t count_;
            CompilerFlagType type_;

            constexpr FlagDefinition(const char* const name, const char* const pattern, const size_t count, const CompilerFlagType type)
                    : name_(name)
                    , pattern_(pattern)
                    , count_(count)
                    , type_(type)
            { }

            [[nodiscard]]
            std::optional<std::tuple<size_t, CompilerFlagType>> match_by_name(const std::string& arg) const {
                return (arg == name_) ? std::make_optional(std::make_pair(count_, type_)) : std::nullopt;
            }

            [[nodiscard]]
            std::optional<std::tuple<size_t, CompilerFlagType>> match_by_pattern(const std::string& arg) const {
                std::regex re(pattern_);
                std::cmatch m;
                return (std::regex_match(arg.c_str(), m, re)) ? std::make_optional(std::make_pair(count_, type_)) : std::nullopt;
            }

        public:
            constexpr static FlagDefinition by_name(const char* const name, const size_t count, const CompilerFlagType type) {
                return FlagDefinition { name, nullptr, count, type };
            }

            constexpr static FlagDefinition by_pattern(const char* const pattern, const size_t count, const CompilerFlagType type) {
                return FlagDefinition { nullptr, pattern, count, type };
            }

            [[nodiscard]]
            std::optional<std::tuple<size_t, CompilerFlagType>> match(const std::string& arg) const {
                return (name_ != nullptr) ? match_by_name(arg) : match_by_pattern(arg);
            }
        };

        // Generic flag matcher, which takes a list of flag definition and tries to match it.
        class FlagMatcher {
        protected:
            FlagMatcher(const FlagDefinition *const begin, const FlagDefinition *const end)
                    : begin_(begin)
                    , end_(end)
            { }

        public:
            [[nodiscard]]
            rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const
            {
                const std::string front = *input.begin;
                for (auto it = begin_; it != end_; ++it) {
                    if (auto match = it->match(front); match) {
                        const auto& [count, type] = match.value();

                        auto begin = input.begin;
                        auto end = std::next(begin, count + 1);

                        CompilerFlag compiler_flag = { Arguemnts(begin, end), type };
                        Input remainder = { end, input.end };
                        return rust::Ok(std::make_pair(compiler_flag, remainder));
                    }
                }
                return rust::Err(input);
            }

        private:
            const FlagDefinition *const begin_;
            const FlagDefinition *const end_;
        };

        class KindOfOutputFlagMatcher : public FlagMatcher {
            constexpr static const FlagDefinition FLAGS[] = {
                    FlagDefinition::by_name("-x", 1, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-c", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-S", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-E", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-o", 1, CompilerFlagType::KIND_OF_OUTPUT_OUTPUT),
                    FlagDefinition::by_name("-dumpbase", 1, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-dumpbase-ext", 1, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-dumpdir", 1, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-v", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-###", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("--help", 0, CompilerFlagType::KIND_OF_OUTPUT_INFO),
                    FlagDefinition::by_name("--target-help", 0, CompilerFlagType::KIND_OF_OUTPUT_INFO),
                    FlagDefinition::by_pattern("--help=(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT_INFO),
                    FlagDefinition::by_name("--version", 0, CompilerFlagType::KIND_OF_OUTPUT_INFO),
                    FlagDefinition::by_name("-pass-exit-codes", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-pipe", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("-specs=(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-wrapper", 1, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("-ffile-prefix-map=(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-fplugin", 1, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("-fplugin=(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_name("-fplugin-arg-name-key", 1, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("-fplugin-arg-name-key=(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("-fdump-ada-spec(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("-fada-spec-parent=(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("-fdump-go-sepc=(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
                    FlagDefinition::by_pattern("@(.*)", 0, CompilerFlagType::KIND_OF_OUTPUT),
            };

        public:
            KindOfOutputFlagMatcher()
                    : FlagMatcher(FLAGS, FLAGS + (sizeof(FLAGS) / sizeof(FlagDefinition)))
            { }
        };

        class PreprocessorFlagMatcher : public FlagMatcher {
            constexpr static const FlagDefinition FLAGS[] = {
                    FlagDefinition::by_name("-A", 1, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_pattern("-A(.*)", 0, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_name("-D", 1, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_pattern("-D(.*)", 0, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_name("-U", 1, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_pattern("-U(.*)", 0, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_name("-include", 1, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_name("-imacros", 1, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_name("-undef", 0, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_name("-pthread", 0, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_pattern("-M(|M|G|P|D|MD)", 0, CompilerFlagType::PREPROCESSOR_MAKE),
                    FlagDefinition::by_pattern("-M(F|T|Q)", 1, CompilerFlagType::PREPROCESSOR_MAKE),
                    FlagDefinition::by_pattern("-(C|CC|P|traditional|traditional-cpp|trigraphs|remap|H)", 0, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_pattern("-d[MDNIU]", 0, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_name("-Xpreprocessor", 1, CompilerFlagType::PREPROCESSOR),
                    FlagDefinition::by_pattern("-Wp,(.*)", 0, CompilerFlagType::PREPROCESSOR),
            };

        public:
            PreprocessorFlagMatcher()
                    : FlagMatcher(FLAGS, FLAGS + (sizeof(FLAGS) / sizeof(FlagDefinition)))
            { }
        };

        class DirectorySearchFlagMatcher : public FlagMatcher {
            constexpr static const FlagDefinition FLAGS[] = {
                    FlagDefinition::by_name("-I", 1, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_name("-I(.*)", 0, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_name("-iplugindir", 1, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_pattern("-iplugindir=(.*)", 0, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_pattern("-i(.*)", 1, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_pattern(R"(-no(stdinc|stdinc\+\+|-canonical-prefixes|-sysroot-suffix))", 0, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_pattern("-L(.*)", 0, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_pattern("-B(.*)", 0, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_name("--sysroot", 1, CompilerFlagType::DIRECTORY_SEARCH),
                    FlagDefinition::by_pattern("--sysroot=(.*)", 0, CompilerFlagType::DIRECTORY_SEARCH),
            };

        public:
            DirectorySearchFlagMatcher()
                    : FlagMatcher(FLAGS, FLAGS + (sizeof(FLAGS) / sizeof(FlagDefinition)))
            { }
        };

        class LinkerFlagMatcher : public FlagMatcher {
            constexpr static const FlagDefinition FLAGS[] = {
                    FlagDefinition::by_pattern("-flinker-output=(.*)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-fuse-ld=(.*)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-l(.*)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_name("-l", 1, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-no(startfiles|defaultlibs|libc|stdlib)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_name("-e", 1, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-entry=(.*)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-(pie|no-pie|static-pie)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-(r|rdynamic|s|symbolic)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-(static|shared)(|-libgcc)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern(R"(-static-lib(asan|tsan|lsan|ubsan|stdc\+\+))", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_name("-T", 1, CompilerFlagType::LINKER),
                    FlagDefinition::by_name("-Xlinker", 1, CompilerFlagType::LINKER),
                    FlagDefinition::by_pattern("-Wl,(.*)", 0, CompilerFlagType::LINKER),
                    FlagDefinition::by_name("-u", 1, CompilerFlagType::LINKER),
                    FlagDefinition::by_name("-z", 1, CompilerFlagType::LINKER),
            };

        public:
            LinkerFlagMatcher()
                    : FlagMatcher(FLAGS, FLAGS + (sizeof(FLAGS) / sizeof(FlagDefinition)))
            { }
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

                        CompilerFlag compiler_flag = { Arguemnts(begin, end), CompilerFlagType::SOURCE };
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

        class EverythingElseFlagMatcher : public FlagMatcher {
            constexpr static const FlagDefinition FLAGS[] = {
                    FlagDefinition::by_name("-Xassembler", 1, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern("-Wa,(.*)", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_name("-ansi", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_name("-aux-info", 1, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern("-std=(.*)", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern("-[Og](.*)", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern("-[fmpW](.+)", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern("-(no|tno|save|d|Wa,)(.+)", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern("-[EQXY](.+)", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern("--(.+)", 0, CompilerFlagType::OTHER),
                    FlagDefinition::by_pattern(".+", 0, CompilerFlagType::LINKER_OBJECT_FILE)
            };

        public:
            EverythingElseFlagMatcher()
                    : FlagMatcher(FLAGS, FLAGS + (sizeof(FLAGS) / sizeof(FlagDefinition)))
            { }
        };
    }

    CompilerFlags flags_from_environment(const std::map<std::string, std::string> &environment) {
        CompilerFlags flags;
        for (auto env : { "CPATH", "C_INCLUDE_PATH", "CPLUS_INCLUDE_PATH" }) {
            if (auto it = environment.find(env); it != environment.end()) {
                for (const auto& path : sys::path::split(it->second)) {
                    auto directory = (path.empty()) ? "." : path;
                    CompilerFlag flag = { {"-I", directory }, CompilerFlagType::DIRECTORY_SEARCH };
                    flags.emplace_back(flag);
                }
            }
        }
        if (auto it = environment.find("OBJC_INCLUDE_PATH"); it != environment.end()) {
            for (const auto& path : sys::path::split(it->second)) {
                auto directory = (path.empty()) ? "." : path;
                CompilerFlag flag = { {"-isystem", directory }, CompilerFlagType::DIRECTORY_SEARCH };
                flags.emplace_back(flag);
            }
        }
        return flags;
    }

    rust::Result<CompilerFlags> parse(const report::Command &command)
    {
        auto parser = parser::Any(
                parser::KindOfOutputFlagMatcher(),
                parser::PreprocessorFlagMatcher(),
                parser::DirectorySearchFlagMatcher(),
                parser::LinkerFlagMatcher(),
                parser::SourceMatcher(),
                parser::EverythingElseFlagMatcher());

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
        if (auto extra = flags_from_environment(command.environment); !extra.empty()) {
            std::copy(extra.begin(), extra.end(), std::back_inserter(flags));
        }
        return rust::Ok(flags);
    }

    bool runs_compilation_pass(const CompilerFlags& flags)
    {
//        // for Clang
//        constexpr static const char* NO_COMPILATION_FLAG[] {
//            "-cc1",
//            "-cc1as"
//        };
//        constexpr static size_t NO_COMPILATION_FLAG_SIZE = sizeof(NO_COMPILATION_FLAG) / sizeof(const char*);

        if (flags.empty() || std::find_if(flags.begin(), flags.end(), [](auto flag) { return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_INFO); }) != flags.end()) {
            return false;
        }
//        for (const auto& flag : flags) {
//            if ((flag.type == CompilerFlagType::KIND_OF_OUTPUT) && (flag.arguments.size() == 1)) {
//                std::string candidate = flag.arguments.front();
//
//                const auto begin = NO_COMPILATION_FLAG;
//                const auto end = NO_COMPILATION_FLAG + NO_COMPILATION_FLAG_SIZE;
//                return std::find_if(begin, end, [&candidate](const char *it) { return candidate == it; }) == end;
//            }
//        }
        return true;
    }

    std::optional<fs::path> source_file(const CompilerFlag& flag, const fs::path& working_dir)
    {
        if (flag.type == CompilerFlagType::SOURCE) {
            auto source = fs::path(flag.arguments.front());
            // TODO: check if source file is exists
            return (source.is_absolute())
                    ? std::make_optional(source)
                    : std::make_optional(working_dir / source);
        }
        return std::optional<fs::path>();
    }

    std::list<fs::path> source_files(const CompilerFlags& flags, const fs::path& working_dir)
    {
        std::list<fs::path> result;
        for (const auto& flag : flags) {
            if (auto source = source_file(flag, working_dir); source) {
                result.push_back(source.value());
            }
        }
        return result;
    }

    std::optional<fs::path> output_file(const CompilerFlag& flag, const fs::path& working_dir)
    {
        if ((flag.type == CompilerFlagType::KIND_OF_OUTPUT_OUTPUT) && (flag.arguments.size() == 2)) {
            auto output = fs::path(flag.arguments.back());
            return (output.is_absolute())
                   ? std::make_optional(output)
                   : std::make_optional(working_dir / output);
        }
        return std::optional<fs::path>();
    }

    std::optional<fs::path> output_files(const CompilerFlags& flags, const fs::path& working_dir)
    {
        std::list<fs::path> result;
        for (const auto& flag : flags) {
            if (auto output = output_file(flag, working_dir); output) {
                return output;
            }
        }
        return std::optional<fs::path>();
    }

    Arguemnts filter_arguments(const CompilerFlags& flags)
    {
        Arguemnts result;
        for (const auto& flag : flags) {
            if (flag.type != CompilerFlagType::LINKER || flag.type == CompilerFlagType::PREPROCESSOR_MAKE) {
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
                R"(^([^-]*-)*[g]?fortran(-?\d+(\.\d+){0,2})?$)",
        };
        auto pattern =
                fmt::format("({})", fmt::join(patterns.begin(), patterns.end(), "|"));

        auto basename = fs::path(program).filename();
        std::cmatch m;
        return std::regex_match(basename.c_str(), m, std::regex(pattern));
    }
}

namespace cs {

    GnuCompilerCollection::GnuCompilerCollection(std::list<std::string> paths)
            : Tool()
            , paths(std::move(paths))
    { }

    rust::Result<output::Entries> GnuCompilerCollection::recognize(const report::Command &command) const {
        const bool match_compiler_name =
                (std::find(paths.begin(), paths.end(), command.program) != paths.end())
                || match_gcc_name(command.program);

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
