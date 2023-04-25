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

#include "Parsers.h"

#include <set>

#include <iostream>

namespace {

    std::string_view take_extension(const std::string_view &file) {
        const auto pos = file.rfind('.');
        return (pos == std::string::npos) ? file : file.substr(pos);
    }

    std::optional<std::string_view> split_extra(const std::string_view &prefix, const std::string_view &candidate) {
        if (prefix.empty()) {
            return std::make_optional(candidate);
        }
        if (candidate.empty()) {
            return std::nullopt;
        }
        if (prefix.size() > candidate.size()) {
            return std::nullopt;
        }
        const auto common = candidate.substr(0, prefix.size());
        if (common == prefix) {
            return std::make_optional(candidate.substr(prefix.size()));
        }
        return std::nullopt;
    }

    enum class FlagMatch {
        SEP,
        GLUED,
        GLUED_WITH_EQ,
    };

    FlagMatch classify_flag_matching(const std::string_view &flag) {
        if (flag.empty()) {
            return FlagMatch::SEP;
        } else {
            if (flag[0] == '=') {
                return FlagMatch::GLUED_WITH_EQ;
            } else {
                return FlagMatch::GLUED;
            }
        }
    }

    using namespace cs::semantic;

    bool is_exact_match_only(const MatchInstruction match_instruction) {
        switch (match_instruction) {
            case MatchInstruction::EXACTLY:
            case MatchInstruction::EXACTLY_WITH_1_OPT_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_OR_WITHOUT_EQ_OR_SEP:
            case MatchInstruction::EXACTLY_WITH_2_OPTS:
            case MatchInstruction::EXACTLY_WITH_3_OPTS:
                return true;
            default:
                return false;
        }
    }

    bool is_prefix_match(const MatchInstruction match_instruction) {
        switch (match_instruction) {
            case MatchInstruction::PREFIX:
            case MatchInstruction::PREFIX_WITH_1_OPT:
            case MatchInstruction::PREFIX_WITH_2_OPTS:
            case MatchInstruction::PREFIX_WITH_3_OPTS:
                return true;
            default:
                return false;
        }
    }

    bool is_glue_allowed(const MatchInstruction match_instruction) {
        switch (match_instruction) {
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_OR_WITHOUT_EQ_OR_SEP:
            case MatchInstruction::PREFIX:
            case MatchInstruction::PREFIX_WITH_1_OPT:
            case MatchInstruction::PREFIX_WITH_2_OPTS:
            case MatchInstruction::PREFIX_WITH_3_OPTS:
                return true;
            default:
                return false;
        }
    }

    bool is_glue_with_equal_allowed(const MatchInstruction match_instruction) {
        switch (match_instruction) {
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_OR_WITHOUT_EQ_OR_SEP:
                return true;
            default:
                return false;
        }
    }

    [[nodiscard]] size_t count_of_arguments(MatchInstruction match_instruction) {
        switch (match_instruction) {
            case MatchInstruction::EXACTLY:
                return 1;
            case MatchInstruction::EXACTLY_WITH_1_OPT_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP:
            case MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_OR_WITHOUT_EQ_OR_SEP:
                return 2;
            case MatchInstruction::EXACTLY_WITH_2_OPTS:
                return 3;
            case MatchInstruction::EXACTLY_WITH_3_OPTS:
                return 4;
            case MatchInstruction::PREFIX:
                return 1;
            case MatchInstruction::PREFIX_WITH_1_OPT:
                return 2;
            case MatchInstruction::PREFIX_WITH_2_OPTS:
                return 3;
            case MatchInstruction::PREFIX_WITH_3_OPTS:
                return 4;
        }
        return 0;
    }
}

namespace cs::semantic {

    std::tuple<ArgumentsView, ArgumentsView> ArgumentsView::take(const size_t count) const {
        const size_t size = std::distance(begin_, end_);
        if (size < count) {
            auto arguments = ArgumentsView(begin_, begin_);
            auto remainder = ArgumentsView(end_, end_);
            return std::make_tuple(arguments, remainder);
        } else {
            const auto end = std::next(begin_, count);
            auto arguments = ArgumentsView(begin_, end);
            auto remainder = ArgumentsView(end, end_);
            return std::make_tuple(arguments, remainder);
        }
    }

    Arguments::value_type ArgumentsView::front() const {
        return *begin_;
    }

    Arguments::value_type ArgumentsView::back() const {
        return *std::prev(end_);
    }

    rust::Result<std::pair<CompilerFlag, ArgumentsView>, ArgumentsView> FlagParser::parse(const ArgumentsView &input) const {
        // early exit if there is nothing to do.
        if (input.empty()) {
            return rust::Err(input);
        }
        // early exit if the flag is an empty string.
        const auto key = input.front();
        if (key.empty()) {
            return rust::Err(input);
        }
        // based on the lookup result, consume the input.
        if (auto match = lookup(key); match) {
            const auto&[count, type] = match.value();
            const auto&[arguments, remainder] = input.take(count);
            if (arguments.empty()) {
                return rust::Err(input);
            }
            auto flag = CompilerFlag { arguments, type };
            return rust::Ok(std::make_pair(flag, remainder));
        }
        return rust::Err(input);
    }

    std::optional<FlagParser::Match> FlagParser::lookup(const std::string_view &key) const {
        // try to find if the key has an associated instruction
        if (const auto candidate = flags_.lower_bound(key); flags_.end() != candidate) {
            // exact matches are preferred in all cases.
            if (auto result = check_equal(key, *candidate); result) {
                return result;
            }
        }

        std::optional<std::string> candidate_longest = std::nullopt;
        for (const auto &[candidate, info] : flags_) {
            if (const auto& extra = split_extra(candidate, key); extra) {
                const size_t prefix_length = key.size() - extra.value().size();
                if (!candidate_longest || (prefix_length > candidate_longest.value().size())) {
                    candidate_longest = std::make_optional(std::string(candidate));
                }
            }
        }

        return (candidate_longest.has_value())
            ? check_partial(key, *(flags_.find(candidate_longest.value())))
            : std::nullopt;
    }

    std::optional<FlagParser::Match>
    FlagParser::check_equal(const std::string_view &key, const FlagsByName::value_type &candidate) {
        const auto &flag_definition = candidate.second;
        if ((is_exact_match_only(flag_definition.match) || is_prefix_match(flag_definition.match)) && key == candidate.first) {
            const size_t count = count_of_arguments(flag_definition.match);
            return std::make_optional(std::make_tuple(count, flag_definition.type));
        }
        return std::nullopt;
    }

    std::optional<FlagParser::Match>
    FlagParser::check_partial(const std::string_view &key, const FlagsByName::value_type &candidate) {
        const auto &flag_definition = candidate.second;
        const auto flag_matching = classify_flag_matching(key.substr(candidate.first.size()));
        switch (flag_matching) {
            case FlagMatch::GLUED:
                if (is_glue_allowed(flag_definition.match)) {
                    const size_t decrease = is_prefix_match(flag_definition.match) ? 0 : 1;
                    const size_t count = count_of_arguments(flag_definition.match) - decrease;
                    return std::make_optional(std::make_tuple(count, flag_definition.type));
                }
                break;
            case FlagMatch::GLUED_WITH_EQ:
                if (is_glue_with_equal_allowed(flag_definition.match)) {
                    const size_t count = count_of_arguments(flag_definition.match) - 1;
                    return std::make_optional(std::make_tuple(count, flag_definition.type));
                }
                break;
            default:
                // This should not happen here. Exact match is already filtered out.
                __builtin_unreachable();
        }
        return std::nullopt;
    }

    rust::Result<std::pair<CompilerFlag, ArgumentsView>, ArgumentsView> SourceMatcher::parse(const ArgumentsView &input) {
        static const std::set<std::string_view> extensions = {
                // header files
                ".h", ".hh", ".H", ".hp", ".hxx", ".hpp", ".HPP", ".h++", ".tcc",
                // C
                ".c", ".C",
                // C++
                ".cc", ".CC", ".c++", ".C++", ".cxx", ".cpp", ".cp",
                // CUDA
                ".cu",
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

        if (input.empty()) {
            return rust::Err(input);
        }
        const auto &candidate = input.front();
        const auto &extension = take_extension(candidate);
        if (extensions.find(extension) != extensions.end()) {
            const auto &[arguments, remainder] = input.take(1);
            if (arguments.empty()) {
                return rust::Err(input);
            }
            auto flag = CompilerFlag { arguments, CompilerFlagType::SOURCE };
            return rust::Ok(std::make_pair(flag, remainder));
        }
        return rust::Err(input);
    }

    rust::Result<std::pair<CompilerFlag, ArgumentsView>, ArgumentsView> ObjectFileMatcher::parse(const ArgumentsView &input) {
        if (input.empty()) {
            return rust::Err(input);
        }
        const auto &candidate = input.front();
        const auto &extension = take_extension(candidate);
        if (".o" == extension) {
            const auto &[arguments, remainder] = input.take(1);
            if (arguments.empty()) {
                return rust::Err(input);
            }
            auto flag = CompilerFlag { arguments, CompilerFlagType::OBJECT_FILE };
            return rust::Ok(std::make_pair(flag, remainder));
        }
        return rust::Err(input);
    }

    rust::Result<std::pair<CompilerFlag, ArgumentsView>, ArgumentsView> LibraryMatcher::parse(const ArgumentsView &input) {
        static const std::set<std::string_view> extensions = {
                // unix
                ".so", ".a", ".la",
                // macos
                ".dylib",
                // windows
                ".dll", ".DLL", ".ocx", ".OCX", ".lib", ".LIB",
                // amigaOS
                ".library"
        };

        if (input.empty()) {
            return rust::Err(input);
        }
        const auto &candidate = input.front();
        const auto &extension = take_extension(candidate);
        if (extensions.find(extension) != extensions.end() || candidate.find(".so.") != std::string::npos) {
            const auto &[arguments, remainder] = input.take(1);
            if (arguments.empty()) {
                return rust::Err(input);
            }
            auto flag = CompilerFlag { arguments, CompilerFlagType::LIBRARY };
            return rust::Ok(std::make_pair(flag, remainder));
        }
        return rust::Err(input);
    }

    rust::Result<std::pair<CompilerFlag, ArgumentsView>, ArgumentsView> EverythingElseFlagMatcher::parse(const ArgumentsView &input) {
        if (input.empty()) {
            return rust::Err(input);
        }
        if (const auto &front = input.front(); !front.empty()) {
            const auto &[arguments, remainder] = input.take(1);
            if (arguments.empty()) {
                return rust::Err(input);
            }
            auto flag = CompilerFlag { arguments, CompilerFlagType::UNKNOWN };
            return rust::Ok(std::make_pair(flag, remainder));
        }
        return rust::Err(input);
    }
}
