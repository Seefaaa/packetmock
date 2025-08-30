mod menu;
mod toast;
mod window;

use std::{
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};

use log::info;
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
                    MENU_ID_START_SERVICE => {}
                    MENU_ID_STOP_SERVICE => {}
                    MENU_ID_INSTALL_SERVICE => {}
                    MENU_ID_UNINSTALL_SERVICE => {}
                    MENU_ID_EXIT => PostQuitMessage(0),
                    _ => {}
                };
            },
            _ => {}
        },
        _ => {}
    };

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
