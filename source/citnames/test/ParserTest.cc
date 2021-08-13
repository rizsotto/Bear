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

#include "gtest/gtest.h"

#include "semantic/Parsers.h"

using namespace cs::semantic;

namespace cs::semantic {

    std::ostream &operator<<(std::ostream &os, const CompilerFlag &value) {
        os << '[';
        for (const auto &v : value.arguments) {
            os << v << ',';
        }
        os << ']';
        return os;
    }

    bool operator==(const CompilerFlag &lhs, const CompilerFlag &rhs) {
        return (lhs.arguments == rhs.arguments) && (lhs.type == rhs.type);
    }
}

namespace {

    TEST(Parser, EverythingElseFlagMatcher) {
        const std::list<std::string> input = {"compiler", "this", "is", "all", "parameter"};

        const auto parser = Repeat(EverythingElseFlagMatcher());
        const auto flags = parse(parser, input);

        EXPECT_TRUE(flags.is_ok());

        const CompilerFlags expected = {
                CompilerFlag { .arguments = {"this"}, .type = CompilerFlagType::LINKER_OBJECT_FILE },
                CompilerFlag { .arguments = {"is"}, .type = CompilerFlagType::LINKER_OBJECT_FILE },
                CompilerFlag { .arguments = {"all"}, .type = CompilerFlagType::LINKER_OBJECT_FILE },
                CompilerFlag { .arguments = {"parameter"}, .type = CompilerFlagType::LINKER_OBJECT_FILE },
        };
        EXPECT_EQ(expected, flags.unwrap());
    }

    TEST(Parser, SourceMatcher) {
        const std::list<std::string> input = {"compiler", "source1.c", "source2.c", "source1.c"};

        const auto parser = Repeat(SourceMatcher());
        const auto flags = parse(parser, input);

        EXPECT_TRUE(flags.is_ok());

        const CompilerFlags expected = {
                CompilerFlag { .arguments = {"source1.c"}, .type = CompilerFlagType::SOURCE },
                CompilerFlag { .arguments = {"source2.c"}, .type = CompilerFlagType::SOURCE },
                CompilerFlag { .arguments = {"source1.c"}, .type = CompilerFlagType::SOURCE },
        };
        EXPECT_EQ(expected, flags.unwrap());
    }

    TEST(Parser, FlagParser) {
        const std::list<std::string> input = {
                "compiler",
                "-a",
                "-belle",
                "-c",
                "-copilot",
                "-d", "option",
                "-e", "option",
                "-e-key", "value",
                "-f", "option",
                "-f-key", "value",
                "--d", "option",
                "--e=option",
                "--e", "option",
                "--e-key=value",
                "--e-key", "value",
                "--f=option",
                "--f", "option",
                "--f-key=value",
                "--f-key", "value",
        };

        const FlagsByName flags_by_name = {
                {"-a",  {Instruction(0, Match::EXACT, false),   CompilerFlagType::OTHER}},
                {"-b",  {Instruction(0, Match::PARTIAL, false), CompilerFlagType::OTHER}},
                {"-c",  {Instruction(0, Match::BOTH, false),    CompilerFlagType::OTHER}},
                {"-d", {Instruction(1, Match::EXACT, false),   CompilerFlagType::OTHER}},
                {"-e", {Instruction(1, Match::PARTIAL, false), CompilerFlagType::OTHER}},
                {"-f", {Instruction(1, Match::BOTH, false),    CompilerFlagType::OTHER}},
                {"--d", {Instruction(1, Match::EXACT, true),    CompilerFlagType::OTHER}},
                {"--e", {Instruction(1, Match::PARTIAL, true),  CompilerFlagType::OTHER}},
                {"--f", {Instruction(1, Match::BOTH, true),     CompilerFlagType::OTHER}},
        };
        const auto parser = Repeat(FlagParser(flags_by_name));
        const auto flags = parse(parser, input);

        if (flags.is_err()) {
            std::cout << flags.unwrap_err().what() << std::endl;
        }
        EXPECT_TRUE(flags.is_ok());

        const CompilerFlags expected = {
                CompilerFlag { .arguments = {"-a"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-belle"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-c"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-copilot"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-d", "option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-e", "option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-e-key", "value"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-f", "option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"-f-key", "value"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--d", "option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--e=option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--e", "option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--e-key=value"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--e-key", "value"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--f=option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--f", "option"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--f-key=value"}, .type = CompilerFlagType::OTHER },
                CompilerFlag { .arguments = {"--f-key", "value"}, .type = CompilerFlagType::OTHER },
        };
        EXPECT_EQ(expected, flags.unwrap());
    }
}