use std::{
    mem::zeroed,
    ptr::{null, null_mut},
};

use color_eyre::eyre::bail;
use winapi::{
    shared::{
        minwindef::HMODULE,
        windef::{HICON, HWND},
    },
    um::winuser::{
        CREATESTRUCTW, CW_USEDEFAULT, CreateWindowExW, DispatchMessageW, GWLP_USERDATA,
        GetMessageW, GetWindowLongPtrW, RegisterClassExW, SetWindowLongPtrW, TranslateMessage,
        WNDCLASSEXW, WNDPROC,
    },
};
use windows::core::PCWSTR;

pub struct Window {
    pub hwnd: HWND,
}

impl Window {
    pub fn new<'a>(
        hinstance: HMODULE,
        window_class: PCWSTR,
        window_name: PCWSTR,
        icon: HICON,
        wnd_proc: WNDPROC,
    ) -> color_eyre::Result<&'a mut Self> {
        let window = Box::leak(Box::new(Window { hwnd: null_mut() }));

        window.hwnd = create_window(window, hinstance, window_class, window_name, icon, wnd_proc)?;

        Ok(window)
    }

    pub fn event_loop(&self) {
        unsafe {
            let mut message = zeroed();

            while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    pub fn nc_create(hwnd: HWND, lparam: isize) {
        let create_struct = unsafe { *(lparam as *const CREATESTRUCTW) };
        let window = create_struct.lpCreateParams as *mut Window;

        if window.is_null() {
            panic!("WM_NCCREATE: l_param is null");
        }

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, window as _) };
    }

    pub fn drop(window: &mut Self) {
        unsafe {
            drop(Box::from_raw(window));
        }
    }
}

impl From<HWND> for &'_ mut Window {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from(value: HWND) -> Self {
        let ptr = unsafe { GetWindowLongPtrW(value, GWLP_USERDATA) } as *mut Window;

        if ptr.is_null() {
            panic!("GetWindowLongPtrW returned null");
        }

        unsafe { &mut *ptr }
    }
}

fn create_window(
    window: &mut Window,
    hinstance: HMODULE,
    window_class: PCWSTR,
    window_name: PCWSTR,
    icon: HICON,
    wnd_proc: WNDPROC,
) -> color_eyre::Result<HWND> {
    let wnd_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as _,
        style: 0,
        lpfnWndProc: wnd_proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: icon,
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: window_class.as_ptr(),
        hIconSm: icon,
    };

    if unsafe { RegisterClassExW(&wnd_class) } == 0 {
        bail!("Failed to register window class");
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            window_class.as_ptr(),
            window_name.as_ptr(),
            0,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            null_mut(),
            null_mut(),
            hinstance,
            window as *mut _ as _,
        )
    };

    if hwnd.is_null() {
        bail!("Failed to create window");
    }

    Ok(hwnd)
}
