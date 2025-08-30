#[cfg(not(windows))]
compile_error!("This crate only supports Windows");

use std::{env::var, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let base_path = PathBuf::from("../windivert").canonicalize()?;

    let header_file = base_path.join("include/windivert.h");
    let lib_path = base_path.join(if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        "x86"
    });

    if !header_file.exists() {
        panic!("WinDivert header not found at {}", header_file.display());
    }
    if !lib_path.exists() {
        panic!("WinDivert library not found at {}", lib_path.display());
    }

    println!("cargo:rustc-link-search={}", lib_path.display());
    println!("cargo:rustc-link-lib=WinDivert");

    let bindings = bindgen::Builder::default()
        .header(header_file.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .derive_debug(true)
        .derive_default(true)
        .generate_comments(false) // Reduce noise
        .layout_tests(false) // Reduce generated test code
        // Include WinDivert items
        .allowlist_type("WINDIVERT_.*")
        .allowlist_function("WinDivert.*")
        .allowlist_var("WINDIVERT_.*")
        // Required Windows types
        .allowlist_type("HANDLE")
        .allowlist_type("BOOL")
        .allowlist_type("DWORD")
        .allowlist_type("UINT.*")
        .allowlist_type("INT.*")
        .allowlist_type("PVOID")
        .allowlist_type("OVERLAPPED")
        .allowlist_type("LPOVERLAPPED")
        // Exclude problematic items
        .blocklist_function(
            ".*(?i:createfile|openprocess|createthread|messagebox|winmain|dllmain).*",
        )
        .blocklist_type("IMAGE_.*")
        .blocklist_type("PROCESS_INFORMATION")
        .blocklist_type("STARTUPINFO.*")
        .blocklist_type("SECURITY_.*")
        .generate()?;

    let out_path = PathBuf::from(var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}
