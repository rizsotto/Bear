// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang;
use crate::output::formats::SerializationError;
use std::{fs, path};

/// The type represents a writer that writes JSON compilation database files atomically.
///
/// The file is first written to a temporary file and then renamed to the final file name.
/// This ensures that the output file is not left in an inconsistent state in case of errors.
pub(crate) struct AtomicClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    temp_path: path::PathBuf,
    final_path: path::PathBuf,
}

impl<T: IteratorWriter<clang::Entry>> AtomicClangOutputWriter<T> {
    pub(crate) fn new(writer: T, temp_path: &path::Path, final_path: &path::Path) -> Self {
        Self { writer, temp_path: temp_path.to_path_buf(), final_path: final_path.to_path_buf() }
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for AtomicClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
        self.writer.write(entries)?;

        fs::rename(&self.temp_path, &self.final_path)
            .map_err(|err| WriterError::Io(self.final_path, SerializationError::Io(err)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::writers::fixtures::CollectingWriter;

    #[test]
    fn test_atomic_clang_output_writer_success() {
        let dir = tempfile::tempdir().unwrap();
        let temp_path = dir.path().join("temp_file.json");
        let final_path = dir.path().join("final_file.json");

        // Create the temp file
        fs::File::create(&temp_path).unwrap();

        let sut = AtomicClangOutputWriter::new(CollectingWriter::new().0, &temp_path, &final_path);
        sut.write(std::iter::empty()).unwrap();

        // Verify the final file exists
        assert!(final_path.exists());
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_atomic_clang_output_writer_temp_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let temp_path = dir.path().join("temp_file.json");
        let final_path = dir.path().join("final_file.json");

        let sut = AtomicClangOutputWriter::new(CollectingWriter::new().0, &temp_path, &final_path);
        let result = sut.write(std::iter::empty());

        // Verify the operation fails
        assert!(result.is_err());
        assert!(!final_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_clang_output_writer_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        // Skip this test when running as root (root bypasses permission checks)
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let temp_path = dir.path().join("temp_file.json");

        // Create a read-only target directory
        let readonly_dir = dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();
        let final_path = readonly_dir.join("final_file.json");

        // Create the temp file
        fs::File::create(&temp_path).unwrap();

        // Make target directory read-only
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let sut = AtomicClangOutputWriter::new(CollectingWriter::new().0, &temp_path, &final_path);
        let result = sut.write(std::iter::empty());

        // Restore permissions before asserting (so tempdir cleanup works)
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            WriterError::Io(path, _) => {
                assert_eq!(path, final_path);
            }
        }
    }

    #[test]
    fn test_atomic_clang_output_writer_final_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let temp_path = dir.path().join("temp_file.json");
        let final_path = dir.path().join("final_file.json");

        // Create the temp file and final file
        fs::File::create(&temp_path).unwrap();
        fs::File::create(&final_path).unwrap();

        let sut = AtomicClangOutputWriter::new(CollectingWriter::new().0, &temp_path, &final_path);
        let result = sut.write(std::iter::empty());

        // Verify the operation fails
        assert!(result.is_ok());
        assert!(final_path.exists());
        assert!(!temp_path.exists());
    }
}
