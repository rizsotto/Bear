/*  Copyright (C) 2012-2017 by László Nagy
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

namespace pear {
    namespace flag {

        constexpr char verbose[]        = "--verbose";
        constexpr char destination[]    = "--report-destination";
        constexpr char library[]        = "--session-library";
        constexpr char wrapper[]        = "--session-wrapper";
        constexpr char file[]           = "--exec-file";
        constexpr char search_path[]    = "--exec-search_path";
        constexpr char command[]        = "--exec-command";

    }

    namespace env {

        constexpr char reporter_key[]    = "INTERCEPT_REPORT_COMMAND";
        constexpr char destination_key[] = "INTERCEPT_REPORT_DESTINATION";
        constexpr char verbose_key[]     = "INTERCEPT_VERBOSE";
        constexpr char library_key[]     = "INTERCEPT_SESSION_LIBRARY";
        constexpr char cc_key[]          = "INTERCEPT_SESSION_CC";
        constexpr char cxx_key[]         = "INTERCEPT_SESSION_CXX";

    }

    struct Execution {
        const char **command;
        const char *file;
        const char *search_path;
    };

    struct Context {
        char const *reporter;
        char const *destination;
        bool verbose;
    };

}