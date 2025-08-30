use std::sync::mpsc;

use clap::{CommandFactory as _, Parser, Subcommand};
use log::{error, info};
use smol::{block_on, future::or, unblock};

use crate::{
    service::{install_service, start_service, stop_service, uninstall_service},
    tray::show_tray_icon,
    windivert::intercept,
};

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

pub fn handle_cli(is_terminal: bool) -> color_eyre::Result<()> {
    let cli = Cli::parse();

    if let Err(e) = match cli.command {
        Some(Commands::Run) => run_cli(),
        None if !is_terminal => run_tray(),
        Some(Commands::Install) => install_service(),
        Some(Commands::Uninstall) => uninstall_service(),
        Some(Commands::Start) => start_service(),
        Some(Commands::Stop) => stop_service(),
        None => Ok(Cli::command().print_help()?),
        #[cfg(debug_assertions)]
        Some(Commands::RunTray) => run_tray(),
    } {
        error!("{e:?}");
    }

    Ok(())
}

pub fn run_cli() -> color_eyre::Result<()> {
    let (sx, rx) = mpsc::channel();

    ctrlc::set_handler(move || {
        sx.send(()).expect("Could not send terminate signal");
    })?;

    block_on(or(
        unblock(move || {
            match rx.recv() {
                Ok(_) => info!("Terminate signal received."),
                Err(e) => error!("Failed to receive terminate signal: {e}"),
            };
            Ok(())
        }),
        unblock(intercept),
    ))
}

fn run_tray() -> color_eyre::Result<()> {
    block_on(or(unblock(show_tray_icon), unblock(run_cli)))
}
