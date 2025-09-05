use std::{iter::once, ptr::null};

use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
    System::Threading::{CreateMutexW, ReleaseMutex},
};

pub struct MutexGuard {
    handle: HANDLE,
    owned: bool,
}

impl MutexGuard {
    pub fn new(name: &str) -> Self {
        let name = name.encode_utf16().chain(once(0)).collect::<Vec<_>>();

        let handle = unsafe { CreateMutexW(null(), 1, name.as_ptr()) };
        let owned = !handle.is_null() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS;

        MutexGuard { handle, owned }
    }

    #[inline]
    pub fn is_owned(&self) -> bool {
        self.owned
    }
}

impl Drop for MutexGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            if self.owned {
                unsafe { ReleaseMutex(self.handle) };
            }
            unsafe { CloseHandle(self.handle) };
        }
    }
}
