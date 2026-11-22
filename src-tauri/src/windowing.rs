//! Brings an agent's own window to the front and maximises it.
//!
//! Fanal never confirms a prompt on the user's behalf. "Open" means exactly
//! that the agent's window becomes the foreground window.

#![cfg(windows)]

use std::path::Path;

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, SW_MAXIMIZE, SW_RESTORE,
};

struct Search {
    wanted: Vec<u32>,
    found: HWND,
}

unsafe extern "system" fn visit(hwnd: HWND, param: LPARAM) -> i32 {
    unsafe {
        let search = &mut *(param as *mut Search);
        if IsWindowVisible(hwnd) == 0 || GetWindowTextLengthW(hwnd) == 0 {
            return 1;
        }
        let mut owner = 0u32;
        GetWindowThreadProcessId(hwnd, &mut owner);
        if search.wanted.contains(&owner) {
            search.found = hwnd;
            return 0;
        }
        1
    }
}

fn window_for(pids: &[u32]) -> Option<HWND> {
    let mut search = Search {
        wanted: pids.to_vec(),
        found: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(Some(visit), &mut search as *mut Search as LPARAM);
    }
    if search.found.is_null() {
        None
    } else {
        Some(search.found)
    }
}

/// Maximise and focus the window belonging to one of these processes. A helper
/// process shares the agent's name and has no window of its own.
pub fn focus(pids: &[u32]) -> bool {
    let Some(hwnd) = window_for(pids) else {
        return false;
    };
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        ShowWindow(hwnd, SW_MAXIMIZE);
        SetForegroundWindow(hwnd) != 0
    }
}

/// Start an executable detached and return its process id.
pub fn launch(path: &Path) -> Option<u32> {
    use std::process::Command;
    let child = Command::new(path).spawn().ok()?;
    Some(child.id())
}

