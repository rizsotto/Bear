// SPDX-License-Identifier: GPL-3.0-or-later

//! The temporary artifacts of a run, known to a process-wide list.
//!
//! The output temp file and the wrapper directory add their path when they are created and
//! forget it when they are done. [`remove_tracked`] removes whatever is left, so an abort can
//! clean up without owning the values that hold the artifacts.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

static TRACKED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn lock() -> MutexGuard<'static, Vec<PathBuf>> {
    // A panic elsewhere must not stop the abort from cleaning up.
    TRACKED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `create` and records `path` for removal on abort.
///
/// The path is recorded only when `create` succeeds, so a failure (a name that belongs to
/// someone else) never puts a foreign path on the list.
pub fn track<T, E>(path: &Path, create: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let mut tracked = lock();
    let result = create();
    if result.is_ok() {
        tracked.push(path.to_path_buf());
    }
    result
}

/// Runs `finish` (a rename or a removal) and forgets `path`.
pub fn untrack<T>(path: &Path, finish: impl FnOnce() -> T) -> T {
    let mut tracked = lock();
    let result = finish();
    tracked.retain(|tracked_path| tracked_path != path);
    result
}

/// Tells whether `path` is recorded. The list is shared by the whole process.
#[cfg(test)]
pub(crate) fn is_tracked(path: &Path) -> bool {
    lock().iter().any(|tracked_path| tracked_path == path)
}

/// Removes every recorded path. Keeps the lock: the caller dies next.
pub fn remove_tracked() {
    let tracked = lock();
    remove_all(&tracked);
    std::mem::forget(tracked);
}

/// Removes files and directory trees. A path that is already gone is fine.
fn remove_all(paths: &[PathBuf]) {
    for path in paths {
        let _ = match path.symlink_metadata() {
            Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path),
            Ok(_) => std::fs::remove_file(path),
            Err(_) => Ok(()),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_records_the_path_and_untrack_forgets_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact");

        track(&path, || std::fs::write(&path, "x")).unwrap();
        let tracked_after_track = is_tracked(&path);
        untrack(&path, || std::fs::remove_file(&path).unwrap());
        let sut = is_tracked(&path);

        assert!(tracked_after_track);
        assert!(!sut);
    }

    #[test]
    fn test_track_does_not_record_a_failed_create() {
        let path = Path::new("/nonexistent/failed");

        let sut = track(path, || Err::<(), _>("failed"));

        assert_eq!(sut, Err("failed"));
        assert!(!is_tracked(path));
    }

    #[test]
    fn test_remove_all_deletes_a_file_and_a_directory_tree() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        let tree = dir.path().join("tree");
        std::fs::write(&file, "x").unwrap();
        std::fs::create_dir_all(tree.join("nested")).unwrap();
        std::fs::write(tree.join("nested").join("leaf"), "x").unwrap();

        remove_all(&[file.clone(), tree.clone()]);

        assert!(!file.exists());
        assert!(!tree.exists());
    }

    #[test]
    fn test_remove_all_accepts_a_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");

        remove_all(std::slice::from_ref(&missing));

        assert!(!missing.exists());
    }
}
