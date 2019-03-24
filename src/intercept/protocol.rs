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

use std::fs;
use std::path;
use tempfile;

use crate::intercept::event::Event;
use crate::Result;


pub mod sender {
    use super::*;

    pub struct Protocol {
        path: path::PathBuf,
    }

    impl Protocol {
        pub fn new(path: &path::Path) -> Result<Protocol> {
            Ok(Protocol { path: path.to_path_buf() })
        }

        pub fn send(&mut self, event: Event) {
            debug!("Event to save: {:?}", &event);
            let name = save(&self.path, &event)
                .expect("Persist event on filesystem failed.");
            debug!("Event saved into file: {:?}", name);
        }
    }
}

pub mod collector {
    use super::*;

    pub struct Protocol {
        directory: tempfile::TempDir,
    }

    impl Protocol {
        pub fn new() -> Result<Protocol> {
            let directory = tempfile::Builder::new()
                .prefix("bear-")
                .rand_bytes(12)
                .tempdir()?;
            debug!("Created temporary directory: {:?}", directory.path());

            Ok(Protocol { directory })
        }

        pub fn path(&self) -> &path::Path {
            self.directory.path()
        }

        pub fn events(&self) -> EventIterator {
            EventIterator::new(self.path())
                .expect("Event directory does not seems to exist.")
        }
    }

    pub struct EventIterator {
        input: fs::ReadDir,
    }

    impl EventIterator {
        pub fn new(path: &path::Path) -> Result<EventIterator> {
            let input = fs::read_dir(path)?;
            Ok(EventIterator { input })
        }
    }

    impl Iterator for EventIterator {
        type Item = Event;

        fn next(&mut self) -> Option<<Self as Iterator>::Item> {
            match self.input.next() {
                Some(Ok(entry)) => {
                    match load(entry.path().as_path()) {
                        Ok(event) => {
                            debug!("candidate {:?} has read as: {:?}", entry.path(), event);
                            Some(event)
                        },
                        Err(error) => {
                            debug!("candidate {:?} failed to read: {:?}", entry.path(), error);
                            self.next()
                        },
                    }
                }
                Some(Err(_)) => self.next(),
                _ => None,
            }
        }
    }
}


const PREFIX: &str = "report-";
const SUFFIX: &str = ".json";

/// Read a single event file content from given source.
fn load(path: &path::Path) -> Result<Event> {
    let file = fs::File::open(path)?;
    let result = serde_json::from_reader(file)?;
    Ok(result)
}

/// Write a single event entry into the given target.
fn save(target: &path::Path, event: &Event) -> Result<path::PathBuf> {
    let mut output = tempfile::Builder::new()
        .prefix(PREFIX)
        .suffix(SUFFIX)
        .rand_bytes(12)
        .tempfile_in(target)?;
    serde_json::to_writer(&mut output, event)?;

    let name = output.path().to_path_buf();
    std::mem::forget(output.into_temp_path());
    Ok(name)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::intercept::event::ProcessId;
    use std::io::Write;

    #[allow(unused_assignments)]
    fn assert_in_temporary_directory<F>(op: F)
        where F: Fn(&mut collector::Protocol) -> Result<()>
    {
        let mut path: path::PathBuf = path::PathBuf::new();
        {
            let mut sut = collector::Protocol::new().unwrap();
            path = sut.path().to_path_buf();

            op(&mut sut).expect("given test failed.");
        }
        assert!(!path.exists())
    }

    #[test]
    fn temp_directory_created_and_deleted() {
        assert_in_temporary_directory(|collector| {
            assert!(collector.path().is_dir());
            Ok(())
        });
    }

    #[test]
    fn temp_directory_content_removed() {
        assert_in_temporary_directory(|collector| {
            let mut name = collector.path().to_path_buf();
            name.push("greeting.txt");
            let mut file = fs::File::create(name).unwrap();
            file.write_all(b"Hello world!")
                .map_err(|error| error.into())
        });
    }

    #[test]
    fn temp_directory_finds_event_files() {
        assert_in_temporary_directory(|collector| {
            const PID: ProcessId = 42u32;
            const SIGNAL: &'static str = "signal";
            let when = chrono::Utc::now();

            let mut sut = sender::Protocol::new(collector.path())?;

            sut.send(Event::Stopped {
                pid: PID,
                signal: SIGNAL.to_string(),
                when: when.clone(),
            });

            let mut it = collector.events();

            assert_eq!(it.next(), Some(Event::Stopped {
                pid: PID,
                signal: SIGNAL.to_string(),
                when: when.clone(),
            }));
            assert_eq!(it.next(), None);

            Ok(())
        });
    }
}
