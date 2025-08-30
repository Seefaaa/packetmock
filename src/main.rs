#![cfg(windows)]
#![windows_subsystem = "windows"]

mod cli;
mod http;
mod service;
mod tray;
mod windivert;

use env_logger::Env;
use winapi::um::wincon::{ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole};

use crate::{cli::handle_cli, service::handle_if_service};

const TTL: u8 = 4;

/// Main entry point for the application.
fn main() -> color_eyre::Result<()> {
    let is_terminal = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;

    color_eyre::install()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    handle_if_service()?;
    handle_cli(is_terminal)?;

    if is_terminal {
        unsafe { FreeConsole() };
    }

    Ok(())
}
