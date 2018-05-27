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

 namespace ear {

     constexpr char verbose_flag[]        = "--verbose";
     constexpr char destination_flag[]    = "--report-destination";
     constexpr char library_flag[]        = "--session-library";
     constexpr char wrapper_flag[]        = "--session-wrapper";
     constexpr char file_flag[]           = "--exec-file";
     constexpr char search_flag[]         = "--exec-search_path";
     constexpr char command_flag[]        = "--exec-command";

     constexpr char reporter_env_key[]    = "INTERCEPT_REPORT_COMMAND";
     constexpr char destination_env_key[] = "INTERCEPT_REPORT_DESTINATION";
     constexpr char verbose_env_key[]     = "INTERCEPT_VERBOSE";
     constexpr char library_env_key[]     = "INTERCEPT_SESSION_LIBRARY";
     constexpr char cc_env_key[]          = "INTERCEPT_SESSION_CC";
     constexpr char cxx_env_key[]         = "INTERCEPT_SESSION_CXX";

 }