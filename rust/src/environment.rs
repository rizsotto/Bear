/*  Copyright (C) 2012-2018 by László Nagy
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

pub const KEY_CC: &'static str = "INTERCEPT_CC";
pub const KEY_CXX: &'static str = "INTERCEPT_CXX";

pub const KEY_PARENT: &'static str = "INTERCEPT_PARENT_PID";

pub const KEY_LIBRARY: &'static str = "INTERCEPT_SESSION_LIBRARY";
pub const KEY_REPORTER: &'static str = "INTERCEPT_REPORT_COMMAND";
pub const KEY_DESTINATION: &'static str = "INTERCEPT_REPORT_DESTINATION";
pub const KEY_VERBOSE: &'static str = "INTERCEPT_VERBOSE";
