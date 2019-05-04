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

fn main() {
    println!("Hello, world!");

    use winapi::shared::windef::HDESK;
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use winapi::um::winuser::EnumDesktopWindows;
    use winapi::um::winuser::GetThreadDesktop;
    use winapi::um::winuser::WNDENUMPROC;

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
    unsafe extern "system" fn fn1(hwnd: HWND, lp: LPARAM) -> BOOL {
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
        println!("Some title: {}", title);
        let phandle = GetProcessHandleFromHwnd(hwnd);
        if phandle == std::ptr::null_mut() {
            eprintln!("GetProcessHandleFromHwnd failed: {}", get_last_error_ex());
            return 1;
        }

        use winapi::um::processthreadsapi;
        let procid = processthreadsapi::GetProcessId(phandle);
        println!("process id: {}", procid);


        // winapi::um::winbase::QueryFullProcessImageNameW(phandle, 0, )
        let cc = winapi::um::psapi::GetProcessImageFileNameW(phandle, &mut buf[0], buf.len() as u32);
        let processfilename = OsString::from_wide(&buf[0..cc as usize]);
        let exepath = processfilename
            .into_string()
            .expect("Conv osstring -> String fejlede");
        println!("Process file name: {}", exepath);

        // Hent modulnavn for hwnd-ens proces.
        use winapi::um::libloaderapi::GetModuleFileNameW;
        let cc = GetModuleFileNameW(
            phandle as winapi::shared::minwindef::HMODULE,
            &mut buf[0],
            buf.len() as u32,
        );
        if cc == 0 {
            eprintln!("GetModuleFileNameW gav 0 tegn: {}", get_last_error_ex());
            return 1;
        }
        let osstring2 = OsString::from_wide(&buf[0..cc as usize]);
        let exepath = osstring2
            .into_string()
            .expect("Conv osstring -> String fejlede");
        println!("exe path: {}", exepath);

        let mut cbs = lp as *mut CallbackState;
        (*cbs).windows.push(title);
        1
    }
    let mut cbstate = CallbackState { windows: vec![] };
    let enum_windows_rc = unsafe {
        EnumDesktopWindows(
            desktop,
            Some(fn1),
            &mut cbstate as *mut CallbackState as isize,
        )
    };
    if enum_windows_rc == 0 {
        let errormsg = get_last_error_ex();

        eprintln!("EnumDesktopWindows failed: {}", errormsg);
        return;
    }

    // EnumDesktopWindows()
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
