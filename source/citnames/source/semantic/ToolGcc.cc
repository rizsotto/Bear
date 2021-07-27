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

#include "ToolGcc.h"
#include "Parsers.h"

#include "libsys/Path.h"

#include <regex>
#include <utility>
#include <functional>
#include <set>
#include <string_view>

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

    bool is_prerpocessor(const CompilerFlags& flags)
    {
        // one of those make dependency generation also not count as compilation.
        // (will cause duplicate element, which is hard to detect.)
        static const std::set<std::string_view> NO_COMPILATION_FLAG =
                { "-M", "-MM" };

        return std::any_of(flags.begin(), flags.end(), [](const auto &flag) {
            const std::string &candidate = flag.arguments.front();
            return ((flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING) && (candidate == "-E"))
                || ((flag.type == CompilerFlagType::PREPROCESSOR_MAKE) && (NO_COMPILATION_FLAG.find(candidate) != NO_COMPILATION_FLAG.end()));
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
            std::optional<fs::path>
    > split(const CompilerFlags &flags) {
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
}

namespace cs::semantic {

    rust::Result<SemanticPtr> ToolGcc::recognize(const Execution &execution) const {
        if (is_compiler_call(execution.executable)) {
            return compilation(execution);
        }
        return rust::Ok(SemanticPtr());
    }

    bool ToolGcc::is_compiler_call(const fs::path& program) const {
        static const auto pattern = std::regex(
                // - cc
                // - c++
                // - cxx
                // - CC
                // - mcc, gcc, m++, g++, gfortran, fortran
                //   - with prefixes like: arm-none-eabi-
                //   - with postfixes like: -7.0 or 6.4.0
            R"(^(cc|c\+\+|cxx|CC|(([^-]*-)*([mg](cc|\+\+)|[g]?fortran)(-?\d+(\.\d+){0,2})?))$)"
        );

        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }

    rust::Result<SemanticPtr> ToolGcc::compilation(const Execution &execution) const {
        return compilation(FLAG_DEFINITION, execution);
    }

    rust::Result<SemanticPtr> ToolGcc::compilation(const FlagsByName &flags, const Execution &execution) {
        auto const parser =
                Repeat(
                        OneOf(
                                FlagParser(flags),
                                SourceMatcher(),
                                EverythingElseFlagMatcher()
                        )
                );

        return parse(parser, execution)
                .and_then<SemanticPtr>([&execution](auto flags) -> rust::Result<SemanticPtr> {
                    if (is_compiler_query(flags)) {
                        SemanticPtr result = std::make_shared<QueryCompiler>();
                        return rust::Ok(std::move(result));
                    }
                    if (is_prerpocessor(flags)) {
                        SemanticPtr result = std::make_shared<Preprocess>();
                        return rust::Ok(std::move(result));
                    }

                    auto[arguments, sources, output] = split(flags);
                    // Validate: must have source files.
                    if (sources.empty()) {
                        return rust::Err(std::runtime_error("Source files not found for compilation."));
                    }
                    // TODO: introduce semantic type for linking
                    if (linking(flags)) {
                        arguments.insert(arguments.begin(), "-c");
                    }
                    // Create compiler flags from environment variables if present.
                    Arguments extra = flags_from_environment(execution.environment);
                    std::copy(extra.begin(), extra.end(), std::back_inserter(arguments));

                    SemanticPtr result = std::make_shared<Compile>(
                            execution.working_dir,
                            execution.executable,
                            std::move(arguments),
                            std::move(sources),
                            std::move(output)
                    );
                    return rust::Ok(std::move(result));
                });
    }
}
