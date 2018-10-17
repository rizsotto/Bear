extern crate intercept;

use std::path;
use std::fs;
use intercept::trace::Trace;

#[test]
fn read_write_works() {
    let args: Vec<_> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    let dir = path::Path::new("/tmp").to_path_buf();
    let value = Trace::new(1234, dir, args);
    {
        let mut file = fs::File::create("execution.test.read_write_works.json").unwrap();
        let _result = Trace::write(&mut file, &value);
    }
    {
        let mut file = fs::File::open("execution.test.read_write_works.json").unwrap();
        let result = Trace::read(&mut file).unwrap();
        assert_eq!(value.get_pid(), result.get_pid());
        assert_eq!(value.get_cwd(), result.get_cwd());
        assert_eq!(value.get_cmd(), result.get_cmd());
    }
}
