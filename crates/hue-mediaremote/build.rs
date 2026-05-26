use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/bridge/HueBridge.m");
    println!("cargo:rerun-if-changed=src/bridge/HueBridge.h");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let obj = out.join("HueBridge.o");
    let lib = out.join("libHueBridge.a");

    assert!(
        Command::new("clang")
            .args([
                "-c",
                "src/bridge/HueBridge.m",
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
    println!("cargo:rustc-link-lib=static=HueBridge");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=dylib=System");
}
