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

use std::path;

/// Represents a compilation database building strategy.
pub struct Config {
    pub format: Format,
    pub append_to_existing: bool,
    pub include_headers: bool,      // TODO
    pub include_linking: bool,
    pub relative_to: Option<path::PathBuf>, // TODO
    pub compilers: CompilerFilter,  // TODO
    pub sources: SourceFilter,      // TODO
    pub flags: FlagFilter,          // TODO
}

impl Default for Config {
    fn default() -> Self {
        Config {
            format: Format::default(),
            append_to_existing: false,
            include_headers: false,
            include_linking: false,
            relative_to: None,
            compilers: CompilerFilter::default(),
            sources: SourceFilter::default(),
            flags: FlagFilter::default(),
        }
    }
}

/// Represents the expected format of the JSON compilation database.
pub struct Format {
    pub command_as_array: bool,
    pub drop_output_field: bool,            // TODO
}

impl Default for Format {
    fn default() -> Self {
        Format {
            command_as_array: true,
            drop_output_field: false,
        }
    }
}

pub struct CompilerFilter {
    pub drop_wrapper: bool,                 // TODO
//    c_compilers: Vec<String>,
//    cxx_compilers: Vec<String>,
}

impl Default for CompilerFilter {
    fn default() -> Self {
        CompilerFilter {
            drop_wrapper: true,
        }
    }
}

pub struct FlagFilter {

}

impl Default for FlagFilter {
    fn default() -> Self {
        unimplemented!()
    }
}

pub struct SourceFilter {
    pub extensions_to_exclude: Vec<String>,
    pub extensions_to_include: Vec<String>,
    pub path_to_exclude: Vec<std::path::PathBuf>,
    pub path_to_include: Vec<std::path::PathBuf>,
}

impl Default for SourceFilter {
    fn default() -> Self {
        unimplemented!()
    }
}
