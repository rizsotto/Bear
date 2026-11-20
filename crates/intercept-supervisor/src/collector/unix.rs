// SPDX-License-Identifier: GPL-3.0-or-later

//! Where the UNIX report socket lives.

use super::{Listener, SocketAccess, bind_loopback};
use crate::cleanup;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, ErrorKind};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

/// Binds a socket in the first base that works, else a loopback TCP port when `fallback` is set.
pub(super) fn bind(bases: &[PathBuf], access: SocketAccess, fallback: bool) -> io::Result<Listener> {
    let mut last_error = None;
    for base in bases {
        match bind_unix(base, access) {
            Ok(listener) => return Ok(listener),
            Err(err) => {
                log::debug!("{err}");
                last_error = Some(err);
            }
        }
    }
    if fallback {
        log::info!("no report socket can be created, reporting over the loopback interface");
        return Ok(Listener::Tcp(bind_loopback()?));
    }
    Err(last_error.unwrap_or_else(|| io::Error::other("no directory is available for the report socket")))
}

fn bind_unix(base: &Path, access: SocketAccess) -> io::Result<Listener> {
    let wrap = |err: io::Error| {
        io::Error::new(err.kind(), format!("cannot create the report socket in {}: {err}", base.display()))
    };
    if !base.is_absolute() {
        return Err(wrap(io::Error::new(ErrorKind::InvalidInput, "not an absolute path")));
    }
    // The address is built from the real path, so it is the one that is checked.
    let base = &base.canonicalize().map_err(wrap)?;
    if base.to_str().is_none() {
        return Err(wrap(io::Error::new(ErrorKind::InvalidInput, "not a UTF-8 path")));
    }
    if access == SocketAccess::All {
        check_traversable(base).map_err(wrap)?;
    }
    let dir = SocketDir::create(base, access).map_err(wrap)?;
    let listener = UnixListener::bind(&dir.socket).map_err(wrap)?;
    if access == SocketAccess::All {
        // The mode of the socket file is what `connect` checks, and `bind` applied the umask.
        std::fs::set_permissions(&dir.socket, std::fs::Permissions::from_mode(0o777)).map_err(wrap)?;
    }
    Ok(Listener::Unix(listener, dir))
}

/// Every directory from the canonical `base` up to `/` must let other users enter, or an open socket
/// still cannot be reached (`XDG_RUNTIME_DIR` is 0700, so is the macOS `TMPDIR`).
fn check_traversable(base: &Path) -> io::Result<()> {
    for directory in base.ancestors() {
        let mode = std::fs::metadata(directory)?.permissions().mode();
        if mode & 0o001 == 0 {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                format!(
                    "{} cannot be entered by other users: choose a directory they can enter, \
                     or set intercept.socket.access to owner",
                    directory.display()
                ),
            ));
        }
    }
    Ok(())
}

/// How many times `create` picks another name when `bear-<random>` already exists.
const CREATE_RETRIES: u32 = 4;

/// The private directory that holds the socket. Removed on drop.
pub(super) struct SocketDir {
    dir: PathBuf,
    pub(super) socket: PathBuf,
}

impl SocketDir {
    /// Makes `<base>/bear-<random>` with the mode that matches the access.
    fn create(base: &Path, access: SocketAccess) -> io::Result<Self> {
        let mode = match access {
            SocketAccess::Owner => 0o700,
            SocketAccess::All => 0o711,
        };
        let mut attempt = 0;
        let dir = loop {
            let dir = base.join(format!("bear-{:08x}", random() as u32));
            let created = cleanup::track(&dir, || std::fs::DirBuilder::new().mode(mode).create(&dir));
            match created {
                Ok(()) => break dir,
                Err(err) if err.kind() == ErrorKind::AlreadyExists && attempt < CREATE_RETRIES => {
                    attempt += 1;
                }
                Err(err) => return Err(err),
            }
        };
        let socket = dir.join("sock");
        let socket_dir = Self { dir, socket };
        // `mkdir` applies the umask; the mode must be exact.
        std::fs::set_permissions(&socket_dir.dir, std::fs::Permissions::from_mode(mode))?;
        Ok(socket_dir)
    }
}

impl Drop for SocketDir {
    fn drop(&mut self) {
        cleanup::untrack(&self.dir, || {
            let _ = std::fs::remove_dir_all(&self.dir);
        });
    }
}

/// 64 random bits from `std`: the keys of a `RandomState` are seeded by the OS, and differ
/// for each instance.
fn random() -> u64 {
    std::hash::RandomState::new().build_hasher().finish()
}

#[cfg(test)]
mod tests {
    use crate::collector::tests::fixture::*;
    use crate::collector::{Collector, SocketAccess};
    use intercept::reporter::Reporter;
    use intercept::wire::{Address, SocketReporter};
    use std::os::unix::fs::FileTypeExt;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    // Requirements: interception-report-channel
    #[test]
    fn socket_modes_follow_the_access() {
        let cases = [(SocketAccess::Owner, 0o700, None), (SocketAccess::All, 0o711, Some(0o777))];

        for (access, directory_mode, socket_mode) in cases {
            let base = short_base();

            let (_collector, address) = open_in(base.path(), access);

            let socket = socket_path(&address);
            assert_eq!(mode(socket.parent().unwrap()), directory_mode, "case: {access:?}");
            if let Some(socket_mode) = socket_mode {
                assert_eq!(mode(&socket), socket_mode, "case: {access:?}");
            }
        }
    }

    // Requirements: interception-report-channel
    #[test]
    fn an_open_socket_needs_a_base_that_others_can_enter() {
        let private = short_base();
        set_mode(private.path(), 0o700);
        let nested = private.path().join("inner");
        std::fs::create_dir(&nested).unwrap();
        set_mode(&nested, 0o755);

        for base in [private.path(), nested.as_path()] {
            let sut = Collector::new(&[base.to_path_buf()], SocketAccess::All, false)
                .err()
                .expect("others cannot enter the base")
                .to_string();
            assert!(sut.contains(&private.path().display().to_string()), "{sut}");
            assert!(sut.contains("intercept.socket.access to owner"), "{sut}");
        }
        assert_eq!(entries(&nested), 0);
        let (_collector, _address) = open_in(private.path(), SocketAccess::Owner);
    }

    // Requirements: interception-report-channel
    #[test]
    fn the_address_uses_the_real_path_of_a_symlinked_base() {
        let private = short_base();
        set_mode(private.path(), 0o700);
        let open = short_base();
        let link = private.path().join("link");
        std::os::unix::fs::symlink(open.path(), &link).unwrap();

        for access in [SocketAccess::Owner, SocketAccess::All] {
            let (_collector, sut) = open_in(&link, access);

            assert!(socket_path(&sut).starts_with(real(open.path())), "case: {access:?}: {sut}");
        }
    }

    // Requirements: interception-report-channel
    #[test]
    fn a_symlink_into_a_private_directory_is_refused_for_an_open_socket() {
        let private = short_base();
        set_mode(private.path(), 0o700);
        let open = short_base();
        let link = open.path().join("link");
        std::os::unix::fs::symlink(private.path(), &link).unwrap();

        let sut = Collector::new(&[link], SocketAccess::All, false);

        assert!(sut.is_err());
    }

    // Requirements: interception-report-channel
    #[test]
    fn a_base_that_others_cannot_enter_is_skipped_for_an_open_socket() {
        let private = short_base();
        set_mode(private.path(), 0o700);
        let open = short_base();
        let bases = [private.path().to_path_buf(), open.path().to_path_buf()];

        let (_collector, address) = Collector::new(&bases, SocketAccess::All, false).unwrap();

        assert!(socket_path(&address).starts_with(real(open.path())));
        assert_eq!(entries(private.path()), 0);
    }

    // Requirements: interception-report-channel
    #[test]
    fn socket_is_tracked_while_the_collector_lives_and_removed_after() {
        let base = short_base();

        let (sut, address) = open_in(base.path(), SocketAccess::Owner);

        let socket = socket_path(&address);
        let directory = socket.parent().unwrap().to_path_buf();
        assert!(std::fs::metadata(&socket).unwrap().file_type().is_socket());
        assert!(directory.starts_with(real(base.path())));
        assert!(crate::cleanup::is_tracked(&directory));
        drop(sut);
        assert!(!directory.exists());
        assert!(!crate::cleanup::is_tracked(&directory));
        assert_eq!(entries(base.path()), 0);
    }

    // Requirements: interception-report-channel
    #[test]
    fn the_first_usable_base_wins() {
        let first = short_base();
        let second = short_base();
        let bases = [PathBuf::from("relative/base"), first.path().to_path_buf(), second.path().to_path_buf()];

        let (_collector, address) = Collector::new(&bases, SocketAccess::Owner, false).unwrap();

        assert!(socket_path(&address).starts_with(real(first.path())));
        assert_eq!(entries(second.path()), 0);
    }

    // Requirements: interception-report-channel
    #[test]
    fn a_base_that_is_too_long_is_skipped() {
        let long = tempfile::Builder::new().prefix(&"l".repeat(120)).tempdir_in("/tmp").unwrap();
        let next = short_base();
        let bases = [long.path().to_path_buf(), next.path().to_path_buf()];

        let (_collector, address) = Collector::new(&bases, SocketAccess::Owner, false).unwrap();

        assert!(socket_path(&address).starts_with(real(next.path())));
        assert_eq!(entries(long.path()), 0);
    }

    // Requirements: interception-report-channel
    #[test]
    fn no_usable_base_falls_back_to_tcp_when_allowed() {
        let bases = [PathBuf::from("relative/base"), PathBuf::from("/nonexistent/bear/base")];

        let (_collector, address) = Collector::new(&bases, SocketAccess::Owner, true).unwrap();

        assert!(matches!(address, Address::Tcp(_)), "{address}");
    }

    // Requirements: interception-report-channel
    #[test]
    fn no_usable_base_is_an_error_naming_the_path_when_not_allowed() {
        let bases = [PathBuf::from("/nonexistent/bear/base")];

        let sut = Collector::new(&bases, SocketAccess::Owner, false);

        let message = sut.err().expect("no base works").to_string();
        assert!(message.contains("/nonexistent/bear/base"), "{message}");
        assert!(Collector::new(&[], SocketAccess::Owner, false).is_err());
    }

    // Requirements: interception-report-channel
    #[test]
    fn two_collectors_in_one_base_get_only_their_own_reports() {
        let base = short_base();
        let (first, first_address) = open_in(base.path(), SocketAccess::Owner);
        let (second, second_address) = open_in(base.path(), SocketAccess::Owner);
        let (first, second) = (Arc::new(first), Arc::new(second));
        let workers: Vec<_> = [&first, &second].into_iter().map(collect_in_thread).collect();

        SocketReporter::new(first_address.clone()).report(numbered(1)).unwrap();
        SocketReporter::new(second_address.clone()).report(numbered(2)).unwrap();
        first.shutdown().unwrap();
        second.shutdown().unwrap();
        let mut results = workers.into_iter().map(|worker| join_within(worker, Duration::from_secs(5)));

        assert_ne!(first_address, second_address);
        assert_eq!(results.next().unwrap(), vec![numbered(1)]);
        assert_eq!(results.next().unwrap(), vec![numbered(2)]);
    }

    // Requirements: interception-report-channel
    #[test]
    fn a_leftover_directory_does_not_matter() {
        let base = short_base();
        let leftover = base.path().join("bear-deadbeef");
        std::fs::create_dir(&leftover).unwrap();
        std::fs::write(leftover.join("sock"), "").unwrap();

        let (_collector, address) = open_in(base.path(), SocketAccess::Owner);

        assert!(std::fs::metadata(socket_path(&address)).unwrap().file_type().is_socket());
        assert!(leftover.join("sock").exists());
    }

    // Requirements: interception-report-channel
    #[test]
    fn shutdown_fails_when_the_socket_file_is_removed() {
        let base = short_base();
        let (collector, address) = open_in(base.path(), SocketAccess::Owner);
        std::fs::remove_file(socket_path(&address)).unwrap();

        let sut = collector.shutdown();

        assert!(sut.is_err());
    }
}
