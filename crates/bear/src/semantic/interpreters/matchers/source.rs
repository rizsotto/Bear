// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FileKind {
    CFamilyHeader,
    CFamilySource,
    ObjCSource,
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
            ("swift", Some(FileKind::OtherCompilable)),
            ("cu", Some(FileKind::OtherCompilable)),
            ("f90", Some(FileKind::OtherCompilable)),
            ("o", None),
            ("vapi", None),
        ];

        for (extension, expected) in cases {
            let sut = file_kind(extension);

            assert_eq!(sut, expected, "extension: {extension}");
        }
    }
}
