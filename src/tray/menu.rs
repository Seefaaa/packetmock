use std::{
    iter::once,
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};

use color_eyre::Result;
use winapi::{
    shared::windef::{HMENU, HWND},
    um::{
        winnt::LPWSTR,
        winuser::{
            AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, InsertMenuItemW,
            MENUITEMINFOW, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
            MFS_DEFAULT, MIIM_STATE, MIIM_STRING, SetForegroundWindow, TPM_BOTTOMALIGN,
            TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TrackPopupMenuEx,
        },
    },
};
use windows_sys::w;

use crate::{
    service::{ServiceState, query_service},
    tasksch::run_on_startup,
    windivert::ttl::get_ttl,
};

pub const MENU_ID_EXIT: usize = 1000;
pub const MENU_ID_STOP: usize = 1001;
pub const MENU_ID_UNINSTALL: usize = 1002;
pub const MENU_ID_START: usize = 1003;
pub const MENU_ID_INSTALL: usize = 1004;
pub const MENU_ID_TTL: usize = 2000;
pub const MENU_ID_STARTUP: usize = 3000;

/// Create the popup menu based on the current service state.
fn create_popup_menu() -> Result<HMENU> {
    unsafe {
        let service = query_service()?;

        let menu = CreatePopupMenu();
        let settings_menu = create_settings_menu()?;

        InsertMenuItemW(menu, 0, 1, &service.into());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());

        match service {
            ServiceState::NotInstalled => {
                AppendMenuW(menu, MF_STRING, MENU_ID_INSTALL, w!("Install service"));
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            }
            ServiceState::Stopped => {
                AppendMenuW(menu, MF_STRING, MENU_ID_START, w!("Start service"));
                AppendMenuW(menu, MF_STRING, MENU_ID_UNINSTALL, w!("Uninstall service"));
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            }
            ServiceState::Running => {
                AppendMenuW(menu, MF_STRING, MENU_ID_STOP, w!("Stop service"));
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            }
            _ => {}
        }

        AppendMenuW(menu, MF_POPUP, settings_menu as _, w!("Settings"));
        AppendMenuW(menu, MF_STRING, MENU_ID_EXIT, w!("Exit"));

        Ok(menu)
    }
}

/// Create the settings submenu.
fn create_settings_menu() -> Result<HMENU> {
    unsafe {
        let menu = CreatePopupMenu();
        let ttl_menu = create_ttl_menu()?;

        let ttl = get_ttl();

        let ttl_text = format!("TTL (current: {ttl})");
        let ttl_wide: Vec<u16> = ttl_text.encode_utf16().chain(once(0)).collect();

        AppendMenuW(menu, MF_POPUP, ttl_menu as _, ttl_wide.as_ptr());

        let checked = if run_on_startup() {
            MF_CHECKED
        } else {
            MF_UNCHECKED
        };

        AppendMenuW(
            menu,
            MF_STRING | checked,
            MENU_ID_STARTUP,
            w!("Show tray on startup"),
        );

        Ok(menu)
    }
}

/// Create the TTL submenu.
fn create_ttl_menu() -> Result<HMENU> {
    unsafe {
        let menu = CreatePopupMenu();

        AppendMenuW(menu, MF_STRING, MENU_ID_TTL + 5, w!("Increment (+5)"));
        AppendMenuW(menu, MF_STRING, MENU_ID_TTL + 1, w!("Increment (+1)"));
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, MENU_ID_TTL - 1, w!("Decrement (-1)"));
        AppendMenuW(menu, MF_STRING, MENU_ID_TTL - 5, w!("Decrement (-5)"));

        Ok(menu)
    }
}

/// Show the popup menu and return the selected menu item ID.
pub fn show_popup_menu(hwnd: HWND) -> Result<i32> {
    unsafe {
        let menu = create_popup_menu()?;

        let pos = {
            let mut point = zeroed();
            GetCursorPos(&mut point);
            point
        };

        SetForegroundWindow(hwnd);

        let flags = TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_NONOTIFY | TPM_RETURNCMD;
        let result = TrackPopupMenuEx(menu, flags, pos.x, pos.y, hwnd, null_mut());

        DestroyMenu(menu);

        Ok(result)
    }
}

impl From<ServiceState> for MENUITEMINFOW {
    fn from(value: ServiceState) -> Self {
        MENUITEMINFOW {
            cbSize: size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_STRING | MIIM_STATE,
            fType: 0,
            fState: MFS_DEFAULT,
            wID: 0,
            hSubMenu: null_mut(),
            hbmpChecked: null_mut(),
            hbmpUnchecked: null_mut(),
            dwItemData: 0,
            dwTypeData: LPWSTR::from(value),
            cch: 0,
            hbmpItem: null_mut(),
        }
    }
}

impl From<ServiceState> for LPWSTR {
    fn from(value: ServiceState) -> Self {
        match value {
            ServiceState::NotInstalled => w!("Service (Not installed)") as _,
            ServiceState::Running => w!("Service (Running)") as _,
            ServiceState::Stopped => w!("Service (Stopped)") as _,
            ServiceState::StartPending => w!("Service (Starting)") as _,
            ServiceState::StopPending => w!("Service (Stopping)") as _,
            ServiceState::ContinuePending => w!("Service (Continuing)") as _,
            ServiceState::PausePending => w!("Service (Pausing)") as _,
            ServiceState::Paused => w!("Service (Paused)") as _,
        }
    }
}
