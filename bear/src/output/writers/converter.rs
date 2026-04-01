// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang;
use crate::output::statistics::OutputStatistics;
use crate::{config, semantic};
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Converts `semantic::Command` instances into compilation database `Entry` objects,
/// tracking pipeline statistics.
pub(crate) struct ConverterClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    converter: clang::CommandConverter,
    writer: T,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> ConverterClangOutputWriter<T> {
    pub(crate) fn new(writer: T, format: &config::Format, stats: Arc<OutputStatistics>) -> Self {
        Self { converter: clang::CommandConverter::new(format.clone()), writer, stats }
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<semantic::Command> for ConverterClangOutputWriter<T> {
    fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> Result<(), WriterError> {
        let stats = Arc::clone(&self.stats);

        let entries = semantics.flat_map(|cmd| self.converter.to_entries(&cmd));
        let counted_entries = entries.inspect(move |_| {
            stats.compilation_entries_produced.fetch_add(1, Ordering::Relaxed);
        });

        self.writer.write(counted_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::statistics::OutputStatistics;
    use crate::output::writers::fixtures::CollectingWriter;
    use crate::semantic::{ArgumentKind, Command, CompilerPass, PassEffect};
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;

    fn make_compile_command(file: &str) -> Command {
        Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec![file]),
            ],
        )
    }

    fn make_preprocessing_command() -> Command {
        Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)), vec!["-E"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        )
    }

    fn make_link_only_command() -> Command {
        Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: true }, vec!["main.o"]),
                (ArgumentKind::Output, vec!["-o", "program"]),
            ],
        )
    }

    #[test]
    fn test_statistics_counting_compile_commands() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let format = config::Format::default();
        let sut = ConverterClangOutputWriter::new(writer, &format, Arc::clone(&stats));

        let commands = vec![
            make_compile_command("file1.c"),
            make_compile_command("file2.c"),
            make_compile_command("file3.c"),
        ];

        sut.write(commands.into_iter()).unwrap();

        assert_eq!(stats.compilation_entries_produced.load(Ordering::Relaxed), 3);

        let entries = collected.lock().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].file, PathBuf::from("file1.c"));
        assert_eq!(entries[1].file, PathBuf::from("file2.c"));
        assert_eq!(entries[2].file, PathBuf::from("file3.c"));
    }

    #[test]
    fn test_statistics_counting_preprocessing_and_link_only() {
        let stats = OutputStatistics::new();
        let (writer, _collected) = CollectingWriter::new();
        let format = config::Format::default();
        let sut = ConverterClangOutputWriter::new(writer, &format, Arc::clone(&stats));

        let commands = vec![make_preprocessing_command(), make_link_only_command()];

        sut.write(commands.into_iter()).unwrap();

        assert_eq!(stats.compilation_entries_produced.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_statistics_counting_empty_input() {
        let stats = OutputStatistics::new();
        let (writer, _collected) = CollectingWriter::new();
        let format = config::Format::default();
        let sut = ConverterClangOutputWriter::new(writer, &format, Arc::clone(&stats));

        sut.write(std::iter::empty()).unwrap();

        assert_eq!(stats.compilation_entries_produced.load(Ordering::Relaxed), 0);
    }
}
