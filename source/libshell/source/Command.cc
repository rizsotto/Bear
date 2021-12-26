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

#include "libshell/Command.h"

#include <regex>
#include <stdexcept>

namespace sh {

    std::string escape(const std::string& input)
    {
        if (input.empty()) {
            return "''";
        }

        const std::regex ESCAPE_PATTERN(R"#(([^A-Za-z0-9_\-.,:/@\n]))#");
        const std::regex LINE_FEED(R"#(\n)#");

        const auto output = std::regex_replace(input, ESCAPE_PATTERN, "\\$1");
        return std::regex_replace(output, LINE_FEED, "'\n'");
    }

    std::string join(const std::list<std::string>& arguments)
    {
        std::string result;
        for (auto it = arguments.begin(); it != arguments.end(); ++it) {
            if (it != arguments.begin()) {
                result += " ";
            }
            result += escape(*it);
        }
        return result;
    }

    rust::Result<std::list<std::string>> split(const std::string& input)
    {
        const std::regex MAIN_PATTERN(R"#((?:\s*(?:([^\s\\'"]+)|'([^']*)'|"((?:[^"\\]|\\.)*)"|(\\.?)|(\S))(\s|$)?))#",
                                      std::regex::ECMAScript);
        const std::regex ESCAPE_PATTERN(R"#(\\(.))#");
        const std::regex METACHAR_PATTERN(R"(\\([$`"\\\n]))");

        std::list<std::string> words;
        std::string field;

        const auto input_begin = std::sregex_iterator(input.begin(), input.end(), MAIN_PATTERN);
        const auto input_end = std::sregex_iterator();
        for (auto it = input_begin; it != input_end; ++it) {
            if (it->ready()) {
                if (it->operator[](1).matched) {
                    field += it->str(1);
                } else if (it->operator[](2).matched) {
                    field += it->str(2);
                } else if (it->operator[](3).matched) {
                    field += std::regex_replace(it->str(3), METACHAR_PATTERN, "$1");
                } else if (it->operator[](4).matched) {
                    field += std::regex_replace(it->str(4), ESCAPE_PATTERN, "$1");
                } else if (it->operator[](5).matched) {
                    return rust::Err(std::runtime_error("Mismatched quotes."));
                }

                if (it->operator[](6).matched) {
                    words.push_back(field);
                    field.clear();
                }
            }
        }
        return rust::Ok(std::move(words));
    }
}
