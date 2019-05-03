use winapi::shared::minwindef::BOOL;
use winapi::shared::minwindef::LPARAM;
use winapi::shared::windef::HWND;

fn main() {
    println!("Hello, world!");

    use winapi::shared::windef::HDESK;
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use winapi::um::winuser::EnumDesktopWindows;
    use winapi::um::winuser::GetThreadDesktop;
    use winapi::um::winuser::WNDENUMPROC;

    let curthreadid = unsafe { GetCurrentThreadId() };
    let desktop = unsafe { GetThreadDesktop(curthreadid) };
    let windows = unsafe {
        let b = EnumDesktopWindows(desktop, Some(fn1), 666isize);
    };

    // EnumDesktopWindows()
}

unsafe extern "system" fn fn1(_: HWND, _: LPARAM) -> BOOL {
    0
}
