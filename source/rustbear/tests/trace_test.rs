extern crate intercept;

use intercept::trace::Trace;
use std::fs;
use std::path;

const TEST_FILE_NAME: &'static str = "execution.test.read_write_works.json";

#[test]
fn read_write_works() {
    let expected = Trace {
        pid: 1234,
        cwd: path::PathBuf::from("/tmp"),
        cmd: vec!["a".to_string(), "b".to_string()],
    };
    {
        let mut file = fs::File::create(TEST_FILE_NAME).unwrap();
        let _result = Trace::write(&mut file, &expected);
    }
    {
        let mut file = fs::File::open(TEST_FILE_NAME).unwrap();
        let result = Trace::read(&mut file).unwrap();
        assert_eq!(expected.pid, result.pid);
        assert_eq!(expected.cwd, result.cwd);
        assert_eq!(expected.cmd, result.cmd);
    }
    fs::remove_file(TEST_FILE_NAME).unwrap();
}
