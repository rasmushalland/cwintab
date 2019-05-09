#![allow(non_snake_case)]

#[macro_use]
extern crate bitflags;

mod winx;

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
            eprintln!("CloseHandle failed: {}", winx::get_last_error_ex());
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
    let cbs = std::mem::transmute(lp as *mut CallbackState);
    match enum_windows_cb(cbs, hwnd) {
        true => 1,
        false => 0,
    }
}

fn main() -> Result<(), String> {
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use winapi::um::winuser::EnumDesktopWindows;
    use winapi::um::winuser::GetThreadDesktop;
    use winx::WinStyle;

    use std::ptr;
    let curthreadid = unsafe { GetCurrentThreadId() };
    let desktop = unsafe { GetThreadDesktop(curthreadid) };
    if desktop == ptr::null_mut() {
        eprintln!("GetThreadDesktop failed.");
        return Ok(());
    }
    let winlist = {
        let mut cbstate = CallbackState { windows: vec![] };
        match unsafe {
            EnumDesktopWindows(
                desktop,
                Some(enum_win_cb_raw),
                &mut cbstate as *mut CallbackState as isize,
            )
        } {
            0 => {
                eprintln!("EnumDesktopWindows failed: {}", winx::get_last_error_ex());
                return Ok(());
            }
            _ => (),
        };

        let winlist: Vec<_> = cbstate
            .windows
            .into_iter()
            .filter(|winfo| match winx::get_window_info(winfo.hwnd) {
                Ok(info) => {
                    (info.dwStyle & (WinStyle::WS_DISABLED | WinStyle::WS_POPUP)).bits() == 0
                }
                Err(_) => true,
            })
            .collect();

        winlist
    };

    let options: Vec<CbWindowInfo> = winlist.into_iter().take(10).collect::<Vec<_>>();

    for (idx, winfo) in options.iter().enumerate() {
        println!(
            "[{:2}] {}{}{}",
            idx + 1,
            crossterm::Colored::Fg(crossterm::Color::Yellow),
            winfo.title,
            crossterm::Colored::Fg(crossterm::Color::White),
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
    if let Err(err) = winx::focus_window(c.hwnd as *mut winapi::shared::windef::HWND__) {
        eprintln!("Kunne ikke saette fokus: {} ", err);
    }
    Ok(())
}
