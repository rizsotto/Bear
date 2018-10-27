extern crate intercept;

use std::path;
use std::fs;
use intercept::trace::Trace;

const TEST_FILE_NAME: &'static str = "execution.test.read_write_works.json";

#[test]
fn read_write_works() {
    let expected = Trace::new(1234,
                              path::PathBuf::from("/tmp"),
                              vec!["a".to_string(), "b".to_string()]);
    {
        let mut file = fs::File::create(TEST_FILE_NAME).unwrap();
        let _result = Trace::write(&mut file, &expected);
    }
    {
        let mut file = fs::File::open(TEST_FILE_NAME).unwrap();
        let result = Trace::read(&mut file).unwrap();
        assert_eq!(expected.get_pid(), result.get_pid());
        assert_eq!(expected.get_cwd(), result.get_cwd());
        assert_eq!(expected.get_cmd(), result.get_cmd());
    }
    fs::remove_file(TEST_FILE_NAME).unwrap();
}
