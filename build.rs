#[cfg(not(windows))]
compile_error!("This crate only supports Windows");

use std::error::Error;

use winres::WindowsResource;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=resources/icon.ico");

    let mut resource = WindowsResource::new();
    resource.set_manifest(include_str!("app.manifest"));
    resource.set_icon("resources/icon.ico");
    resource.compile()?;

    Ok(())
}
