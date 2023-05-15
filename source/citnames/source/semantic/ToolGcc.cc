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

#include "ToolGcc.h"
#include "Parsers.h"

#include "libsys/Path.h"

#include <regex>
#include <utility>
#include <functional>
#include <set>
#include <string_view>
#include <numeric>

#include <spdlog/spdlog.h>

using namespace cs::semantic;

namespace {

    // https://gcc.gnu.org/onlinedocs/cpp/Environment-Variables.html
    Arguments flags_from_environment(const std::map<std::string, std::string> &environment) {
        Arguments flags;
        // define util function to append the content of a defined variable
        const auto &inserter = [&flags](const std::string& value, const std::string& flag) {
            // the variable value is a colon separated directory list
            for (const auto &path : sys::path::split(value)) {
                // If the expression was ":/opt/thing", that will split into two
                // entries. One which is an empty string and the path. Empty string
                // refers the current working directory.
                auto directory = (path.empty()) ? "." : path.string();
                flags.push_back(flag);
                flags.push_back(directory);
            }
        };
        // check the environment for preprocessor influencing variables
        for (const auto env : { "CPATH", "C_INCLUDE_PATH", "CPLUS_INCLUDE_PATH" }) {
            if (const auto it = environment.find(env); it != environment.end()) {
                inserter(it->second, "-I");
            }
        }
        if (const auto it = environment.find("OBJC_INCLUDE_PATH"); it != environment.end()) {
            inserter(it->second, "-isystem");
        }

        return flags;
    }

    Arguments create_argument_list(const Execution &execution) {
        Arguments input_arguments;
        std::copy(execution.arguments.begin(), execution.arguments.end(), std::back_inserter(input_arguments));
        Arguments extra_arguments = flags_from_environment(execution.environment);
        std::copy(extra_arguments.begin(), extra_arguments.end(), std::back_inserter(input_arguments));
        return input_arguments;
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

    bool has_linker(const CompilerFlags& flags)
    {
        return std::none_of(flags.begin(), flags.end(), [](auto flag) {
            return (flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING);
        });
    }

    std::tuple<
            Arguments,
            std::list<fs::path>,
            std::list<fs::path>,
            std::optional<fs::path>
    > split_compile(const CompilerFlags &flags) {
        Arguments arguments;
        std::list<fs::path> sources;
        std::list<fs::path> dependencies;
        std::optional<fs::path> output;

        for (const auto &flag : flags) {
            switch (flag.type) {
                case CompilerFlagType::KIND_OF_OUTPUT_OUTPUT: {
                    auto candidate = fs::path(flag.arguments.back());
                    output = std::make_optional(std::move(candidate));
                    continue;
                }
                case CompilerFlagType::SOURCE: {
                    auto candidate = fs::path(flag.arguments.front());
                    sources.emplace_back(std::move(candidate));
                    continue;
                }
                case CompilerFlagType::LIBRARY:
                case CompilerFlagType::OBJECT_FILE: {
                    auto candidate = fs::path(flag.arguments.front());
                    dependencies.emplace_back(candidate);
                    break;
                }
                default: {
                    break;
                }
            }
            std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(arguments));
        }
        return std::make_tuple(arguments, sources, dependencies, output);
    }

    std::tuple<
            Arguments,
            std::list<fs::path>,
            std::optional<fs::path>,
            size_t
    > split_link_with_updating_sources(const CompilerFlags &flags) {
        Arguments arguments;
        std::list<fs::path> files;
        std::optional<fs::path> output;
        size_t sources_count = 0;

        for (const auto &flag : flags) {
            switch (flag.type) {
                case CompilerFlagType::KIND_OF_OUTPUT_OUTPUT: {
                    auto candidate = fs::path(flag.arguments.back());
                    output = std::make_optional(std::move(candidate));
                    continue;
                }
                case CompilerFlagType::SOURCE: {
                    sources_count++;
                    const auto source_after_compilation = flag.arguments.front() + ".o";
                    arguments.push_back(source_after_compilation);
                    files.emplace_back(source_after_compilation);
                    break;
                }
                case CompilerFlagType::LIBRARY:
                case CompilerFlagType::OBJECT_FILE: {
                    arguments.push_back(flag.arguments.front());
                    files.emplace_back(flag.arguments.front());
                    break;
                }
                default: {
                    std::copy(flag.arguments.begin(), flag.arguments.end(), std::back_inserter(arguments));
                }
            }
        }
        return std::make_tuple(arguments, files, output, sources_count);
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

    const FlagsByName ToolGcc::FLAG_DEFINITION = {
            {"-x",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT}},
            {"-c",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-S",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-E",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-o",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT_OUTPUT}},
            {"-dumpbase",          {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT}},
            {"-dumpbase-ext",      {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT}},
            {"-dumpdir",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT}},
            {"-v",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT}},
            {"-###",               {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT}},
            {"--help",             {MatchInstruction::PREFIX,                           CompilerFlagType::KIND_OF_OUTPUT_INFO}},
            {"--target-help",      {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_INFO}},
            {"--version",          {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_INFO}},
            {"-pass-exit-codes",   {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT}},
            {"-pipe",              {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT}},
            {"-specs",             {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::KIND_OF_OUTPUT}},
            {"-wrapper",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT}},
            {"-ffile-prefix-map",  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::KIND_OF_OUTPUT}},
            {"-fplugin",           {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::KIND_OF_OUTPUT}},
            {"@",                  {MatchInstruction::PREFIX,                           CompilerFlagType::KIND_OF_OUTPUT}},
            {"-A",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::PREPROCESSOR}},
            {"-D",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::PREPROCESSOR}},
            {"-U",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::PREPROCESSOR}},
            {"-include",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR}},
            {"-imacros",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR}},
            {"-undef",             {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-pthread",           {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-M",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MM",                {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MG",                {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MP",                {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MD",                {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MMD",               {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MF",                {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MT",                {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-MQ",                {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR_MAKE}},
            {"-C",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-CC",                {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-P",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-traditional",       {MatchInstruction::PREFIX,                           CompilerFlagType::PREPROCESSOR}},
            {"-trigraphs",         {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-remap",             {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-H",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
            {"-Xpreprocessor",     {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR}},
            {"-Wp,",               {MatchInstruction::PREFIX,                           CompilerFlagType::PREPROCESSOR}},
            {"-I",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iplugindir",        {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iquote",            {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-isystem",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-idirafter",         {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iprefix",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iwithprefix",       {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-iwithprefixbefore", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-isysroot",          {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-imultilib",         {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
            {"-L",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::DIRECTORY_SEARCH_LINKER}},
            {"-B",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::DIRECTORY_SEARCH}},
            {"--sysroot",          {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::DIRECTORY_SEARCH}},
            {"-flinker-output",    {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::LINKER}},
            {"-fuse-ld",           {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::LINKER}},
            {"-l",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::LINKER}},
            {"-nostartfiles",      {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-nodefaultlibs",     {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-nolibc",            {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-nostdlib",          {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-e",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
            {"-entry",             {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::LINKER}},
            {"-pie",               {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-no-pie",            {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-static-pie",        {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-r",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-rdynamic",          {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-s",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-symbolic",          {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
            {"-static",            {MatchInstruction::PREFIX,                           CompilerFlagType::LINKER}},
            {"-shared",            {MatchInstruction::PREFIX,                           CompilerFlagType::LINKER}},
            {"-T",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
            {"-Xlinker",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
            {"-Wl,",               {MatchInstruction::PREFIX,                           CompilerFlagType::LINKER}},
            {"-u",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
            {"-z",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
            {"-Xassembler",        {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"-Wa,",               {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-ansi",              {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
            {"-aux-info",          {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
            {"-std",               {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::OTHER}},
            {"-O",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-g",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-f",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-m",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-p",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-W",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-no",                {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-tno",               {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-save",              {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-d",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-E",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-Q",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-X",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"-Y",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
            {"--",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
    };

    rust::Result<SemanticPtr> ToolGcc::recognize(const Execution &execution, const BuildTarget target) const {
        switch (target) {
            case BuildTarget::COMPILER: {
                if (is_compiler_call(execution.executable)) {
                    return compilation(execution);
                }
                break;
            }
            case BuildTarget::LINKER: {
                if (is_linker_call(execution.executable)) {
                    return linking(execution);
                }
                break;
            }
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

    bool ToolGcc::is_linker_call(const fs::path& program) const {
        static const auto pattern = std::regex(R"(^(ld|lld)\S*$)");
        std::cmatch m;
        return is_compiler_call(program) || std::regex_match(program.filename().c_str(), m, pattern);
    }

    rust::Result<SemanticPtr> ToolGcc::compilation(const Execution &execution) const {
        return compilation(FLAG_DEFINITION, execution);
    }

    rust::Result<SemanticPtr> ToolGcc::compilation(const FlagsByName &flags, const Execution &execution) {
        const Arguments &input_arguments = create_argument_list(execution);
        return parse(get_parser(flags), input_arguments)
                .and_then<SemanticPtr>([&execution](auto flags) -> rust::Result<SemanticPtr> {
                    if (is_compiler_query(flags)) {
                        SemanticPtr result = std::make_shared<QueryCompiler>();
                        return rust::Ok(std::move(result));
                    }
                    if (is_prerpocessor(flags)) {
                        SemanticPtr result = std::make_shared<Preprocess>();
                        return rust::Ok(std::move(result));
                    }

                    // arguments contains everything except output and sources
                    auto[arguments, sources, dependencies, output] = split_compile(flags);
                    if (sources.empty()) {
                        return rust::Err(std::runtime_error("Source files not found for compilation."));
                    }

                    bool with_linking;
                    if (has_linker(flags)) {
                        with_linking = true;
                        arguments.insert(arguments.begin(), "-c");
                    }
                    else {
                        with_linking = false;
                    }

                    SemanticPtr result = std::make_shared<Compile>(
                        execution.working_dir,
                        execution.executable,
                        std::move(arguments),
                        std::move(sources),
                        std::move(dependencies),
                        std::move(output),
                        with_linking
                    );
                    return rust::Ok(std::move(result));
                });
    }

    rust::Result<SemanticPtr> ToolGcc::linking(const Execution &execution) const {
        return linking(FLAG_DEFINITION, execution);
    }

    rust::Result<SemanticPtr> ToolGcc::linking(const FlagsByName &flags, const Execution &execution) {
        const Arguments &input_arguments = create_argument_list(execution);
        return parse(get_parser(flags), input_arguments)
                .and_then<SemanticPtr>([&execution](auto flags) -> rust::Result<SemanticPtr> {
                    if (is_compiler_query(flags)) {
                        SemanticPtr result = std::make_shared<QueryCompiler>();
                        return rust::Ok(std::move(result));
                    }
                    if (is_prerpocessor(flags)) {
                        SemanticPtr result = std::make_shared<Preprocess>();
                        return rust::Ok(std::move(result));
                    }

                    // arguments contains everything except output
                    auto[arguments, files, output, sources_count] = split_link_with_updating_sources(flags);
                    if (sources_count != 0 && !has_linker(flags)) {
                        return rust::Err(std::runtime_error("Without linking."));
                    }
                    if (files.empty()) {
                        spdlog::debug("Files not found for linking in command: {}", std::accumulate(
                            std::next(arguments.begin()),
                            arguments.end(),
                            arguments.front(),
                            [](std::string res, std::string flag) {
                                return std::move(res) + " " + std::move(flag);
                            }
                        ));
                        return rust::Err(std::runtime_error("Files not found for linking."));
                    }

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
