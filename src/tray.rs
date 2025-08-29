use std::{
    ffi::OsStr,
    iter::once,
    mem::{size_of, zeroed},
    os::windows::ffi::OsStrExt as _,
    ptr::{null, null_mut},
};

use color_eyre::eyre::bail;
use log::info;
use winapi::{
    shared::guiddef::GUID,
    um::{
        libloaderapi::GetModuleHandleW,
        shellapi::{
            NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_SETVERSION,
            NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONDATAW_u, Shell_NotifyIconW,
        },
        winuser::{
            CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
            LoadIconW, PostQuitMessage, RegisterClassExW, TranslateMessage, WM_APP, WM_DESTROY,
            WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSEXW,
        },
    },
};

const WINDOW_NAME: &str = "Packetmock";
const WINDOW_CLASS: &str = "packetmockwndcls";
const TOOLTIP: &str = "Packetmock";

const WMAPP_NOTIFYCALLBACK: u32 = WM_APP + 1;

/// Create a system tray icon and handle its events.
pub fn tray() -> color_eyre::Result<()> {
    info!("Creating tray...");

    let instance = unsafe { GetModuleHandleW(null()) };
    let icon = unsafe { LoadIconW(instance, 1 as _) };

    let class_name = wide(WINDOW_CLASS);
    let window_name = wide(WINDOW_NAME);

    let window_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as _,
        style: 0,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: icon,
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: icon,
    };

    if unsafe { RegisterClassExW(&window_class) } == 0 {
        bail!("Failed to register window class");
    }

    let window = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            0,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };

    if window.is_null() {
        bail!("Failed to create window");
    }

    let mut tooltip = [0u16; 128];
    tooltip[..TOOLTIP.len() + 1].copy_from_slice(&wide(TOOLTIP));

    let mut notify_icon = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as _,
        hWnd: window,
        uID: 0,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_GUID | NIF_SHOWTIP,
        uCallbackMessage: WMAPP_NOTIFYCALLBACK,
        hIcon: icon,
        szTip: tooltip,
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

    unsafe {
        let mut message = zeroed();

        while GetMessageW(&mut message, null_mut(), 0, 0) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    info!("Tray exiting...");

    Ok(())
}

unsafe extern "system" fn wnd_proc(
    hwnd: winapi::shared::windef::HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match msg {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        WMAPP_NOTIFYCALLBACK => match lparam as u32 {
            WM_LBUTTONUP => {
                //
                0
            }
            WM_RBUTTONUP => {
                unsafe { PostQuitMessage(0) };
                0
            }
            _ => 0,
        },
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn wide(arg: &str) -> Vec<u16> {
    OsStr::new(arg).encode_wide().chain(once(0)).collect()
}
