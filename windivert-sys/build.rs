#[cfg(not(windows))]
compile_error!("This crate only supports Windows");

use std::{env::var, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let base_path = PathBuf::from("../windivert")
        .canonicalize()
        .ok()
        .or_else(|| var("LIBWINDIVERT_PATH").map(PathBuf::from).ok())
        .ok_or("WinDivert library not found. Please set LIBWINDIVERT_PATH environment variable")
        .unwrap();

    let arch_dir = if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        "x86"
    };

    let header_file = base_path.join("include/windivert.h");
    let lib_path = base_path.join(arch_dir);

    if !header_file.exists() {
        panic!("WinDivert header not found at {}", header_file.display());
    }
    if !lib_path.exists() {
        panic!(
            "WinDivert library directory not found at {}",
            lib_path.display()
        );
    }

    println!("cargo:rustc-link-search={}", lib_path.display());
    println!("cargo:rustc-link-lib=WinDivert");
    println!("cargo:rerun-if-env-changed=LIBWINDIVERT_PATH");

    let bindings = bindgen::Builder::default()
        .header(header_file.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .derive_debug(true)
        .generate()?;

    let out_path = PathBuf::from(var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}
