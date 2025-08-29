#![cfg(windows)]

mod http;
mod mock;
mod service;
mod windivert;

use std::mem::zeroed;

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::info;
use windivert_sys::WINDIVERT_ADDRESS;

use crate::{
    http::is_client_hello,
    mock::{FAKE_CLIENT_HELLO, FAKE_HTTP_REQUEST},
    service::{handle_if_service, install_service, start_service, stop_service, uninstall_service},
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
}

/// Main entry point for the application.
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    handle_if_service()?;

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run) | None => run()?,
        Some(Commands::Install) => install_service()?,
        Some(Commands::Uninstall) => uninstall_service()?,
        Some(Commands::Start) => start_service()?,
        Some(Commands::Stop) => stop_service()?,
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
