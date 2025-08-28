#[cfg(not(windows))]
compile_error!("This crate only supports Windows");

use std::error::Error;

use winres::{self, WindowsResource};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=app.manifest");

    let mut resource = WindowsResource::new();
    resource.set_manifest(include_str!("app.manifest"));
    resource.compile()?;

    Ok(())
}
