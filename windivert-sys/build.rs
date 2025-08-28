#[cfg(not(windows))]
compile_error!("This crate only supports Windows");

use std::{env::var, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let lib_path = PathBuf::from(var("LIBWINDIVERT_PATH").expect(
        "Please set the LIBWINDIVERT_PATH environment variable to the path where WinDivert is installed",
    ));
    let lib_arch = lib_path.join(if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        "x86"
    });
    let lib_header = lib_path.join("include/windivert.h");

    println!("cargo:rustc-link-search={}", lib_arch.display());
    println!("cargo:rustc-link-lib=WinDivert");

    let bindings = bindgen::Builder::default()
        .header(lib_header.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .derive_debug(true)
        .generate()?;

    let out_path = PathBuf::from(var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}
