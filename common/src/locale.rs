use std::{
    io,
    path::PathBuf,
    sync::atomic::{AtomicU8, Ordering::Relaxed},
};

use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, MF_CHECKED, MF_STRING,
        TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu,
    },
};

use crate::pt_br;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Language {
    English = 1,
    Russian = 2,
    #[default]
    Portuguese = 3,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::English, Self::Russian, Self::Portuguese];

    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Russian => "ru",
            Self::Portuguese => "pt-BR",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Russian => "Русский",
            Self::Portuguese => "Português (Brasil)",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Portuguese => "PT-BR",
            language => language.label(),
        }
    }

    fn parse(value: &str) -> Self {
        let value = value.trim().trim_start_matches('\u{feff}').trim();
        if value.eq_ignore_ascii_case("en") {
            Self::English
        } else if value.eq_ignore_ascii_case("ru") {
            Self::Russian
        } else {
            Self::default()
        }
    }
}

static LANGUAGE: AtomicU8 = AtomicU8::new(0);

fn preference_file() -> Option<PathBuf> {
    std::env::var_os("ProgramData").map(|dir| PathBuf::from(dir).join("WinRust").join("lang.txt"))
}

pub fn language() -> Language {
    let saved = LANGUAGE.load(Relaxed);
    if saved == 0 {
        let language = preference_file()
            .and_then(|file| std::fs::read_to_string(file).ok())
            .map_or_else(Language::default, |text| Language::parse(&text));
        let _ = LANGUAGE.compare_exchange(0, language as u8, Relaxed, Relaxed);
    }
    match LANGUAGE.load(Relaxed) {
        1 => Language::English,
        2 => Language::Russian,
        _ => Language::default(),
    }
}

pub fn set_language(language: Language) -> io::Result<()> {
    let file = preference_file().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "ProgramData"))?;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(file, language.code())?;
    LANGUAGE.store(language as u8, Relaxed);
    Ok(())
}

/// Localizes an English UI label that had no Russian translation.
pub fn text(english: &str) -> &str {
    if language() == Language::Portuguese { portuguese(english) } else { english }
}

pub fn portuguese(english: &str) -> &str {
    pt_br::lookup(english)
}

/// Displays the WinRust language picker at the cursor. The Windows keyboard layout is independent.
///
/// # Safety
/// The caller supplies a valid window on its UI thread.
pub unsafe fn show_menu(owner: HWND) -> io::Result<()> {
    unsafe {
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor) == 0 {
            return Err(io::Error::last_os_error());
        }
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return Err(io::Error::last_os_error());
        }
        let active = language();
        for (index, item) in Language::ALL.iter().enumerate() {
            let label: Vec<u16> = item.label().encode_utf16().chain([0]).collect();
            let flags = MF_STRING | if *item == active { MF_CHECKED } else { 0 };
            if AppendMenuW(menu, flags, index + 1, label.as_ptr()) == 0 {
                let error = io::Error::last_os_error();
                DestroyMenu(menu);
                return Err(error);
            }
        }
        let choice = TrackPopupMenu(
            menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, cursor.x, cursor.y, 0, owner,
            std::ptr::null(),
        );
        DestroyMenu(menu);
        if let Some(item) = (choice as usize).checked_sub(1).and_then(|i| Language::ALL.get(i)) {
            set_language(*item)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn parses_existing_preferences_and_portuguese() {
        for language in Language::ALL {
            assert_eq!(Language::parse(language.code()), language);
        }
        assert_eq!(Language::parse("pt-br\r\n"), Language::Portuguese);
        assert_eq!(Language::parse(" PT "), Language::Portuguese);
        assert_eq!(Language::parse("\u{feff}EN"), Language::English);
        assert_eq!(Language::parse("unknown"), Language::Portuguese);
        assert_eq!(Language::parse(""), Language::Portuguese);
    }
}
