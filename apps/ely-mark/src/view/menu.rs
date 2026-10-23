//! The right-click menu, as Typora's (R4.10).

use std::rc::Rc;

use ely_gpui_component::{
    documents::{DocumentTarget, TargetBlock},
    menus::{ContextMenu, Menu, MenuItem},
    primitives::IconName,
};
use ely_ui::menu::Offer;
use ely_ui::i18n::text;
use gpui::{
    Action, App, ClipboardItem, Context, Keystroke, MouseDownEvent, Pixels, Point, SharedString,
    Window,
};

use super::{Failed, MarkView, Mode};
use crate::{blocks as b, clip, export, tools};

/// What a right press asked a menu for.
pub(crate) enum Asked {
    /// The visual mode's target.
    Text(DocumentTarget),
    Source,
    Preview,
}

impl MarkView {
    pub(super) fn ask_menu(&mut self, asked: Asked, at: Point<Pixels>, cx: &mut Context<Self>) {
        self.offer = Some(Offer::new(self.offer.as_ref(), 0, asked, at));
        cx.notify();
    }

    /// A right press the visual blocks left alone.
    pub(super) fn pressed(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let asked = match self.mode {
            Mode::Visual => return,
            Mode::Source => Asked::Source,
            Mode::Preview => Asked::Preview,
        };
        log::info!("mark: a right press in {}", self.mode.name());
        self.ask_menu(asked, event.position, cx);
    }

    pub(super) fn context_menu(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<ContextMenu> {
        let offer = self.offer.as_ref()?;
        let selected = !self.document.read(cx).selected(cx).is_empty();
        let menu = match &offer.what {
            Asked::Text(target) => self.text_menu(target, selected, window, cx),
            Asked::Source => Rows::default()
                .buttons(clipboard(selected, cx))
                .row(MenuItem::submenu(
                    said("mark.menu.copy_as", cx),
                    copy_as(window, cx),
                ))
                .menu(),
            Asked::Preview => copies(window, cx).into_iter().fold(Menu::new(), Menu::item),
        };
        Some(offer.host("mark-menu", menu))
    }

    /// Typora's groups, shown as the target allows.
    fn text_menu(
        &self,
        target: &DocumentTarget,
        selected: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Menu {
        use TargetBlock as K;
        let view = cx.entity();
        let block = target.block;
        let is = |kinds: &[K]| block.is_some_and(|block| kinds.contains(&block));
        let inline = is(&[
            K::Paragraph,
            K::Quote,
            K::Alert,
            K::List,
            K::Table,
            K::Footnote,
        ]) || matches!(block, Some(K::Heading(_)));
        let mut rows = Rows::default();
        if let Some(url) = &target.image {
            let url = url.clone();
            let key = match web(&url) {
                true => "mark.image.browser",
                false => "mark.image.location",
            };
            let (shown, dropped, at) = (view.clone(), view.clone(), target.at);
            rows =
                rows.row(MenuItem::new(said(key, cx)).on_click(move |_, cx| {
                    shown.update(cx, |view, cx| view.show_picture(&url, cx))
                }))
                .row(MenuItem::new(said("mark.image.delete", cx)).on_click(move |_, cx| {
                    dropped.update(cx, |view, cx| view.delete_image(at, cx))
                }))
                .line();
        }
        if let Some(url) = &target.link {
            let (url, view) = (url.clone(), view.clone());
            rows = rows
                .row(
                    MenuItem::new(said("mark.link.open", cx))
                        .on_click(move |_, cx| view.update(cx, |view, cx| view.follow(&url, cx))),
                )
                .line();
        }
        if is(&[K::Math, K::Html]) {
            let (at, view) = (target.at, view.clone());
            rows = rows.row(MenuItem::new(said("mark.block.edit", cx)).on_click(
                move |window, cx| {
                    let document = view.read(cx).document.clone();
                    document.update(cx, |document, cx| document.show_match(at..at, window, cx));
                },
            ));
        }
        rows = rows.buttons(clipboard(selected, cx));
        if let Some(url) = &target.link {
            rows = rows.row(copy_text(said("mark.link.copy", cx), url));
        }
        rows = rows.row(MenuItem::submenu(
            said("mark.menu.copy_as", cx),
            copy_as(window, cx),
        ));
        if let Some(code) = &target.code {
            rows = rows.row(copy_text(said("mark.code.copy", cx), code));
        }
        rows = rows.line();
        if inline {
            rows = rows.buttons(formats(window, cx));
        }
        if !is(&[K::Math, K::Html, K::FrontMatter]) {
            rows = rows.buttons(wraps(window, cx));
        }
        rows = rows.line();
        if target.in_table {
            rows = rows.row(MenuItem::submenu(
                said("mark.menu.table", cx),
                table(window, cx),
            ));
        } else if block.is_some() && !is(&[K::Math, K::Html]) {
            let name = kind(block, cx);
            rows = rows.row(MenuItem::submenu(name, paragraph(window, cx)));
        }
        rows = rows.row(MenuItem::submenu(
            said("mark.menu.insert", cx),
            insert(inline, window, cx),
        ));
        if block.is_some() && !is(&[K::Paragraph]) && !matches!(block, Some(K::Heading(_))) {
            let name = kind(block, cx);
            let label = text(cx, "mark.block.delete", &[("block", &name)]);
            rows = rows
                .line()
                .row(run(label, Box::new(b::DeleteBlock), window));
        }
        rows.menu()
    }

    /// Deletes the picture at byte `at`, or says why not.
    fn delete_image(&mut self, at: usize, cx: &mut Context<Self>) {
        let deleted = (self.document).update(cx, |document, cx| document.delete_image(at, cx));
        if let Err(why) = deleted {
            cx.emit(super::Refused(why));
        }
    }

    /// Shows a local picture's file, or opens a web one.
    fn show_picture(&mut self, url: &str, cx: &mut Context<Self>) {
        if web(url) {
            log::info!("mark: a picture opens in the browser");
            return cx.open_url(url);
        }
        match crate::output::picture_file(url, self.folder.as_deref()) {
            Some(path) if path.is_file() => cx.reveal_path(&path),
            _ => {
                log::info!("mark: a picture with no file here");
                cx.emit(Failed(said("mark.image.no_file", cx).to_string()));
            }
        }
    }
}

fn web(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn said(key: &str, cx: &App) -> SharedString {
    text(cx, key, &[])
}

/// Rows with lines only between groups.
#[derive(Default)]
struct Rows {
    menu: Menu,
    filled: bool,
    line: bool,
}

impl Rows {
    fn row(self, item: MenuItem) -> Self {
        self.with(|menu| menu.item(item))
    }

    fn buttons(self, items: Vec<MenuItem>) -> Self {
        self.with(|menu| menu.buttons(items))
    }

    fn with(mut self, add: impl FnOnce(Menu) -> Menu) -> Self {
        if self.line && self.filled {
            self.menu = self.menu.separator();
        }
        self.menu = add(self.menu);
        (self.filled, self.line) = (true, false);
        self
    }

    fn line(mut self) -> Self {
        self.line = true;
        self
    }

    fn menu(self) -> Menu {
        self.menu
    }
}

/// A row that runs `action`, its keys shown.
fn run(label: SharedString, action: Box<dyn Action>, window: &Window) -> MenuItem {
    let keys = (window.highest_precedence_binding_for_action(action.as_ref())).map(|binding| {
        let strokes: Vec<String> = binding.keystrokes().iter().map(|k| k.unparse()).collect();
        strokes.join(" ")
    });
    let action: Rc<Box<dyn Action>> = Rc::new(action);
    let row = MenuItem::new(label)
        .on_click(move |window, cx| window.dispatch_action(action.boxed_clone(), cx));
    match keys {
        Some(keys) => row.keys(keys),
        None => row,
    }
}

/// A button that presses `stroke` where focus went back.
fn press(label: SharedString, icon: IconName, stroke: &str, shown: bool) -> MenuItem {
    let keystroke = Keystroke::parse(stroke).expect("a fixed keystroke");
    let button = MenuItem::new(label).icon(icon);
    let button = match shown {
        true => button.keys(keystroke.unparse()),
        false => button,
    };
    button.on_click(move |window, cx| {
        let keystroke = keystroke.clone();
        window.defer(cx, move |window, cx| {
            window.dispatch_keystroke(keystroke, cx);
        });
    })
}

fn clipboard(selected: bool, cx: &App) -> Vec<MenuItem> {
    vec![
        press(
            said("mark.edit.cut", cx),
            IconName::Scissors,
            "secondary-x",
            true,
        )
        .disabled(!selected),
        press(
            said("mark.edit.copy", cx),
            IconName::Copy,
            "secondary-c",
            true,
        )
        .disabled(!selected),
        press(
            said("mark.edit.paste", cx),
            IconName::Clipboard,
            "secondary-v",
            true,
        ),
        press(
            said("mark.edit.delete", cx),
            IconName::Trash2,
            "backspace",
            false,
        )
        .disabled(!selected),
    ]
}

fn copy_text(label: SharedString, copied: &str) -> MenuItem {
    let copied = copied.to_string();
    MenuItem::new(label).on_click(move |_, cx| {
        log::info!("mark: {} bytes copied from the menu", copied.len());
        cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()));
    })
}

/// Typora's Copy / Paste As rows.
fn copies(window: &Window, cx: &App) -> Vec<MenuItem> {
    vec![
        run(
            said("mark.copy.plain", cx),
            Box::new(export::CopyPlain),
            window,
        ),
        run(
            said("mark.copy.markdown", cx),
            Box::new(clip::CopyMarkdown),
            window,
        ),
        run(
            said("mark.copy.html", cx),
            Box::new(export::CopyHtml),
            window,
        ),
        run(
            said("mark.copy.rich", cx),
            Box::new(export::CopyRich),
            window,
        ),
    ]
}

fn copy_as(window: &Window, cx: &App) -> Menu {
    let paste = run(
        said("mark.paste.plain", cx),
        Box::new(clip::PastePlain),
        window,
    );
    copies(window, cx)
        .into_iter()
        .fold(Menu::new(), Menu::item)
        .separator()
        .item(paste)
}

fn formats(window: &Window, cx: &App) -> Vec<MenuItem> {
    let row = |key, action: Box<dyn Action>, name| run(said(key, cx), action, window).icon(name);
    vec![
        row("mark.format.strong", Box::new(b::Strong), IconName::Bold),
        row(
            "mark.format.emphasis",
            Box::new(b::Emphasis),
            IconName::Italic,
        ),
        row("mark.format.code", Box::new(b::InlineCode), IconName::Code),
        row("mark.format.link", Box::new(b::Hyperlink), IconName::Link),
    ]
}

fn wraps(window: &Window, cx: &App) -> Vec<MenuItem> {
    let row = |key, action: Box<dyn Action>, name| run(said(key, cx), action, window).icon(name);
    vec![
        row("mark.block.quote", Box::new(b::Quote), IconName::Quote),
        row(
            "mark.block.ordered",
            Box::new(b::OrderedList),
            IconName::ListOrdered,
        ),
        row("mark.block.bullet", Box::new(b::BulletList), IconName::List),
        row(
            "mark.block.task",
            Box::new(b::TaskList),
            IconName::ListChecks,
        ),
    ]
}

fn paragraph(window: &Window, cx: &App) -> Menu {
    let row = |key, action: Box<dyn Action>| run(said(key, cx), action, window);
    Menu::new()
        .item(row("mark.block.h1", Box::new(b::Heading1)))
        .item(row("mark.block.h2", Box::new(b::Heading2)))
        .item(row("mark.block.h3", Box::new(b::Heading3)))
        .item(row("mark.block.h4", Box::new(b::Heading4)))
        .item(row("mark.block.h5", Box::new(b::Heading5)))
        .item(row("mark.block.h6", Box::new(b::Heading6)))
        .separator()
        .item(row("mark.block.paragraph", Box::new(b::Paragraph)))
}

fn table(window: &Window, cx: &App) -> Menu {
    let row = |key, action: Box<dyn Action>| run(said(key, cx), action, window);
    Menu::new()
        .item(row("mark.table.row_above", Box::new(tools::RowAbove)))
        .item(row("mark.table.row_below", Box::new(tools::RowBelow)))
        .item(row("mark.table.column_left", Box::new(tools::ColumnLeft)))
        .item(row("mark.table.column_right", Box::new(tools::ColumnRight)))
        .separator()
        .item(row("mark.table.move_row_up", Box::new(tools::MoveRowUp)))
        .item(row(
            "mark.table.move_row_down",
            Box::new(tools::MoveRowDown),
        ))
        .item(row(
            "mark.table.move_column_left",
            Box::new(tools::MoveColumnLeft),
        ))
        .item(row(
            "mark.table.move_column_right",
            Box::new(tools::MoveColumnRight),
        ))
        .separator()
        .item(row("mark.table.delete_row", Box::new(tools::DeleteRow)))
        .item(row(
            "mark.table.delete_column",
            Box::new(tools::DeleteColumn),
        ))
        .separator()
        .item(row("mark.table.tidy", Box::new(tools::TidyTable)))
        .separator()
        .item(row("mark.table.delete", Box::new(tools::DeleteTable)))
}

fn insert(inline: bool, window: &Window, cx: &App) -> Menu {
    let row = |key, action: Box<dyn Action>| run(said(key, cx), action, window);
    let menu = match inline {
        true => Menu::new()
            .item(row("mark.insert.image", Box::new(b::InsertImage)))
            .separator(),
        false => Menu::new(),
    };
    menu.item(row("mark.block.footnote", Box::new(b::Footnote)))
        .item(row("mark.block.link_reference", Box::new(b::LinkReference)))
        .item(row("mark.block.rule", Box::new(b::Rule)))
        .item(row("mark.menu.table", Box::new(tools::InsertTable)))
        .item(row("mark.block.code", Box::new(b::CodeFences)))
        .item(row("mark.block.math", Box::new(b::MathBlock)))
        .item(row("mark.block.toc", Box::new(b::Toc)))
        .item(row("mark.block.front_matter", Box::new(b::FrontMatter)))
        .separator()
        .item(row("mark.insert.before", Box::new(b::ParagraphBefore)))
        .item(row("mark.insert.after", Box::new(b::ParagraphAfter)))
}

/// A block kind's name, as Typora's menus say it.
fn kind(block: Option<TargetBlock>, cx: &App) -> SharedString {
    use TargetBlock as K;
    let key = match block {
        None | Some(K::Paragraph) => "mark.block.paragraph",
        Some(K::Heading(level)) => {
            return said(&format!("mark.block.h{}", level.clamp(1, 6)), cx);
        }
        Some(K::Quote) => "mark.block.quote",
        Some(K::Alert) => "mark.kind.alert",
        Some(K::List) => "mark.kind.list",
        Some(K::Code) => "mark.block.code",
        Some(K::Math) => "mark.block.math",
        Some(K::Table) => "mark.menu.table",
        Some(K::Rule) => "mark.block.rule",
        Some(K::Html) => "mark.kind.html",
        Some(K::Footnote) => "mark.block.footnote",
        Some(K::Definition) => "mark.block.link_reference",
        Some(K::FrontMatter) => "mark.block.front_matter",
        Some(K::Toc) => "mark.block.toc",
    };
    said(key, cx)
}
