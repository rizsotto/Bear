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

#pragma once

#include "libresult/Result.h"
#include "Domain.h"

#include <cstdint>
#include <list>
#include <map>
#include <optional>
#include <string>

#include <fmt/format.h>

namespace cs::semantic {

    using namespace domain;

    // Represents command line arguments.
    using Arguments = std::list<std::string>;

    struct Input {
        using iterator = Arguments::const_iterator;

        explicit Input(const Arguments &input) noexcept
                : begin_(std::next(input.begin()))
                , end_(input.end())
        { }

        explicit Input(iterator begin, iterator end) noexcept
                : begin_(begin)
                , end_(end)
        { }

        [[nodiscard]] bool empty() const {
            return (begin_ == end_);
        }

        [[nodiscard]] Arguments::value_type front() const;

        [[nodiscard]] std::tuple<Arguments, Input> take(size_t count) const;

        [[nodiscard]] std::string to_string() const;

    private:
        iterator begin_;
        iterator end_;
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
        STATIC_ANALYZER,
    };

    struct CompilerFlag {
        Arguments arguments;
        CompilerFlagType type;
    };

    using CompilerFlags = std::list<CompilerFlag>;

    enum class MatchInstruction {
        EXACTLY,
        EXACTLY_WITH_1_OPT_SEP,
        EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,
        EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP,
        EXACTLY_WITH_1_OPT_GLUED,
        EXACTLY_WITH_1_OPT_GLUED_OR_SEP,
        EXACTLY_WITH_1_OPT_GLUED_WITH_OR_WITHOUT_EQ_OR_SEP,
        EXACTLY_WITH_2_OPTS,
        EXACTLY_WITH_3_OPTS,
        PREFIX,
        PREFIX_WITH_1_OPT,
        PREFIX_WITH_2_OPTS,
        PREFIX_WITH_3_OPTS,
    };

    struct FlagDefinition {
        MatchInstruction match;
        CompilerFlagType type;
    };

    using FlagsByName = std::map<std::string_view, FlagDefinition>;

    // Parser combinator which takes a list of flag definition and tries to apply
    // to the received input stream. It can recognize only a single compiler flag
    // at the time.
    class FlagParser {
    public:
        explicit FlagParser(FlagsByName const& flags) noexcept
                : flags_(flags)
        { }

        [[nodiscard]]
        rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) const;

    private:
        using Match = std::tuple<size_t, CompilerFlagType>;

        [[nodiscard]]
        std::optional<Match> lookup(const std::string_view &key) const;

        [[nodiscard]]
        static std::optional<Match> check_equal(const std::string_view& key, FlagsByName::const_iterator candidate);

        [[nodiscard]]
        static std::optional<Match> check_partial(const std::string_view& key, FlagsByName::const_iterator candidate);

        FlagsByName const& flags_;
    };

    // Parser combinator which recognize source files as a single argument of a compiler call.
    struct SourceMatcher {
        [[nodiscard]]
        static rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input);
    };

    // A parser combinator, which recognize a single compiler flag without any conditions.
    struct EverythingElseFlagMatcher {
        [[nodiscard]]
        static rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input);
    };

    // A parser combinator, which takes multiple parsers and executes them
    // util one returns successfully and returns that as result. If none of
    // the parser returns success, it fails.
    template <typename ... Parsers>
    struct OneOf {
        using container_type = typename std::tuple<Parsers...>;
        container_type const parsers;

        explicit constexpr OneOf(Parsers const& ...p) noexcept
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

        explicit constexpr Repeat(Parser p) noexcept
                : parser(std::move(p))
        { }

        [[nodiscard]]
        result_type parse(Input input) const
        {
            CompilerFlags flags;
            while (!input.empty()) {
                auto result = parser.parse(input)
                        .on_success([&flags, &input](const auto& tuple) {
                            const auto& [flag, remainder] = tuple;
                            flags.push_back(flag);
                            input = remainder;
                        });
                if (result.is_err()) {
                    break;
                }
            }
            return (input.empty())
                   ? result_type(rust::Ok(flags))
                   : result_type(rust::Err(input));
        }
    };

    template <typename Parser>
    rust::Result<CompilerFlags> parse(const Parser &parser, const Arguments &arguments)
    {
        if (arguments.empty()) {
            return rust::Err(std::runtime_error("Failed to recognize: no arguments found."));
        }

        Input input(arguments);
        return parser.parse(input)
                .template map_err<std::runtime_error>([](auto remainder) {
                    return std::runtime_error(
                            fmt::format("Failed to recognize: {}", remainder.to_string())
                    );
                });
    }
}
