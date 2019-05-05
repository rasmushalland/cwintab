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

struct WinHandle {
    handle: *mut winapi::ctypes::c_void,
}
impl WinHandle {
    fn new(handle: *mut winapi::ctypes::c_void) -> WinHandle {
        WinHandle { handle: handle }
    }
}
impl Drop for WinHandle {
    fn drop(&mut self) {
        let rc = unsafe { winapi::um::handleapi::CloseHandle(self.handle) };
        if rc == 0 {
            eprintln!("CloseHandle failed: {}", get_last_error_ex());
        }
    }
}

fn main() {
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use winapi::um::winuser::EnumDesktopWindows;
    use winapi::um::winuser::GetThreadDesktop;

    use std::ptr;
    let curthreadid = unsafe { GetCurrentThreadId() };
    let desktop = unsafe { GetThreadDesktop(curthreadid) };
    if desktop == ptr::null_mut() {
        eprintln!("GetThreadDesktop failed.");
        return;
    }
    struct CallbackState {
        windows: Vec<String>,
        // hwnd: HWND,
    }
    unsafe extern "system" fn enum_win_cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        let buf = &mut [0u16; 200];

        // Hent title for hwnd.
        let cc = winapi::um::winuser::GetWindowTextW(hwnd, &mut buf[0], buf.len() as i32);
        use std::os::windows::prelude::*;
        let osstring = OsString::from_wide(&buf[0..cc as usize]);
        let title = osstring
            .into_string()
            .expect("Conv osstring -> String fejlede");
        if title.len() == 0 {
            return 1;
        }
        if title == "Default IME" || title == "MSCTFIME UI" {
            return 1;
        }
        let prochandlex = {
            let phandle = GetProcessHandleFromHwnd(hwnd);
            if phandle == std::ptr::null_mut() {
                // Der er 10-50 af disse. Fejler med 'Access denied', og vinduerne
                // er ikke interessante, og oftest ikke rigtige vinduer.
                return 1;
            }
            WinHandle::new(phandle)
        };

        let cc = winapi::um::psapi::GetProcessImageFileNameW(
            prochandlex.handle,
            &mut buf[0],
            buf.len() as u32,
        );
        let processfilename = OsString::from_wide(&buf[0..cc as usize]);
        let exepath = processfilename
            .into_string()
            .expect("Conv osstring -> String fejlede");
        if !exepath.ends_with("chrome.exe") {
            return 1;
        }
        println!("Some title: {}", title);

        let cbs: &mut CallbackState = std::mem::transmute(lp as *mut CallbackState);
        if cbs.windows.len() == 1 {
        // if title.contains("SetForegroundWindow") {
            let rc = winapi::um::winuser::SetForegroundWindow(hwnd);
            if rc == 0 {
                eprintln!("SetForegroundWindow fejlede: {}", get_last_error_ex());
            }
        }
        cbs.windows.push(title);
        1
    }
    let mut cbstate = CallbackState { windows: vec![] };
    let enum_windows_rc = unsafe {
        EnumDesktopWindows(
            desktop,
            Some(enum_win_cb),
            &mut cbstate as *mut CallbackState as isize,
        )
    };
    if enum_windows_rc == 0 {
        eprintln!("EnumDesktopWindows failed: {}", get_last_error_ex());
        return;
    }
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

    osstring
        .into_string()
        .expect("Kan ikke faa string fra OsString?")
}
