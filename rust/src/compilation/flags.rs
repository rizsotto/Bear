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

use std::collections;

lazy_static! {
        /// Map of ignored compiler option for the creation of a compilation database.
        /// This map is used in split_command method, which classifies the parameters
        /// and ignores the selected ones. Please note that other parameters might be
        /// ignored as well.
        ///
        /// Option names are mapped to the number of following arguments which should
        /// be skipped.
        static ref IGNORED_FLAGS: collections::BTreeMap<&'static str, u8> = {
            let mut m = collections::BTreeMap::new();
            // preprocessor macros, ignored because would cause duplicate entries in
            // the output (the only difference would be these flags). this is actual
            // finding from users, who suffered longer execution time caused by the
            // duplicates.
            m.insert("-MD",         0u8);
            m.insert("-MMD",        0u8);
            m.insert("-MG",         0u8);
            m.insert("-MP",         0u8);
            m.insert("-MF",         1u8);
            m.insert("-MT",         1u8);
            m.insert("-MQ",         1u8);
            // linker options, ignored because for compilation database will contain
            // compilation commands only. so, the compiler would ignore these flags
            // anyway. the benefit to get rid of them is to make the output more
            // readable.
            m.insert("-static",     0u8);
            m.insert("-shared",     0u8);
            m.insert("-s",          0u8);
            m.insert("-rdynamic",   0u8);
            m.insert("-l",          1u8);
            m.insert("-L",          1u8);
            m.insert("-u",          1u8);
            m.insert("-z",          1u8);
            m.insert("-T",          1u8);
            m.insert("-Xlinker",    1u8);
            // clang-cl / msvc cl specific flags
            // consider moving visual studio specific warning flags also
            m.insert("-nologo",     0u8);
            m.insert("-EHsc",       0u8);
            m.insert("-EHa",        0u8);
            m
        };

        /// Typical linker flags also not really needed for a compilation.
        static ref LINKER_FLAG: regex::Regex =
            regex::Regex::new(r"^-(l|L|Wl,).+").unwrap();
    }

pub struct FlagIterator {
    inner: Box<Iterator<Item = String>>,
}

impl FlagIterator {
    pub fn from(collection: Vec<String>) -> Self {
        Self {
            inner: Box::new(collection.into_iter()),
        }
    }
}

impl Iterator for FlagIterator {
    type Item = String;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        while let Some(flag) = self.inner.next() {
            // Skip flags which matches from the given map.
            if let Some(skip) = IGNORED_FLAGS.get(flag.as_str()) {
                for _ in 0..*skip {
                    self.inner.next();
                }
                return self.next();
                // Skip linker flags too.
            } else if LINKER_FLAG.is_match(flag.as_str()) {
                return self.next();
            } else {
                return Some(flag);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_ignored_eq(expected: &[&str], input: &[&str]) {
        let input_vec: Vec<String> = input.iter().map(|str| str.to_string()).collect();
        let expected_vec: Vec<String> = expected.iter().map(|str| str.to_string()).collect();

        let mut sut = FlagIterator::from(input_vec);
        let result: Vec<_> = sut.collect();
        assert_eq!(expected_vec, result);
    }

    #[test]
    fn test_empty() {
        assert_ignored_eq(&[], &[]);
    }

    #[test]
    fn test_not_skip() {
        assert_ignored_eq(&["a", "b", "c"], &["a", "b", "c"]);
        assert_ignored_eq(&["-a", "-b", "-c"], &["-a", "-b", "-c"]);
        assert_ignored_eq(&["/a", "/b", "/c"], &["/a", "/b", "/c"]);
    }

    #[test]
    fn test_skip_given_flags() {
        assert_ignored_eq(&["a", "b"], &["a", "-MD", "b"]);
        assert_ignored_eq(&["a", "b"], &["a", "-MMD", "b"]);
        assert_ignored_eq(&["a", "b"], &["a", "-MF", "file", "b"]);

        assert_ignored_eq(&["a", "b"], &["a", "-MG", "-MT", "skip", "b"]);
        assert_ignored_eq(&["a", "b", "c"], &["a", "-MG", "b", "-MT", "skip", "c"]);
    }

    #[test]
    fn test_skip_linker_flags() {
        assert_ignored_eq(&["a", "b"], &["a", "-live", "b"]);
        assert_ignored_eq(&["a", "b"], &["a", "-L/path", "b"]);
        assert_ignored_eq(&["a", "b"], &["a", "-Wl,option", "b"]);

        assert_ignored_eq(&["a", "b"], &["a", "-live", "-L/path", "b"]);
    }
}
