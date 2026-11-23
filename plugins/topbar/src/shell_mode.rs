use super::*;
use std::time::{Duration, Instant};
use i18n::windows_mode;

pub(super) const PAUSE: u32 = WM_APP + 17;
pub(super) const RETRY_TIMER: usize = 8;

thread_local! {
    static REQUEST: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub(super) unsafe fn claim_bar(resuming: bool) -> bool {
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        // Keep the handle for the lifetime of the bar, including while the native shell is starting.
        let mutex = CreateMutexW(null(), 0, windows_sys::w!("Local\\topbar-single"));
        if !mutex.is_null() {
            if GetLastError() != ERROR_ALREADY_EXISTS && FindWindowW(windows_sys::w!("topbar"), null()).is_null() {
                return true;
            }
            CloseHandle(mutex);
        }
        if !resuming || !windows_mode::enabled() || Instant::now() >= until { return false; }
        thread::sleep(Duration::from_millis(100));
    }
}

unsafe fn error(hwnd: HWND, message: &str) {
    MessageBoxW(hwnd, wide(message).as_ptr(), windows_sys::w!("WinRust"), MB_OK | MB_ICONERROR);
}

pub(super) fn resume() -> bool {
    match windows_mode::set_enabled(false) {
        Ok(()) => true,
        Err(e) => {
            unsafe { error(null_mut(), &tf!("Не удалось возобновить WinRust: {e}", "Could not resume WinRust: {e}", "Não foi possível reativar o WinRust ({e}).")); }
            false
        }
    }
}

pub(super) unsafe fn pause(hwnd: HWND) {
    if REQUEST.get().is_some() { return; }
    if let Err(e) = windows_mode::set_enabled(true) {
        error(hwnd, &tf!("Не удалось сохранить режим Windows: {e}", "Could not save Windows mode: {e}", "Não foi possível salvar o Modo Windows ({e})."));
        return;
    }
    REQUEST.set(Some(Instant::now()));
    if find_other("Shell_TrayWnd").is_null() {
        // The shell needs the normal user token even when topbar is an elevated logon task.
        let explorer = windows_mode::explorer();
        let started = Command::new(explorer.with_file_name("System32").join("runas.exe"))
            .arg("/trustlevel:0x20000")
            .arg(format!("\"{}\"", explorer.display()))
            .creation_flags(NO_WINDOW)
            .spawn();
        if started.is_err() || SetTimer(hwnd, RETRY_TIMER, 100, None) == 0 {
            failed(hwnd);
        }
    } else {
        check(hwnd);
    }
}

pub(super) unsafe fn check(hwnd: HWND) {
    let Some(since) = REQUEST.get() else { return };
    if !windows_mode::enabled() {
        clear(hwnd);
    } else if !find_other("Shell_TrayWnd").is_null() {
        clear(hwnd);
        let desktop = FindWindowW(windows_sys::w!("Progman"), windows_sys::w!("rs-desktop"));
        if !desktop.is_null() { PostMessageW(desktop, WM_CLOSE, 0, 0); }
        // The usual shutdown path releases the appbars and gives the real tray back to Explorer.
        DestroyWindow(hwnd);
    } else if since.elapsed() >= Duration::from_secs(8) {
        failed(hwnd);
    }
}

unsafe fn clear(hwnd: HWND) {
    REQUEST.set(None);
    KillTimer(hwnd, RETRY_TIMER);
}

unsafe fn failed(hwnd: HWND) {
    clear(hwnd);
    if let Err(e) = windows_mode::set_enabled(false) {
        error(hwnd, &tf!("Не удалось отменить режим Windows: {e}", "Could not cancel Windows mode: {e}", "Não foi possível cancelar o Modo Windows ({e})."));
    } else {
        error(hwnd, t!("Не удалось запустить панель задач Windows. Попробуйте ещё раз.", "The Windows taskbar did not start. Try again."));
    }
}
