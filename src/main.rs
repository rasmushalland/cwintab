#![allow(non_snake_case)]

use std::ffi::OsString;
use winapi::shared::minwindef::BOOL;
use winapi::shared::minwindef::LPARAM;
use winapi::shared::windef::HWND;
use winapi::um::winbase::LocalFree;
use winapi::um::winnt::LPWSTR;

#[link(name = "oleacc")]
extern "system" {
    fn GetProcessHandleFromHwnd(hwdn: HWND) -> winapi::um::winnt::HANDLE;
}

struct WinHandleDrop(*mut winapi::ctypes::c_void);
impl WinHandleDrop {
    fn new(handle: *mut winapi::ctypes::c_void) -> WinHandleDrop {
        WinHandleDrop(handle)
    }
}
impl Drop for WinHandleDrop {
    fn drop(&mut self) {
        let rc = unsafe { winapi::um::handleapi::CloseHandle(self.0) };
        if rc == 0 {
            eprintln!("CloseHandle failed: {}", get_last_error_ex());
        }
    }
}
#[derive(Clone)]
struct CbWindowInfo {
    title: String,
    hwnd: HWND,
    exepath: String,
}
struct CallbackState {
    windows: Vec<CbWindowInfo>,
}
fn enum_windows_cb(cbs: &mut CallbackState, hwnd: HWND) -> bool {
    let buf = &mut [0u16; 200];

    // Hent title for hwnd.
    let cc = unsafe { winapi::um::winuser::GetWindowTextW(hwnd, &mut buf[0], buf.len() as i32) };
    use std::os::windows::prelude::*;
    let osstring = OsString::from_wide(&buf[0..cc as usize]);
    let title = osstring.into_string().expect("Conv osstring -> String fejlede");
    if title.len() == 0 {
        return true;
    }
    if title == "Default IME" || title == "MSCTFIME UI" {
        return true;
    }
    let prochandlex = {
        let phandle = unsafe { GetProcessHandleFromHwnd(hwnd) };
        if phandle == std::ptr::null_mut() {
            // Der er 10-50 af disse. Fejler med 'Access denied', og vinduerne
            // er ikke interessante, og oftest ikke rigtige vinduer.
            return true;
        }
        WinHandleDrop::new(phandle)
    };

    let cc = unsafe {
        winapi::um::psapi::GetProcessImageFileNameW(prochandlex.0, &mut buf[0], buf.len() as u32)
    };
    let processfilename = OsString::from_wide(&buf[0..cc as usize]);
    let exepath = processfilename.into_string().expect("Conv osstring -> String fejlede");
    // if !exepath.ends_with("chrome.exe") {
    //     return true;
    // }
    let mut exepath =
        exepath.rsplit('\\').nth(0).expect("exepath split by \\ has no elements.").to_string();
    exepath.make_ascii_lowercase();

    cbs.windows.push(CbWindowInfo { title, hwnd, exepath });
    true
}

unsafe extern "system" fn enum_win_cb_raw(hwnd: HWND, lp: LPARAM) -> BOOL {
    let cbs: &mut CallbackState = std::mem::transmute(lp as *mut CallbackState);
    if enum_windows_cb(cbs, hwnd) {
        1
    } else {
        0
    }
}

fn focus_window(hwnd: HWND) -> Result<(), String> {
    // let rc = unsafe { winapi::um::winuser::ShowWindow(hwnd, 0) };
    // if rc == 0 {
    //     return Err(format!("ShowWindow failed: {}", get_last_error_ex()));
    // }
    let rc = unsafe { winapi::um::winuser::BringWindowToTop(hwnd) };
    if rc == 0 {
        return Err(format!("BringWindowToTop failed: {}", get_last_error_ex()));
    }

    let rc = unsafe { winapi::um::winuser::SetForegroundWindow(hwnd) };
    if rc == 0 {
        return Err(format!("SetForegroundWindow fejlede: {}", get_last_error_ex()));
    }
    Ok(())
}

use winapi::shared::minwindef::ATOM;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::UINT;
use winapi::shared::minwindef::WORD;
// use winapi::shared::minwindef::BOOL;
use winapi::shared::windef::RECT;

#[macro_use]
extern crate bitflags;

bitflags! {
    // https://docs.microsoft.com/en-us/windows/desktop/winmsg/window-styles
    #[derive(Default)]
    struct WinStyle : u32 {
       const WS_DISABLED = 0x08000000;
       const WS_MINIMIZE = 0x20000000;
       const WS_POPUP = 0x80000000;
       const WS_MAXIMIZE = 0x01000000;
       const WS_GROUP = 0x00020000;
       const WS_DLGFRAME = 0x00400000;
       const WS_CHILD = 0x40000000;
       const WS_SIZEBOX = 0x00040000;
       const WS_CLIPSIBLINGS = 0x04000000;
       const WS_VISIBLE = 0x10000000;
    }
}

const WS_DISABLED: DWORD = 0x08000000;

#[repr(C)]
struct WindowInfo {
    cbSize: DWORD,
    rcWindow: RECT,
    rcClient: RECT,
    dwStyle: WinStyle,
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

fn get_window_info(hwnd: HWND) -> Result<WindowInfo, String> {
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

fn main() -> Result<(), String> {
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use winapi::um::winuser::EnumDesktopWindows;
    use winapi::um::winuser::GetThreadDesktop;

    use std::ptr;
    let curthreadid = unsafe { GetCurrentThreadId() };
    let desktop = unsafe { GetThreadDesktop(curthreadid) };
    if desktop == ptr::null_mut() {
        eprintln!("GetThreadDesktop failed.");
        return Ok(());
    }
    let mut cbstate = CallbackState { windows: vec![] };
    let enum_windows_rc = unsafe {
        EnumDesktopWindows(
            desktop,
            Some(enum_win_cb_raw),
            &mut cbstate as *mut CallbackState as isize,
        )
    };
    if enum_windows_rc == 0 {
        eprintln!("EnumDesktopWindows failed: {}", get_last_error_ex());
        return Ok(());
    }

    cbstate.windows = cbstate
        .windows
        .into_iter()
        .filter(|winfo| match get_window_info(winfo.hwnd) {
            Ok(info) => (info.dwStyle & (WinStyle::WS_DISABLED | WinStyle::WS_POPUP)).bits() == 0,
            Err(_) => true,
        })
        .collect();

    use itertools::Itertools;
    let mut options: Vec<CbWindowInfo> = Vec::new();
    for winfo in cbstate.windows.into_iter() {
        options.push(winfo);
    }
    // for (_key, group) in &cbstate.windows.iter().map(|v| (&v.exepath, v)).into_group_map() {
    //     if group.len() > 1 {
    //         for winfo in group {
    //             options.push((*winfo).clone());
    //         }
    //     }
    // }

    // let options = cbstate.windows.into_iter().take(10).collect::<Vec<_>>();
    for (idx, winfo) in options.iter().take(150).enumerate() {
        let styledump = match get_window_info(winfo.hwnd) {
            Ok(info) => format!("styles: {:?}", info.dwStyle),
            Err(msg) => msg,
        };
        use crossterm::{Attribute, Color, Colored, Colorize, Styler};

        println!(
            "[{:2}] {}{}{} ({}{}{}) {}",
            idx + 1,
            crossterm::Colored::Fg(crossterm::Color::Yellow),
            winfo.title,
            crossterm::Colored::Fg(crossterm::Color::White),
            crossterm::Colored::Fg(crossterm::Color::Blue),
            winfo.exepath,
            crossterm::Colored::Fg(crossterm::Color::White),
            styledump,
        );
    }
    let num = loop {
        println!("Skriv tal mellem {} og {}.", 1, options.len());
        let choice = crossterm::input().read_line().expect("Kunne ikke laese input.");
        match choice.parse::<u32>() {
            Ok(v) => {
                if v >= 1 && v as usize <= options.len() {
                    break v;
                }
            }
            Err(_) => (),
        };
    };
    let c = &options[num as usize - 1];
    if let Err(err) = focus_window(c.hwnd as *mut winapi::shared::windef::HWND__) {
        eprintln!("Kunne ikke saette fokus: {} ", err);
    }
    Ok(())
}

fn get_last_error_ex() -> String {
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
    let slice = unsafe { std::slice::from_raw_parts(buffer, charcount as usize) };
    let osstring = OsString::from_wide(slice);

    if charcount == 0 {
        panic!("GetLastError failed.");
    }
    unsafe { LocalFree(buffer as *mut winapi::ctypes::c_void) };

    osstring.into_string().expect("Kan ikke faa string fra OsString?")
}
