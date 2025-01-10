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

#include "ToolIntelFortran.h"
#include "Common.h"
#include "Parsers.h"
#include "libsys/Path.h"

#include <regex>
#include <utility>
#include <functional>
#include <set>
#include <string_view>

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

            static const std::set<std::string_view> NO_COMPILATION_FLAG = { "-preprocess-only", "-P", "-E", "-Ep" };

            return ((flag.type == CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING) && (NO_COMPILATION_FLAG.find(candidate) != NO_COMPILATION_FLAG.end()))
                || ((flag.type == CompilerFlagType::PREPROCESSOR_MAKE));
        });
    }
}

namespace cs::semantic {

    const FlagsByName ToolIntelFortran::FLAG_DEFINITION = {
        {"-c",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
        {"-S",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
        {"-E",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
        {"-Ep",                {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
        {"-preprocess-only",   {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
        {"-P",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
        {"-o",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT_OUTPUT}},
        {"-debug",             {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT}},
        {"-debug-parameters",  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::KIND_OF_OUTPUT}},
        {"@",                  {MatchInstruction::PREFIX,                           CompilerFlagType::KIND_OF_OUTPUT}},
        {"-Fa",                {MatchInstruction::PREFIX,                           CompilerFlagType::KIND_OF_OUTPUT}},
        {"-FA",                {MatchInstruction::PREFIX,                           CompilerFlagType::KIND_OF_OUTPUT}},
        {"-shared",            {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT}},
        {"-dryrun",            {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"-dumpmachine",       {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"-v",                 {MatchInstruction::PREFIX,                           CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"-V",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"--help",             {MatchInstruction::PREFIX,                           CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"--version",          {MatchInstruction::EXACTLY,                          CompilerFlagType::KIND_OF_OUTPUT_INFO}},
        {"-D",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::PREPROCESSOR}},
        {"-U",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::PREPROCESSOR}},
        {"-include",           {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::PREPROCESSOR}},
        {"-undef",             {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
        {"-pthread",           {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
        {"-MD",                {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
        {"-MMD",               {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR_MAKE}},
        {"-MF",                {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR_MAKE}},
        {"-gen-dep",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::PREPROCESSOR_MAKE}},
        {"-C",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
        {"-Xoption,cpp",       {MatchInstruction::PREFIX,                           CompilerFlagType::PREPROCESSOR}},
        {"-Xoption,fpp",       {MatchInstruction::PREFIX,                           CompilerFlagType::PREPROCESSOR}},
        {"-fpp",               {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
        {"-nofpp",             {MatchInstruction::EXACTLY,                          CompilerFlagType::PREPROCESSOR}},
        {"-Wp",                {MatchInstruction::PREFIX,                           CompilerFlagType::PREPROCESSOR}},
        {"-I",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::DIRECTORY_SEARCH}},
        {"-iquote",            {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
        {"-isystem",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
        {"-isysroot",          {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::DIRECTORY_SEARCH}},
        {"-L",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::DIRECTORY_SEARCH_LINKER}},
        {"--sysroot",          {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::DIRECTORY_SEARCH}},
        {"-X",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::DIRECTORY_SEARCH}},
        {"-l",                 {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,  CompilerFlagType::LINKER}},
        {"-nostartfiles",      {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-nodefaultlibs",     {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-nostdlib",          {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-r",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-s",                 {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-shared-intel",      {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"shared-libgcc",      {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-static",            {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-static-intel",      {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-static-libgcc",     {MatchInstruction::EXACTLY,                          CompilerFlagType::LINKER}},
        {"-T",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
        {"-Xlinker",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
        {"-Xoption,link",      {MatchInstruction::PREFIX,                           CompilerFlagType::LINKER}},
        {"-u",                 {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::LINKER}},
        {"-Wl",                {MatchInstruction::PREFIX,                           CompilerFlagType::LINKER}},
        {"-Xoption,asm",       {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-std",               {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ, CompilerFlagType::OTHER}},
        {"-O",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-g",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-f",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-m",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-x",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-diag-",             {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-no",                {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        {"-gen-interfaces",    {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,           CompilerFlagType::OTHER}},
        {"-nogen-interfaces",  {MatchInstruction::EXACTLY,                          CompilerFlagType::OTHER}},
        {"--",                 {MatchInstruction::PREFIX,                           CompilerFlagType::OTHER}},
        };

    rust::Result<SemanticPtr> ToolIntelFortran::recognize(const Execution& execution) const
    {
        if (is_compiler_call(execution.executable)) {
            return compilation_impl(FLAG_DEFINITION, execution, create_argument_list, is_preprocessor);
        }
        return rust::Ok(SemanticPtr());
    }

    bool ToolIntelFortran::is_compiler_call(const fs::path& program) const
    {
        static const auto pattern = std::regex(R"(^([^-]*-)*(ifx|ifort)(-?\w+(\.\d+){0,2})?$)");

        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }
}
