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

#include "Parsers.h"

namespace {

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
}

namespace cs::semantic {

    rust::Result<std::pair<CompilerFlag, Input>, Input> FlagParser::parse(const Input &input) const {
        if (input.empty()) {
            return rust::Err(input);
        }
        const auto key = *input.begin();
        if (auto match = lookup(key); match) {
            const auto&[count, type] = match.value();
            auto [arguments, remainder] = input.take(count + 1);
            auto flag = CompilerFlag { .arguments = arguments, .type = type };
            return rust::Ok(std::make_pair(flag, remainder));
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
            return std::make_optional(std::make_tuple(instruction.consumption.count(false), instruction.type));
        }
        return std::nullopt;
    }

    std::optional<FlagParser::Match> FlagParser::check_partial(const std::string_view &key, FlagsByName::const_iterator candidate) {
        if (!key.empty() && candidate->second.consumption.partial_match_allowed()) {
            // TODO: make extra check on mandatory following characters
            if (const auto extra = split_extra(candidate->first, key); extra) {
                const auto &instruction = candidate->second;
                const bool equal = (extra->find('=') != std::string_view::npos);
                return std::make_optional(std::make_tuple(instruction.consumption.count(equal), instruction.type));
            }
        }
        return std::nullopt;
    }

    rust::Result<std::pair<CompilerFlag, Input>, Input> EverythingElseFlagMatcher::parse(const Input &input) {
        if (const auto &front = *input.begin(); !front.empty()) {
            auto [arguments, remainder] = input.take(1);
            auto flag = CompilerFlag { .arguments = arguments, .type = CompilerFlagType::LINKER_OBJECT_FILE };
            return rust::Ok(std::make_pair(flag, remainder));
        }
        return rust::Err(input);
    }
}
