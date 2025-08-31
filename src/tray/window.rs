use std::{
    mem::zeroed,
    pin::Pin,
    ptr::{null, null_mut},
};

use color_eyre::{Result, eyre::bail};
use winapi::{
    shared::windef::{HICON, HWND},
    um::{
        libloaderapi::GetModuleHandleW,
        winuser::{
            CREATESTRUCTW, CW_USEDEFAULT, CreateWindowExW, DispatchMessageW, GWLP_USERDATA,
            GetMessageW, GetWindowLongPtrW, LoadIconW, RegisterClassExW, SetWindowLongPtrW,
            TranslateMessage, WNDCLASSEXW, WNDPROC,
        },
    },
};
use windows::core::PCWSTR;

/// A hidden window to receive tray icon events.
pub struct Window {
    pub hwnd: HWND,
    pub icon: HICON,
}

impl Window {
    /// Create a new hidden window with the specified class name, title, and window procedure.
    pub fn new(class: PCWSTR, title: PCWSTR, proc: WNDPROC) -> Result<Pin<Box<Self>>> {
        let instance = unsafe { GetModuleHandleW(null()) };
        let icon = unsafe { LoadIconW(instance, 1 as _) };

        let mut window = Box::pin(Window {
            hwnd: null_mut(),
            icon,
        });

        let wnd_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as _,
            style: 0,
            lpfnWndProc: proc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: icon,
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
            hIconSm: icon,
        };

        if unsafe { RegisterClassExW(&wnd_class) } == 0 {
            bail!("Failed to register window class");
        }

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                0,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                null_mut(),
                null_mut(),
                instance,
                &*window as *const _ as _,
            )
        };

        if hwnd.is_null() {
            bail!("Failed to create window");
        }

        window.hwnd = hwnd;

        Ok(window)
    }

    /// Start the window's event loop to process messages.
    pub fn event_loop(&self) {
        unsafe {
            let mut message = zeroed();

            while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    /// Handle the `WM_NCCREATE` message to associate the window with the `Window` struct.
    pub fn nc_create(hwnd: HWND, lparam: isize) {
        let create_struct = unsafe { *(lparam as *const CREATESTRUCTW) };
        let window = create_struct.lpCreateParams as *mut Self;

        if window.is_null() {
            panic!("WM_NCCREATE: l_param is null");
        }

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, window as _) };
    }
}

impl From<HWND> for &'_ mut Window {
    /// Get a mutable reference to the `Window` struct from the window handle.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from(value: HWND) -> Self {
        let ptr = unsafe { GetWindowLongPtrW(value, GWLP_USERDATA) } as *mut Window;

        if ptr.is_null() {
            panic!("GetWindowLongPtrW returned null");
        }

        unsafe { &mut *ptr }
    }
}
