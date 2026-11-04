//! Every shell region; page two drops the right panel.
mod dialogs;
mod probe;
mod props;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use dialogs::Overlay;
use ely_gpui_component::{
    feedback::EmptyState,
    forms::TextInput,
    navigation::{EditorTab, EditorTabs},
    primitives::{FocusRing as _, IconName},
    shell::{StatusBar, StatusBarItem},
    theme::ActiveTheme,
};
use ely_platform::AppId;
use ely_ui::{LongTask, Root, Shell, TaskState};
use gpui::{
    App, AppContext as _, Bounds, Context, Entity, FocusHandle, InteractiveElement as _,
    IntoElement, ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement as _,
    Styled, Window, canvas, div, prelude::FluentBuilder as _, px,
};
use props::Props;

fn main() {
    ely_ui::run(AppId::Doc, gallery);
}

fn gallery(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    let geometry = Rc::new(RefCell::new(None));
    let view = cx.new(|cx| Gallery::new(window, cx, geometry));
    let root = ely_ui::root(app, Some(view.clone().into()), window, cx);
    if std::env::var("ELY_GALLERY_PROBE").as_deref() == Ok("1") {
        let next = (root.clone(), view.clone());
        window.on_next_frame(move |window, cx| probe::run(next.0, next.1, 0, window, cx));
    }
    root
}

pub(crate) struct DemoPage {
    id: &'static str,
    title: SharedString,
    format: &'static str,
}

fn page(id: &'static str, title: &'static str, format: &'static str) -> DemoPage {
    DemoPage {
        id,
        title: title.into(),
        format,
    }
}

pub(crate) struct Gallery {
    focus: FocusHandle,
    page: SharedString,
    pages: Vec<DemoPage>,
    selection: Vec<usize>,
    dialog: Option<Overlay>,
    task: Option<Entity<LongTask>>,
    beats_began: Arc<AtomicUsize>,
    chip: FocusHandle,
    until: Option<Instant>,
    waited: Option<Instant>,
    frames: u8,
    probe_focused: Option<FocusHandle>,
    geometry: Rc<RefCell<Option<Bounds<Pixels>>>>,
    content: Entity<Content>,
    nav: Entity<Nav>,
    props: Entity<Props>,
}

impl Gallery {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        geometry: Rc<RefCell<Option<Bounds<Pixels>>>>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let me = cx.weak_entity();
        Gallery {
            focus,
            page: "one".into(),
            pages: vec![
                page("one", "Page One", "Text"),
                page("two", "Page Two", "Grid"),
            ],
            selection: vec![0],
            dialog: None,
            task: None,
            beats_began: Arc::new(AtomicUsize::new(0)),
            chip: cx.focus_handle().tab_index(4).tab_stop(true),
            until: None,
            waited: None,
            frames: 0,
            probe_focused: None,
            geometry: geometry.clone(),
            content: cx.new(|_| Content {
                page: "one".into(),
                anchor: None,
                geometry,
            }),
            nav: cx.new(|cx| Nav {
                anchor: cx.focus_handle().tab_index(1).tab_stop(true),
            }),
            props: cx.new(|cx| Props {
                anchor: cx.focus_handle().tab_index(3).tab_stop(true),
                gallery: me.clone(),
                rows: Vec::new(),
                empty: false,
            }),
        }
    }

    pub(crate) fn props_rows(&self, cx: &App) -> Vec<(String, String)> {
        let pages = ely_ui::i18n::text(cx, "gallery.row.pages", &[]).to_string();
        if self.selection.is_empty() {
            return vec![(pages, "0".into())];
        }
        let formats: Vec<&str> = self
            .selection
            .iter()
            .map(|&index| self.pages[index].format)
            .collect();
        let same = formats.windows(2).all(|pair| pair[0] == pair[1]);
        let format = if same {
            formats[0].to_string()
        } else {
            ely_ui::i18n::text(cx, "gallery.mixed", &[]).to_string()
        };
        vec![
            (
                ely_ui::i18n::text(cx, "gallery.row.format", &[]).to_string(),
                format,
            ),
            (pages, self.selection.len().to_string()),
        ]
    }

    pub(crate) fn page_titles(&self) -> Vec<String> {
        self.pages
            .iter()
            .map(|page| page.title.to_string())
            .collect()
    }

    pub(crate) fn open_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(&index) = self.selection.first() else {
            log::info!("gallery: rename refused: no page is selected");
            return;
        };
        let title = self.pages[index].title.to_string();
        let field = cx.new(|cx| {
            TextInput::new(window, cx).placeholder(ely_ui::i18n::text(
                cx,
                "gallery.rename.placeholder",
                &[],
            ))
        });
        field.update(cx, |field, cx| field.set_text(title, cx));
        self.dialog = Some(Overlay::Rename { field });
        cx.notify();
    }

    pub(crate) fn rename(&mut self, text: &str, cx: &mut Context<Self>) {
        let title: SharedString = text.trim().to_string().into();
        log::info!("gallery: page renamed to {title}");
        self.pages[self.selection[0]].title = title;
        self.dialog = None;
        cx.notify();
    }

    pub(crate) fn open_delete(&mut self, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            log::info!("gallery: delete refused: no page is selected");
            return;
        }
        self.dialog = Some(Overlay::Delete);
        cx.notify();
    }

    pub(crate) fn delete_confirmed(&mut self, cx: &mut Context<Self>) {
        let Some(&index) = self.selection.first() else {
            log::info!("gallery: delete refused: no page is selected");
            self.dialog = None;
            cx.notify();
            return;
        };
        let removed = self.pages.remove(index);
        log::info!("gallery: {} deleted", removed.title);
        self.selection = if self.pages.is_empty() {
            Vec::new()
        } else {
            vec![0]
        };
        if self.pages.iter().any(|page| page.id == self.page.as_ref()) {
        } else if let Some(first) = self.pages.first() {
            self.page = first.id.into();
        } else {
            self.page = "".into();
        }
        self.dialog = None;
        cx.notify();
    }

    pub(crate) fn close_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        cx.notify();
    }

    pub(crate) fn start_task(&mut self, fail: bool, cx: &mut Context<Self>) {
        let blown = Arc::new(AtomicBool::new(false));
        let began = self.beats_began.clone();
        began.store(0, Ordering::Relaxed);
        let disk_full = ely_ui::i18n::text(cx, "gallery.error.disk_full", &[]);
        self.task = Some(LongTask::start(
            "export",
            4,
            move |done, stop| {
                began.fetch_add(1, Ordering::Relaxed);
                if fail && done == 3 {
                    std::thread::sleep(Duration::from_millis(400));
                    if !blown.swap(true, Ordering::Relaxed) {
                        return Err(disk_full.clone());
                    }
                    return Ok(());
                }
                for _ in 0..12 {
                    if stop.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(())
            },
            cx,
        ));
        cx.notify();
    }

    pub(crate) fn beats_began(&self) -> usize {
        self.beats_began.load(Ordering::Relaxed)
    }

    pub(crate) fn clicked_task(&mut self, cx: &mut Context<Self>) {
        if let Some(task) = &self.task {
            match task.read(cx).state() {
                TaskState::Running { .. } => task.update(cx, |task, cx| task.cancel(cx)),
                TaskState::Failed(_) => task.update(cx, |task, cx| task.retry(cx)),
                _ => {}
            }
        }
    }

    pub(crate) fn task_state(&self, cx: &App) -> Option<String> {
        self.task.as_ref().map(|task| match task.read(cx).state() {
            TaskState::Running { done, total } => format!("Running {done}/{total}"),
            TaskState::Succeeded => "Succeeded".into(),
            TaskState::Failed(error) => format!("Failed({error})"),
            TaskState::Cancelled => "Cancelled".into(),
        })
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = self.page.clone();
        let right = (page.as_ref() == "one").then(|| self.props.clone().into());
        let rows = self.props_rows(cx);
        self.props.update(cx, |props, _| {
            props.rows = rows;
            props.empty = self.pages.is_empty();
        });
        self.content
            .update(cx, |content, _| content.page = page.clone());
        let task_label = self.task_state(cx).map(|state| match state.as_str() {
            "Succeeded" => ely_ui::i18n::text(cx, "gallery.task.done", &[]),
            "Cancelled" => ely_ui::i18n::text(cx, "gallery.task.cancelled", &[]),
            _ if state.starts_with("Failed(") => ely_ui::i18n::text(
                cx,
                "gallery.task.retry",
                &[(
                    "error",
                    state.trim_start_matches("Failed(").trim_end_matches(")"),
                )],
            ),
            _ => ely_ui::i18n::text(
                cx,
                "gallery.task.progress",
                &[("state", state.trim_start_matches("Running "))],
            ),
        });
        let this = cx.entity();
        let theme = cx.theme();
        let status = StatusBar::new()
            .when_some(task_label, |bar, label| {
                bar.left(
                    div()
                        .id("task-chip")
                        .track_focus(&self.chip)
                        .role(gpui::Role::Button)
                        .aria_label(label.clone())
                        .flex()
                        .items_center()
                        .h_full()
                        .px_1p5()
                        .rounded(theme.radius(ely_gpui_component::theme::Radius::Sm))
                        .focus_ring(cx)
                        .hover(|style| style.bg(theme.colors.hover))
                        .on_click(move |_, _, cx| {
                            this.update(cx, |this, cx| this.clicked_task(cx));
                        })
                        .child(label),
                )
            })
            .left(StatusBarItem::new("state").label(ely_ui::i18n::text(cx, "gallery.ready", &[])))
            .right(StatusBarItem::new("page").label(page.clone()));
        let body: gpui::AnyElement = if self.pages.is_empty() {
            EmptyState::new(
                "no-pages",
                IconName::File,
                ely_ui::i18n::text(cx, "gallery.empty.pages", &[]),
            )
            .into_any_element()
        } else {
            self.content.clone().into_any_element()
        };
        let shell = Shell::new("Shell Gallery", &self.focus)
            .left(Some(self.nav.clone().into()))
            .right(right)
            .status(status)
            .page(page.clone())
            .child(body);
        let shell = if self.pages.is_empty() {
            shell
        } else {
            shell.tabs(
                EditorTabs::new(
                    "tabs",
                    self.pages
                        .iter()
                        .map(|page| EditorTab::new(page.id, page.title.clone()))
                        .collect::<Vec<_>>(),
                )
                .selected(page.clone())
                .on_select(cx.listener(|this, id: &SharedString, _, cx| {
                    this.page = id.clone();
                    this.selection = vec![
                        this.pages
                            .iter()
                            .position(|page| page.id == id.as_ref())
                            .unwrap_or(0),
                    ];
                    cx.notify();
                })),
            )
        };
        let overlay = match &self.dialog {
            Some(Overlay::Rename { field }) => Some(dialogs::rename_overlay(field.clone(), cx)),
            Some(Overlay::Delete) => Some(dialogs::delete_overlay(
                self.pages[self.selection[0]].title.clone(),
                cx,
            )),
            None => None,
        };
        div()
            .size_full()
            .child(shell)
            .when_some(overlay, |layer, dialog| layer.child(dialog))
    }
}

struct Content {
    page: SharedString,
    anchor: Option<FocusHandle>,
    geometry: Rc<RefCell<Option<Bounds<Pixels>>>>,
}

impl Render for Content {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let handle = window
            .use_keyed_state("content-anchor", cx, |_, cx| {
                cx.focus_handle().tab_index(2).tab_stop(true)
            })
            .read(cx)
            .clone();
        self.anchor = Some(handle.clone());
        let geometry = self.geometry.clone();
        div()
            .flex_1()
            .relative()
            .child(
                canvas(
                    move |bounds, _, _| {
                        *geometry.borrow_mut() = Some(bounds);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(anchor(
                        "content-anchor",
                        ely_ui::i18n::text(cx, "gallery.content", &[]),
                        &handle,
                        cx,
                    ))
                    .child(ely_ui::i18n::text(
                        cx,
                        "gallery.content_of",
                        &[("page", self.page.as_ref())],
                    )),
            )
    }
}

fn close(a: Pixels, b: Pixels) -> bool {
    (a - b).abs() <= px(2.0)
}

fn anchor(
    id: &'static str,
    label: impl Into<SharedString>,
    handle: &FocusHandle,
    cx: &mut App,
) -> impl IntoElement {
    let theme = cx.theme();
    let label = label.into();
    div()
        .id(id)
        .track_focus(handle)
        .tab_index(0)
        .role(gpui::Role::Button)
        .aria_label(label.clone())
        .border_1()
        .border_color(theme.colors.border)
        .rounded(theme.radius(ely_gpui_component::theme::Radius::Sm))
        .px_2()
        .py_1()
        .focus_ring(cx)
        .child(label)
}

struct Nav {
    anchor: FocusHandle,
}

impl Render for Nav {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .h_full()
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .text_color(theme.colors.fg)
            .child(ely_ui::i18n::text(cx, "gallery.nav", &[]))
            .child(anchor(
                "nav-anchor",
                ely_ui::i18n::text(cx, "gallery.inbox", &[]),
                &self.anchor,
                cx,
            ))
            .child(ely_ui::i18n::text(cx, "gallery.archive", &[]))
    }
}
