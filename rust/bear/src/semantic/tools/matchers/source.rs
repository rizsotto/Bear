/*  Copyright (C) 2012-2024 by László Nagy
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

use lazy_static::lazy_static;
use std::collections::HashSet;

#[cfg(target_family = "unix")]
pub fn looks_like_a_source_file(argument: &str) -> bool {
    // not a command line flag
    if argument.starts_with('-') {
        return false;
    }
    if let Some((_, extension)) = argument.rsplit_once('.') {
        return EXTENSIONS.contains(extension);
    }
    false
}

#[cfg(target_family = "windows")]
pub fn looks_like_a_source_file(argument: &str) -> bool {
    // not a command line flag
    if argument.starts_with('/') {
        return false;
    }
    if let Some((_, extension)) = argument.rsplit_once('.') {
        return EXTENSIONS.contains(extension);
    }
    false
}

lazy_static! {
    static ref EXTENSIONS: HashSet<&'static str> = {
        HashSet::from([
            // header files
            "h", "hh", "H", "hp", "hxx", "hpp", "HPP", "h++", "tcc",
            // C
            "c", "C",
            // C++
            "cc", "CC", "c++", "C++", "cxx", "cpp", "cp",
            // CUDA
            "cu",
            // ObjectiveC
            "m", "mi", "mm", "M", "mii",
            // Preprocessed
            "i", "ii",
            // Assembly
            "s", "S", "sx", "asm",
            // Fortran
            "f", "for", "ftn",
            "F", "FOR", "fpp", "FPP", "FTN",
            "f90", "f95", "f03", "f08",
            "F90", "F95", "F03", "F08",
            // go
            "go",
            // brig
            "brig",
            // D
            "d", "di", "dd",
            // Ada
            "ads", "abd"
        ])
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_filenames() {
        assert!(looks_like_a_source_file("source.c"));
        assert!(looks_like_a_source_file("source.cpp"));
        assert!(looks_like_a_source_file("source.cxx"));
        assert!(looks_like_a_source_file("source.cc"));

        assert!(looks_like_a_source_file("source.h"));
        assert!(looks_like_a_source_file("source.hpp"));

        assert!(!looks_like_a_source_file("gcc"));
        assert!(!looks_like_a_source_file("clang"));
        assert!(!looks_like_a_source_file("-o"));
        assert!(!looks_like_a_source_file("-Wall"));
        assert!(!looks_like_a_source_file("/o"));
    }
}
