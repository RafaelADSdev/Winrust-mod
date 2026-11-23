//! Safety net for topbar: if the bar process is gone while the real taskbar is still hidden (crash, kill, bad boot),
//! the user would have no taskbar at all. reshell is already running from the Run key, so it watches: first
//! `topbar.exe --restore` (the taskbar is back at once), then it starts the bar again. A bar that keeps dying is given up
//! after `MAX_RESTARTS` tries in a row, so a bar that crashes on start cannot loop forever; the taskbar then stays.
use std::{os::windows::process::CommandExt, process::Command, thread, time::Duration};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetPropW, IsWindowVisible}};
use winrust_common::desktop::MASKED_TASKBAR;

const MAX_RESTARTS: u32 = 3;
const STABLE_CHECKS: u32 = 12; // ~60 s of a running bar: it was a one-off crash, the tries start over

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn any_masked_secondary() -> bool {
    let mut tray: HWND = std::ptr::null_mut();
    unsafe {
        loop {
            tray = FindWindowExW(std::ptr::null_mut(), tray, windows_sys::w!("Shell_SecondaryTrayWnd"), std::ptr::null());
            if tray.is_null() { return false; }
            if !GetPropW(tray, MASKED_TASKBAR).is_null() { return true; }
        }
    }
}

/// Broken = no WinRust bar, and the native taskbar is still suppressed on at least one monitor.
fn broken(bar_up: bool, tray_present: bool, tray_visible: bool, tray_masked: bool) -> bool {
    !bar_up && (tray_masked || (tray_present && !tray_visible))
}

#[derive(Debug, PartialEq, Eq)]
enum Act {
    Nothing,
    /// Bring the taskbar back, then start the bar again.
    Recover,
}

#[derive(Default)]
struct Guard {
    strikes: u32,
    restarts: u32,
    stable: u32,
}

impl Guard {
    /// One look (every 5 s): what to do now.
    fn step(&mut self, bar_up: bool, broke: bool) -> Act {
        self.stable = if bar_up { self.stable + 1 } else { 0 };
        if self.stable >= STABLE_CHECKS {
            self.restarts = 0;
        }
        self.strikes = if broke { self.strikes + 1 } else { 0 };
        // two checks in a row (~10 s): a bar that is only just starting must not be "restored" under its feet
        if self.strikes == 2 && self.restarts < MAX_RESTARTS {
            self.restarts += 1;
            self.strikes = 0;
            return Act::Recover;
        }
        Act::Nothing
    }
}

pub fn run() {
    let mut g = Guard::default();
    loop {
        thread::sleep(Duration::from_secs(5));
        let (bar, tray) = unsafe { (FindWindowW(wide("topbar").as_ptr(), std::ptr::null()), FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null())) };
        let tray_masked = bar.is_null() && (!unsafe { GetPropW(tray, MASKED_TASKBAR) }.is_null() || any_masked_secondary());
        let broke = broken(!bar.is_null(), !tray.is_null(), !tray.is_null() && unsafe { IsWindowVisible(tray) } != 0, tray_masked);
        if g.step(!bar.is_null(), broke) == Act::Recover {
            if let Ok(exe) = std::env::current_exe() {
                let topbar = exe.with_file_name("topbar.exe");
                let _ = Command::new(&topbar).arg("--restore").creation_flags(0x0800_0000).status();
                if !winrust_common::windows_mode::enabled() {
                    let _ = Command::new(&topbar).creation_flags(0x0800_0000).spawn(); // detached: the bar outlives this call
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_bar_gone_and_taskbar_hidden_is_broken() {
        assert!(broken(false, true, false, false));
        assert!(!broken(true, true, false, false)); // bar running: taskbar is meant to be hidden
        assert!(!broken(false, true, true, false)); // taskbar already back
        assert!(!broken(false, false, false, false)); // explorer restarting
    }

    #[test]
    fn recovers_a_masked_secondary_even_if_the_primary_is_visible() {
        assert!(broken(false, true, true, true));
        assert!(!broken(true, true, true, true));
    }

    #[test]
    fn recovers_after_two_broken_checks() {
        let mut g = Guard::default();
        assert_eq!(g.step(false, true), Act::Nothing);
        assert_eq!(g.step(false, true), Act::Recover);
    }

    #[test]
    fn a_bar_that_keeps_crashing_is_given_up() {
        let mut g = Guard::default();
        let mut recovered = 0;
        for _ in 0..40 {
            // the bar starts, dies at once: broken again
            if g.step(false, true) == Act::Recover {
                recovered += 1;
            }
        }
        assert_eq!(recovered, MAX_RESTARTS);
    }

    #[test]
    fn a_stable_bar_gives_the_tries_back() {
        let mut g = Guard::default();
        for _ in 0..4 {
            g.step(false, true);
        }
        assert_eq!(g.restarts, 2);
        for _ in 0..STABLE_CHECKS {
            g.step(true, false);
        }
        assert_eq!(g.restarts, 0);
    }
}
