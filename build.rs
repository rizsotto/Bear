extern crate cmake;

#[cfg(unix)]
fn main()
{
    let dst = cmake::build("src/intercept-libexec");

    //println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-env=INTERCEPT_LIBEXEC={}", dst.display());
    println!("cargo:rerun-if-changed=src/intercept-libexec");
}

#[cfg(not(unix))]
fn main()
{
}
