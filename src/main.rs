#![cfg(windows)]
#![windows_subsystem = "windows"]

mod http;
mod service;
mod tray;
mod windivert;

use std::sync::mpsc;

use color_eyre::{Result, config::HookBuilder};
use env_logger::Env;
use log::{error, info};
use smol::{block_on, future::or, unblock};
use winapi::um::wincon::{ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole};

use crate::{service::handle_service, tray::run_tray};

const REGISTRY_NAME: &str = "Packetmock";

/// Main entry point for the application.
fn main() -> Result<()> {
    let is_terminal = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;

    init_logger();
    init_color_eyre()?;

    handle_service()?;

    block_on(or(unblock(ctrlc_handler()?), unblock(run_tray)))?;

    if is_terminal {
        unsafe { FreeConsole() };
    }

    Ok(())
}

/// Set up a Ctrl-C handler to gracefully handle termination signals.
fn ctrlc_handler() -> Result<impl FnOnce() -> Result<()>> {
    let (sx, rx) = mpsc::channel();

    ctrlc::set_handler(move || {
        sx.send(()).expect("Could not send terminate signal");
    })?;

    Ok(move || {
        match rx.recv() {
            Ok(_) => info!("Terminate signal received."),
            Err(e) => error!("Failed to receive terminate signal: {e:?}"),
        };
        Ok(())
    })
}

/// Initialize the logger with environment variable configuration.
fn init_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

/// Initialize the color_eyre error reporting library.
fn init_color_eyre() -> Result<()> {
    #[allow(unused_mut)]
    let mut hook = HookBuilder::default();

    #[cfg(not(debug_assertions))]
    {
        hook = hook
            .display_env_section(false)
            .display_location_section(false);
    }

    hook.install()
}
