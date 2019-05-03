use winapi::shared::minwindef::BOOL;
use winapi::shared::minwindef::LPARAM;
use winapi::shared::windef::HWND;
use winapi::um::winbase::LocalFree;
use winapi::um::winnt::LPWSTR;

fn main() {
    println!("Hello, world!");

    use winapi::shared::windef::HDESK;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use winapi::um::winbase;
    use winapi::um::winbase::FormatMessageW;
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
    let enum_windows_rc = unsafe { EnumDesktopWindows(desktop, Some(fn1), 666isize) };
    fn GetLastErrorEx() -> String {
        let err = unsafe { GetLastError() };
        use winapi::um::winnt::WCHAR;
        let msg: *mut WCHAR = ptr::null_mut();
        use winapi::ctypes::c_char;
        use winapi::vc::vadefs::va_list;
        let mut valistx = ptr::null_mut::<c_char>();

        let mut buffer: LPWSTR = ptr::null_mut();
        use std::ffi::OsString;
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
        let xx = OsString::from_wide(slice);
        unsafe { LocalFree(buffer as *mut winapi::ctypes::c_void) };

        eprintln!("get_last_error char count: {}", charcount);
        if charcount == 0 {
            panic!("GetLastError failed.");
        }
        xx.into_string().expect("Kan ikke faa string fra OsString?")
    }
    if enum_windows_rc == 0 {
        let errormsg = GetLastErrorEx();

        eprintln!("EnumDesktopWindows failed: {:?}", errormsg);
        return;
    }

    // EnumDesktopWindows()
}

unsafe extern "system" fn fn1(_: HWND, _: LPARAM) -> BOOL {
    0
}
