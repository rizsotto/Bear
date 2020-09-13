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

#pragma once

#include "libresult/Result.h"

#include <string>
#include <list>

namespace sh {

    // Escapes a string so it will be interpreted as a single word by the UNIX Bourne shell.
    //
    // If the input string is empty, this function returns an empty quoted string.
    std::string escape(const std::string& input);

    // Builds a command line string from a list of arguments.
    //
    // The arguments are combined into a single string with each word separated by a space.
    // Each individual word is escaped as necessary via `escape`.
    std::string join(const std::list<std::string>& arguments);

    // Splits a string into a vector of words in the same way the UNIX Bourne shell does.
    //
    // This function does not behave like a full command line parser. Only single quotes,
    // double quotes, and backslashes are treated as metacharacters. Within double quoted
    // strings, backslashes are only treated as metacharacters when followed by one of the
    // following characters:
    //
    // * $
    // * `
    // * "
    // * backslash
    // * newline
    //
    // The pipe character has no special meaning.
    //
    // If the input contains mismatched quotes (a quoted string missing a matching ending
    // quote), an error is returned.
    rust::Result<std::list<std::string>> split(const std::string& input);
}
