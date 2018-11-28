extern crate cc;

fn main() {
    cc::Build::new().file("src/execs.c").compile("cexecs");
}
