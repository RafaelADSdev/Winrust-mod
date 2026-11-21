//! Replacement for SuperSaver.ps1: 60 Hz, Power Saver-based scheme, launchers closed.
use std::{fs, path::PathBuf, process::Command};
use windows_sys::Win32::Graphics::Gdi::*;

const NAME: &str = "SuperSaver";
const SAVER: &str = "a1841308-3541-4fab-bc81-f71556f20b4a"; // built-in Power Saver
const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
const BRIGHTNESS: &str = "aded5e82-b909-4619-9949-f5d71dac0bcb"; // SUB_VIDEO / VIDEO BRIGHTNESS
const SUB_VIDEO: &str = "7516b95f-f776-4464-8c53-06167f40cc99";
const LAUNCHERS: &[&str] = &["steam", "steamwebhelper", "Discord", "EpicGamesLauncher", "Battle.net", "RiotClientServices", "GalaxyClient", "EADesktop", "Origin"];

fn hz_file() -> PathBuf {
    let dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("reshell");
    let _ = fs::create_dir_all(&dir);
    dir.join("saver_hz.txt")
}

fn devmode() -> DEVMODEA {
    let mut dm: DEVMODEA = unsafe { std::mem::zeroed() };
    dm.dmSize = size_of::<DEVMODEA>() as u16;
    unsafe { EnumDisplaySettingsA(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut dm) };
    dm
}

fn get_hz() -> u32 { devmode().dmDisplayFrequency }

fn set_hz(hz: u32) -> bool {
    let mut dm = devmode();
    dm.dmDisplayFrequency = hz;
    dm.dmFields = DM_DISPLAYFREQUENCY;
    // CDS_TEST first so an unsupported rate never touches the display; flags 0 = this session only
    unsafe { ChangeDisplaySettingsA(&dm, CDS_TEST) == DISP_CHANGE_SUCCESSFUL && ChangeDisplaySettingsA(&dm, 0) == DISP_CHANGE_SUCCESSFUL }
}

fn powercfg(args: &[&str]) -> String {
    Command::new("powercfg").args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// GUID of our scheme from `powercfg /l`, if it exists.
fn find_scheme() -> Option<String> {
    powercfg(&["/l"]).lines().find(|l| l.contains(NAME)).and_then(guid_in)
}

fn guid_in(s: &str) -> Option<String> {
    let i = s.find("GUID:")? + 5;
    s[i..].split_whitespace().next().map(str::to_owned)
}

pub fn run(on: bool) -> i32 {
    let existing = find_scheme();
    if !on {
        powercfg(&["/setactive", BALANCED]);
        if let Some(g) = existing { powercfg(&["/d", &g]); }
        if let Some(hz) = fs::read_to_string(hz_file()).ok().and_then(|s| s.trim().parse().ok()) {
            if set_hz(hz) { let _ = fs::remove_file(hz_file()); }
        }
        return 0;
    }
    let cur = get_hz();
    if cur != 60 {
        if !hz_file().exists() { let _ = fs::write(hz_file(), cur.to_string()); } // remember for 'off'
        if !set_hz(60) { eprintln!("{}", winrust_common::pf!("60 Hz not supported on this display", "60 Hz não suportado neste monitor")); }
    }
    let mut kill = Command::new("taskkill");
    kill.arg("/F");
    for n in LAUNCHERS { kill.args(["/IM", &format!("{n}.exe")]); }
    let _ = kill.output(); // not running -> error exit, ignored on purpose

    let g = match existing.or_else(|| guid_in(&powercfg(&["/duplicatescheme", SAVER]))) {
        Some(g) => g,
        None => { eprintln!("{}", winrust_common::pf!("could not create power scheme", "não foi possível criar esquema de energia")); return 1; }
    };
    powercfg(&["/changename", &g, NAME, winrust_common::pt!("Max battery life")]);
    // DC = on battery: CPU 40%, no turbo, passive cooling, screen off 1 min, sleep 10 min, brightness 20%
    for (sub, set, v) in [
        ("SUB_PROCESSOR", "PROCTHROTTLEMAX", "40"), ("SUB_PROCESSOR", "PERFBOOSTMODE", "0"), ("SUB_PROCESSOR", "SYSCOOLPOL", "0"),
        ("SUB_VIDEO", "VIDEOIDLE", "60"), ("SUB_SLEEP", "STANDBYIDLE", "600"), (SUB_VIDEO, BRIGHTNESS, "20"),
    ] {
        powercfg(&["/setdcvalueindex", &g, sub, set, v]);
    }
    powercfg(&["/setactive", &g]);
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_guid() {
        assert_eq!(super::guid_in("Power Scheme GUID: abc-123  (SuperSaver)").as_deref(), Some("abc-123"));
        assert_eq!(super::guid_in("nothing"), None);
    }
}
