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

    ArgumentsView slice(const Arguments &input, size_t start, size_t stop = 0) {
        const auto begin = std::next(input.begin(), start);
        const auto end = (stop < start)
                ? std::next(begin)
                : std::next(input.begin(), stop);
        return ArgumentsView(begin, end);
    }
}

namespace {

    TEST(Parser, EverythingElseFlagMatcher) {
        const auto sut = Repeat(EverythingElseFlagMatcher());

        const Arguments input = {"compiler", "this", "is", "all", "parameter"};
        const auto flags = parse(sut, input);
        EXPECT_TRUE(flags.is_ok());
        const CompilerFlags expected = {
                CompilerFlag{slice(input, 1), CompilerFlagType::LINKER_OBJECT_FILE},
                CompilerFlag{slice(input, 2), CompilerFlagType::LINKER_OBJECT_FILE},
                CompilerFlag{slice(input, 3), CompilerFlagType::LINKER_OBJECT_FILE},
                CompilerFlag{slice(input, 4), CompilerFlagType::LINKER_OBJECT_FILE},
        };
        EXPECT_EQ(expected, flags.unwrap());
    }

    TEST(Parser, SourceMatcher) {
        const auto sut = Repeat(SourceMatcher());

        {
            const Arguments input = {"compiler", "source1.c", "source2.c", "source1.c"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::SOURCE},
                    CompilerFlag{slice(input, 2), CompilerFlagType::SOURCE},
                    CompilerFlag{slice(input, 3), CompilerFlagType::SOURCE},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "source1.f", "source2.f95", "source1.f08"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::SOURCE},
                    CompilerFlag{slice(input, 2), CompilerFlagType::SOURCE},
                    CompilerFlag{slice(input, 3), CompilerFlagType::SOURCE},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
    }

    TEST(Parser, parse_flags_with_separate_options) {
        const FlagsByName flags_by_name = {
                {"-a", {MatchInstruction::EXACTLY,                CompilerFlagType::OTHER}},
                {"-b", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::OTHER}},
                {"-c", {MatchInstruction::EXACTLY_WITH_2_OPTS,    CompilerFlagType::OTHER}},
                {"-d", {MatchInstruction::EXACTLY_WITH_3_OPTS,    CompilerFlagType::OTHER}},
        };
        const auto sut = Repeat(FlagParser(flags_by_name));

        {
            const Arguments input = {"compiler", "-a", "-b", "op1", "-c", "op1", "op2", "-d", "op1", "op2", "op3"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 2, 4), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 4, 7), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 7, 11), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-a", "op1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
        {
            const Arguments input = {"compiler", "-b"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
        {
            const Arguments input = {"compiler", "-c", "op1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
        {
            const Arguments input = {"compiler", "-b", "op1", "op2"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
    }

    TEST(Parser, parse_flags_with_glued_options) {
        const FlagsByName flags_by_name = {
                {"-a", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP, CompilerFlagType::OTHER}},
                {"-b", {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ,                   CompilerFlagType::OTHER}},
                {"-c", {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_EQ_OR_SEP,            CompilerFlagType::OTHER}},
                {"-d", {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED,                           CompilerFlagType::OTHER}},
                {"-e", {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP,                    CompilerFlagType::OTHER}},
                {"-f", {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_WITH_OR_WITHOUT_EQ_OR_SEP, CompilerFlagType::OTHER}},
        };
        const auto sut = Repeat(FlagParser(flags_by_name));

        {
            const Arguments input = {"compiler", "-a", "op1", "-c", "op1", "-e", "op1", "-f", "op1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1, 3), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 3, 5), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 5, 7), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 7, 9), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-b=op1", "-c=op1", "-f=op1" };
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 2), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 3), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-dop1", "-eop1", "-fop1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 2), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 3), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-aopt1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
        {
            const Arguments input = {"compiler", "-a=opt1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
        {
            const Arguments input = {"compiler", "-b", "opt1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
        {
            const Arguments input = {"compiler", "-a"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
    }

    TEST(Parser, parse_flags_with_partial_matches) {
        const FlagsByName flags_by_name = {
                {"-a", {MatchInstruction::PREFIX,             CompilerFlagType::OTHER}},
                {"-b", {MatchInstruction::PREFIX_WITH_1_OPT,  CompilerFlagType::OTHER}},
                {"-c", {MatchInstruction::PREFIX_WITH_2_OPTS, CompilerFlagType::OTHER}},
                {"-d", {MatchInstruction::PREFIX_WITH_3_OPTS, CompilerFlagType::OTHER}},
        };
        const auto sut = Repeat(FlagParser(flags_by_name));

        {
            const Arguments input = {"compiler", "-a", "-b", "op1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 2, 4), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-alice", "-bob", "op1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 2, 4), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-cecil", "opt1", "opt2", "-dave", "opt1", "opt2", "opt3"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1, 4), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 4, 8), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-alice=op1", "-bob=op1", "op2" };
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 2, 4), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-f=op1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
        {
            const Arguments input = {"compiler", "-a=op1"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
    }

    TEST(Parser, parse_flags_with_common_prefixes) {
        const FlagsByName flags_by_name = {
                {"-a", {MatchInstruction::EXACTLY, CompilerFlagType::OTHER}},
                {"-l", {MatchInstruction::EXACTLY_WITH_1_OPT_GLUED_OR_SEP, CompilerFlagType::LINKER}},
                {"-language", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,  CompilerFlagType::OTHER}},
                {"-linker", {MatchInstruction::EXACTLY, CompilerFlagType::OTHER}},
                {"-z", {MatchInstruction::EXACTLY, CompilerFlagType::OTHER}},
        };
        const auto sut = Repeat(FlagParser(flags_by_name));

        {
            const Arguments input = {"compiler", "-library", "-language", "c"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::LINKER},
                    CompilerFlag{slice(input, 2, 4), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-language", "c", "-library"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1, 3), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 3), CompilerFlagType::LINKER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-linker", "-lthing",};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 2), CompilerFlagType::LINKER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-l", "m", "-link", "-linker", "-lexec"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1, 3), CompilerFlagType::LINKER},
                    CompilerFlag{slice(input, 3), CompilerFlagType::LINKER},
                    CompilerFlag{slice(input, 4), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 5), CompilerFlagType::LINKER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
        {
            const Arguments input = {"compiler", "-l=thing"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_err());
        }
    }

    TEST(Parser, parse_flags_with_several_suitable_prefixes) {
        const FlagsByName flags_by_name = {
                {"-l", {MatchInstruction::PREFIX, CompilerFlagType::LINKER}},
                {"-language", {MatchInstruction::EXACTLY_WITH_1_OPT_SEP,  CompilerFlagType::OTHER}},
                {"-linker", {MatchInstruction::PREFIX_WITH_2_OPTS, CompilerFlagType::OTHER}},
        };
        const auto sut = Repeat(FlagParser(flags_by_name));

        {
            const Arguments input = {"compiler", "-lin", "-language", "s", "-linkeriasds", "opt1", "opt2"};
            const auto flags = parse(sut, input);
            EXPECT_TRUE(flags.is_ok());
            const CompilerFlags expected = {
                    CompilerFlag{slice(input, 1), CompilerFlagType::LINKER},
                    CompilerFlag{slice(input, 2, 4), CompilerFlagType::OTHER},
                    CompilerFlag{slice(input, 4, 7), CompilerFlagType::OTHER},
            };
            EXPECT_EQ(expected, flags.unwrap());
        }
    }
}
