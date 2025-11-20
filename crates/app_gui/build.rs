use std::env;

fn main() {
    let version =
        env::var("FEEDIE_VERSION").unwrap_or_else(|_| env::var("CARGO_PKG_VERSION").unwrap());
    println!("cargo:rustc-env=FEEDIE_VERSION={version}");
}
