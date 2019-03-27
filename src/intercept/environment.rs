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

use super::InterceptMode;

pub const KEY_CC: &str = "INTERCEPT_CC";
pub const KEY_CXX: &str = "INTERCEPT_CXX";

pub const KEY_PARENT: &str = "INTERCEPT_PARENT_PID";

pub const KEY_LIBRARY: &str = "INTERCEPT_SESSION_LIBRARY";
pub const KEY_REPORTER: &str = "INTERCEPT_REPORT_COMMAND";
pub const KEY_DESTINATION: &str = "INTERCEPT_REPORT_DESTINATION";
pub const KEY_VERBOSE: &str = "INTERCEPT_VERBOSE";


pub type Environment = std::collections::HashMap<String, String>;

pub struct EnvironmentBuilder {}

impl EnvironmentBuilder {

    fn new() -> EnvironmentBuilder {
        unimplemented!()
    }

    fn from(_environment: &Environment) -> EnvironmentBuilder {
        unimplemented!()
    }

    fn build(&self) -> Environment {
        unimplemented!()
    }

    fn with_mode(&mut self, _mode: &InterceptMode) -> &mut EnvironmentBuilder {
        unimplemented!()
    }

    fn with_modes(&mut self, modes: &[InterceptMode]) -> &mut EnvironmentBuilder {
        for mode in modes {
            self.with_mode(mode);
        }
        self
    }

    fn with_verbose(&mut self, _verbose: bool) -> &mut EnvironmentBuilder {
        unimplemented!()
    }

    fn with_destination(&mut self, _destination: &std::path::Path) -> &mut EnvironmentBuilder {
        unimplemented!()
    }
}
