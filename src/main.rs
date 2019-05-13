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

    // Get title for hwnd.
    let cc = unsafe { winapi::um::winuser::GetWindowTextW(hwnd, &mut buf[0], buf.len() as i32) };
    use std::os::windows::prelude::*;
    let osstring = OsString::from_wide(&buf[0..cc as usize]);
    let title = osstring.to_string_lossy().into_owned();
    if title.is_empty() {
        return true;
    }
    let prochandlex = {
        let phandle = unsafe { GetProcessHandleFromHwnd(hwnd) };
        if phandle.is_null() {
            // There is often tens of this kind of windows. They are not even
            // real windows, so we just ignore them.
            return true;
        }
        WinHandleDrop::new(phandle)
    };

    // Get window process application executable name.
    let cc = unsafe {
        winapi::um::psapi::GetProcessImageFileNameW(prochandlex.0, &mut buf[0], buf.len() as u32)
    };
    let processfilename = OsString::from_wide(&buf[0..cc as usize]);
    let exepath = processfilename.to_string_lossy().into_owned();
    let mut exepath =
        exepath.rsplit('\\').nth(0).expect("exepath split by \\ has no elements.").to_string();
    exepath.make_ascii_lowercase();

    cbs.windows.push(CbWindowInfo { title, hwnd, exepath });
    true
}

unsafe extern "system" fn enum_win_cb_raw(hwnd: HWND, lp: LPARAM) -> BOOL {
    let cbs = &mut *(lp as *mut CallbackState);
    if enum_windows_cb(cbs, hwnd) {
        1
    } else {
        0
    }
}

fn get_window_list() -> Result<Vec<CbWindowInfo>, String> {
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use winapi::um::winuser::EnumDesktopWindows;
    use winapi::um::winuser::GetThreadDesktop;
    use winx::WinStyle;

    let curthreadid = unsafe { GetCurrentThreadId() };
    let desktop = unsafe { GetThreadDesktop(curthreadid) };
    if desktop.is_null() {
        return Err("GetThreadDesktop failed.".to_string());
    }
    let winlist_raw = {
        let mut cbstate = CallbackState { windows: vec![] };
        if unsafe {
            EnumDesktopWindows(
                desktop,
                Some(enum_win_cb_raw),
                &mut cbstate as *mut CallbackState as isize,
            )
        } == 0
        {
            return Err(format!("EnumDesktopWindows failed: {}", winx::get_last_error_ex()));
        };

        let winlist: Vec<_> = cbstate
            .windows
            .into_iter()
            .filter(|winfo| match winx::get_window_info(winfo.hwnd) {
                Ok(info) => {
                    if (info.dwStyle & (WinStyle::WS_DISABLED)).bits() != 0 {
                        false
                    } else {
                        // 20190513: Edge windows are WS_POPUP and are owned by 'applicationframehost.exe'.
                        let looks_like_edge = || winfo.title.ends_with("- Microsoft Edge");
                        (info.dwStyle & WinStyle::WS_POPUP).bits() == 0 || looks_like_edge()
                    }
                }
                Err(_) => true,
            })
            .collect();
        winlist
    };

    let curproc: Option<String> = match std::env::current_exe() {
        Ok(path) => path.file_name().map(|osstr| osstr.to_string_lossy().into_owned()),
        Err(_) => {
            // nothing, not removing ourselves from the list is not terrible.
            None
        }
    };
    let winlist = match curproc {
        Some(exename) => winlist_raw.into_iter().filter(|winfo| winfo.title != exename).collect(),
        None => winlist_raw,
    };
    Ok(winlist)
}

fn main() -> Result<(), String> {
    use std::collections::HashMap;

    let winlist = get_window_list()?;
    // We want a list of some browser windows first, and then (some of) the other windows.
    let (brwins, otherwins): (Vec<_>, Vec<_>) = winlist.iter().partition(|&v| {
        v.exepath.find("chrome.exe").is_some()
            || v.exepath.find("firefox.exe").is_some()
            || v.exepath.find("iexplore.exe").is_some()
            || v.title.ends_with("- Microsoft Edge")
    });
    let mut keyed_wins: HashMap<String, (&CbWindowInfo, u32)> = HashMap::new();
    for (digit, win) in "123456789".chars().zip(brwins) {
        let mut ss = String::new();
        ss.push(digit);
        keyed_wins.insert(ss, (win, keyed_wins.len() as u32));
    }
    for (ch, win) in "abcdefghij".chars().zip(otherwins) {
        let mut ss = String::new();
        ss.push(ch);
        keyed_wins.insert(ss, (win, keyed_wins.len() as u32));
    }

    let mut sorted: Vec<_> = keyed_wins.iter().map(|(k, &(win, ord))| (k, win, ord)).collect();
    sorted.sort_by_key(|x| x.2);

    fn is_numeric(key: &str) -> bool {
        key.chars().next().unwrap().is_numeric()
    }

    for (idx, &(key, winfo, _ord)) in sorted.iter().enumerate() {
        if idx > 0 && is_numeric(sorted[idx - 1].0) != is_numeric(key) {
            println!();
        }
        println!(
            "[{}{}{}] {}{}{}",
            crossterm::Colored::Fg(crossterm::Color::Green),
            key,
            crossterm::Colored::Fg(crossterm::Color::White),
            crossterm::Colored::Fg(crossterm::Color::Yellow),
            winfo.title,
            crossterm::Colored::Fg(crossterm::Color::White),
        );
    }
    let winfo = loop {
        println!("Press one of the digits or letters to focus the window. Press ctrl+c, escape or q to abort:");
        let chr = crossterm::input()
            .read_char()
            .map_err(|err| format!("Read input failed: {:?}", err))?;
        // ascii 3 is ctrl-c.
        if chr == 3 as char || chr == 27 as char || chr == 'q' {
            return Ok(());
        }
        let mut mystr = String::new();
        mystr.push(chr);
        if let Some(v) = keyed_wins.get(&mystr) {
            break v.0;
        };
    };
    winx::focus_window(winfo.hwnd).map_err(|err| format!("Could not focus window: {} ", err))?;
    Ok(())
}
