//! Per-monitor bar placement and UI state. App buttons, tray icons and hotkeys are shared.
use super::*;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub(super) struct Display {
    monitor: HMONITOR,
    pub bounds: RECT,
    pub work: RECT,
    primary: bool,
}

fn display(monitor: HMONITOR) -> Option<Display> {
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..unsafe { zeroed() } };
    (unsafe { GetMonitorInfoW(monitor, &mut info) } != 0).then_some(Display {
        monitor, bounds: info.rcMonitor, work: info.rcWork, primary: info.dwFlags & 1 != 0,
    })
}

fn displays() -> Vec<Display> {
    unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
        if let Some(screen) = display(monitor) {
            (*(data as *mut Vec<Display>)).push(screen);
        }
        1
    }
    let mut found = Vec::new();
    unsafe { EnumDisplayMonitors(null_mut(), null(), Some(collect), &mut found as *mut _ as LPARAM) };
    found
}

fn primary() -> HWND { BAR.load(Relaxed) as HWND }

pub(super) struct State {
    monitor: Cell<HMONITOR>,
    pub progress: Cell<f32>,
    pub last_step: Cell<u64>,
    pub last_interaction: Cell<u64>,
    pub fullscreen: Cell<bool>,
    pub idle: Cell<bool>,
    pub hot: Cell<i32>,
    reservation: Cell<Option<(RECT, RECT)>>,
}

impl State {
    fn new(monitor: HMONITOR) -> Self {
        Self {
            monitor: Cell::new(monitor),
            progress: Cell::new(0.0), last_step: Cell::new(0),
            last_interaction: Cell::new(unsafe { now_ms() }),
            fullscreen: Cell::new(false), idle: Cell::new(false), hot: Cell::new(NONE),
            reservation: Cell::new(None),
        }
    }
}

thread_local! {
    static WINDOWS: RefCell<HashMap<HWND, Rc<State>>> = RefCell::new(HashMap::new());
    static FALLBACK: Rc<State> = Rc::new(State::new(null_mut()));
    static CURRENT: Cell<HWND> = const { Cell::new(null_mut()) };
}

pub(super) struct Context(HWND);
impl Drop for Context {
    fn drop(&mut self) { CURRENT.set(self.0); }
}

pub(super) fn enter(hwnd: HWND) -> Context { Context(CURRENT.replace(hwnd)) }

pub(super) fn active() -> HWND {
    let current = CURRENT.get();
    if current.is_null() { primary() } else { current }
}

pub(super) fn state() -> Rc<State> {
    WINDOWS.with_borrow(|windows| windows.get(&active()).cloned()).unwrap_or_else(|| FALLBACK.with(Rc::clone))
}

pub(super) fn current_display() -> Option<Display> {
    display(state().monitor.get()).or_else(|| unsafe { display(MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY)) })
}

pub(super) fn screen() -> RECT {
    current_display().map_or_else(|| unsafe { RECT { left: 0, top: 0, right: GetSystemMetrics(SM_CXSCREEN), bottom: GetSystemMetrics(SM_CYSCREEN) } }, |display| display.bounds)
}

pub(super) fn main_bar(hwnd: HWND) -> bool { hwnd == primary() }

pub(super) fn handles() -> Vec<HWND> {
    WINDOWS.with_borrow(|windows| windows.keys().copied().collect())
}

pub(super) unsafe fn repaint() {
    for hwnd in handles() { InvalidateRect(hwnd, null(), 0); }
}

pub(super) unsafe fn place_all() {
    let main = primary();
    for hwnd in std::iter::once(main).chain(handles().into_iter().filter(|&hwnd| hwnd != main)) {
        if hwnd.is_null() { continue; }
        let _context = enter(hwnd);
        place(hwnd);
        fs_update(hwnd);
    }
}

pub(super) fn same_rect(a: RECT, b: RECT) -> bool {
    (a.left, a.top, a.right, a.bottom) == (b.left, b.top, b.right, b.bottom)
}

unsafe fn set_area(monitor: HMONITOR, mut want: RECT) {
    if display(monitor).is_some_and(|current| !same_rect(current.work, want)) {
        SystemParametersInfoW(SPI_SETWORKAREA, 0, &mut want as *mut _ as *mut _, SPIF_SENDCHANGE);
    }
}

pub(super) unsafe fn set_primary_area(want: RECT) {
    if let Some(screen) = displays().into_iter().find(|screen| screen.primary) { set_area(screen.monitor, want); }
}

pub(super) unsafe fn release_work_areas() {
    for screen in displays() { set_area(screen.monitor, screen.bounds); }
}

/// A hidden secondary Windows taskbar can still reserve its strip. Reuse it
/// and preserve reservations on the other edges.
unsafe fn without_native_strip(screen: Display, mut area: RECT) -> RECT {
    for native in tray().into_iter().skip(1) {
        if IsWindowVisible(native) != 0 || MonitorFromWindow(native, MONITOR_DEFAULTTONULL) != screen.monitor { continue; }
        let mut rect = zeroed();
        if GetWindowRect(native, &mut rect) == 0 { continue; }
        let m = screen.bounds;
        if rect.left <= m.left && rect.right >= m.right {
            if rect.bottom == m.bottom && rect.top == area.bottom { area.bottom = m.bottom; }
            if rect.top == m.top && rect.bottom == area.top { area.top = m.top; }
        } else if rect.top <= m.top && rect.bottom >= m.bottom {
            if rect.left == m.left && rect.right == area.left { area.left = m.left; }
            if rect.right == m.right && rect.left == area.right { area.right = m.right; }
        }
    }
    area
}

pub(super) unsafe fn reserve_secondary(edge: u32, strip: i32) {
    let state = state();
    let Some(screen) = display(state.monitor.get()) else { return };
    let original = state.reservation.get().filter(|&(_, reserved)| same_rect(reserved, screen.work))
        .map_or(screen.work, |(original, _)| original);
    let mut area = without_native_strip(screen, original);
    let m = screen.bounds;
    match edge {
        ABE_LEFT => area.left = area.left.max(m.left + strip),
        ABE_RIGHT => area.right = area.right.min(m.right - strip),
        ABE_TOP => area.top = area.top.max(m.top + strip),
        _ => area.bottom = area.bottom.min(m.bottom - strip),
    }
    state.reservation.set((!same_rect(original, area)).then_some((original, area)));
    set_area(screen.monitor, area);
}

pub(super) unsafe fn remove(hwnd: HWND) {
    let state = WINDOWS.with_borrow_mut(|windows| windows.remove(&hwnd));
    let Some(state) = state else { return };
    let Some((original, reserved)) = state.reservation.get() else { return };
    let monitor = state.monitor.get();
    if display(monitor).is_some_and(|screen| same_rect(screen.work, reserved)) { set_area(monitor, original); }
}

pub(super) unsafe fn destroy_secondary() {
    for hwnd in handles().into_iter().filter(|&hwnd| !main_bar(hwnd)) { DestroyWindow(hwnd); }
}

pub(super) unsafe fn sync() {
    let monitors = displays();
    let main = primary();
    let Some(screen) = monitors.iter().find(|screen| screen.primary) else { return };
    WINDOWS.with_borrow_mut(|windows| windows.entry(main).or_insert_with(|| Rc::new(State::new(screen.monitor))).monitor.set(screen.monitor));

    let stale: Vec<HWND> = WINDOWS.with_borrow(|windows| windows.iter().filter_map(|(&hwnd, state)| {
        (hwnd != main && !monitors.iter().any(|screen| !screen.primary && screen.monitor == state.monitor.get())).then_some(hwnd)
    }).collect());
    for hwnd in stale { DestroyWindow(hwnd); }

    let class = wide("topbar_secondary");
    let instance = GetModuleHandleW(null());
    RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: instance, hCursor: LoadCursorW(null_mut(), IDC_ARROW), lpszClassName: class.as_ptr(), ..zeroed() });
    for screen in monitors.into_iter().filter(|screen| !screen.primary) {
        if WINDOWS.with_borrow(|windows| windows.values().any(|state| state.monitor.get() == screen.monitor)) { continue; }
        let bounds = screen.bounds;
        let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE, class.as_ptr(), null(),
            WS_POPUP, bounds.left, bounds.bottom - bar_h(), bounds.right - bounds.left, bar_h(), null_mut(), null_mut(), instance, null());
        if hwnd.is_null() { continue; }
        WINDOWS.with_borrow_mut(|windows| windows.insert(hwnd, Rc::new(State::new(screen.monitor))));
        DragAcceptFiles(hwnd, 1);
        for message in [WM_DROPFILES, 0x0049 /* WM_COPYGLOBALDATA */, WM_COPYDATA] {
            ChangeWindowMessageFilterEx(hwnd, message, MSGFLT_ALLOW, null_mut());
        }
        bar_anim(hwnd);
        SetTimer(hwnd, 3, 250, None);
    }
    place_all();
}
