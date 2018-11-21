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
