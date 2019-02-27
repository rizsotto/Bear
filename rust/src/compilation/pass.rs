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
use std::mem;

#[derive(Debug, PartialEq)]
pub enum CompilerPass {
    Preprocessor,
    Compilation,
    Linking,
    Internal,
}

impl Default for CompilerPass {
    fn default() -> CompilerPass {
        CompilerPass::Linking
    }
}

impl CompilerPass {

    /// Query method to get if the compilation pass was running or not.
    ///
    /// # Returns
    /// true if the compiler pass was running.
    pub fn is_compiling(&self) -> bool {
        self == &CompilerPass::Compilation || self == &CompilerPass::Linking
    }

    pub fn to_string(&self) -> String {
        // TODO!!!
        "-c".to_string()
    }

    /// Consume a single argument and update the compiler pass if the argument
    /// is one which influence it.
    ///
    /// # Arguments
    /// `string` the argument to evaluate.
    ///
    /// # Returns
    /// true if the argument change the compiler pass.
    pub fn take(&mut self, string: &str) -> bool {
        if let Some(pass) = PHASE_FLAGS.get(string) {
            self.update(pass);
            true
        } else {
            false
        }
    }

    fn update(&mut self, new_state: &CompilerPass) {
        match (&self, new_state) {
            (CompilerPass::Linking, CompilerPass::Internal) => {
                mem::replace(self, CompilerPass::Internal);
            }
            (CompilerPass::Linking, CompilerPass::Compilation) => {
                mem::replace(self, CompilerPass::Compilation);
            }
            (CompilerPass::Linking, CompilerPass::Preprocessor) => {
                mem::replace(self, CompilerPass::Preprocessor);
            }
            (CompilerPass::Compilation, CompilerPass::Internal) => {
                mem::replace(self, CompilerPass::Internal);
            }
            (CompilerPass::Compilation, CompilerPass::Preprocessor) => {
                mem::replace(self, CompilerPass::Preprocessor);
            }
            (CompilerPass::Preprocessor, CompilerPass::Internal) => {
                mem::replace(self, CompilerPass::Internal);
            }
            _ => (),
        }
    }
}

lazy_static! {
    static ref PHASE_FLAGS: collections::BTreeMap<&'static str, CompilerPass> = {
        let mut m = collections::BTreeMap::new();
        m.insert("-v", CompilerPass::Internal);
        m.insert("-###", CompilerPass::Internal);
        m.insert("-cc1", CompilerPass::Internal);
        m.insert("-cc1as", CompilerPass::Internal);
        m.insert("-E", CompilerPass::Preprocessor);
        m.insert("-M", CompilerPass::Preprocessor);
        m.insert("-MM", CompilerPass::Preprocessor);
        m.insert("-c", CompilerPass::Compilation);
        m.insert("-S", CompilerPass::Compilation);
        m
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stays_linker() {
        let mut sut: CompilerPass = Default::default();
        assert_eq!(CompilerPass::Linking, sut);

        assert_eq!(false, sut.take("--not_this"));
        assert_eq!(CompilerPass::Linking, sut);
    }

    #[test]
    fn test_compilation_updates_linker() {
        let mut sut: CompilerPass = Default::default();
        assert_eq!(CompilerPass::Linking, sut);

        assert_eq!(true, sut.take("-c"));
        assert_eq!(false, sut.take("--not_this"));
        assert_eq!(CompilerPass::Compilation, sut);
    }

    #[test]
    fn test_prepocessor_updates_linker() {
        let mut sut: CompilerPass = Default::default();
        assert_eq!(CompilerPass::Linking, sut);

        assert_eq!(true, sut.take("-E"));
        assert_eq!(false, sut.take("--not_this"));
        assert_eq!(CompilerPass::Preprocessor, sut);
    }

    #[test]
    fn test_internal_updates_linker() {
        let mut sut: CompilerPass = Default::default();
        assert_eq!(CompilerPass::Linking, sut);

        assert_eq!(true, sut.take("-###"));
        assert_eq!(false, sut.take("--not_this"));
        assert_eq!(CompilerPass::Internal, sut);
    }

    #[test]
    fn test_is_compiling() {
        assert_eq!(true, CompilerPass::Compilation.is_compiling());
        assert_eq!(true, CompilerPass::Linking.is_compiling());

        assert_eq!(false, CompilerPass::Preprocessor.is_compiling());
        assert_eq!(false, CompilerPass::Internal.is_compiling());
    }
}
