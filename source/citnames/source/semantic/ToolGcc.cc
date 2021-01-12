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

#include "ToolGcc.h"
#include "Parsers.h"

#include "libsys/Path.h"

#include <regex>
#include <utility>
#include <functional>

using namespace cs::semantic;

namespace {

    const FlagsByName FLAG_DEFINITION = {
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

    rust::Result<CompilerFlags> parse_flags(const Command &command)
    {
        static auto const parser =
                Repeat(
                        OneOf(
                                FlagParser(FLAG_DEFINITION),
                                SourceMatcher(),
                                EverythingElseFlagMatcher()
                        )
                );

        return parse(parser, command);
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

    bool is_prerpocessor_only(const CompilerFlags& flags)
    {
        constexpr static const char* NO_COMPILATION_FLAG[] {
                "-M",
                "-MM",
        };
        constexpr static size_t NO_COMPILATION_FLAG_SIZE = sizeof(NO_COMPILATION_FLAG) / sizeof(const char*);

        // one of those make dependency generation also not count as compilation.
        // (will cause duplicate element, which is hard to detect.)
        return std::none_of(flags.begin(), flags.end(), [](const auto& flag) {
            if (flag.type != CompilerFlagType::PREPROCESSOR_MAKE) {
                return false;
            }

            const std::string candidate = flag.arguments.front();
            auto begin = NO_COMPILATION_FLAG;
            auto end = NO_COMPILATION_FLAG + NO_COMPILATION_FLAG_SIZE;
            return (std::any_of(begin, end, [&candidate](const char* it) { return candidate == it; }));
        });
    }

    bool is_prerpocessor(const CompilerFlags& flags)
    {
        return std::any_of(flags.begin(), flags.end(), [](const auto &flag) {
            return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING) &&
                   (flag.arguments.front() == "-E");
        });
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

    bool linking(const CompilerFlags& flags)
    {
        return std::none_of(flags.begin(), flags.end(), [](auto flag) {
            return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING);
        });
    }

    Arguments filter_flags(const CompilerFlags& flags, const fs::path source)
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

        Arguments result;
        for (const auto& flag : flags) {
            if (!type_filter_out(flag.type) && source_filter(flag)) {
                std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(result));
            }
        }
        return result;
    }
}

namespace cs::semantic {

    const char* ToolGcc::name() const {
        return "GCC";
    }

    bool ToolGcc::recognize(const fs::path& program) const {
        static const std::list<std::string> patterns = {
                R"(^(cc|c\+\+|cxx|CC)$)",
                R"(^([^-]*-)*[mg]cc(-?\d+(\.\d+){0,2})?$)",
                R"(^([^-]*-)*[mg]\+\+(-?\d+(\.\d+){0,2})?$)",
                R"(^([^-]*-)*[g]?fortran(-?\d+(\.\d+){0,2})?$)",
        };
        static const auto pattern = std::regex(
                fmt::format("({})", fmt::join(patterns.begin(), patterns.end(), "|")));

        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }

    rust::Result<SemanticPtrs> ToolGcc::compilations(const Command &command) const {
        return parse_flags(command)
                .and_then<SemanticPtrs>([&command](auto flags) -> rust::Result<SemanticPtrs> {
                    if (is_compiler_query(flags)) {
                        SemanticPtrs result = { SemanticPtr(new QueryCompiler(command)) };
                        return rust::Ok(result);
                    }
                    if (!is_prerpocessor_only(flags)) {
                        return rust::Err(std::runtime_error("Compiler call does not run compilation pass."));
                    }
                    const bool preprocess = is_prerpocessor(flags);

                    auto output_opt = output_files(flags);
                    auto sources = source_files(flags);
                    if (sources.empty()) {
                        return rust::Err(std::runtime_error("Source files not found for compilation."));
                    }
                    Arguments prefix = { command.program.string() };
                    Arguments extra = flags_from_environment(command.environment);

                    SemanticPtrs result;
                    if (linking(flags)) {
                        // TODO: add a linker entry
                        prefix.emplace_back("-c");
                    }
                    for (const auto &source : sources) {
                        Arguments arguments = filter_flags(flags, source);
                        std::copy(prefix.begin(), prefix.end(), std::inserter(arguments, arguments.begin()));
                        std::copy(extra.begin(), extra.end(), std::back_inserter(arguments));

                        auto output = ((!output_opt.has_value()) || linking(flags))
                                      ? fs::path(source).replace_extension(preprocess ? ".i" : ".o")
                                      : output_opt.value();
                        result.emplace_back(
                                preprocess
                                ? SemanticPtr(new Preprocess(command, source, output, arguments))
                                : SemanticPtr(new Compile(command, source, output, arguments)));
                    }
                    return rust::Ok(result);
                });
    }
}
