use super::*;
use winapi::shared::minwindef::ATOM;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::UINT;
use winapi::shared::minwindef::WORD;
use winapi::shared::windef::RECT;

bitflags! {
    // https://docs.microsoft.com/en-us/windows/desktop/winmsg/window-styles
    #[derive(Default)]
    #[repr(C)]
    pub struct WinStyle : u32 {
       const WS_DISABLED = 0x0800_0000;
       const WS_MINIMIZE = 0x2000_0000;
       const WS_MINIMIZEBOX = 0x0002_0000;
       const WS_POPUP = 0x8000_0000;
       const WS_MAXIMIZE = 0x0100_0000;
       const WS_MAXIMIZEBOX = 0x0001_0000;
       const WS_GROUP = 0x0002_0000;
       const WS_DLGFRAME = 0x0040_0000;
       const WS_CHILD = 0x4000_0000;
       const WS_SIZEBOX = 0x0004_0000;
       const WS_CLIPSIBLINGS = 0x0400_0000;
       const WS_VISIBLE = 0x1000_0000;
       const WS_BORDER = 0x0080_0000;
       const WS_CAPTION = 0x00C0_0000;
       const WS_CLIPCHILDREN = 0x0200_0000;
       const WS_SYSMENU = 0x0008_0000;
       const WS_TABSTOP = 0x0001_0000;
       const WS_THICKFRAME = 0x0004_0000;
       const WS_VSCROLL = 0x0020_0000;
    }
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct WindowInfo {
    cbSize: DWORD,
    rcWindow: RECT,
    rcClient: RECT,
    pub dwStyle: WinStyle,
    // dwStyle: DWORD,
    dwExStyle: DWORD,
    dwWindowStatus: DWORD,
    cxWindowBorders: UINT,
    cyWindowBorders: UINT,
    atomWindowType: ATOM,
    wCreatorVersion: WORD,
}

impl Default for WindowInfo {
    fn default() -> WindowInfo {
        WindowInfo {
            cbSize: std::mem::size_of::<WindowInfo>() as u32,
            rcWindow: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            rcClient: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            dwStyle: WinStyle::default(),
            dwExStyle: 0,
            dwWindowStatus: 0,
            cxWindowBorders: 0,
            cyWindowBorders: 0,
            atomWindowType: 0,
            wCreatorVersion: 0,
        }
    }
}

#[link(name = "user32")]
extern "system" {
    fn GetWindowInfo(hwdn: HWND, winfo: *mut WindowInfo) -> BOOL;
}

pub(crate) fn get_window_info(hwnd: HWND) -> Result<WindowInfo, String> {
    let mut wi = WindowInfo::default();
    match unsafe { GetWindowInfo(hwnd, &mut wi as *mut WindowInfo) } {
        0 => Err(get_last_error_ex()),
        _ => Ok(wi),
    }
}

pub(crate) fn is_window_minimized(hwnd: HWND) -> Result<bool, String> {
    let wi = get_window_info(hwnd)?;
    Ok((wi.dwStyle & WinStyle::WS_MINIMIZE).bits() != 0)
}

pub fn focus_window(hwnd: HWND) -> Result<(), String> {
    if winx::is_window_minimized(hwnd)? {
        const SW_RESTORE: i32 = 9;
        let rc = unsafe { winapi::um::winuser::ShowWindow(hwnd, SW_RESTORE) };
        if rc == 0 {
            return Err(format!("ShowWindow failed: {}", get_last_error_ex()));
        }
    }
    let rc = unsafe { winapi::um::winuser::BringWindowToTop(hwnd) };
    if rc == 0 {
        return Err(format!("BringWindowToTop failed: {}", get_last_error_ex()));
    }

    let rc = unsafe { winapi::um::winuser::SetForegroundWindow(hwnd) };
    if rc == 0 {
        return Err(format!("SetForegroundWindow failed: {}", get_last_error_ex()));
    }
    Ok(())
}

pub fn get_last_error_ex() -> String {
    use std::ptr;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winbase;
    use winapi::um::winbase::FormatMessageW;
    let err = unsafe { GetLastError() };
    use winapi::ctypes::c_char;
    let mut valistx = ptr::null_mut::<c_char>();

    let mut buffer: LPWSTR = ptr::null_mut();
    use std::os::windows::prelude::*;

    let charcount = unsafe {
        FormatMessageW(
            winbase::FORMAT_MESSAGE_ALLOCATE_BUFFER
                | winbase::FORMAT_MESSAGE_FROM_SYSTEM
                | winbase::FORMAT_MESSAGE_IGNORE_INSERTS,
            ptr::null(),
            err,
            0,
            (&mut buffer as *mut LPWSTR) as LPWSTR,
            500,
            &mut valistx,
        )
    };
    if charcount == 0 {
        panic!("GetLastError failed.");
    }
    let slice = unsafe { std::slice::from_raw_parts(buffer, charcount as usize) };
    let osstring = OsString::from_wide(slice);

    unsafe { LocalFree(buffer as *mut winapi::ctypes::c_void) };

    osstring.into_string().expect("Can't get String fra OsString?")
}
