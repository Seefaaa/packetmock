#![cfg(windows)]
#![windows_subsystem = "windows"]

mod http;
mod service;
mod tray;
mod windivert;

use std::sync::mpsc;

use env_logger::Env;
use log::{error, info};
use smol::{block_on, future::or, unblock};
use winapi::um::wincon::{ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole};

use crate::{service::handle_service, tray::show_tray_icon};

const TTL: u8 = 4;

/// Main entry point for the application.
fn main() -> color_eyre::Result<()> {
    let is_terminal = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;

    color_eyre::install()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    handle_service()?;

    let (sx, rx) = mpsc::channel();

    ctrlc::set_handler(move || {
        sx.send(()).expect("Could not send terminate signal");
    })?;

    let recv = move || {
        match rx.recv() {
            Ok(_) => info!("Terminate signal received."),
            Err(e) => error!("Failed to receive terminate signal: {e}"),
        };
        Ok(())
    };

    block_on(or(unblock(recv), unblock(show_tray_icon)))?;

    if is_terminal {
        unsafe { FreeConsole() };
    }

    Ok(())
}
