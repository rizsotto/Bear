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

#include <cstdint>
#include <filesystem>
#include <iterator>
#include <regex>
#include <set>
#include <utility>
#include <functional>

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

namespace fs = std::filesystem;

// Common type definitions...
namespace {

    // Represents command line arguments.
    using Arguments = std::list<std::string>;

    // Represents a segment of a whole command line arguments,
    // which belongs together.
    struct ArgumentsSegment {
        Arguments::const_iterator begin;
        Arguments::const_iterator end;
    };

//    enum class Category {
//        CONTROL,
//        INPUT,
//        OUTPUT,
//        DEBUG,
//        OPTIMIZE,
//        DIAGNOSTIC,
//    };
//
//    enum class CompilerPass {
//        PREPROCESSOR,
//        COMPILER,
//        ANALYZER,
//        LINKER,
//    };
//
//    struct Meaning {
//        Category category;
//        std::optional<CompilerPass> affects;
//    };

    enum class CompilerFlagType {
        KIND_OF_OUTPUT,
        KIND_OF_OUTPUT_NO_LINKING,
        KIND_OF_OUTPUT_INFO,
        KIND_OF_OUTPUT_OUTPUT,
        PREPROCESSOR,
        PREPROCESSOR_MAKE,
        LINKER,
        LINKER_OBJECT_FILE,
        DIRECTORY_SEARCH,
        DIRECTORY_SEARCH_LINKER,
        SOURCE,
        OTHER,
    };

    struct CompilerFlag {
        Arguments arguments;
        CompilerFlagType type;
    };

    using CompilerFlags = std::list<CompilerFlag>;
    using Input = ArgumentsSegment;

    enum class Match {
        EXACT,
        PARTIAL,
        BOTH,
    };

    struct Instruction {

        constexpr Instruction(const uint8_t count, const Match match, const bool equal)
                : count_(count)
                , match_exact_((match == Match::EXACT || match == Match::BOTH) ? 1u : 0u)
                , match_partial_((match == Match::PARTIAL || match == Match::BOTH) ? 1u : 0u)
                , equal_sign_(equal ? 1u : 0u)
        { }

        [[nodiscard]] constexpr size_t count(bool exact_match) const {
            if (count_ > 0) {
                return (exact_match) ? count_ : count_ - 1;
            } else {
                return count_;
            }
        }

        [[nodiscard]] constexpr bool exact_match_allowed() const {
            return (match_exact_ == 1u);
        }

        [[nodiscard]] constexpr bool partial_match_allowed() const {
            return (match_partial_ == 1u);
        }

        [[nodiscard]] constexpr bool equal() const {
            return (equal_sign_ == 1);
        }

    private:
        uint16_t count_:8;
        uint16_t match_exact_:1;
        uint16_t match_partial_:1;
        uint16_t equal_sign_:1;
    };

    struct FlagDefinition {
        Instruction consumption;
        CompilerFlagType type;
    };

    using FlagsByName = std::map<std::string_view, FlagDefinition>;

    // Parser combinator which takes a list of flag definition and tries to apply
    // for the the received input stream. It can recognize only a single compiler
    // flag at the time.
    class FlagParser {
    public:
        explicit FlagParser(FlagsByName const& flags)
                : flags_(flags)
        { }

        [[nodiscard]]
        rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const
        {
            if (input.begin == input.end) {
                return rust::Err(input);
            }
            const std::string_view key(*input.begin);
            if (auto match = lookup(key); match) {
                const auto& [count, type] = match.value();

                auto begin = input.begin;
                auto end = std::next(begin, count + 1);

                CompilerFlag compiler_flag = {Arguments(begin, end), type };
                Input remainder = { end, input.end };
                return rust::Ok(std::make_pair(compiler_flag, remainder));
            }
            return rust::Err(input);
        }

    private:
        using Match = std::tuple<size_t, CompilerFlagType>;

        [[nodiscard]]
        std::optional<Match> lookup(const std::string_view &key) const
        {
            // try to find if the key has an associated instruction
            if (const auto candidate = flags_.lower_bound(key); flags_.end() != candidate) {
                // exact matches are preferred in all cases.
                if (auto result = check_equal(key, candidate); result) {
                    return result;
                }
                // check if the argument is allowed to stick to the flag
                if (auto result = check_partial(key, candidate); result) {
                    return result;
                }
                // check if this is the first element or not.
                if (flags_.begin() != candidate) {
                    const auto previous = std::prev(candidate);
                    if (auto result = check_partial(key, previous); result) {
                        return result;
                    }
                }
            }
            // check if the last element is not the one we are looking for.
            // (this is a limitation of `lower_bound` method.)
            const auto candidate = std::prev(flags_.end());
            if (auto result = check_partial(key, candidate); result) {
                return result;
            }
            return std::nullopt;
        }

        [[nodiscard]]
        static std::optional<Match> check_equal(const std::string_view& key, FlagsByName::const_iterator candidate) {
            if (!key.empty() && candidate->first == key && candidate->second.consumption.exact_match_allowed()) {
                const auto& instruction = candidate->second;
                return std::make_optional(std::make_tuple(instruction.consumption.count(true), instruction.type));
            }
            return std::nullopt;
        }

        [[nodiscard]]
        static std::optional<Match> check_partial(const std::string_view& key, FlagsByName::const_iterator candidate) {
            if (!key.empty() && candidate->second.consumption.partial_match_allowed()) {
                const auto length = std::min(key.size(), candidate->first.size());
                // TODO: make extra check on equal sign
                // TODO: make extra check on mandatory following characters
                if (key.substr(0, length) == candidate->first.substr(0, length)) {
                    const auto &instruction = candidate->second;
                    return std::make_optional(std::make_tuple(instruction.consumption.count(false), instruction.type));
                }
            }
            return std::nullopt;
        }

        FlagsByName const& flags_;
    };

    // Parser combinator which recognize source files as a single argument
    // of a compiler call.
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
        static rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) {
            const std::string candidate = take_extension(*input.begin);
            for (auto extension : EXTENSIONS) {
                if (candidate == extension) {
                    auto begin = input.begin;
                    auto end = std::next(begin, 1);

                    CompilerFlag compiler_flag = {Arguments(begin, end), CompilerFlagType::SOURCE };
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

    // A parser combinator, which recognize a single compiler flag without any conditions.
    struct EverythingElseFlagMatcher {
        [[nodiscard]]
        static rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input)
        {
            if (const std::string& front = *input.begin; !front.empty()) {
                auto begin = input.begin;
                auto end = std::next(begin);

                CompilerFlag compiler_flag = {Arguments(begin, end), CompilerFlagType::LINKER_OBJECT_FILE};
                Input remainder = {end, input.end};
                return rust::Ok(std::make_pair(compiler_flag, remainder));
            }
            return rust::Err(input);
        }
    };

    // A parser combinator, which takes multiple parsers and executes them
    // util one returns successfully and returns that as result. If none of
    // the parser returns success, it fails.
    template <typename ... Parsers>
    struct OneOf {
        using container_type = typename std::tuple<Parsers...>;
        container_type const parsers;

        explicit constexpr OneOf(Parsers const& ...p)
                : parsers(p...)
        { }

        [[nodiscard]]
        rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const
        {
            rust::Result<std::pair<CompilerFlag, Input>, Input> result = rust::Err(input);
            const bool valid =
                    std::apply([&input, &result](auto &&... parser) {
                        return ((result = parser.parse(input), result.is_ok()) || ... );
                    }, parsers);

            return (valid) ? result : rust::Err(input);
        }
    };

    // A parser combinator, which takes single parser and executes it util
    // returns successfully or consumes all input. If the parser fails before
    // the input is all consumed, it fails.
    template <typename Parser>
    struct Repeat {
        using result_type = rust::Result<CompilerFlags, Input>;
        Parser const parser;

        explicit constexpr Repeat(Parser  p)
                : parser(std::move(p))
        { }

        [[nodiscard]]
        result_type parse(const Input& input) const
        {
            CompilerFlags flags;
            auto it = Input { input.begin, input.end };
            for (; it.begin != it.end;) {
                auto result = parser.parse(it)
                        .on_success([&flags, &it](const auto& tuple) {
                            const auto& [flag, remainder] = tuple;
                            flags.push_back(flag);
                            it = remainder;
                        });
                if (result.is_err()) {
                    break;
                }
            }
            return (it.begin == it.end)
                   ? result_type(rust::Ok(flags))
                   : result_type(rust::Err(it));
        }
    };

    template <typename Parser>
    rust::Result<CompilerFlags> parse(const Parser &parser, const report::Command &command)
    {
        auto input = Input { std::next(command.arguments.begin()), command.arguments.end() };
        return parser.parse(input)
                .template map_err<std::runtime_error>([](auto remainder) {
                    return std::runtime_error(
                            fmt::format("Failed to recognize: {}",
                                        fmt::join(remainder.begin, remainder.end, ", ")));
                });
    }
}

namespace gcc {

    static const FlagsByName FLAG_DEFINITION = {
            {"-x",                 {Instruction(1, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-c",                 {Instruction(0, Match::EXACT, false), CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-S",                 {Instruction(0, Match::EXACT, false), CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-E",                 {Instruction(0, Match::EXACT, false), CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-o",                 {Instruction(1, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT_OUTPUT}},
            {"-dumpbase",          {Instruction(1, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-dumpbase-ext",      {Instruction(1, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-dumpdir",           {Instruction(1, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-v",                 {Instruction(0, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-###",               {Instruction(0, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"--help",             {Instruction(0, Match::BOTH, true),     CompilerFlagType::KIND_OF_OUTPUT_INFO}},
            {"--target-help",      {Instruction(0, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT_INFO}},
            {"--version",          {Instruction(0, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT_INFO}},
            {"-pass-exit-codes",   {Instruction(0, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-pipe",              {Instruction(0, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-specs",             {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::KIND_OF_OUTPUT}},
            {"-wrapper",           {Instruction(1, Match::EXACT, false),   CompilerFlagType::KIND_OF_OUTPUT}},
            {"-ffile-prefix-map",  {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::KIND_OF_OUTPUT}},
            {"-fplugin",           {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::KIND_OF_OUTPUT}},
            {"@",                  {Instruction(0, Match::PARTIAL, false), CompilerFlagType::KIND_OF_OUTPUT}},
            {"-A",                 {Instruction(1, Match::BOTH, false),    CompilerFlagType::PREPROCESSOR}},
            {"-D",                 {Instruction(1, Match::BOTH, false),    CompilerFlagType::PREPROCESSOR}},
            {"-U",                 {Instruction(1, Match::BOTH, false),    CompilerFlagType::PREPROCESSOR}},
            {"-include",           {Instruction(1, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-imacros",           {Instruction(1, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-undef",             {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-pthread",           {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-M",                 {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MM",                {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MG",                {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MP",                {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MD",                {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MMD",               {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MF",                {Instruction(1, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MT",                {Instruction(1, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MQ",                {Instruction(1, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-C",                 {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-CC",                {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-P",                 {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-traditional",       {Instruction(0, Match::BOTH, false),    CompilerFlagType::PREPROCESSOR}},
            {"-trigraphs",         {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-remap",             {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-H",                 {Instruction(0, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-Xpreprocessor",     {Instruction(1, Match::EXACT, false),   CompilerFlagType::PREPROCESSOR}},
            {"-Wp,",               {Instruction(0, Match::PARTIAL, false), CompilerFlagType::PREPROCESSOR}},
            {"-I",                 {Instruction(1, Match::BOTH, false),    CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iplugindir",        {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iquote",            {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-isystem",           {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-idirafter",         {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iprefix",           {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iwithprefix",       {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iwithprefixbefore", {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-isysroot",          {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-imultilib",         {Instruction(1, Match::EXACT, false),   CompilerFlagType::DIRECTORY_SEARCH}},
            {"-L",                 {Instruction(1, Match::BOTH, false),    CompilerFlagType::DIRECTORY_SEARCH_LINKER}},
            {"-B",                 {Instruction(1, Match::BOTH, false),    CompilerFlagType::DIRECTORY_SEARCH}},
            {"--sysroot",          {Instruction(1, Match::BOTH, true),     CompilerFlagType::DIRECTORY_SEARCH}},
            {"-flinker-output",    {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::LINKER}},
            {"-fuse-ld",           {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::LINKER}},
            {"-l",                 {Instruction(1, Match::BOTH, false),    CompilerFlagType::LINKER}},
            {"-nostartfiles",      {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-nodefaultlibs",     {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-nolibc",            {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-nostdlib",          {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-e",                 {Instruction(1, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-entry",             {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::LINKER}},
            {"-pie",               {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-no-pie",            {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-static-pie",        {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-r",                 {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-rdynamic",          {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-s",                 {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-symbolic",          {Instruction(0, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-static",            {Instruction(0, Match::BOTH, false),    CompilerFlagType::LINKER}},
            {"-shared",            {Instruction(0, Match::BOTH, false),    CompilerFlagType::LINKER}},
            {"-T",                 {Instruction(1, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-Xlinker",           {Instruction(1, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-Wl,",               {Instruction(0, Match::PARTIAL, false), CompilerFlagType::LINKER}},
            {"-u",                 {Instruction(1, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-z",                 {Instruction(1, Match::EXACT, false),   CompilerFlagType::LINKER}},
            {"-Xassembler",        {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-Wa,",               {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-ansi",              {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-aux-info",          {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
            {"-std",               {Instruction(0, Match::PARTIAL, true),  CompilerFlagType::OTHER}},
            {"-O",                 {Instruction(0, Match::BOTH, false),    CompilerFlagType::OTHER}},
            {"-g",                 {Instruction(0, Match::BOTH, false),    CompilerFlagType::OTHER}},
            {"-f",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-m",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-p",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-W",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-no",                {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-tno",               {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-save",              {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-d",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-E",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-Q",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-X",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"-Y",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
            {"--",                 {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
    };

    // https://gcc.gnu.org/onlinedocs/cpp/Environment-Variables.html
    Arguments flags_from_environment(const std::map<std::string, std::string> &environment) {
        Arguments flags;
        // define util function to append the content of a defined variable
        const auto inserter = [&flags](const std::string& value, const std::string& flag) {
            // the variable value is a colon separated directory list
            for (const auto& path : sys::path::split(value)) {
                // If the expression was ":/opt/thing", that will split into two
                // entries. One which is an empty string and the path. Empty string
                // refers the current working directory.
                auto directory = (path.empty()) ? "." : path.string();
                flags.push_back(flag);
                flags.push_back(directory);
            }
        };
        // check the environment for preprocessor influencing variables
        for (auto env : { "CPATH", "C_INCLUDE_PATH", "CPLUS_INCLUDE_PATH" }) {
            if (auto it = environment.find(env); it != environment.end()) {
                inserter(it->second, "-I");
            }
        }
        if (auto it = environment.find("OBJC_INCLUDE_PATH"); it != environment.end()) {
            inserter(it->second, "-isystem");
        }

        return flags;
    }

    rust::Result<CompilerFlags> parse(const report::Command &command)
    {
        static auto const parser =
                Repeat(
                        OneOf(
                                FlagParser(gcc::FLAG_DEFINITION),
                                SourceMatcher(),
                                EverythingElseFlagMatcher()
                        )
                );

        return parse(parser, command);
    }

    bool runs_compilation_pass(const CompilerFlags& flags)
    {
        constexpr static const char* NO_COMPILATION_FLAG[] {
                "-M",
                "-MM"
        };
        constexpr static size_t NO_COMPILATION_FLAG_SIZE = sizeof(NO_COMPILATION_FLAG) / sizeof(const char*);

        // no flag is a no compilation
        if (flags.empty()) {
            return false;
        }
        // help or version query is a no compilation
        if  (flags.end() != std::find_if(flags.begin(), flags.end(), [](const auto& flag) {
            return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_INFO);
        })) {
            return false;
        }
        // one of those make dependency generation also not count as compilation.
        // (will cause duplicate element, which is hard to detect.)
        if (flags.end() != std::find_if(flags.begin(), flags.end(), [](const auto& flag) {
            if (flag.type != CompilerFlagType::PREPROCESSOR_MAKE) {
                return false;
            }
            const std::string candidate = flag.arguments.front();
            auto begin = NO_COMPILATION_FLAG;
            auto end = NO_COMPILATION_FLAG + NO_COMPILATION_FLAG_SIZE;
            return (end != std::find_if(begin, end, [&candidate](const char* it) { return candidate == it; }));
        })) {
            return false;
        }
        return true;
    }

    std::optional<fs::path> source_file(const CompilerFlag& flag)
    {
        if (flag.type == CompilerFlagType::SOURCE) {
            auto source = fs::path(flag.arguments.front());
            return std::make_optional(std::move(source));
        }
        return std::optional<fs::path>();
    }

    std::list<fs::path> source_files(const CompilerFlags& flags)
    {
        std::list<fs::path> result;
        for (const auto& flag : flags) {
            if (auto source = source_file(flag); source) {
                result.push_back(source.value());
            }
        }
        return result;
    }

    std::optional<fs::path> output_file(const CompilerFlag& flag)
    {
        if (flag.type == CompilerFlagType::KIND_OF_OUTPUT_OUTPUT) {
            auto output = fs::path(flag.arguments.back());
            return std::make_optional(std::move(output));
        }
        return std::optional<fs::path>();
    }

    std::optional<fs::path> output_files(const CompilerFlags& flags)
    {
        std::list<fs::path> result;
        for (const auto& flag : flags) {
            if (auto output = output_file(flag); output) {
                return output;
            }
        }
        return std::optional<fs::path>();
    }

    Arguments filter_arguments(const CompilerFlags& flags, const fs::path source)
    {
        static const auto type_filter_out = [](CompilerFlagType type) -> bool {
            return (type == CompilerFlagType::LINKER)
                || (type == CompilerFlagType::PREPROCESSOR_MAKE)
                || (type == CompilerFlagType::DIRECTORY_SEARCH_LINKER);
        };

        const auto source_filter = [&source](const CompilerFlag& flag) -> bool {
            auto candidate = source_file(flag);
            return (!candidate) || (candidate && (candidate.value() == source));
        };

        const bool no_linking =
                flags.end() != std::find_if(flags.begin(), flags.end(), [](auto flag) {
                    return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING);
                });

        Arguments result;
        if (!no_linking) {
            result.push_back("-c");
        }
        for (const auto& flag : flags) {
            if (!type_filter_out(flag.type) && source_filter(flag)) {
                std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(result));
            }
        }
        return result;
    }

    bool match_executable_name(const fs::path& program)
    {
        static const std::list<std::string> patterns = {
                R"(^(cc|c\+\+|cxx|CC)$)",
                R"(^([^-]*-)*[mg]cc(-?\d+(\.\d+){0,2})?$)",
                R"(^([^-]*-)*[mg]\+\+(-?\d+(\.\d+){0,2})?$)",
                R"(^([^-]*-)*[g]?fortran(-?\d+(\.\d+){0,2})?$)",
        };
        static const auto pattern = std::regex(
                fmt::format("({})", fmt::join(patterns.begin(), patterns.end(), "|")));

        auto basename = program.filename();
        std::cmatch m;
        return std::regex_match(basename.c_str(), m, pattern);
    }
}

namespace cs {

    Tools::Tools(ToolPtrs&& tools) noexcept
            : tools_(tools)
    { }

    rust::Result<Tools> Tools::from(const cfg::Compilation& cfg)
    {
        ToolPtrs tools = {
                std::make_shared<GnuCompilerCollection>(cfg.compilers),
        };
        return rust::Ok(Tools(std::move(tools)));
    }

    output::Entries Tools::transform(const report::Report& report) const
    {
        output::Entries result;
        for (const auto& execution : report.executions) {
            spdlog::debug("Checking [pid: {}], command: {}", execution.run.pid, execution.command);
            recognize(execution.command)
                    .on_success([&execution, &result](auto items) {
                        // copy to results if the config allows it
                        std::copy(items.begin(), items.end(), std::back_inserter(result));
                        spdlog::debug("Checking [pid: {}], Recognized as: [{}]", execution.run.pid, items);
                    })
                    .on_error([&execution](const auto& error) {
                        spdlog::debug("Checking [pid: {}], {}", execution.run.pid, error.what());
                    });
        }
        return result;
    }

    rust::Result<output::Entries> Tools::recognize(const report::Command& command) const
    {
        // check if any tool can recognize the command.
        for (const auto& tool : tools_) {
            // the first it recognize it won't seek for more.
            if (auto semantic = tool->recognize(command); semantic.is_ok()) {
                return semantic;
            }
        }
        return rust::Err(std::runtime_error("No tools recognize this command."));
    }

    cs::output::Entry make_absolute(cs::output::Entry&& entry)
    {
        const auto transform = [&entry](const fs::path& path) {
            return (path.is_absolute()) ? path : entry.directory / path;
        };

        entry.file = transform(entry.file);
        if (entry.output) {
            entry.output.value() = transform(entry.output.value());
        }
        return std::move(entry);
    }

    GnuCompilerCollection::GnuCompilerCollection(std::list<fs::path> paths)
            : Tool()
            , paths(std::move(paths))
    { }

    rust::Result<output::Entries> GnuCompilerCollection::recognize(const report::Command &command) const {
        if (!recognize(command.program)) {
            return rust::Err(std::runtime_error("Not recognized program name."));
        }

        spdlog::debug("Recognized as a GnuCompiler execution.");
        return gcc::parse(command)
                .map<output::Entries>([&command](auto flags) {
                    if (!gcc::runs_compilation_pass(flags)) {
                        spdlog::debug("Compiler call does not run compilation pass.");
                        return output::Entries();
                    }
                    auto output = gcc::output_files(flags);
                    auto sources = gcc::source_files(flags);
                    if (sources.empty()) {
                        spdlog::debug("Source files not found for compilation.");
                        return output::Entries();
                    }

                    output::Entries result;
                    for (const auto &source : sources) {
                        auto arguments = gcc::filter_arguments(flags, source);
                        arguments.push_front(command.program);
                        auto extra = gcc::flags_from_environment(command.environment);
                        arguments.insert(arguments.end(), extra.begin(), extra.end());
                        cs::output::Entry entry = {source, command.working_dir, output, arguments};
                        result.emplace_back(make_absolute(std::move(entry)));
                    }
                    return result;
                });
    }

    bool GnuCompilerCollection::recognize(const fs::path& program) const {
        return (std::find(paths.begin(), paths.end(), program) != paths.end())
               || gcc::match_executable_name(program);
    }
}
