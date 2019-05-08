
use winapi::shared::minwindef::ATOM;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::UINT;
use winapi::shared::minwindef::WORD;
// use winapi::shared::minwindef::BOOL;
use super::*;
use winapi::shared::windef::RECT;

bitflags! {
    // https://docs.microsoft.com/en-us/windows/desktop/winmsg/window-styles
    #[derive(Default)]
    #[repr(C)]
    pub struct WinStyle : u32 {
       const WS_DISABLED = 0x08000000;
       const WS_MINIMIZE = 0x20000000;
       const WS_MINIMIZEBOX = 0x00020000;
       const WS_POPUP = 0x80000000;
       const WS_MAXIMIZE = 0x01000000;
       const WS_MAXIMIZEBOX = 0x00010000;
       const WS_GROUP = 0x00020000;
       const WS_DLGFRAME = 0x00400000;
       const WS_CHILD = 0x40000000;
       const WS_SIZEBOX = 0x00040000;
       const WS_CLIPSIBLINGS = 0x04000000;
       const WS_VISIBLE = 0x10000000;
       const WS_BORDER = 0x00800000;
       const WS_CAPTION = 0x00C00000;
       const WS_CLIPCHILDREN = 0x02000000;
       const WS_SYSMENU = 0x00080000;
       const WS_TABSTOP = 0x00010000;
       const WS_THICKFRAME = 0x00040000;
       const WS_VSCROLL = 0x00200000;
    }
}

#[repr(C)]
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

#[link(name = "user32")]
extern "system" {
    fn GetWindowInfo(hwdn: HWND, winfo: *mut WindowInfo) -> BOOL;
}

pub(crate) fn get_window_info(hwnd: HWND) -> Result<WindowInfo, String> {
    let mut wi = WindowInfo {
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
    };
    match unsafe { GetWindowInfo(hwnd, &mut wi as *mut WindowInfo) } {
        0 => Err(get_last_error_ex()),
        _ => Ok(wi),
    }
}
