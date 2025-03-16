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

#include "ToolClang.h"
#include "ToolGcc.h"

#include <regex>

using namespace cs::semantic;

namespace {

    // https://clang.llvm.org/docs/ClangCommandLineReference.html
    const FlagsByName CLANG_FLAG_DEFINITION = {
            {"-cc1",              {MatchInstruction::EXACTLY,                                 CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"--prefix",          {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::DIRECTORY_SEARCH}},
            {"-F",                {MatchInstruction::PREFIX,                                  CompilerFlagType::DIRECTORY_SEARCH}},
            {"-ObjC",             {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-ObjC++",           {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-Xarch",            {MatchInstruction::PREFIX_WITH_1_OPT,                       CompilerFlagType::OTHER}},
            {"-Xcuda",            {MatchInstruction::PREFIX_WITH_1_OPT,                       CompilerFlagType::OTHER}},
            {"-Xopenmp-target",   {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-Xopenmp-target=",  {MatchInstruction::PREFIX_WITH_1_OPT,                       CompilerFlagType::OTHER}},
            {"-Z",                {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::STATIC_ANALYZER}},
            {"-a",                {MatchInstruction::PREFIX,                                  CompilerFlagType::STATIC_ANALYZER}},
            {"--profile-blocks",  {MatchInstruction::EXACTLY,                                 CompilerFlagType::STATIC_ANALYZER}},
            {"-all_load",         {MatchInstruction::EXACTLY,                                 CompilerFlagType::STATIC_ANALYZER}},
            {"-allowable_client", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::STATIC_ANALYZER}},
            {"--analyze",         {MatchInstruction::EXACTLY,                                 CompilerFlagType::STATIC_ANALYZER}},
            {"--analyzer-no-default-checks",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::STATIC_ANALYZER}},
            {"--analyzer-output", {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,                CompilerFlagType::STATIC_ANALYZER}},
            {"-Xanalyzer",        {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::STATIC_ANALYZER}},
            {"-arch",             {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-arch_errors_fatal",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-arch_only",        {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-arcmt-migrate-emit-errors",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-arcmt-migrate-report-output",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"--autocomplete",    {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-bind_at_load",     {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-bundle",           {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-bundle_loader",    {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-client_name",      {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"-compatibility_version",
                                  {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"--config",          {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"--constant-cfstrings",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--cuda-compile-host-device",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--cuda-device-only",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--cuda-host-only",  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--cuda-include-ptx",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"--no-cuda-include-ptx",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"--cuda-noopt-device-debug",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--no-cuda-noopt-device-debug",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-cuid",             {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-current_version",  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,                CompilerFlagType::OTHER}},
            {"-dead_strip",       {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-dependency-dot",   {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-dependency-file",  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-dsym-dir",         {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,                CompilerFlagType::OTHER}},
            {"-dumpmachine",      {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-dumpversion",      {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--dyld-prefix",     {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP, CompilerFlagType::OTHER}},
            {"-dylib_file",       {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-dylinker",         {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-dylinker_install_name",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,                CompilerFlagType::OTHER}},
            {"-dynamic",          {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-dynamiclib",       {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-emit-ast",         {MatchInstruction::EXACTLY,                                 CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-enable-trivial-auto-var-init-zero-knowing-it-will-be-removed-from-clang",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-exported_symbols_list",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-faligned-new",     {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-force_load",       {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-framework",        {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"--gcc-toolchain",   {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-gcodeview",        {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-gcodeview-ghash",  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-gno-codeview-ghash",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--gpu-instrument-lib",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"--gpu-max-threads-per-block",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-headerpad_max_install_names",
                                  {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"--hip-link",        {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--hip-version",     {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-ibuiltininc",      {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-image_base",       {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-index-header-map", {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-init",             {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-install_name",     {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-interface-stub-version",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-keep_private_externs",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-lazy",             {MatchInstruction::PREFIX_WITH_1_OPT,                       CompilerFlagType::OTHER}},
            {"-EB",               {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--migrate",         {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-mllvm",            {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-module-dependency-dir",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-multiply_defined", {MatchInstruction::PREFIX_WITH_1_OPT,                       CompilerFlagType::OTHER}},
            {"--output",          {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP, CompilerFlagType::OTHER}},
            {"-objcmt",           {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"-object",           {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--profile",         {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--pipe",            {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--print-diagnostic-categories",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-r",                {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"--save",            {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"-sect",             {MatchInstruction::PREFIX_WITH_3_OPTS,                      CompilerFlagType::OTHER}},
            {"-seg1addr",         {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,                CompilerFlagType::OTHER}},
            {"-seg_",             {MatchInstruction::PREFIX_WITH_1_OPT,                       CompilerFlagType::OTHER}},
            {"-segaddr",          {MatchInstruction::EXACTLY_WITH_2_OPTS,                     CompilerFlagType::OTHER}},
            {"-segcreate",        {MatchInstruction::EXACTLY_WITH_3_OPTS,                     CompilerFlagType::OTHER}},
            {"-seglinkedit",      {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-segprot",          {MatchInstruction::EXACTLY_WITH_3_OPTS,                     CompilerFlagType::OTHER}},
            {"-serialize-diagnostics",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"--serialize-diagnostics",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-single_module",    {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-sub_",             {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"--sysroot",         {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP, CompilerFlagType::OTHER}},
            {"--target",          {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"-target",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-time",             {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"--traditional",     {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"-traditional",      {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"-twolevel",         {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"-umbrella",         {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-unexported_symbols_list",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-unwindlib",        {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"--unwindlib",       {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-x",                {MatchInstruction::PREFIX,                                  CompilerFlagType::OTHER}},
            {"--language",        {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP, CompilerFlagType::OTHER}},
            {"-Xassembler",       {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-Xclang",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-Xpreprocessor",    {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
    };

    // Taken from the LLVM 20.1 at:
    // https://github.com/llvm/llvm-project/blob/llvmorg-20.1.0/clang/include/clang/Driver/Options.td
    // Only flang exclusive flags are specified here (the ones without
    // ClangOption visibility)
    const FlagsByName FLANG_FLAG_DEFINITION = {
            {"-J",                {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,         CompilerFlagType::KIND_OF_OUTPUT_NO_LINKING}},
            {"-Xflang",           {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-cpp",              {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-nocpp",              {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-falternative-parameter-statement",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fbackslash",       {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-backslash",    {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fconvert",         {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-fdefault-double-8",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fdefault-integer-8",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fdefault-real-8",  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fdisable-integer-16",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fdisable-integer-2",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fdisable-real-10", {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fdisable-real-3",  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-ffixed-form",      {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-ffixed-line-length",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::OTHER}},
            {"-ffixed-line-length-",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,                CompilerFlagType::OTHER}},
            {"-ffree-form",       {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-finit-global-zero",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-init-global-zero",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fhermetic-module-files",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fimplicit-none",   {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-implicit-none",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fintrinsic-modules-path",
                                  {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,                  CompilerFlagType::OTHER}},
            {"-flang-deprecated-no-hlfir",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-flang-experimental-hlfir",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-flarge-sizes",     {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-flogical-abbreviations",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-logical-abbreviations",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-automatic",    {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-frealloc-lhs",  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-realloc-lhs",  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fsave-main-program",
                                  {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-funderscoring",    {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-underscoring", {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-funsigned",        {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-unsigned",     {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fxor-operator",    {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-fno-xor-operator", {MatchInstruction::EXACTLY,                                 CompilerFlagType::OTHER}},
            {"-module-dir",       {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,         CompilerFlagType::OTHER}},
            {"--romc-path",       {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,        CompilerFlagType::DIRECTORY_SEARCH_LINKER}},
    };

    FlagsByName clang_flags(const FlagsByName &base) {
        FlagsByName flags(base);
        flags.insert(CLANG_FLAG_DEFINITION.begin(), CLANG_FLAG_DEFINITION.end());
        flags.insert(FLANG_FLAG_DEFINITION.begin(), FLANG_FLAG_DEFINITION.end());
        return flags;
    }
}

namespace cs::semantic {

    ToolClang::ToolClang() noexcept
            : flag_definition(clang_flags(ToolGcc::FLAG_DEFINITION))
    { }

    rust::Result<SemanticPtr> ToolClang::recognize(const Execution &execution) const {
        if (is_compiler_call(execution.executable)) {
            return ToolGcc::compilation(ToolClang::flag_definition, execution);
        }
        return rust::Ok(SemanticPtr());
    }

    bool ToolClang::is_compiler_call(const fs::path &program) const {
        static const auto pattern = std::regex(R"(^([^-]*-)*(clang(|\+\+)|flang(-new)?)(-?\d+(\.\d+){0,2})?$)");

        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }
}
