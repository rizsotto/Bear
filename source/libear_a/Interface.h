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

     constexpr char destination_flag[]    = "--report-destination";
     constexpr char verbose_flag[]        = "--report-verbose";
     constexpr char library_flag[]        = "--session-library";
     constexpr char cc_flag[]             = "--session-wrapper-cc";
     constexpr char cxx_flag[]            = "--session-wrapper-cxx";
     constexpr char file_flag[]           = "--exec-file";
     constexpr char search_flag[]         = "--exec-search_path";
     constexpr char command_flag[]        = "--exec-command";

     constexpr char reporter_env_key[]    = "EAR_REPORT_COMMAND";
     constexpr char destination_env_key[] = "EAR_REPORT_DESTINATION";
     constexpr char verbose_env_key[]     = "EAR_REPORT_VERBOSE";
     constexpr char library_env_key[]     = "EAR_SESSION_LIBRARY";
     constexpr char cc_env_key[]          = "EAR_SESSION_WRAPPER_CC";
     constexpr char cxx_env_key[]         = "EAR_SESSION_WRAPPER_CXX";

 }