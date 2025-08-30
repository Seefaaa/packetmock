use std::{
    env::temp_dir,
    ffi::OsStr,
    fs::{create_dir_all, write},
    iter::once,
    mem::{size_of, zeroed},
    os::windows::ffi::OsStrExt as _,
    ptr::{null, null_mut},
};

use color_eyre::eyre::bail;
use log::{error, info};
use winapi::{
    shared::guiddef::GUID,
    um::{
        libloaderapi::GetModuleHandleW,
        shellapi::{
            NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_SETVERSION,
            NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONDATAW_u, Shell_NotifyIconW,
        },
        winuser::{
            CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
            DispatchMessageW, GetCursorPos, GetMessageW, InsertMenuItemW, LoadIconW, MENUITEMINFOW,
            MIIM_ID, MIIM_STRING, PostMessageW, PostQuitMessage, RegisterClassExW,
            SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
            TPM_RIGHTBUTTON, TPM_VERPOSANIMATION, TrackPopupMenuEx, TranslateMessage, WM_APP,
            WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSEXW,
        },
    },
};
use windows::{
    UI::Notifications::{ToastNotification, ToastNotificationManager, ToastTemplateType},
    core::w,
};
use windows_registry::LOCAL_MACHINE;

const WINDOW_NAME: &str = "Packetmock";
const WINDOW_CLASS: &str = "packetmockwndcls";

const TRAY_TOOLTIP: &str = "Packetmock";

const TOAST_DISPLAY_NAME: &str = "Packetmock";
const TOAST_APPID: &str = "Seefaaa.Packetmock";
const TOAST_ICON: &[u8] = include_bytes!("../resources/pink48.png");
const TOAST_ICON_TEMP: &str = "toast.png";

const WMAPP_NOTIFYCALLBACK: u32 = WM_APP + 1;

/// Create a system tray icon and handle its events.
pub fn show_system_tray() -> color_eyre::Result<()> {
    info!("Creating tray");

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
    tooltip[..TRAY_TOOLTIP.len() + 1].copy_from_slice(&wide(TRAY_TOOLTIP));

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

    show_toast("Running in system tray")?;

    unsafe {
        let mut message = zeroed();

        while GetMessageW(&mut message, null_mut(), 0, 0) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    info!("Tray exited");

    Ok(())
}

unsafe extern "system" fn wnd_proc(
    hwnd: winapi::shared::windef::HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match msg {
        WM_DESTROY => unsafe {
            PostQuitMessage(0);
            return 0;
        },
        WMAPP_NOTIFYCALLBACK => match lparam as u32 {
            WM_LBUTTONUP => {}
            WM_RBUTTONUP => unsafe {
                let menu = CreatePopupMenu();

                let items = [
                    #[cfg(debug_assertions)]
                    (w!("Show Toast"), 1001),
                    (w!("Exit"), 1000),
                ];

                for (pos, item) in items.iter().enumerate() {
                    let menu_item = MENUITEMINFOW {
                        cbSize: size_of::<MENUITEMINFOW>() as _,
                        fMask: MIIM_ID | MIIM_STRING,
                        fType: 0,
                        fState: 0,
                        wID: item.1,
                        hSubMenu: null_mut(),
                        hbmpChecked: null_mut(),
                        hbmpUnchecked: null_mut(),
                        dwItemData: 0,
                        dwTypeData: item.0.as_ptr() as _,
                        cch: (item.0.len() - 1) as _,
                        hbmpItem: null_mut(),
                    };

                    if InsertMenuItemW(menu, pos as _, 1, &menu_item) == 0 {
                        error!("Failed to insert menu item");
                        DestroyMenu(menu);
                        return 0;
                    }
                }

                let pos = {
                    let mut point = zeroed();
                    GetCursorPos(&mut point);
                    point
                };

                SetForegroundWindow(hwnd);

                let result = TrackPopupMenuEx(
                    menu,
                    TPM_LEFTALIGN
                        | TPM_BOTTOMALIGN
                        | TPM_NONOTIFY
                        | TPM_RETURNCMD
                        | TPM_RIGHTBUTTON
                        | TPM_VERPOSANIMATION,
                    pos.x,
                    pos.y,
                    hwnd,
                    null_mut(),
                );

                match result {
                    1000 => {
                        PostQuitMessage(0);
                    }
                    #[cfg(debug_assertions)]
                    1001 => {
                        if let Err(e) = show_toast("This is a test toast") {
                            error!("Failed to show toast: {e}");
                        }
                    }
                    _ => {}
                }

                DestroyMenu(menu);
                PostMessageW(hwnd, 0, 0, 0);
            },
            _ => {}
        },
        _ => {}
    };

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn wide(arg: &str) -> Vec<u16> {
    OsStr::new(arg).encode_wide().chain(once(0)).collect()
}

fn show_toast(message: &str) -> color_eyre::Result<()> {
    let temp = temp_dir().join(TOAST_APPID);
    create_dir_all(&temp)?;
    let icon_path = temp.join(TOAST_ICON_TEMP);
    write(&icon_path, TOAST_ICON)?;

    let key = LOCAL_MACHINE
        .options()
        .volatile()
        .read()
        .write()
        .create()
        .open(format!("Software\\Classes\\AppUserModelId\\{TOAST_APPID}"))?;

    key.set_string("DisplayName", TOAST_DISPLAY_NAME)?;
    key.set_string("IconUri", icon_path.to_string_lossy().as_ref())?;

    let toast_template = ToastTemplateType::ToastImageAndText01;
    let toast_xml = ToastNotificationManager::GetTemplateContent(toast_template)?;

    let text_elements = toast_xml.GetElementsByTagName(&"text".into())?;

    if text_elements.Length()? > 0 {
        let message_element = text_elements.Item(0)?;
        message_element.SetInnerText(&message.into())?;
    }

    let toast = ToastNotification::CreateToastNotification(&toast_xml)?;

    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&TOAST_APPID.into())?;
    notifier.Show(&toast)?;

    Ok(())
}
