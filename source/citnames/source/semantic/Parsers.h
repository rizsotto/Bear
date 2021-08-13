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

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

namespace cs::semantic {

    using namespace domain;

    // Represents command line arguments.
    using Arguments = std::list<std::string>;

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

        [[nodiscard]] iterator begin() const {
            return begin_;
        }

        [[nodiscard]] iterator end() const {
            return end_;
        }

        [[nodiscard]] bool empty() const {
            return (begin_ == end_);
        }

        [[nodiscard]] std::tuple<Arguments, Input> take(size_t count) const {
            auto end = std::next(begin_, count);

            auto arguments = Arguments(begin_, end);
            auto remainder = Input(end, end_);

            return std::make_tuple(std::move(arguments), std::move(remainder));
        }

    private:
        iterator begin_;
        iterator end_;
    };

    enum class Match {
        EXACT,
        PARTIAL,
        BOTH,
    };

    struct Instruction {

        constexpr Instruction(const uint8_t count, const Match match, const bool equal) noexcept
                : count_(count)
                , match_exact_((match == Match::EXACT || match == Match::BOTH) ? 1u : 0u)
                , match_partial_((match == Match::PARTIAL || match == Match::BOTH) ? 1u : 0u)
                , equal_sign_(equal ? 1u : 0u)
        { }

        [[nodiscard]] constexpr size_t count(bool option_attached) const {
            if ((count_ > 0) && equal()) {
                return option_attached ? (count_ - 1) : count_;
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
            return (equal_sign_ == 1u);
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

        [[nodiscard]]
        static rust::Result<std::pair<CompilerFlag, Input>, Input> parse(const Input &input) {
            const auto candidate = take_extension(*input.begin());
            for (auto extension : EXTENSIONS) {
                if (candidate == extension) {
                    auto [arguments, remainder] = input.take(1);
                    auto flag = CompilerFlag { .arguments = arguments, .type = CompilerFlagType::SOURCE };
                    return rust::Ok(std::make_pair(flag, remainder));
                }
            }
            return rust::Err(input);
        }

        [[nodiscard]]
        static std::string_view take_extension(const std::string_view& file) {
            auto pos = file.rfind('.');
            return (pos == std::string::npos) ? file : file.substr(pos);
        }
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
        result_type parse(const Input &input) const
        {
            CompilerFlags flags;
            Input it = input;
            while (!it.empty()) {
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
            return (it.empty())
                   ? result_type(rust::Ok(flags))
                   : result_type(rust::Err(it));
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
                            fmt::format("Failed to recognize: {}",
                                        fmt::join(remainder.begin(), remainder.end(), ", ")));
                });
    }
}
