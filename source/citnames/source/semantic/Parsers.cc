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

#include "Parsers.h"

namespace cs::parser {

    rust::Result<std::pair<CompilerFlag, Input>, Input> FlagParser::parse(const Input &input) const {
        if (input.begin == input.end) {
            return rust::Err(input);
        }
        const std::string_view key(*input.begin);
        if (auto match = lookup(key); match) {
            const auto&[count, type] = match.value();

            auto begin = input.begin;
            auto end = std::next(begin, count + 1);

            CompilerFlag compiler_flag = {Arguments(begin, end), type};
            Input remainder = {end, input.end};
            return rust::Ok(std::make_pair(compiler_flag, remainder));
        }
        return rust::Err(input);
    }

    std::optional<FlagParser::Match> FlagParser::lookup(const std::string_view &key) const {
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

    std::optional<FlagParser::Match> FlagParser::check_equal(const std::string_view &key, FlagsByName::const_iterator candidate) {
        if (!key.empty() && candidate->first == key && candidate->second.consumption.exact_match_allowed()) {
            const auto& instruction = candidate->second;
            return std::make_optional(std::make_tuple(instruction.consumption.count(true), instruction.type));
        }
        return std::nullopt;
    }

    std::optional<FlagParser::Match> FlagParser::check_partial(const std::string_view &key, FlagsByName::const_iterator candidate) {
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

    rust::Result<std::pair<CompilerFlag, Input>, Input> EverythingElseFlagMatcher::parse(const Input &input) {
        if (const std::string& front = *input.begin; !front.empty()) {
            auto begin = input.begin;
            auto end = std::next(begin);

            CompilerFlag compiler_flag = {Arguments(begin, end), CompilerFlagType::LINKER_OBJECT_FILE};
            Input remainder = {end, input.end};
            return rust::Ok(std::make_pair(compiler_flag, remainder));
        }
        return rust::Err(input);
    }
}
