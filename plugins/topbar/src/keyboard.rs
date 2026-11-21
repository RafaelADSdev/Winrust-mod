//! Select a keyboard layout for the window where the user was typing before opening the picker.
use super::wide;
use std::{cell::Cell, ptr::{null, null_mut}};
use windows_sys::{
    Win32::{
        Foundation::{ERROR_SUCCESS, HWND},
        Globalization::{GetLocaleInfoW, LOCALE_SABBREVLANGNAME, LOCALE_SLOCALIZEDDISPLAYNAME},
        Graphics::Gdi::InvalidateRect,
        System::Registry::*,
        UI::{
            Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardLayoutList, HKL},
            WindowsAndMessaging::*,
        },
    },
    w,
};

pub(super) const RETRY_TIMER: usize = 7;
const LAYOUTS_KEY: *const u16 = w!("SYSTEM\\CurrentControlSet\\Control\\Keyboard Layouts");

#[derive(Clone, Copy)]
pub(super) struct Target {
    window: HWND,
    thread: u32,
}

impl Target {
    pub(super) unsafe fn new(window: HWND) -> Option<Self> {
        let mut owner = 0;
        let thread = GetWindowThreadProcessId(window, &mut owner);
        (thread != 0 && owner != std::process::id()).then_some(Self { window, thread })
    }

    unsafe fn focus(self) -> (HWND, u32) {
        let mut info = GUITHREADINFO { cbSize: size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        if GetGUIThreadInfo(self.thread, &mut info) != 0
            && GetAncestor(info.hwndFocus, GA_ROOT) == self.window
        {
            let thread = GetWindowThreadProcessId(info.hwndFocus, null_mut());
            if thread != 0 {
                return (info.hwndFocus, thread);
            }
        }
        (self.window, self.thread)
    }

    pub(super) unsafe fn current(self) -> HKL {
        GetKeyboardLayout(self.focus().1)
    }

    unsafe fn exists(self) -> bool {
        GetWindowThreadProcessId(self.window, null_mut()) == self.thread
    }
}

#[derive(Clone, Copy)]
struct Pending {
    target: Target,
    receiver: u32,
    next: HKL,
}

thread_local! {
    static PENDING: Cell<Option<Pending>> = const { Cell::new(None) };
}

pub(super) unsafe fn loaded() -> Vec<HKL> {
    let count = GetKeyboardLayoutList(0, null_mut()).max(0);
    let mut list = vec![null_mut(); count as usize];
    if count != 0 {
        let actual = GetKeyboardLayoutList(count, list.as_mut_ptr()).max(0);
        list.truncate(actual as usize);
    }
    list
}

pub(super) fn locale_id(k: HKL) -> u32 {
    k as usize as u32 & 0xffff
}

pub(super) unsafe fn locale_name(k: HKL) -> String {
    locale_text(k, LOCALE_SLOCALIZEDDISPLAYNAME)
}

pub(super) unsafe fn short_name(k: HKL) -> String {
    let name = locale_text(k, LOCALE_SABBREVLANGNAME).to_uppercase();
    if name == "ENU" { "ENG".into() } else { name }
}

unsafe fn locale_text(k: HKL, kind: u32) -> String {
    let mut buf = [0u16; 96];
    let length = GetLocaleInfoW(locale_id(k), kind, buf.as_mut_ptr(), buf.len() as i32).max(1) - 1;
    String::from_utf16_lossy(&buf[..length as usize])
}

/// Resolve Windows' keyboard variant name without changing the active layout to query it.
pub(super) unsafe fn variant_name(k: HKL) -> Option<String> {
    let mut root = null_mut();
    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, LAYOUTS_KEY, 0, KEY_READ, &mut root) != ERROR_SUCCESS {
        return None;
    }
    let device = k as usize as u32 >> 16;
    let key = if device & 0xf000 == 0xf000 {
        // WinForms uses Layout Id for extended handles. Unknown handles can still use the locale label.
        let id = device & 0x0fff;
        (0..).map_while(|i| {
            let mut key = [0u16; 32];
            let mut length = key.len() as u32;
            let result = RegEnumKeyExW(root, i, key.as_mut_ptr(), &mut length, null(), null_mut(), null_mut(), null_mut());
            (result == ERROR_SUCCESS).then(|| String::from_utf16_lossy(&key[..length as usize]))
        }).find(|key| {
            read_text(root, &wide(key), w!("Layout Id"))
                .and_then(|value| u32::from_str_radix(value.trim(), 16).ok()) == Some(id)
        })
    } else {
        Some(format!("{:08X}", if device == 0 { locale_id(k) } else { device }))
    };
    let name = key.and_then(|key| {
        let path = wide(&key);
        let mut layout = null_mut();
        let localized = if RegOpenKeyExW(root, path.as_ptr(), 0, KEY_QUERY_VALUE, &mut layout) == ERROR_SUCCESS {
            let mut text = [0u16; 160];
            let result = RegLoadMUIStringW(layout, w!("Layout Display Name"), text.as_mut_ptr(), size_of_val(&text) as u32, null_mut(), 0, null());
            RegCloseKey(layout);
            (result == ERROR_SUCCESS).then(|| super::wstr_of(&text))
        } else {
            None
        };
        localized.or_else(|| read_text(root, &path, w!("Layout Text")))
    });
    RegCloseKey(root);
    name.filter(|value| !value.is_empty())
}

unsafe fn read_text(key: HKEY, subkey: &[u16], name: *const u16) -> Option<String> {
    let mut buf = [0u16; 160];
    let mut bytes = size_of_val(&buf) as u32;
    let result = RegGetValueW(key, subkey.as_ptr(), name, RRF_RT_REG_SZ, null_mut(), buf.as_mut_ptr().cast(), &mut bytes);
    (result == ERROR_SUCCESS).then(|| super::wstr_of(&buf))
}

pub(super) unsafe fn select(bar: HWND, target: Target, next: HKL) {
    KillTimer(bar, RETRY_TIMER);
    PENDING.set(None);
    if !target.exists() {
        return;
    }
    winrust_common::desktop::activate_window(target.window);
    let (focus, thread) = target.focus();
    if GetKeyboardLayout(thread) == next {
        return;
    }
    if PostMessageW(focus, WM_INPUTLANGCHANGEREQUEST, 0, next as isize) == 0 {
        if focus != target.window {
            PostMessageW(target.window, WM_INPUTLANGCHANGEREQUEST, 0, next as isize);
        }
    } else if focus != target.window {
        PENDING.set(Some(Pending { target, receiver: thread, next }));
        SetTimer(bar, RETRY_TIMER, 180, None);
    }
    InvalidateRect(bar, null(), 0);
}

pub(super) unsafe fn retry(bar: HWND) {
    KillTimer(bar, RETRY_TIMER);
    if let Some(request) = PENDING.take() {
        if request.target.exists() && GetForegroundWindow() == request.target.window
            && GetKeyboardLayout(request.receiver) != request.next
        {
            PostMessageW(request.target.window, WM_INPUTLANGCHANGEREQUEST, 0, request.next as isize);
        }
    }
    InvalidateRect(bar, null(), 0);
}
