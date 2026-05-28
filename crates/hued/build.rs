use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/artwork/HueArtwork.m");
    println!("cargo:rerun-if-changed=src/artwork/HueArtwork.h");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let obj = out.join("HueArtwork.o");
    let lib = out.join("libHueArtwork.a");

    assert!(
        Command::new("clang")
            .args([
                "-c",
                "src/artwork/HueArtwork.m",
                "-o",
                obj.to_str().unwrap(),
                "-fobjc-arc",
            ])
            .status()
            .unwrap()
            .success()
    );

    assert!(
        Command::new("ar")
            .args(["crus", lib.to_str().unwrap(), obj.to_str().unwrap(),])
            .status()
            .unwrap()
            .success()
    );

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=HueArtwork");

    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
    println!("cargo:rustc-link-lib=dylib=System");
}
