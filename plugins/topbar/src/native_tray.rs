//! Keep Explorer's taskbars out of view while WinRust owns the desktop.
use super::*;
use i18n::desktop::MASKED_TASKBAR;

fn masked(hwnd: HWND) -> bool {
    let (mut alpha, mut flags) = (255, 0);
    unsafe {
        !GetPropW(hwnd, MASKED_TASKBAR).is_null()
            && GetLayeredWindowAttributes(hwnd, null_mut(), &mut alpha, &mut flags) != 0
            && flags & LWA_ALPHA != 0
            && alpha == 0
    }
}

pub(super) fn hidden(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd) == 0 || masked(hwnd) }
}

unsafe fn set_exstyle(hwnd: HWND, style: isize) -> bool {
    SetLastError(0);
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style) != 0 || GetLastError() == 0
}

pub(super) unsafe fn mask(hwnd: HWND) -> bool {
    if masked(hwnd) { return true; }
    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let layered = WS_EX_LAYERED as isize;
    // Leave windows that already use their own layered style to the ShowWindow fallback.
    if GetPropW(hwnd, MASKED_TASKBAR).is_null() {
        if style & layered != 0 || SetPropW(hwnd, MASKED_TASKBAR, 1 as HANDLE) == 0 { return false; }
    }
    if (style & layered != 0 || set_exstyle(hwnd, style | layered))
        && SetLayeredWindowAttributes(hwnd, 0, 0, LWA_ALPHA) != 0
    {
        return true;
    }
    restore(hwnd);
    false
}

pub(super) unsafe fn restore(hwnd: HWND) {
    if GetPropW(hwnd, MASKED_TASKBAR).is_null() { return; }
    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    // Restore opacity first so even a failed style reset gives Explorer pixels back.
    SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
    if style & WS_EX_LAYERED as isize == 0 || set_exstyle(hwnd, style & !(WS_EX_LAYERED as isize)) {
        RemovePropW(hwnd, MASKED_TASKBAR);
        RedrawWindow(hwnd, null(), null_mut(), RDW_INVALIDATE | RDW_FRAME | RDW_ALLCHILDREN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_survives_reshow_and_region_changes_then_restores_the_window() {
        unsafe {
            let style = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            let hwnd = CreateWindowExW(style, windows_sys::w!("STATIC"), null(), WS_POPUP,
                -32000, -32000, 100, 40, null_mut(), null_mut(), GetModuleHandleW(null()), null());
            assert!(!hwnd.is_null());
            // It is safe to show only the disposable window, outside all displays.
            assert!(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL).is_null());
            let original = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            assert!(mask(hwnd));
            for _ in 0..5 {
                let region = CreateRectRgn(0, 0, 100, 40);
                assert!(!region.is_null());
                assert_ne!(SetWindowRgn(hwnd, region, 1), 0);
                ShowWindow(hwnd, SW_SHOWNA);
                assert_ne!(IsWindowVisible(hwnd), 0);
                assert_ne!(SetWindowRgn(hwnd, null_mut(), 1), 0);
                assert!(masked(hwnd));
                ShowWindow(hwnd, SW_HIDE);
            }
            restore(hwnd);
            assert!(GetPropW(hwnd, MASKED_TASKBAR).is_null());
            assert_eq!(GetWindowLongPtrW(hwnd, GWL_EXSTYLE), original);
            DestroyWindow(hwnd);
        }
    }
}
