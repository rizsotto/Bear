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

#include "gtest/gtest.h"

#include "libflags/Flags.h"

#include <sstream>
#include <iostream>

using namespace flags;

namespace {

    constexpr char HELP[] = "--help";
    constexpr char FLAG[] = "--flag";
    constexpr char OPTION[] = "--option";
    constexpr char OPTIONS[] = "--options";
    constexpr char SEPARATOR[] = "--";

    TEST(flags, parse_successful)
    {
        const char* argv[] = { "executable", FLAG, OPTION, "0", OPTIONS, "1", "2", "3", SEPARATOR, "4", "5" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser sut("test", "version", {
                {FLAG,      {0,  false, "a single flag",         std::nullopt, std::nullopt}},
                {OPTION,    {1,  false, "a flag with a value",   std::nullopt, std::nullopt}},
                {OPTIONS,   {3,  false, "a flag with 3 values",  std::nullopt, std::nullopt}},
                {SEPARATOR, {-1, false, "rest of the arguments", std::nullopt, std::nullopt}}
        });
        sut.parse(argc, const_cast<const char**>(argv))
            .on_success([](auto params) {
                EXPECT_TRUE(params.as_bool(HELP).is_ok());
                EXPECT_FALSE(params.as_bool(HELP).unwrap_or(true));

                EXPECT_TRUE(params.as_bool(FLAG).is_ok());
                EXPECT_TRUE(params.as_bool(FLAG).unwrap_or(true));

                auto option = params.as_string(OPTION);
                EXPECT_TRUE(option.is_ok());
                EXPECT_STREQ(option.unwrap_or("").data(), "0");

                std::vector<std::string_view> expected_options = { "1", "2", "3" };
                auto options = params.as_string_list(OPTIONS);
                EXPECT_TRUE(options.is_ok());
                EXPECT_EQ(expected_options, options.unwrap_or({}));

                std::vector<std::string_view> expected_separator = { "4", "5" };
                auto separator = params.as_string_list(SEPARATOR);
                EXPECT_TRUE(separator.is_ok());
                EXPECT_EQ(expected_separator, separator.unwrap_or({}));
            })
            .on_error([](auto) {
                EXPECT_FALSE(true);
            });
    }

    TEST(flags, parse_with_default_values)
    {
        const char* argv[] = { "executable" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser sut("test", "version", {
                {FLAG,   {0, false, "a single flag",       {"true"},     std::nullopt}},
                {OPTION, {1, false, "a flag with a value", {"42"},       std::nullopt}}
        });
        sut.parse(argc, const_cast<const char**>(argv))
            .on_success([](auto params) {
                EXPECT_TRUE(params.as_bool(HELP).is_ok());
                EXPECT_FALSE(params.as_bool(HELP).unwrap_or(true));

                EXPECT_TRUE(params.as_bool(FLAG).is_ok());
                EXPECT_TRUE(params.as_bool(FLAG).unwrap_or(true));

                auto option = params.as_string(OPTION);
                EXPECT_TRUE(option.is_ok());
                EXPECT_STREQ(option.unwrap_or("").data(), "42");
            })
            .on_error([](auto) {
                EXPECT_FALSE(true);
            });
    }

    TEST(flags, parse_fails_for_unkown_flags)
    {
        const char* argv[] = { "executable", FLAG, OPTION, "0" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser sut("test", "version", {
                {FLAG, {0, false, "a single flag", std::nullopt, std::nullopt}}
        });
        sut.parse(argc, const_cast<const char**>(argv))
            .on_success([](auto) {
                EXPECT_FALSE(true);
            })
            .on_error([](auto error) {
                EXPECT_TRUE(true);
                EXPECT_STREQ(error.what(), "Unrecognized parameter: \"--option\"");
            });
    }

    TEST(flags, parse_fails_for_not_enough_params)
    {
        const char* argv[] = { "executable", FLAG, OPTIONS, "1" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser sut("test", "version", {
                {FLAG,    {0, false, "a single flag",        std::nullopt, std::nullopt}},
                {OPTIONS, {3, false, "a flag with 3 values", std::nullopt, std::nullopt}}
        });
        sut.parse(argc, const_cast<const char**>(argv))
            .on_success([](auto) {
                EXPECT_FALSE(true);
            })
            .on_error([](auto error) {
                EXPECT_TRUE(true);
                EXPECT_STREQ(error.what(), "Not enough parameters for: \"--options\"");
            });
    }

    TEST(flags, parse_fails_for_required_parameters_missing)
    {
        const char* argv[] = { "executable", OPTIONS, "1", "2" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser sut("test", "version", {
                {OPTION,  {1, true,  "a flag with 1 value",  std::nullopt, std::nullopt}},
                {OPTIONS, {2, false, "a flag with 2 values", std::nullopt, std::nullopt}}
        });
        sut.parse(argc, const_cast<const char**>(argv))
            .on_success([](auto) {
                EXPECT_FALSE(true);
            })
            .on_error([](auto error) {
                EXPECT_TRUE(true);
                EXPECT_STREQ(error.what(), "Parameter is required, but not given: \"--option\"");
            });
    }

    TEST(flags, usage_for_simple_parser)
    {
        const Parser sut("test", "version", {
                {FLAG,      {0,  false, "a single flag",         std::nullopt, std::nullopt}},
                {OPTION,    {1,  false, "a flag with a value",   std::nullopt, std::nullopt}},
                {OPTIONS,   {3,  false, "a flag with 3 values",  std::nullopt, std::nullopt}},
                {SEPARATOR, {-1, false, "rest of the arguments", std::nullopt, std::nullopt}}
        });
        {
            const char *expected =
                    "Usage: test [--flag] [--option <arg>] [--options <arg0> <arg1> <arg2>] [--verbose] [-- ...]\n";

            std::ostringstream out;
            sut.print_usage(nullptr, out);
            EXPECT_EQ(
                    expected,
                    out.str()
            );
        }
        {
            const char *expected =
                    "Usage: test [--flag] [--option <arg>] [--options <arg0> <arg1> <arg2>] [--verbose] [-- ...]\n"
                    "\n"
                    "  --flag               a single flag\n"
                    "  --option <arg>       a flag with a value\n"
                    "  --options <arg0> <arg1> <arg2>\n"
                    "               a flag with 3 values\n"
                    "  --verbose            run in verbose mode\n"
                    "  -- ...               rest of the arguments\n"
                    "\n"
                    "query options\n"
                    "  --help               print help and exit\n"
                    "  --version            print version and exit\n";

            std::ostringstream out;
            sut.print_help(nullptr, out);
            EXPECT_EQ(expected, out.str());
        }
        {
            const char *expected =
                    "test version\n";

            std::ostringstream out;
            sut.print_version(out);
            EXPECT_EQ(expected, out.str());
        }
    }

    TEST(flags, parse_successful_subcommands)
    {
        const Parser append("append", {
                {OPTION,    {1,  false, "a flag with a value",   std::nullopt, std::nullopt}}
        });
        const Parser dump("dump", {
                {OPTIONS,   {3,  false, "a flag with 3 values",  std::nullopt, std::nullopt}}
        });
        const Parser sut("test", "version", { append, dump }, {
                {OPTION,    {1,  false, "a flag with a value",   std::nullopt, std::nullopt}}
        });
        {
            const char *argv[] = {"executable", "append", OPTION, "0"};
            const int argc = sizeof(argv) / sizeof(const char *);
            const auto result = sut.parse(argc, const_cast<const char **>(argv));
            EXPECT_TRUE(result.is_ok());
            result.on_success([](auto params) {
                        EXPECT_TRUE(params.as_bool(HELP).is_ok());
                        EXPECT_FALSE(params.as_bool(HELP).unwrap_or(true));

                        auto command = params.as_string(COMMAND);
                        EXPECT_TRUE(command.is_ok());
                        EXPECT_STREQ("append", command.unwrap_or("").data());

                        auto option = params.as_string(OPTION);
                        EXPECT_TRUE(option.is_ok());
                        EXPECT_STREQ(option.unwrap_or("").data(), "0");

                        auto options = params.as_string_list(OPTIONS);
                        EXPECT_TRUE(options.is_err());
                    })
                    .on_error([](auto) {
                        EXPECT_FALSE(true);
                    });
        }
        {
            const char* argv[] = {"executable", "dump", OPTIONS, "1", "2", "3"};
            const int argc = sizeof(argv) / sizeof(const char *);
            const auto result = sut.parse(argc, const_cast<const char **>(argv));
            EXPECT_TRUE(result.is_ok());
            result.on_success([](auto params) {
                        EXPECT_TRUE(params.as_bool(HELP).is_ok());
                        EXPECT_FALSE(params.as_bool(HELP).unwrap_or(true));

                        auto command = params.as_string(COMMAND);
                        EXPECT_TRUE(command.is_ok());
                        EXPECT_STREQ("dump", command.unwrap_or("").data());

                        auto option = params.as_string(OPTION);
                        EXPECT_TRUE(option.is_err());

                        std::vector<std::string_view> expected_options = { "1", "2", "3" };
                        auto options = params.as_string_list(OPTIONS);
                        EXPECT_TRUE(options.is_ok());
                        EXPECT_EQ(expected_options, options.unwrap_or({}));
                    })
                    .on_error([](auto) {
                        EXPECT_FALSE(true);
                    });
        }
        {
            const char* argv[] = {"executable", OPTION, "0"};
            const int argc = sizeof(argv) / sizeof(const char*);
            const auto result = sut.parse(argc, argv);
            EXPECT_TRUE(result.is_ok());
            result.on_success([](auto params) {
                        EXPECT_TRUE(params.as_bool(HELP).is_ok());
                        EXPECT_FALSE(params.as_bool(HELP).unwrap_or(true));

                        auto command = params.as_string(COMMAND);
                        EXPECT_TRUE(command.is_err());

                        auto option = params.as_string(OPTION);
                        EXPECT_TRUE(option.is_ok());
                        EXPECT_STREQ(option.unwrap_or("").data(), "0");

                        auto options = params.as_string_list(OPTIONS);
                        EXPECT_TRUE(options.is_err());
                    })
                    .on_error([](auto) {
                        EXPECT_FALSE(true);
                    });
        }
        {
            const char* argv[] = {"executable", "--help"};
            const int argc = sizeof(argv) / sizeof(const char *);
            const auto result = sut.parse(argc, const_cast<const char **>(argv));
            EXPECT_TRUE(result.is_ok());
            result.on_success([](auto params) {
                        EXPECT_TRUE(params.as_bool(HELP).is_ok());
                        EXPECT_TRUE(params.as_bool(HELP).unwrap_or(true));

                        EXPECT_TRUE(params.as_string(COMMAND).is_err());
                    })
                    .on_error([](auto) {
                        EXPECT_FALSE(true);
                    });
        }
        {
            const char* argv[] = {"executable", "append", "--help"};
            const int argc = sizeof(argv) / sizeof(const char *);
            const auto result = sut.parse(argc, const_cast<const char **>(argv));
            EXPECT_TRUE(result.is_ok());
            result.on_success([](auto params) {
                        EXPECT_TRUE(params.as_bool(HELP).is_ok());
                        EXPECT_TRUE(params.as_bool(HELP).unwrap_or(true));

                        auto command = params.as_string(COMMAND);
                        EXPECT_TRUE(command.is_ok());
                        EXPECT_STREQ("append", command.unwrap_or("").data());
                    })
                    .on_error([](auto) {
                        EXPECT_FALSE(true);
                    });
        }
        {
            const char* argv[] = {"executable", "--version"};
            const int argc = sizeof(argv) / sizeof(const char *);
            const auto result = sut.parse(argc, const_cast<const char **>(argv));
            EXPECT_TRUE(result.is_ok());
            result.on_success([](auto params) {
                        EXPECT_TRUE(params.as_bool(VERSION).is_ok());
                        EXPECT_TRUE(params.as_bool(VERSION).unwrap_or(true));
                    })
                    .on_error([](auto) {
                        EXPECT_FALSE(true);
                    });
        }
        {
            const char* argv[] = {"executable", "remove"};
            const int argc = sizeof(argv) / sizeof(const char *);
            const auto result = sut.parse(argc, const_cast<const char **>(argv));
            EXPECT_TRUE(result.is_err());
        }
    }

    TEST(flags, usage_for_sub_command_parser)
    {
        const Parser append("append", {
                {OPTION,    {1,  false, "a flag with a value",   std::nullopt, std::nullopt}}
        });
        const Parser dump("dump", {
                {OPTIONS,   {3,  false, "a flag with 3 values",  std::nullopt, std::nullopt}}
        });
        const Parser sut("test", "1.0", { append, dump });
        {
            const char *expected =
                    "Usage: test <command>\n";

            std::ostringstream out;
            sut.print_usage(nullptr, out);
            EXPECT_EQ(
                    expected,
                    out.str()
            );
        }
        {
            const char *expected =
                    "Usage: test <command>\n"
                    "\n"
                    "commands\n"
                    "  append\n"
                    "  dump\n"
                    "\n"
                    "query options\n"
                    "  --help               print help and exit\n"
                    "  --version            print version and exit\n";

            std::ostringstream out;
            sut.print_help(nullptr, out);
            EXPECT_EQ(expected, out.str());
        }
        {
            const char *expected =
                    "Usage: test append [--option <arg>] [--verbose]\n"
                    "\n"
                    "  --option <arg>       a flag with a value\n"
                    "  --verbose            run in verbose mode\n"
                    "\n"
                    "query options\n"
                    "  --help               print help and exit\n";

            std::ostringstream out;
            sut.print_help(&append, out);
            EXPECT_EQ(expected, out.str());
        }
        {
            const char *expected =
                    "test 1.0\n";

            std::ostringstream out;
            sut.print_version(out);
            EXPECT_EQ(expected, out.str());
        }
    }
}
