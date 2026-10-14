// SPDX-License-Identifier: GPL-3.0-or-later

use super::formats::SerializationError;
use super::{WriterCreationError, WriterError};
use std::{fs, path};

/// An output file that is published all at once, or not at all.
///
/// Bear writes each output file of a run (the compilation database, the events file) as a
/// transaction. The content goes to a temporary file. Only the coordinator of the run knows
/// whether the run succeeded, so it alone decides: [`AtomicFile::commit`] publishes the file,
/// dropping the value rolls it back. Rollback is the default, so every early return on an
/// error leaves the previous output untouched and no temporary file behind.
///
/// The temporary file is created up front, so a destination that cannot be written fails
/// before the build runs.
///
/// Publishing is a rename, so the previous file survives any failure before it, including
/// signals that cannot be caught such as `SIGKILL`; the guarantee does not rely on signal
/// handling (only the cleanup does). `rename(2)` is atomic only when source and destination
/// share a filesystem, so the temporary file lives next to the output. Its name is random, so
/// it never collides with the output or with the temporary file of another Bear run. On
/// Windows the underlying `MoveFileEx`/`MOVEFILE_REPLACE_EXISTING` is not fully atomic under
/// all conditions, a platform limitation Bear accepts.
///
/// An output that exists and is not a regular file (`/dev/null`, a pipe) is written in place:
/// a rename would replace it with a regular file. Such an output, like a stream, has nothing
/// to commit.
///
/// The file handle goes to the writer of the content, which is consumed by its write. So the
/// handle is closed before the commit or the rollback, as Windows can not rename or remove an
/// open file.
pub struct AtomicFile(Option<Staged>);

struct Staged {
    temp: tempfile::TempPath,
    final_path: path::PathBuf,
}

impl AtomicFile {
    /// Creates the temporary file for `final_path` and returns it open for writing.
    pub(crate) fn create(final_path: &path::Path) -> Result<(Self, fs::File), WriterCreationError> {
        let error = |err| WriterCreationError::Io(final_path.to_path_buf(), err);

        // This is also where a directory as output fails: it can not be opened for writing.
        if fs::metadata(final_path).is_ok_and(|metadata| !metadata.is_file()) {
            let file = fs::OpenOptions::new().write(true).open(final_path).map_err(error)?;
            return Ok((Self::none(), file));
        }

        let directory = match final_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => path::Path::new("."),
        };
        let mut prefix = std::ffi::OsString::from(".");
        prefix.push(final_path.file_name().unwrap_or_default());
        prefix.push(".");

        // Opening the file here, instead of in `tempfile`, keeps two things as `File::create` has
        // them: the permissions (`tempfile` uses 0600), and the plain OS error (`tempfile` adds
        // the random temporary path to it).
        let (file, temp) = tempfile::Builder::new()
            .prefix(&prefix)
            .suffix(".tmp")
            .make_in(directory, |path| fs::OpenOptions::new().write(true).create_new(true).open(path))
            .map_err(error)?
            .into_parts();

        Ok((Self(Some(Staged { temp, final_path: final_path.to_path_buf() })), file))
    }

    /// An output with nothing to commit: a stream, or a file written in place.
    pub(crate) fn none() -> Self {
        Self(None)
    }

    /// Publishes the content by renaming the temporary file to the final path.
    ///
    /// When the rename fails, the error references the final path (the likely cause), and the
    /// temporary file is kept for debugging.
    pub(crate) fn commit(self) -> Result<(), WriterError> {
        let Some(Staged { temp, final_path }) = self.0 else {
            return Ok(());
        };
        temp.persist(&final_path).map_err(|err| {
            match err.path.keep() {
                Ok(kept) => log::warn!("Kept the temporary file for debugging: {}", kept.display()),
                Err(keep_err) => log::warn!("Failed to keep the temporary file: {}", keep_err.error),
            }
            WriterError::Io(final_path, SerializationError::Io(err.error))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// The names in `dir`, to spot a leftover temporary file.
    fn entries(dir: &path::Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn test_commit_publishes_the_content() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");

        let (sut, mut file) = AtomicFile::create(&final_path).unwrap();
        file.write_all(b"content").unwrap();
        drop(file);
        sut.commit().unwrap();

        assert_eq!(fs::read_to_string(&final_path).unwrap(), "content");
        assert_eq!(entries(dir.path()), vec!["output.json"]);
    }

    #[test]
    fn test_commit_replaces_the_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");
        fs::write(&final_path, "previous").unwrap();

        let (sut, mut file) = AtomicFile::create(&final_path).unwrap();
        file.write_all(b"content").unwrap();
        drop(file);
        sut.commit().unwrap();

        assert_eq!(fs::read_to_string(&final_path).unwrap(), "content");
    }

    #[cfg(unix)]
    #[test]
    fn test_commit_gives_the_permissions_of_a_created_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");
        let expected_path = dir.path().join("expected.json");
        fs::File::create(&expected_path).unwrap();

        let (sut, file) = AtomicFile::create(&final_path).unwrap();
        drop(file);
        sut.commit().unwrap();

        let mode = |path: &path::Path| fs::metadata(path).unwrap().permissions().mode();
        assert_eq!(mode(&final_path), mode(&expected_path));
    }

    #[test]
    fn test_drop_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");
        fs::write(&final_path, "previous").unwrap();

        let (sut, mut file) = AtomicFile::create(&final_path).unwrap();
        file.write_all(b"content").unwrap();
        drop(file);
        drop(sut);

        assert_eq!(fs::read_to_string(&final_path).unwrap(), "previous");
        assert_eq!(entries(dir.path()), vec!["output.json"]);
    }

    #[test]
    fn test_temp_file_is_hidden_next_to_the_output() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");

        let (_sut, _file) = AtomicFile::create(&final_path).unwrap();

        let names = entries(dir.path());
        assert_eq!(names.len(), 1);
        assert!(
            names[0].starts_with(".output.json.") && names[0].ends_with(".tmp"),
            "unexpected name: {names:?}"
        );
    }

    #[test]
    fn test_output_named_like_a_temp_file_survives_a_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.tmp");
        fs::write(&final_path, "previous").unwrap();

        let (sut, file) = AtomicFile::create(&final_path).unwrap();
        drop(file);
        drop(sut);

        assert_eq!(fs::read_to_string(&final_path).unwrap(), "previous");
    }

    #[cfg(unix)]
    #[test]
    fn test_special_file_is_written_in_place() {
        // Only create and drop: a commit gone wrong would replace /dev/null when run as root.
        let final_path = path::Path::new("/dev/null");

        let (sut, mut file) = AtomicFile::create(final_path).unwrap();
        file.write_all(b"content").unwrap();
        let staged =
            entries(path::Path::new("/dev")).into_iter().filter(|name| name.starts_with(".null.")).count();
        drop(file);
        drop(sut);

        assert_eq!(staged, 0, "no temporary file may be staged next to /dev/null");
        assert!(std::os::unix::fs::FileTypeExt::is_char_device(
            &fs::metadata(final_path).unwrap().file_type()
        ));
    }

    #[test]
    fn test_create_fails_on_missing_directory() {
        let final_path = path::Path::new("/nonexistent/directory/output.json");

        let sut = AtomicFile::create(final_path);

        match sut {
            Err(WriterCreationError::Io(path, _)) => assert_eq!(path, final_path),
            _ => panic!("Expected WriterCreationError::Io"),
        }
    }

    #[test]
    fn test_failed_commit_keeps_the_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");
        let (output, file) = AtomicFile::create(&final_path).unwrap();
        let temp_path = dir.path().join(&entries(dir.path())[0]);
        drop(file);
        // Renaming a file over a non-empty directory fails even for root.
        fs::create_dir(&final_path).unwrap();
        fs::write(final_path.join("sentinel"), "untouched").unwrap();

        let sut = output.commit();

        match sut {
            Err(WriterError::Io(path, _)) => assert_eq!(path, final_path),
            Ok(()) => panic!("Expected the commit to fail"),
        }
        assert!(temp_path.exists());
        assert_eq!(fs::read_to_string(final_path.join("sentinel")).unwrap(), "untouched");
    }
}
