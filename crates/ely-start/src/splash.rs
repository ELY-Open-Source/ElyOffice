//! The splash window: the start scene, drawn with GDI.

use std::{
    io,
    path::Path,
    process::{Child, Command, ExitStatus},
    thread::JoinHandle,
    time::Instant,
};

use ely_start::scene;

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateFontW, CreateSolidBrush, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW,
        EndPaint, FillRect, InvalidateRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, UpdateWindow,
    },
    System::{
        LibraryLoader::GetModuleHandleW,
        Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
    },
    UI::{HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, SetProcessDpiAwarenessContext}, WindowsAndMessaging::*},
};

/// The spawn returned.
const SPAWNED: u32 = WM_APP + 1;
/// Sizes in 96-dpi pixels.
const WIDTH: i32 = 360;
const HEIGHT: i32 = 240;
/// One frame, ms.
const FRAME: u32 = 16;
/// Hidden, the app is checked this often, ms.
const POLL: u32 = 100;

struct Shown {
    name: Vec<u16>,
    icon: HICON,
    scale: i32,
    dark: bool,
    still: bool,
    entered: bool,
    began: Instant,
    leaving: Option<Instant>,
    spawner: Option<JoinHandle<io::Result<Child>>>,
    child: Option<Child>,
    failed: Option<io::Error>,
    /// How the app ended before its first frame.
    ended: Option<ExitStatus>,
    /// Past `scene::LONGEST` with no first frame yet.
    outlasted: bool,
    /// Gone from the screen; still waiting.
    hidden: bool,
}

impl Shown {
    fn px(&self, at96: f32) -> i32 {
        (at96 * self.scale as f32 / 96.0).round() as i32
    }

    fn ms(&self) -> u64 {
        self.began.elapsed().as_millis() as u64
    }

    /// Top of the icon box, the name line, the bar.
    fn rows(&self, height: i32) -> (i32, i32, i32) {
        let tall = scene::ICON + scene::GAP + scene::NAME * 1.4 + scene::GAP + scene::THICK;
        let top = (height - self.px(tall)) / 2;
        let name = top + self.px(scene::ICON + scene::GAP);
        (top, name, name + self.px(scene::NAME * 1.4 + scene::GAP))
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// Splash until the app shows or exits; returns its exit.
pub fn run(name: &str, icon: &Path, mut command: Command, began: Instant) -> io::Result<Option<ExitStatus>> {
    // SAFETY: plain Win32 calls on this thread's own window.
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let scale = GetDpiForSystem() as i32;
        let side = (scene::ICON * scale as f32 / 96.0).round() as i32;
        let path = wide(&icon.to_string_lossy());
        let handle = LoadImageW(std::ptr::null_mut(), path.as_ptr(), IMAGE_ICON, side, side, LR_LOADFROMFILE);
        if handle.is_null() {
            return Err(io::Error::other(format!("no icon at {}: {}", icon.display(), io::Error::last_os_error())));
        }
        let still = still();
        let mut shown = Shown { name: wide(name), icon: handle, scale, dark: dark(), still, entered: still, began, leaving: None, spawner: None, child: None, failed: None, ended: None, outlasted: false, hidden: false };
        let module = GetModuleHandleW(std::ptr::null());
        let class = wide("ElySplash");
        let kind = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(proc_),
            hInstance: module,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_APPSTARTING),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassExW(&kind) == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut area: RECT = std::mem::zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, (&raw mut area).cast(), 0);
        let (width, height) = (WIDTH * scale / 96, HEIGHT * scale / 96);
        let title = wide(name);
        let window = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            area.left + (area.right - area.left - width) / 2,
            area.top + (area.bottom - area.top - height) / 2,
            width,
            height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            module,
            (&raw mut shown).cast(),
        );
        if window.is_null() {
            return Err(io::Error::last_os_error());
        }
        fade(window, scene::enter(0, still).0);
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
        SetTimer(window, 1, FRAME, None);
        // The app may take the foreground from us.
        AllowSetForegroundWindow(ASFW_ANY);
        let target = window as isize;
        // The app closes it at its first frame.
        command.env("ELY_SPLASH", target.to_string());
        shown.spawner = Some(std::thread::spawn(move || {
            let started = command.spawn();
            PostMessageW(target as HWND, SPAWNED, 0, 0);
            started
        }));
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if let Some(error) = shown.failed {
            return Err(error);
        }
        // The splash may close first.
        if let Some(spawner) = shown.spawner.take() {
            joined(spawner)?;
        }
        Ok(shown.ended)
    }
}

fn joined(spawner: JoinHandle<io::Result<Child>>) -> io::Result<Child> {
    spawner.join().map_err(|_| io::Error::other("the start thread panicked"))?
}

/// Windows' "animation effects" off: reduced motion.
fn still() -> bool {
    let mut on = 1i32;
    // SAFETY: `on` holds the BOOL the call writes.
    let read = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut on).cast(), 0) };
    read != 0 && on == 0
}

/// The whole window at `opacity`.
fn fade(window: HWND, opacity: f32) {
    // SAFETY: a layered window of this thread.
    unsafe { SetLayeredWindowAttributes(window, 0, (opacity.clamp(0.0, 1.0) * 255.0) as u8, LWA_ALPHA) };
}

/// Windows' app mode: dark unless apps use light.
fn dark() -> bool {
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value = wide("AppsUseLightTheme");
    let mut light = 1u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `light` holds the DWORD the size names.
    let read = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&raw mut light).cast(), &mut size)
    };
    read == 0 && light == 0
}

/// SAFETY: `run` keeps the state alive while messages run.
unsafe fn state<'a>(window: HWND) -> Option<&'a mut Shown> {
    // SAFETY: the slot holds null or `run`'s `Shown`.
    unsafe { (GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Shown).as_mut() }
}

unsafe extern "system" fn proc_(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: window thread; no borrow spans DestroyWindow.
    unsafe {
        match message {
            WM_NCCREATE => {
                let create = &*(lparam as *const CREATESTRUCTW);
                SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
                DefWindowProcW(window, message, wparam, lparam)
            }
            WM_PAINT => {
                if let Some(shown) = state(window) {
                    paint(window, shown);
                }
                0
            }
            SPAWNED => {
                let failed = match state(window).and_then(|shown| shown.spawner.take().map(|spawner| (shown, joined(spawner)))) {
                    Some((shown, Ok(child))) => {
                        shown.child = Some(child);
                        false
                    }
                    Some((shown, Err(error))) => {
                        shown.failed = Some(error);
                        true
                    }
                    None => false,
                };
                if failed {
                    DestroyWindow(window);
                }
                0
            }
            WM_CLOSE => {
                match state(window) {
                    Some(shown) if !shown.still && !shown.outlasted && shown.leaving.is_none() => shown.leaving = Some(Instant::now()),
                    _ => {
                        DestroyWindow(window);
                    }
                }
                0
            }
            WM_TIMER => {
                let gone = state(window).is_none_or(exited);
                match (gone, state(window)) {
                    (false, Some(shown)) => tick(window, shown),
                    _ => {
                        DestroyWindow(window);
                    }
                }
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

/// The app exited before its first frame.
fn exited(shown: &mut Shown) -> bool {
    let Some(child) = shown.child.as_mut() else {
        return false;
    };
    match child.try_wait() {
        Ok(status) => {
            shown.ended = status;
            status.is_some()
        }
        Err(error) => {
            shown.failed = Some(error);
            true
        }
    }
}

/// Hides the splash; first frame or exit still ends it.
unsafe fn hide(window: HWND, shown: &mut Shown) {
    shown.hidden = true;
    // SAFETY: this thread's window; the timer keeps its id.
    unsafe {
        ShowWindow(window, SW_HIDE);
        SetTimer(window, 1, POLL, None);
    }
}

/// One frame: the fade, the entrance, the bar.
unsafe fn tick(window: HWND, shown: &mut Shown) {
    if shown.hidden {
        return;
    }
    let ms = shown.ms();
    match shown.leaving {
        Some(left) => {
            let opacity = scene::leave(left.elapsed().as_millis() as u64, false);
            fade(window, opacity);
            if opacity <= 0.0 {
                match shown.outlasted {
                    // SAFETY: this thread's window.
                    true => unsafe { hide(window, shown) },
                    // SAFETY: this thread's window; no borrow outlives it.
                    false => unsafe {
                        DestroyWindow(window);
                    },
                }
                return;
            }
        }
        None if ms >= scene::LONGEST => {
            shown.outlasted = true;
            match shown.still {
                // SAFETY: this thread's window.
                true => unsafe { hide(window, shown) },
                false => shown.leaving = Some(Instant::now()),
            }
            return;
        }
        None if shown.still => return,
        None if !shown.entered => {
            // A late tick lands at full.
            shown.entered = ms >= scene::ENTER;
            fade(window, scene::enter(ms, false).0);
        }
        None => {}
    }
    let mut whole: RECT = unsafe { std::mem::zeroed() };
    // SAFETY: this thread's window.
    unsafe { GetClientRect(window, &mut whole) };
    // Past the entrance only the bar moves.
    let only = shown.entered.then(|| track(shown, &whole));
    // SAFETY: this thread's window; null repaints it all.
    unsafe { InvalidateRect(window, only.as_ref().map_or(std::ptr::null(), |rect| rect as *const RECT), 0) };
}

/// The bar's lane.
fn track(shown: &Shown, whole: &RECT) -> RECT {
    let (_, _, top) = shown.rows(whole.bottom);
    let width = shown.px(scene::TRACK);
    let left = (whole.right - width) / 2;
    RECT { left, top, right: left + width, bottom: top + shown.px(scene::THICK).max(1) }
}

/// Background, icon, name and bar.
unsafe fn paint(window: HWND, shown: &Shown) {
    // SAFETY: a paint cycle on this thread's window.
    unsafe {
        let mut sheet: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(window, &mut sheet);
        let (back, ink, lane) = match shown.dark {
            true => (scene::DARK, 0x00f0_f0f0, 0x0040_4040),
            false => (scene::LIGHT, 0x0020_2020, 0x00e0_e0e0),
        };
        let fill = |rect: &RECT, color: u32| {
            let brush = CreateSolidBrush(color);
            FillRect(dc, rect, brush);
            DeleteObject(brush);
        };
        let mut whole: RECT = std::mem::zeroed();
        GetClientRect(window, &mut whole);
        let ms = shown.ms();
        let lane_at = track(shown, &whole);
        // A bar-only repaint leaves the rest.
        if sheet.rcPaint.top < lane_at.top || sheet.rcPaint.bottom > lane_at.bottom {
            fill(&whole, back);
            let (top, name, _) = shown.rows(whole.bottom);
            let (_, scale) = scene::enter(ms, shown.still);
            let (side, size) = (shown.px(scene::ICON), shown.px(scene::ICON * scale));
            let (x, y) = ((whole.right - size) / 2, top + (side - size) / 2);
            DrawIconEx(dc, x, y, shown.icon, size, size, 0, std::ptr::null_mut(), DI_NORMAL);
            let face = wide("Segoe UI Semibold");
            let font = CreateFontW(-shown.px(scene::NAME), 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
            let old = SelectObject(dc, font);
            SetBkMode(dc, TRANSPARENT as i32);
            SetTextColor(dc, ink);
            let mut line = RECT { left: 0, top: name, right: whole.right, bottom: name + shown.px(scene::NAME * 1.4) };
            DrawTextW(dc, shown.name.as_ptr(), shown.name.len() as i32 - 1, &mut line, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
            SelectObject(dc, old);
            DeleteObject(font);
        }
        fill(&lane_at, lane);
        let width = lane_at.right - lane_at.left;
        let left = lane_at.left + (scene::bar(ms, shown.still) * width as f32) as i32;
        let bar = RECT {
            left: left.max(lane_at.left),
            top: lane_at.top,
            right: (left + (scene::BAR * width as f32) as i32).min(lane_at.right),
            bottom: lane_at.bottom,
        };
        if bar.right > bar.left {
            fill(&bar, ink);
        }
        EndPaint(window, &sheet);
    }
}

/// A blocking error box.
pub fn alert(title: &str, text: &str) {
    let (title, text) = (wide(title), wide(text));
    // SAFETY: both strings end in NUL and outlive the call.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_ICONERROR | MB_OK) };
}
