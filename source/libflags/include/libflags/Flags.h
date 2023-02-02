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

#pragma once

#include "config.h"
#include "libresult/Result.h"

#include <list>
#include <map>
#include <optional>
#include <string_view>
#include <vector>
#include <iosfwd>
#include <optional>

namespace flags {

    constexpr char HELP[] = "--help";
    constexpr char VERSION[] = "--version";
    constexpr char VERBOSE[] = "--verbose";
    constexpr char COMMAND[] = "command";

    class Parser;

    // Represents a successful parsing result.
    //
    // Instance can be created by the `Parser` object `parse` method. The flag
    // values can be queried by the `as_*` methods, which are returning result
    // objects.
    //
    // The object is hold references to the parser input.
    class Arguments {
    public:
        [[nodiscard]] rust::Result<bool> as_bool(const std::string_view& key) const;
        [[nodiscard]] rust::Result<std::string_view> as_string(const std::string_view& key) const;
        [[nodiscard]] rust::Result<std::vector<std::string_view>> as_string_list(const std::string_view& key) const;

    public:
        NON_DEFAULT_CONSTRUCTABLE(Arguments)

    private:
        using Parameter = std::vector<std::string_view>;
        using Parameters = std::map<std::string_view, Parameter>;

        friend class Parser;
        friend std::ostream& operator<<(std::ostream&, const Arguments&);

        Arguments(std::string_view program, Parameters&& parameters);

    private:
        std::string_view program_;
        Parameters parameters_;
    };

    std::ostream& operator<<(std::ostream&, const Arguments&);

    // Represent instruction how the associated parsing option shall be interpreted.
    //
    // `arguments` tells how many argument it has.
    //    - negative value represent zero or more.
    //    - zero value represent zero
    //    - positive value represent exact number of arguments
    // `required` tells that it is a mandatory option.
    // `help` is a short message about the option.
    // `default_value` is a string representation of the value it will have if the
    //    user was not given any.
    // `group_name` is a label like name, which is used to group flags and option
    //    which are semantically belongs together.
    struct Option {
        int arguments;
        bool required;
        const std::string_view help;
        const std::optional<std::string_view> default_value;
        const std::optional<std::string_view> group_name;
    };

    using OptionMap = std::map<std::string_view, Option>;
    using OptionValue = OptionMap::value_type;

    // Represents a command line parser.
    //
    // Why write another one when `getopt` is available. Simply because `getopt` is
    // not standard enough across operating systems.
    //
    // Usage of the parser is the following:
    // - Create it on the stack. (Make sure all passed parameter outlives the parser)
    // - Call the `parse` or `parse_or_exit` method. (Can call the same parser multiple
    //   times with different arguments. Note that the result `Arguments` object will
    //   holds reference to the input.)
    //
    // Functionalities:
    // - It adds `--help` flag automatically to every parser. Which will produce a
    //   usage description in case of `parse_or_exit` method is called.
    // - It adds `--version` flag, which will produce a simple output if `parse_or_exit`
    //   method is called.
    // - It adds `--verbose` flag automatically to every parser. Which will appear in
    //   the result `Arguments` object.
    // - Sub-command can be created by passing parser objects.
    class Parser {
    public:
        Parser(std::string_view name, std::string_view version, std::initializer_list<OptionValue> options);
        Parser(std::string_view name, std::initializer_list<OptionValue> options);
        Parser(std::string_view name, std::string_view version, std::initializer_list<Parser> commands, std::initializer_list<OptionValue> default_options = {});

        ~Parser() = default;

        rust::Result<Arguments> parse(int argc, const char** argv) const;
        rust::Result<Arguments> parse_or_exit(int argc, const char** argv) const;

        void print_help(const Parser*, std::ostream&) const;
        void print_usage(const Parser*, std::ostream&) const;

        void print_version(std::ostream&) const;

    public:
        NON_DEFAULT_CONSTRUCTABLE(Parser)

    private:
        const std::string_view name_;
        const std::string_view version_;
        OptionMap options_;
        std::list<Parser> commands_;
    };
}
