//! Modern's window list with a single live preview of the highlighted window.
use super::{app_icon, fill, icon_of, mix, pal, sc, text, ui_face, wide};
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::{HWND, RECT, SIZE},
    Graphics::{
        Dwm::{
            DWM_THUMBNAIL_PROPERTIES, DWM_TNP_RECTDESTINATION, DWM_TNP_VISIBLE,
            DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
        },
        Gdi::*,
    },
    UI::WindowsAndMessaging::*,
};

const PAD: i32 = 14;
const ROW: i32 = 52;

thread_local! {
    static THUMBNAIL: Cell<isize> = const { Cell::new(0) };
}

pub(super) unsafe fn panel(origin: HWND, count: usize) -> RECT {
    let mut monitor = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    let screen = MonitorFromWindow(origin, MONITOR_DEFAULTTONEAREST);
    let work = if GetMonitorInfoW(screen, &mut monitor) != 0 {
        monitor.rcWork
    } else {
        RECT { left: 0, top: 0, right: GetSystemMetrics(SM_CXSCREEN), bottom: GetSystemMetrics(SM_CYSCREEN) }
    };
    let w = sc(900).min((work.right - work.left - sc(32)).max(1));
    let desired_height = sc(PAD * 2 + ROW * count.clamp(5, 7) as i32);
    let h = desired_height.min((work.bottom - work.top - sc(32)).max(1));
    let x = (work.left + work.right - w) / 2;
    let y = (work.top + work.bottom - h) / 2;
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

struct Layout {
    list: RECT,
    preview: RECT,
    row_height: i32,
}

impl Layout {
    fn new(bounds: RECT) -> Self {
        let pad = sc(PAD);
        let split = (bounds.right / 3).min(sc(272));
        Self {
            list: RECT { left: pad, top: pad, right: split, bottom: bounds.bottom - pad },
            preview: RECT { left: split + pad * 2, top: pad + sc(40), right: bounds.right - pad, bottom: bounds.bottom - pad },
            row_height: sc(ROW).max(1),
        }
    }

    fn visible_rows(&self, count: usize, selected: usize) -> std::ops::Range<usize> {
        let visible = ((self.list.bottom - self.list.top) / self.row_height).max(1) as usize;
        let first = selected.saturating_sub(visible / 2).min(count.saturating_sub(visible));
        first..(first + visible).min(count)
    }
}

pub(super) unsafe fn clear_preview() {
    let thumb = THUMBNAIL.replace(0);
    if thumb != 0 {
        DwmUnregisterThumbnail(thumb);
    }
}

pub(super) unsafe fn preview(hwnd: HWND, source: HWND) {
    clear_preview();
    let mut thumb = 0;
    if DwmRegisterThumbnail(hwnd, source, &mut thumb) < 0 {
        return;
    }
    let mut bounds = RECT::default();
    let mut source_size = SIZE::default();
    GetClientRect(hwnd, &mut bounds);
    let target = Layout::new(bounds).preview;
    let (w, h) = (target.right - target.left - sc(8), target.bottom - target.top - sc(8));
    if DwmQueryThumbnailSourceSize(thumb, &mut source_size) >= 0 && source_size.cx > 0 && source_size.cy > 0 && w > 0 && h > 0 {
        let factor = (w as f32 / source_size.cx as f32).min(h as f32 / source_size.cy as f32);
        let (tw, th) = ((source_size.cx as f32 * factor) as i32, (source_size.cy as f32 * factor) as i32);
        let (x, y) = ((target.left + target.right - tw) / 2, (target.top + target.bottom - th) / 2);
        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE,
            rcDestination: RECT { left: x, top: y, right: x + tw, bottom: y + th },
            fVisible: 1,
            ..Default::default()
        };
        if DwmUpdateThumbnailProperties(thumb, &props) >= 0 {
            THUMBNAIL.set(thumb);
            return;
        }
    }
    DwmUnregisterThumbnail(thumb);
}

unsafe fn label(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    String::from_utf16_lossy(&buf[..len])
}

pub(super) unsafe fn draw(dc: HDC, bounds: RECT, windows: &[HWND], selected: usize) {
    let Some(&window) = windows.get(selected) else { return };
    let l = Layout::new(bounds);
    let p = pal();
    fill(dc, l.preview, mix(p.bg, p.text, 0.08), sc(10));
    let divider = l.list.right + sc(13);
    fill(dc, RECT { left: divider, right: divider + 1, ..l.list }, p.line, 0);
    let make_font = |size, weight| CreateFontW(-sc(size), 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, wide(ui_face()).as_ptr());
    let row_font = make_font(14, 400);
    let title_font = make_font(18, 600);
    let old = SelectObject(dc, row_font);
    SetBkMode(dc, TRANSPARENT as i32);
    let rows = l.visible_rows(windows.len(), selected);
    let first = rows.start;
    let visible = rows.len();
    for (slot, index) in rows.enumerate() {
        let top = l.list.top + slot as i32 * l.row_height;
        let row = RECT { top, bottom: top + l.row_height, ..l.list };
        if index == selected {
            fill(dc, RECT { top: top + sc(2), bottom: row.bottom - sc(2), ..row }, p.active, sc(10));
            fill(dc, RECT { left: row.left + sc(4), right: row.left + sc(7), top: top + sc(13), bottom: row.bottom - sc(13) }, p.accent, sc(3));
        }
        let w = windows[index];
        app_icon(dc, icon_of(w), row.left + sc(26), (row.top + row.bottom) / 2, sc(20));
        text(dc, &label(w), RECT { left: row.left + sc(49), right: row.right - sc(12), ..row }, if index == selected { p.text } else { p.dim }, DT_LEFT);
    }
    if visible < windows.len() {
        let height = l.list.bottom - l.list.top;
        let start = l.list.top + (height as usize * first / windows.len()) as i32;
        let end = start + (height as usize * visible / windows.len()) as i32;
        fill(dc, RECT { left: l.list.right + sc(4), top: start, right: l.list.right + sc(6), bottom: end }, p.dim, sc(2));
    }
    SelectObject(dc, title_font);
    let heading = RECT { bottom: l.preview.top - sc(6), top: l.list.top, ..l.preview };
    text(dc, &label(window), heading, p.text, DT_LEFT);
    if THUMBNAIL.get() == 0 {
        app_icon(dc, icon_of(window), (l.preview.left + l.preview.right) / 2, (l.preview.top + l.preview.bottom) / 2, sc(40));
    }
    SelectObject(dc, old);
    DeleteObject(row_font);
    DeleteObject(title_font);
}

pub(super) fn row_at(bounds: RECT, x: i32, y: i32, count: usize, selected: usize) -> Option<usize> {
    let l = Layout::new(bounds);
    if x < l.list.left || x >= l.list.right || y < l.list.top || y >= l.list.bottom {
        return None;
    }
    let rows = l.visible_rows(count, selected);
    let index = rows.start + ((y - l.list.top) / l.row_height) as usize;
    rows.contains(&index).then_some(index)
}
