#![cfg(windows)]
#![windows_subsystem = "windows"]

mod http;
mod mock;
mod service;
mod tray;
mod windivert;

use std::mem::zeroed;

use clap::{CommandFactory as _, Parser, Subcommand};
use env_logger::Env;
use log::info;
use smol::{block_on, future::or, unblock};
use winapi::um::wincon::{ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole};
use windivert_sys::WINDIVERT_ADDRESS;

use crate::{
    http::is_client_hello,
    mock::{FAKE_CLIENT_HELLO, FAKE_HTTP_REQUEST},
    service::{handle_if_service, install_service, start_service, stop_service, uninstall_service},
    tray::show_system_tray,
    windivert::{BUFFER_SIZE, WINDIVERT_FILTER, WinDivert},
};

const TTL: u8 = 4;

/// Command line interface for the application.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Commands that the application can execute.
#[derive(Subcommand)]
enum Commands {
    /// Run as CLI application
    Run,
    /// Install as Windows service
    Install,
    /// Uninstall Windows service
    Uninstall,
    /// Start Windows service
    Start,
    /// Stop Windows service
    Stop,
    /// Run as system tray application (debug only)
    #[cfg(debug_assertions)]
    RunTray,
}

/// Main entry point for the application.
fn main() -> color_eyre::Result<()> {
    let is_terminal = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;

    color_eyre::install()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    handle_if_service()?;

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run) => run()?,
        None if !is_terminal => run_tray()?,
        Some(Commands::Install) => install_service()?,
        Some(Commands::Uninstall) => uninstall_service()?,
        Some(Commands::Start) => start_service()?,
        Some(Commands::Stop) => stop_service()?,
        None => Cli::command().print_help()?,
        #[cfg(debug_assertions)]
        Some(Commands::RunTray) => run_tray()?,
    }

    if is_terminal {
        unsafe { FreeConsole() };
    }

    Ok(())
}

fn run() -> color_eyre::Result<()> {
    let windivert = WinDivert::open(WINDIVERT_FILTER)?;

    let mut buffer = [0; BUFFER_SIZE];
    let mut address: WINDIVERT_ADDRESS = unsafe { zeroed() };

    info!("Intercepting packets...");

    while let Ok(packet) = windivert.recv(&mut buffer, &mut address) {
        match u16::from_be(packet.tcp_header().DstPort) {
            // HTTP
            80 => {
                if packet.data().is_some() {
                    let mut packet_copy = packet.try_clone()?;
                    packet_copy.set_data(FAKE_HTTP_REQUEST)?;
                    packet_copy.ip_header_mut().TTL = TTL;
                    windivert.send(packet_copy)?;
                }
            }
            // HTTPS
            443 => {
                if is_client_hello(&packet) {
                    let mut packet_copy = packet.try_clone()?;
                    packet_copy.set_data(FAKE_CLIENT_HELLO)?;
                    packet_copy.ip_header_mut().TTL = TTL;
                    windivert.send(packet_copy)?;
                }
            }
            _ => unreachable!(),
        }

        windivert.send(packet)?;
    }

    Ok(())
}

fn run_tray() -> color_eyre::Result<()> {
    block_on(or(unblock(show_system_tray), unblock(run)))
}
