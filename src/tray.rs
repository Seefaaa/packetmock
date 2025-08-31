mod menu;
mod toast;
mod window;

use std::{
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};

use color_eyre::eyre::Report;
use log::{error, info};
use winapi::{
    shared::{
        guiddef::GUID,
        windef::{HICON, HWND},
    },
    um::{
        libloaderapi::GetModuleHandleW,
        shellapi::{
            NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_SETVERSION,
            NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONDATAW_u, Shell_NotifyIconW,
        },
        winuser::{
            DefWindowProcW, LoadIconW, PostQuitMessage, WM_APP, WM_DESTROY, WM_LBUTTONUP,
            WM_NCCREATE, WM_RBUTTONUP,
        },
    },
};
use windows::core::{PCWSTR, w};

use crate::service::{
    ServiceState, install_service, query_service, start_service, stop_service, uninstall_service,
};

use self::{
    menu::{
        MENU_ID_EXIT, MENU_ID_INSTALL_SERVICE, MENU_ID_START_SERVICE, MENU_ID_STOP_SERVICE,
        MENU_ID_UNINSTALL_SERVICE, show_popup_menu,
    },
    toast::show_toast,
    window::Window,
};

const WINDOW_NAME: PCWSTR = w!("Packetmock");
const WINDOW_CLASS: PCWSTR = w!("packetmockwndcls");

const TRAY_TOOLTIP: PCWSTR = WINDOW_NAME;

const WMAPP_NOTIFYCALLBACK: u32 = WM_APP + 1;

/// Create a system tray icon and handle its events.
pub fn show_tray_icon() -> color_eyre::Result<()> {
    info!("Creating tray");

    let instance = unsafe { GetModuleHandleW(null()) };
    let icon = unsafe { LoadIconW(instance, 1 as _) };
    let window = Window::new(instance, WINDOW_CLASS, WINDOW_NAME, icon, Some(wnd_proc))?;

    create_tray_icon(window, icon);
    show_toast("Running in system tray")?;

    window.event_loop();

    info!("Tray exited");

    Window::drop(window);

    Ok(())
}

fn create_tray_icon(window: &Window, icon: HICON) {
    let mut notify_icon = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as _,
        hWnd: window.hwnd,
        uID: 0,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_GUID | NIF_SHOWTIP,
        uCallbackMessage: WMAPP_NOTIFYCALLBACK,
        hIcon: icon,
        szTip: {
            let mut tip = [0; 128];
            unsafe { tip[..TRAY_TOOLTIP.len()].copy_from_slice(TRAY_TOOLTIP.as_wide()) };
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
        Shell_NotifyIconW(NIM_ADD, &mut notify_icon);
        Shell_NotifyIconW(NIM_SETVERSION, &mut notify_icon);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_NCCREATE => Window::nc_create(hwnd, lparam),
        WM_DESTROY => unsafe { PostQuitMessage(0) },
        WMAPP_NOTIFYCALLBACK => match lparam as u32 {
            WM_LBUTTONUP => {}
            WM_RBUTTONUP => unsafe {
                match show_popup_menu(hwnd) {
                    MENU_ID_START_SERVICE => {
                        let _ = match start_service() {
                            Ok(_) => toast_ok("Service started"),
                            Err(e) => toast_err("Failed to start service", e),
                        };
                    }
                    MENU_ID_STOP_SERVICE => {
                        let _ = match stop_service() {
                            Ok(_) => toast_ok("Service stopped"),
                            Err(e) => toast_err("Failed to stop service", e),
                        };
                    }
                    MENU_ID_INSTALL_SERVICE => {
                        let _ = match install_service() {
                            Ok(_) => toast_ok("Service installed"),
                            Err(e) => toast_err("Failed to install service", e),
                        };
                    }
                    MENU_ID_UNINSTALL_SERVICE => {
                        let _ = match uninstall_service() {
                            Ok(_) => toast_ok("Service uninstalled"),
                            Err(e) => toast_err("Failed to uninstall service", e),
                        };
                    }
                    MENU_ID_EXIT => {
                        let status = query_service().unwrap();

                        if status == ServiceState::Running {
                            let _ = show_toast("Service is running in background");
                        }

                        PostQuitMessage(0)
                    }
                    _ => {}
                };
            },
            _ => {}
        },
        _ => {}
    };

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn toast_ok(msg: &str) -> color_eyre::Result<()> {
    info!("{msg}");
    show_toast(msg)?;
    Ok(())
}

fn toast_err(msg: &str, e: Report) -> color_eyre::Result<()> {
    error!("{e:?}");
    show_toast(&format!("{msg}: {e}"))?;
    Ok(())
}
