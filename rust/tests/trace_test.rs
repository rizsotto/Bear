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

extern crate intercept;
extern crate tempfile;

use intercept::trace::Trace;
use std::fs;
use std::path;

const TEST_FILE_NAME: &'static str = "execution.test.read_write_works.json";

#[test]
fn read_write_works() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join(TEST_FILE_NAME);

    let expected = Trace {
        pid: 1234,
        cwd: path::PathBuf::from("/tmp"),
        cmd: vec!["a".to_string(), "b".to_string()],
    };
    {
        let mut file = fs::File::create(&file_path).unwrap();
        let _result = Trace::write(&mut file, &expected);
    }
    {
        let mut file = fs::File::open(&file_path).unwrap();
        let result = Trace::read(&mut file).unwrap();
        assert_eq!(expected.pid, result.pid);
        assert_eq!(expected.cwd, result.cwd);
        assert_eq!(expected.cmd, result.cmd);
    }
}
