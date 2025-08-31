use std::{
    env::{args_os, current_exe},
    ffi::OsString,
    process::exit,
    sync::mpsc,
    time::Duration,
};

use color_eyre::Result;
use log::{error, info};
use smol::{block_on, future::or, unblock};
use windows_service::{
    Error as WSError, define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState as WSServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

use crate::windivert::intercept;

/// Name of the Windows service.
#[cfg(not(debug_assertions))]
const SERVICE_NAME: &str = "PacketmockSrv";
#[cfg(debug_assertions)]
const SERVICE_NAME: &str = "PacketmockDevSrv";
/// Display name of the Windows service.
const SERVICE_DISPLAY_NAME: &str = "Packetmock Service";
/// Type of the Windows service.
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

/// Run the service if the program was started with the "run-service" argument.
pub fn handle_service() -> Result<()> {
    let args = args_os().skip(1).take(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "run-service") {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        exit(0);
    }
    Ok(())
}

/// Entry point for the Windows service.
fn service_main(_: Vec<OsString>) {
    info!("Service is starting");
    if let Err(e) = run_service() {
        error!("Service encountered an error: {e:?}");
    }
}

/// Main logic for running the Windows service.
fn run_service() -> Result<()> {
    let (shudown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Sending shutdown signal to service");
                if let Err(e) = shudown_tx.send(()) {
                    error!("Failed to send shutdown signal: {e:?}");
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
        current_state: WSServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    let shutdown = move || {
        match shutdown_rx.recv() {
            Ok(_) => info!("Shutdown signal received."),
            Err(e) => error!("Failed to receive shutdown signal: {e:?}"),
        };
        Ok(())
    };

    block_on(or(unblock(shutdown), unblock(intercept)))?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: WSServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("Service has stopped");

    Ok(())
}

/// Installs the exe as a Windows service
pub fn install_service() -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: current_exe()?,
        launch_arguments: vec!["run-service".into()],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(env!("CARGO_PKG_DESCRIPTION"))?;

    Ok(())
}

/// Uninstalls the Windows service
pub fn uninstall_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(SERVICE_NAME, ServiceAccess::DELETE)?;

    service.delete()?;

    Ok(())
}

/// Starts the Windows service
pub fn start_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(SERVICE_NAME, ServiceAccess::START)?;

    service.start::<&str>(&[])?;

    Ok(())
}

/// Stops the Windows service
pub fn stop_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(SERVICE_NAME, ServiceAccess::STOP)?;

    service.stop()?;

    Ok(())
}

pub fn query_service() -> Result<ServiceState> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;

    let service = match manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
        Ok(s) => s,
        Err(e) => match e {
            WSError::Winapi(e) if e.raw_os_error() == Some(1060) => {
                return Ok(ServiceState::NotInstalled);
            }
            _ => return Err(e.into()),
        },
    };

    let status = service.query_status()?;

    Ok(status.current_state.into())
}

/// Represents the state of the Windows service.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    NotInstalled,
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
}

impl From<WSServiceState> for ServiceState {
    /// Convert a `WSServiceState` from the Windows API into a `ServiceState`.
    fn from(state: WSServiceState) -> Self {
        match state {
            WSServiceState::Stopped => ServiceState::Stopped,
            WSServiceState::StartPending => ServiceState::StartPending,
            WSServiceState::StopPending => ServiceState::StopPending,
            WSServiceState::Running => ServiceState::Running,
            WSServiceState::ContinuePending => ServiceState::ContinuePending,
            WSServiceState::PausePending => ServiceState::PausePending,
            WSServiceState::Paused => ServiceState::Paused,
        }
    }
}
