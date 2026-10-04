use std::{env, fs, path::PathBuf};

fn main() {
    // Put the linker script where rust-lld can find it.
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::copy("gba.ld", out.join("gba.ld")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=gba.ld");
    println!("cargo:rerun-if-changed=../fixtures/linux.bin");
    println!("cargo:rerun-if-changed=../fixtures/default.dtb");
}
