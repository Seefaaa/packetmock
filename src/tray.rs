mod menu;
pub mod toast;
mod window;

use std::{
    mem::{size_of, zeroed},
    ptr::null_mut,
};

use color_eyre::{
    Result,
    eyre::{Report, bail},
};
use log::{error, info};
use winapi::{
    shared::{guiddef::GUID, windef::HWND},
    um::{
        shellapi::{
            NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE,
            NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONDATAW_u,
            Shell_NotifyIconW,
        },
        winuser::{
            DefWindowProcW, PostQuitMessage, WM_APP, WM_DESTROY, WM_LBUTTONUP, WM_NCCREATE,
            WM_RBUTTONUP,
        },
    },
};
use windows::core::{PCWSTR, w};

use crate::service::{
    ServiceState, install_service, query_service, start_service, stop_service, uninstall_service,
};

use self::{
    menu::{
        MENU_ID_EXIT, MENU_ID_INSTALL, MENU_ID_START, MENU_ID_STOP, MENU_ID_UNINSTALL,
        show_popup_menu,
    },
    toast::show_toast,
    window::Window,
};

/// Title and class name for the hidden window.
const WINDOW_TITLE: PCWSTR = w!("Packetmock");
const WINDOW_CLASS: PCWSTR = w!("packetmockwndcls");

/// Windows message ID for tray icon callbacks.
const WMAPP_NOTIFYCALLBACK: u32 = WM_APP + 1;

/// Create a system tray icon and handle its events.
pub fn run_tray() -> Result<()> {
    info!("Creating tray");

    let window = Window::new(WINDOW_CLASS, WINDOW_TITLE, Some(wnd_proc))?;
    let tray_icon = TrayIcon::new(&window)?;

    show_toast("Running in system tray")?;

    window.event_loop();

    info!("Tray exited");

    drop(tray_icon);
    drop(window);

    Ok(())
}

/// Window procedure to handle window messages.
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_NCCREATE => Window::nc_create(hwnd, lparam),
        WM_DESTROY => unsafe { PostQuitMessage(0) },
        WMAPP_NOTIFYCALLBACK => match lparam as u32 {
            WM_LBUTTONUP => {}
            WM_RBUTTONUP => unsafe {
                if let Err(e) = match show_popup_menu(hwnd) {
                    Ok(id) => match id as usize {
                        MENU_ID_START => match start_service() {
                            Ok(_) => toast_ok("Service started"),
                            Err(e) => toast_err("Failed to start service", e),
                        },
                        MENU_ID_STOP => match stop_service() {
                            Ok(_) => toast_ok("Service stopped"),
                            Err(e) => toast_err("Failed to stop service", e),
                        },
                        MENU_ID_INSTALL => match install_service() {
                            Ok(_) => toast_ok("Service installed"),
                            Err(e) => toast_err("Failed to install service", e),
                        },
                        MENU_ID_UNINSTALL => match uninstall_service() {
                            Ok(_) => toast_ok("Service uninstalled"),
                            Err(e) => toast_err("Failed to uninstall service", e),
                        },
                        MENU_ID_EXIT => {
                            let _ = match query_service() {
                                Ok(ServiceState::Running) => {
                                    show_toast("Service is running in background")
                                }
                                Err(e) => {
                                    error!("Failed to query service status: {e:?}");
                                    Ok(())
                                }
                                _ => Ok(()),
                            };

                            PostQuitMessage(0);

                            Ok(())
                        }
                        _ => Ok(()),
                    },
                    Err(e) => {
                        error!("Failed to show popup menu: {e:?}");
                        Ok(())
                    }
                } {
                    error!("Failed to handle menu action: {e:?}");
                }
            },
            _ => {}
        },
        _ => {}
    };

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Show a toast notification for a successful operation.
fn toast_ok(msg: &str) -> Result<()> {
    info!("{msg}");
    show_toast(msg)?;
    Ok(())
}

/// Show a toast notification for an error.
fn toast_err(msg: &str, e: Report) -> Result<()> {
    error!("{e:?}");
    show_toast(&format!("{msg}: {e}"))?;
    Ok(())
}

/// Represents the tray icon in the system tray.
struct TrayIcon {
    notify_icon: NOTIFYICONDATAW,
}

impl TrayIcon {
    /// Create a new tray icon associated with the given window.
    fn new(window: &Window) -> Result<Self> {
        let mut notify_icon = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as _,
            hWnd: window.hwnd,
            uID: 0,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_GUID | NIF_SHOWTIP,
            uCallbackMessage: WMAPP_NOTIFYCALLBACK,
            hIcon: window.icon,
            szTip: {
                let mut tip = [0; 128];
                unsafe { tip[..WINDOW_TITLE.len()].copy_from_slice(WINDOW_TITLE.as_wide()) };
                tip
            },
            dwState: 0,
            dwStateMask: 0,
            szInfo: [0; 256],
            u: unsafe {
                let mut u: NOTIFYICONDATAW_u = zeroed();
                *u.uVersion_mut() = NOTIFYICON_VERSION_4;
                u
            },
            szInfoTitle: [0; 64],
            dwInfoFlags: 0,
            guidItem: GUID {
                Data1: 0x71df8b00,
                Data2: 0xe359,
                Data3: 0x4cf3,
                Data4: [0xb1, 0x7b, 0x57, 0x62, 0xae, 0xd8, 0x44, 0x14],
            },
            hBalloonIcon: null_mut(),
        };

        unsafe {
            if Shell_NotifyIconW(NIM_ADD, &mut notify_icon) != 1
                || Shell_NotifyIconW(NIM_SETVERSION, &mut notify_icon) != 1
            {
                bail!("Failed to create tray icon");
            }
        }

        Ok(Self { notify_icon })
    }
}

impl Drop for TrayIcon {
    /// Remove the tray icon when the `TrayIcon` is dropped.
    fn drop(&mut self) {
        unsafe {
            if Shell_NotifyIconW(NIM_DELETE, &mut self.notify_icon) == 0 {
                error!("Failed to delete tray icon");
            }
        }
    }
}
