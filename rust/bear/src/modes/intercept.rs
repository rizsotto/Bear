// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::collector::{EventCollector, EventCollectorOnTcp};
use crate::intercept::{Envelope, KEY_DESTINATION, KEY_PRELOAD_PATH};
use crate::{args, config};
use crossbeam_channel::{bounded, Receiver};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;
use std::{env, thread};

pub(crate) struct InterceptService {
    collector: Arc<EventCollectorOnTcp>,
    receiver: Receiver<Envelope>,
    collector_thread: Option<thread::JoinHandle<()>>,
}

impl InterceptService {
    pub fn new() -> anyhow::Result<Self> {
        let collector = EventCollectorOnTcp::new()?;
        let collector_arc = Arc::new(collector);
        let (sender, receiver) = bounded(32);

        let collector_in_thread = collector_arc.clone();
        let collector_thread = thread::spawn(move || {
            collector_in_thread.collect(sender).unwrap();
        });

        Ok(InterceptService {
            collector: collector_arc,
            receiver,
            collector_thread: Some(collector_thread),
        })
    }

    pub fn receiver(&self) -> Receiver<Envelope> {
        self.receiver.clone()
    }

    pub fn address(&self) -> String {
        self.collector.address()
    }
}

impl Drop for InterceptService {
    fn drop(&mut self) {
        self.collector.stop().expect("Failed to stop the collector");
        if let Some(thread) = self.collector_thread.take() {
            thread.join().expect("Failed to join the collector thread");
        }
    }
}

pub(crate) enum InterceptEnvironment {
    Wrapper {
        bin_dir: tempfile::TempDir,
        address: String,
    },
    Preload {
        path: PathBuf,
        address: String,
    },
}

impl InterceptEnvironment {
    pub fn new(config: &config::Intercept, address: String) -> anyhow::Result<Self> {
        let result = match config {
            config::Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                // Create a temporary directory and populate it with the executables.
                let bin_dir = tempfile::TempDir::with_prefix_in(directory, "bear-")?;
                for executable in executables {
                    std::fs::hard_link(&executable, &path)?;
                }
                InterceptEnvironment::Wrapper { bin_dir, address }
            }
            config::Intercept::Preload { path } => InterceptEnvironment::Preload {
                path: path.clone(),
                address,
            },
        };
        Ok(result)
    }

    pub fn execute_build_command(self, input: args::BuildCommand) -> anyhow::Result<ExitCode> {
        let environment = self.environment();
        let mut child = Command::new(input.arguments[0].clone())
            .args(input.arguments)
            .envs(environment)
            .spawn()?;

        let result = child.wait()?;

        if result.success() {
            Ok(ExitCode::SUCCESS)
        } else {
            result
                .code()
                .map_or(Ok(ExitCode::FAILURE), |code| Ok(ExitCode::from(code as u8)))
        }
    }

    fn environment(&self) -> Vec<(String, String)> {
        match self {
            InterceptEnvironment::Wrapper {
                bin_dir, address, ..
            } => {
                let path_original = env::var("PATH").unwrap_or_else(|_| String::new());
                let path_updated = InterceptEnvironment::insert_to_path(
                    &path_original,
                    Self::to_string(bin_dir.path()),
                );
                vec![
                    ("PATH".to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
            InterceptEnvironment::Preload { path, address, .. } => {
                let path_original = env::var(KEY_PRELOAD_PATH).unwrap_or_else(|_| String::new());
                let path_updated =
                    InterceptEnvironment::insert_to_path(&path_original, Self::to_string(path));
                vec![
                    (KEY_PRELOAD_PATH.to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
        }
    }

    fn insert_to_path(original: &str, first: String) -> String {
        let mut paths: Vec<_> = original.split(':').filter(|it| it != &first).collect();
        paths.insert(0, first.as_str());
        paths.join(":")
    }

    fn to_string(path: &Path) -> String {
        path.to_str().unwrap_or("").to_string()
    }
}
