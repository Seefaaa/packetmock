use std::{
    env::{args_os, current_exe},
    ffi::OsString,
    process::{Command, exit},
    sync::mpsc,
    time::Duration,
};

use color_eyre::owo_colors::OwoColorize;
use log::{error, info};
use smol::{block_on, future, unblock};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

pub const SERVICE_NAME: &str = "PacketmockService";
const SERVICE_DISPLAY_NAME: &str = "Packetmock Service";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

pub fn handle_if_service() -> color_eyre::Result<()> {
    let args = args_os().skip(1).take(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "run-service") {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        exit(0);
    }
    Ok(())
}

fn service_main(_: Vec<OsString>) {
    info!("Service is starting...");
    if let Err(e) = run_service() {
        error!("Service encountered an error: {e}");
    }
}

fn run_service() -> color_eyre::Result<()> {
    let (shudown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Sending shutdown signal to service...");
                if let Err(e) = shudown_tx.send(()) {
                    error!("Failed to send shutdown signal: {e}");
                    return ServiceControlHandlerResult::NotImplemented;
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    block_on(future::or(
        unblock(move || {
            match shutdown_rx.recv() {
                Ok(_) => info!("Shutdown signal received."),
                Err(e) => error!("Failed to receive shutdown signal: {e}"),
            };
            Ok(())
        }),
        unblock(crate::run),
    ))?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("Service has stopped.");

    Ok(())
}

// i could have used the windows-service crate to do these (👇) but
// this was easier and less code to write

/// Installs the exe as a Windows service
pub fn install_service() -> color_eyre::Result<()> {
    let output = Command::new("sc")
        .args([
            "create",
            SERVICE_NAME,
            "start=",
            "auto",
            "binPath=",
            &format!("\"{}\" run-service", current_exe()?.display()),
            "DisplayName=",
            SERVICE_DISPLAY_NAME,
        ])
        .output()?;

    if output.status.success() {
        println!("{}", "Service installed successfully!".bright_green());
    } else {
        eprintln!(
            "{} {}",
            "Failed to install service:".bright_red(),
            String::from_utf8_lossy(&output.stdout).bright_red()
        );
    }

    Ok(())
}

/// Uninstalls the Windows service
pub fn uninstall_service() -> color_eyre::Result<()> {
    let output = Command::new("sc").args(["delete", SERVICE_NAME]).output()?;

    if output.status.success() {
        println!("{}", "Service uninstalled successfully!".bright_green());
    } else {
        eprintln!(
            "{} {}",
            "Failed to uninstall service:".bright_red(),
            String::from_utf8_lossy(&output.stdout).bright_red()
        );
    }

    Ok(())
}

/// Starts the Windows service
pub fn start_service() -> color_eyre::Result<()> {
    let output = Command::new("sc").args(["start", SERVICE_NAME]).output()?;

    if output.status.success() {
        println!("{}", "Service started successfully!".bright_green());
    } else {
        eprintln!(
            "{} {}",
            "Failed to start service:".bright_red(),
            String::from_utf8_lossy(&output.stdout).bright_red()
        );
    }

    Ok(())
}

/// Stops the Windows service
pub fn stop_service() -> color_eyre::Result<()> {
    let output = Command::new("sc").args(["stop", SERVICE_NAME]).output()?;

    if output.status.success() {
        println!("{}", "Service stopped successfully!".bright_green());
    } else {
        eprintln!(
            "{} {}",
            "Failed to stop service:".bright_red(),
            String::from_utf8_lossy(&output.stdout).bright_red()
        );
    }

    Ok(())
}
