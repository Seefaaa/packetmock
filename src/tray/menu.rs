use std::{
    mem::{size_of, zeroed},
    ptr::null_mut,
};

use log::error;
use winapi::{
    shared::windef::{HMENU, HWND},
    um::winuser::{
        CreatePopupMenu, DestroyMenu, GetCursorPos, InsertMenuItemW, MENUITEMINFOW, MFS_DEFAULT,
        MFT_SEPARATOR, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_TYPE, SetForegroundWindow,
        TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TrackPopupMenuEx,
    },
};
use windows::core::{PCWSTR, w};

use crate::service::{ServiceState, query_service};

pub const MENU_ID_EXIT: i32 = 1000;
pub const MENU_ID_STOP_SERVICE: i32 = 1001;
pub const MENU_ID_UNINSTALL_SERVICE: i32 = 1002;
pub const MENU_ID_START_SERVICE: i32 = 1003;
pub const MENU_ID_INSTALL_SERVICE: i32 = 1004;

const START_SERVICE: MenuItem = MenuItem::new()
    .text(w!("Start service"))
    .id(MENU_ID_START_SERVICE);
const STOP_SERVICE: MenuItem = MenuItem::new()
    .text(w!("Stop service"))
    .id(MENU_ID_STOP_SERVICE);
const INSTALL_SERVICE: MenuItem = MenuItem::new()
    .text(w!("Install service"))
    .id(MENU_ID_INSTALL_SERVICE);
const UNINSTALL_SERVICE: MenuItem = MenuItem::new()
    .text(w!("Uninstall service"))
    .id(MENU_ID_UNINSTALL_SERVICE);
const EXIT: MenuItem = MenuItem::new().text(w!("Exit")).id(MENU_ID_EXIT);

fn create_popup_menu() -> HMENU {
    unsafe {
        let menu = CreatePopupMenu();

        let service = query_service().expect("Failed to query service status");

        let mut items = Vec::with_capacity(6);

        items.push(service.into());
        items.push(MenuItem::seperator());

        match service {
            ServiceState::NotInstalled => {
                items.push(INSTALL_SERVICE);
                items.push(MenuItem::seperator());
            }
            ServiceState::Stopped => {
                items.push(START_SERVICE);
                items.push(UNINSTALL_SERVICE);
                items.push(MenuItem::seperator());
            }
            ServiceState::Running => {
                items.push(STOP_SERVICE);
                items.push(MenuItem::seperator());
            }
            _ => {}
        }

        items.push(EXIT);

        for (pos, item) in items.iter().enumerate() {
            if InsertMenuItemW(menu, pos as _, 1, &item.into()) == 0 {
                error!("Failed to insert menu item: {item:?}");
            }
        }

        menu
    }
}

pub fn show_popup_menu(hwnd: HWND) -> i32 {
    unsafe {
        let menu = create_popup_menu();

        let pos = {
            let mut point = zeroed();
            GetCursorPos(&mut point);
            point
        };

        SetForegroundWindow(hwnd);

        let flags = TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_NONOTIFY | TPM_RETURNCMD;
        let result = TrackPopupMenuEx(menu, flags, pos.x, pos.y, hwnd, null_mut());

        DestroyMenu(menu);

        result
    }
}

#[derive(Debug)]
pub struct MenuItem {
    fmask: u32,
    ftype: u32,
    fstate: u32,
    wid: u32,
    type_data: Option<PCWSTR>,
}

impl MenuItem {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            fmask: 0,
            ftype: 0,
            fstate: 0,
            wid: 0,
            type_data: None,
        }
    }

    pub const fn text(mut self, text: PCWSTR) -> Self {
        self.fmask |= MIIM_STRING;
        self.type_data = Some(text);
        self
    }

    pub const fn id(mut self, id: i32) -> Self {
        self.fmask |= MIIM_ID;
        self.wid = id as _;
        self
    }

    pub const fn bold(mut self) -> Self {
        self.fmask |= MIIM_STATE;
        self.fstate = MFS_DEFAULT;
        self
    }

    pub const fn seperator() -> Self {
        Self {
            fmask: MIIM_TYPE,
            ftype: MFT_SEPARATOR,
            fstate: 0,
            wid: 0,
            type_data: None,
        }
    }
}

impl From<&MenuItem> for MENUITEMINFOW {
    fn from(item: &MenuItem) -> Self {
        MENUITEMINFOW {
            cbSize: size_of::<MENUITEMINFOW>() as u32,
            fMask: item.fmask,
            fType: item.ftype,
            fState: item.fstate,
            wID: item.wid,
            hSubMenu: null_mut(),
            hbmpChecked: null_mut(),
            hbmpUnchecked: null_mut(),
            dwItemData: 0,
            dwTypeData: item.type_data.map(|s| s.0 as _).unwrap_or(null_mut()),
            cch: 0,
            hbmpItem: null_mut(),
        }
    }
}

impl From<ServiceState> for PCWSTR {
    fn from(state: ServiceState) -> Self {
        match state {
            ServiceState::NotInstalled => w!("Service (Not installed)"),
            ServiceState::Running => w!("Service (Running)"),
            ServiceState::Stopped => w!("Service (Stopped)"),
            ServiceState::StartPending => w!("Service (Starting)"),
            ServiceState::StopPending => w!("Service (Stopping)"),
            ServiceState::ContinuePending => w!("Service (Continuing)"),
            ServiceState::PausePending => w!("Service (Pausing)"),
            ServiceState::Paused => w!("Service (Paused)"),
        }
    }
}

impl From<ServiceState> for MenuItem {
    fn from(value: ServiceState) -> Self {
        MenuItem::new().text(value.into()).bold()
    }
}
