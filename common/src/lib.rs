//! Code shared by the WinRust programs: localization, the NTFS MFT reader and Win+D.
//! Use it with `#[macro_use] extern crate winrust_common;` at the crate root.
//!
//! The installer and topbar store the selected language in %ProgramData%\WinRust\lang.txt.
use std::path::PathBuf;

pub mod desktop;
pub mod journal;
pub mod locale;
pub mod mem;
pub mod mft;
pub mod proc;
mod pt_br;
pub mod reg;
pub mod settings;
pub mod style;
pub mod windows_mode;

/// "Show in folder": opens `path`'s folder with the file selected in the installed rust-explorer, else in Windows Explorer.
pub fn reveal(path: &str) {
    use std::{os::windows::process::CommandExt, process::Command};
    let rx = std::env::var_os("ProgramFiles").map(|pf| PathBuf::from(pf).join(r"rust-explorer\rust-explorer.exe")).filter(|p| p.is_file());
    let _ = match rx {
        Some(rx) => Command::new(rx).raw_arg(format!("/select,\"{path}\"")).spawn(),
        None => Command::new("explorer.exe").raw_arg(format!("/select,\"{path}\"")).spawn(),
    };
}

/// Existing bilingual unit tests keep their deterministic Russian expectations.
#[macro_export]
macro_rules! lang {
    () => {
        if cfg!(test) { $crate::locale::Language::Russian } else { $crate::locale::language() }
    };
}

/// A label in the current language. Portuguese labels are in the shared catalog.
#[macro_export]
macro_rules! t {
    ($ru:expr, $en:expr) => {
        match $crate::lang!() {
            $crate::locale::Language::English => $en,
            $crate::locale::Language::Russian => $ru,
            $crate::locale::Language::Portuguese => $crate::locale::portuguese($en),
        }
    };
}

/// Translate a label shared by the existing English and Russian interfaces.
#[macro_export]
macro_rules! pt {
    ($en:expr) => {
        if cfg!(test) { $en } else { $crate::locale::text($en) }
    };
}

/// Format all three translations with the same arguments, checked by the compiler.
#[macro_export]
macro_rules! tf {
    ($ru:literal, $en:literal, $pt:literal $(, $a:expr)* $(,)?) => {
        match $crate::lang!() {
            $crate::locale::Language::English => format!($en $(, $a)*),
            $crate::locale::Language::Russian => format!($ru $(, $a)*),
            $crate::locale::Language::Portuguese => format!($pt $(, $a)*),
        }
    };
}

/// Equivalent to format! for messages that previously had only an English template.
#[macro_export]
macro_rules! pf {
    ($en:literal, $pt:literal $(, $a:expr)* $(,)?) => {
        if !cfg!(test) && $crate::locale::language() == $crate::locale::Language::Portuguese {
            format!($pt $(, $a)*)
        } else {
            format!($en $(, $a)*)
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn tests_run_in_russian() {
        assert_eq!(t!("да", "yes"), "да");
        assert_eq!(tf!("{} шт", "{} pcs", "{} un.", 3), "3 шт");
        assert_eq!(crate::locale::portuguese("Cancel"), "Cancelar");
    }
}
