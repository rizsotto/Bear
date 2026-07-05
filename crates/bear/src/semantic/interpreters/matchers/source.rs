// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FileKind {
    CFamilyHeader,
    CFamilySource,
    ObjCSource,
    CxxModule,
    OtherCompilable,
}

#[rustfmt::skip]
fn file_kind(extension: &str) -> Option<FileKind> {
    match extension {
        // header files
        "h" | "hh" | "H" | "hp" | "hxx" | "hpp" | "HPP" | "h++" | "tcc" =>
            Some(FileKind::CFamilyHeader),
        // C / C++
        "c" | "C" | "cc" | "CC" | "c++" | "C++" | "cxx" | "cpp" | "cp" =>
            Some(FileKind::CFamilySource),
        // ObjectiveC
        "m" | "mi" | "mm" | "M" | "mii" =>
            Some(FileKind::ObjCSource),
        // C++20 module interfaces
        "cppm" | "ixx" | "mxx" | "ccm" | "cxxm" | "c++m" =>
            Some(FileKind::CxxModule),
        // Preprocessed
        "i" | "ii" |
        // CUDA
        "cu" |
        // Assembly
        "s" | "S" | "sx" | "asm" |
        // Fortran
        "f" | "for" | "fpp" | "ftn" |
        "F" | "FOR" | "FPP" | "FTN" |
        "f90" | "f95" | "f03" | "f08" |
        "F90" | "F95" | "F03" | "F08" |
        // go
        "go" |
        // brig
        "brig" |
        // D
        "d" | "di" | "dd" |
        // Ada
        "ads" | "abd" |
        // Vala / Genie (valac translation units; .vapi/.gir are bindings, not TUs)
        "vala" | "gs" |
        // Swift
        "swift" =>
            Some(FileKind::OtherCompilable),
        _ => None,
    }
}

pub fn looks_like_a_source_file(argument: &str) -> bool {
    argument.rsplit_once('.').is_some_and(|(_, extension)| file_kind(extension).is_some())
}

/// True when the path's extension is a C-family header (.h, .hpp, ...).
pub fn is_header_file(path: &Path) -> bool {
    matches!(kind_of(path), Some(FileKind::CFamilyHeader))
}

/// True when the path is a C, C++, or Objective-C translation unit - the file
/// types eligible to donate compile flags to a synthesized header entry.
///
/// C++20 module interfaces (`.cppm`, `.ixx`, ...) are intentionally excluded:
/// a module-interface compile carries module-specific flags (e.g.
/// `--precompile`) that would be wrong to clone onto a plain header entry, so
/// they never donate.
pub fn is_c_family_source(path: &Path) -> bool {
    matches!(kind_of(path), Some(FileKind::CFamilySource | FileKind::ObjCSource))
}

fn kind_of(path: &Path) -> Option<FileKind> {
    path.extension().and_then(|ext| ext.to_str()).and_then(file_kind)
}

#[cfg(test)]
mod test {
    use super::*;

    // Requirements: semantic-cpp20-modules
    #[test]
    fn test_filenames() {
        assert!(looks_like_a_source_file("source.c"));
        assert!(looks_like_a_source_file("source.cpp"));
        assert!(looks_like_a_source_file("source.cxx"));
        assert!(looks_like_a_source_file("source.cc"));

        assert!(looks_like_a_source_file("source.h"));
        assert!(looks_like_a_source_file("source.hpp"));

        assert!(looks_like_a_source_file("mod.cppm"));
        assert!(looks_like_a_source_file("mod.ixx"));
        // Precompiled module artifacts must never be treated as sources.
        assert!(!looks_like_a_source_file("foo.pcm"));

        assert!(looks_like_a_source_file("source.vala"));
        assert!(looks_like_a_source_file("source.gs"));
        // .vapi/.gir are bindings consumed by valac, not translation units
        assert!(!looks_like_a_source_file("gio-2.0.vapi"));
        assert!(!looks_like_a_source_file("Gtk.gir"));

        assert!(looks_like_a_source_file("source.swift"));

        assert!(!looks_like_a_source_file("gcc"));
        assert!(!looks_like_a_source_file("clang"));
        assert!(!looks_like_a_source_file("-o"));
        assert!(!looks_like_a_source_file("-Wall"));
        assert!(!looks_like_a_source_file("/o"));
    }

    // Requirements: semantic-cpp20-modules
    #[test]
    fn test_file_kind() {
        let cases = [
            ("h", Some(FileKind::CFamilyHeader)),
            ("hpp", Some(FileKind::CFamilyHeader)),
            ("cpp", Some(FileKind::CFamilySource)),
            ("C", Some(FileKind::CFamilySource)),
            ("c", Some(FileKind::CFamilySource)),
            ("mm", Some(FileKind::ObjCSource)),
            ("m", Some(FileKind::ObjCSource)),
            ("cppm", Some(FileKind::CxxModule)),
            ("ixx", Some(FileKind::CxxModule)),
            ("mxx", Some(FileKind::CxxModule)),
            ("ccm", Some(FileKind::CxxModule)),
            ("cxxm", Some(FileKind::CxxModule)),
            ("c++m", Some(FileKind::CxxModule)),
            ("swift", Some(FileKind::OtherCompilable)),
            ("cu", Some(FileKind::OtherCompilable)),
            ("f90", Some(FileKind::OtherCompilable)),
            ("o", None),
            ("vapi", None),
            ("pcm", None),
        ];

        for (extension, expected) in cases {
            let sut = file_kind(extension);

            assert_eq!(sut, expected, "extension: {extension}");
        }
    }

    // Requirements: semantic-cpp20-modules
    #[test]
    fn test_is_header_file_and_is_c_family_source() {
        let cases = [
            ("util.h", true, false),
            ("main.c", false, true),
            ("a.mm", false, true),
            ("a.swift", false, false),
            ("a.o", false, false),
            // Module interfaces are recognized as sources (looks_like_a_source_file)
            // but are deliberately excluded as header-synthesis donors.
            ("mod.cppm", false, false),
        ];

        for (path, expected_header, expected_source) in cases {
            let sut = std::path::Path::new(path);

            assert_eq!(is_header_file(sut), expected_header, "path: {path}");
            assert_eq!(is_c_family_source(sut), expected_source, "path: {path}");
        }
    }
}
