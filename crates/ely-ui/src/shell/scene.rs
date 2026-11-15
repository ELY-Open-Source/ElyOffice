//! The start scene over the window until its document draws.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use ely_gpui_component::theme::ActiveTheme as _;
use ely_platform::{AppId, startup};
use ely_start::scene;
use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, Image, ImageFormat, InteractiveElement as _, IntoElement, ParentElement as _,
    Styled as _, Window, div, img, px, relative,
};

use super::Root;

/// The scene while it shows.
pub(super) struct Scene {
    icon: Arc<Image>,
    leaving: bool,
}

impl Scene {
    /// The scene, once, for the start's window.
    pub(super) fn new(app: AppId) -> Option<Self> {
        WANTED.swap(false, Ordering::Relaxed).then(|| Self {
            icon: Arc::new(Image::from_bytes(ImageFormat::Png, icon(app).to_vec())),
            leaving: false,
        })
    }
}

/// The next root is the start's window.
static WANTED: AtomicBool = AtomicBool::new(false);

/// The start's window will draw the scene.
pub(crate) fn want() {
    WANTED.store(true, Ordering::Relaxed);
}

fn icon(app: AppId) -> &'static [u8] {
    match app {
        AppId::Doc => include_bytes!("../../../../assets/icons/ely-doc.png"),
        AppId::Table => include_bytes!("../../../../assets/icons/ely-table.png"),
        AppId::Show => include_bytes!("../../../../assets/icons/ely-show.png"),
        AppId::Pdf => include_bytes!("../../../../assets/icons/ely-pdf.png"),
        AppId::Mail => include_bytes!("../../../../assets/icons/ely-mail.png"),
        AppId::Tree => include_bytes!("../../../../assets/icons/ely-tree.png"),
        AppId::Mark => include_bytes!("../../../../assets/icons/ely-mark.png"),
        AppId::Agent => include_bytes!("../../../../assets/icons/ely-agent.png"),
    }
}

/// Draws `scene` until the first frame or document.
pub(super) fn draw(scene: &mut Scene, app: AppId, window: &mut Window, cx: &mut Context<Root>) -> AnyElement {
    if !scene.leaving && startup::has_started() && !startup::awaiting() {
        scene.leaving = true;
        reveal(window);
        let gone = Duration::from_millis(if cx.theme().reduced_motion { 0 } else { scene::LEAVE });
        cx.spawn(async move |root, cx| {
            cx.background_executor().timer(gone).await;
            root.update(cx, |root, cx| root.scene_gone(cx)).ok();
        })
        .detach();
    }
    // Unseen frames cost the engine CPU.
    let hidden = CLOAKED.load(Ordering::Relaxed);
    let still = cx.theme().reduced_motion || hidden;
    let since = startup::since_start();
    let entering = since < scene::ENTER && !still;
    let rest = Duration::from_millis(scene::ENTER.saturating_sub(since));
    // Where the entrance is at `t` of what remains.
    let at = move |t: f32| scene::enter(since + (rest.as_millis() as f32 * t) as u64, false);
    let mark = img(scene.icon.clone()).size(px(scene::ICON));
    let mark = match entering {
        true => mark
            .with_animation("scene-icon", Animation::new(rest), move |mark, t| {
                let (opacity, scale) = at(t);
                mark.size(px(scene::ICON * scale)).opacity(opacity)
            })
            .into_any_element(),
        false => mark.into_any_element(),
    };
    let name = div().text_size(px(scene::NAME)).font_weight(gpui::FontWeight::SEMIBOLD).child(app.name());
    let name = match entering {
        true => name.with_animation("scene-name", Animation::new(rest), move |name, t| name.opacity(at(t).0)).into_any_element(),
        false => name.into_any_element(),
    };
    let arrival = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(scene::GAP))
        .child(div().size(px(scene::ICON)).flex().items_center().justify_center().child(mark))
        .child(name);
    let whole = div()
        .id("start-scene")
        .occlude()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(scene::GAP))
        .bg(cx.theme().colors.bg)
        .child(arrival)
        .child(bar(still, cx));
    match scene.leaving && !still {
        true => whole
            .with_animation("scene-leave", Animation::new(Duration::from_millis(scene::LEAVE)), |whole, t| {
                whole.opacity(scene::leave((t * scene::LEAVE as f32) as u64, false))
            })
            .into_any_element(),
        false => whole.into_any_element(),
    }
}

/// The bar on the launcher's clock.
fn bar(still: bool, cx: &Context<Root>) -> AnyElement {
    let colors = &cx.theme().colors;
    let at = move || relative(scene::bar(startup::since_start(), still));
    let sweep = div().absolute().top_0().h_full().w(relative(scene::BAR)).bg(colors.fg).left(at());
    let sweep = match still {
        true => sweep.into_any_element(),
        false => sweep
            .with_animation("scene-bar", Animation::new(Duration::from_millis(scene::SWEEP)).repeat(), move |sweep, _| sweep.left(at()))
            .into_any_element(),
    };
    div().relative().overflow_hidden().w(px(scene::TRACK)).h(px(scene::THICK)).bg(colors.border).child(sweep).into_any_element()
}

/// Whether the window is cloaked now.
static CLOAKED: AtomicBool = AtomicBool::new(false);

/// Whether the window already took the launcher's place.
static REVEALED: AtomicBool = AtomicBool::new(false);

/// Shows the window the launcher hid and closes the launcher.
pub(crate) fn reveal(window: &Window) {
    if REVEALED.swap(true, Ordering::Relaxed) {
        return;
    }
    #[cfg(windows)]
    cloak(window, false);
    #[cfg(not(windows))]
    let _ = window;
    startup::close_splash();
}

/// Hides a shown window from the screen; it still draws.
#[cfg(windows)]
pub(crate) fn cloak(window: &Window, on: bool) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{DWMWA_CLOAK, DwmSetWindowAttribute};
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return log::error!("startup: the window has no handle to cloak");
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return log::error!("startup: the window is not a Win32 window");
    };
    let value: i32 = on.into();
    // SAFETY: a live window; a BOOL of this size.
    let code = unsafe { DwmSetWindowAttribute(handle.hwnd.get() as _, DWMWA_CLOAK as u32, (&raw const value).cast(), 4) };
    match code {
        0 => {
            CLOAKED.store(on, Ordering::Relaxed);
            log::info!("startup: window {}", if on { "cloaked" } else { "shown" })
        }
        code => log::error!("startup: cloaking failed: {code:#x}"),
    }
}
