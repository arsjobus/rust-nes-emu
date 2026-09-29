use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/runes.rc");
    println!("cargo:rerun-if-changed=assets/runes-icon.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let resource = out_dir.join("runes.res");
    let status = Command::new("rc.exe")
        .current_dir(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory is set"))
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg("assets/runes.rc")
        .status()
        .expect("Windows resource compiler rc.exe is required to embed the application icon");

    assert!(status.success(), "rc.exe failed to compile assets/runes.rc");
    println!("cargo:rustc-link-arg={}", resource.display());
}
