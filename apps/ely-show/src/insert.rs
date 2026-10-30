//! The Insert and View menus (SHOW-03 to 06).

use ely_office::prelude::*;
use gpui::{App, Context, Div, InteractiveElement as _, Menu, MenuItem, actions};

use crate::deck::Deck;

actions!(
    show,
    [
        InsertTextBox,
        InsertRectangle,
        InsertEllipse,
        InsertTriangle,
        InsertLine,
        InsertArrow,
        InsertConnector,
        InsertPicture,
        InsertMedia,
        InsertTable,
        InsertChart,
        InsertRemark,
        ShowRemarks,
        SnapToGrid,
        SnapToGuides,
        AddHorizontalGuide,
        AddVerticalGuide
    ]
);

/// Insert and View; View checks what is on.
pub fn menus((grid, guides, remarks): (bool, bool, bool), cx: &App) -> [Menu; 2] {
    let label = |key: &str| Deck::text(cx, key, &[]);
    let media = ely_platform::media::BUILT.then(|| MenuItem::action(label("show.insert.media"), InsertMedia));
    let insert = [
        MenuItem::action(label("show.insert.text"), InsertTextBox),
        MenuItem::separator(),
        MenuItem::action(label("show.insert.rectangle"), InsertRectangle),
        MenuItem::action(label("show.insert.ellipse"), InsertEllipse),
        MenuItem::action(label("show.insert.triangle"), InsertTriangle),
        MenuItem::action(label("show.insert.line"), InsertLine),
        MenuItem::action(label("show.insert.arrow"), InsertArrow),
        MenuItem::action(label("show.insert.connector"), InsertConnector),
        MenuItem::separator(),
        MenuItem::action(label("show.insert.picture"), InsertPicture),
    ]
    .into_iter()
    .chain(media)
    .chain([
        MenuItem::action(label("show.insert.table"), InsertTable),
        MenuItem::action(label("show.insert.chart"), InsertChart),
        MenuItem::separator(),
        MenuItem::action(label("show.insert.remark"), InsertRemark),
    ]);
    let insert = Menu::new(label("menu.insert")).items(insert);
    let view = Menu::new(label("menu.view")).items([
        MenuItem::action(label("show.view.remarks"), ShowRemarks).checked(remarks),
        MenuItem::separator(),
        MenuItem::action(label("show.view.snap_grid"), SnapToGrid).checked(grid),
        MenuItem::action(label("show.view.snap_guides"), SnapToGuides).checked(guides),
        MenuItem::separator(),
        MenuItem::action(label("show.view.guide_across"), AddHorizontalGuide),
        MenuItem::action(label("show.view.guide_down"), AddVerticalGuide),
        MenuItem::separator(),
        ely_ui::commands::left_panel_item(cx),
        ely_ui::commands::right_panel_item(cx),
        MenuItem::separator(),
    ]);
    let view = view.items(ely_office::zoom_items(true, cx));
    [insert, view]
}

/// Binds an action to an object command.
macro_rules! run {
    ($root:expr, $cx:expr, $action:ty, $id:literal, $values:expr) => {
        $root.on_action(
            $cx.listener(|deck, _: &$action, _, cx| deck.object_command($id, $values, cx)),
        )
    };
}

pub fn bind(root: Div, cx: &mut Context<Deck>) -> Div {
    let root = run!(root, cx, InsertTextBox, "draw.text", &["true"]);
    let root = run!(
        root,
        cx,
        InsertRectangle,
        "draw.shape",
        &["rectangle", "true"]
    );
    let root = run!(root, cx, InsertEllipse, "draw.shape", &["ellipse", "true"]);
    let root = run!(
        root,
        cx,
        InsertTriangle,
        "draw.shape",
        &["isosceles-triangle", "true"]
    );
    let root = run!(root, cx, InsertLine, "draw.line", &[]);
    let root = run!(root, cx, InsertArrow, "draw.arrow", &[]);
    let root = run!(root, cx, InsertConnector, "draw.connector", &[]);
    let root = run!(root, cx, SnapToGrid, "view.snap_grid", &[]);
    let root = run!(root, cx, SnapToGuides, "view.snap_guides", &[]);
    root.on_action(cx.listener(|deck, _: &InsertPicture, _, cx| deck.pick_picture(false, cx)))
        .on_action(cx.listener(|deck, _: &InsertMedia, _, cx| deck.pick_media(cx)))
        .on_action(cx.listener(|deck, _: &InsertTable, _, cx| deck.ask_table(cx)))
        .on_action(cx.listener(|deck, _: &InsertChart, _, cx| deck.ask_chart(cx)))
        .on_action(
            cx.listener(|deck, _: &InsertRemark, window, cx| deck.ask_remark(None, window, cx)),
        )
        .on_action(cx.listener(|deck, _: &ShowRemarks, _, cx| deck.toggle_remarks(cx)))
        .on_action(cx.listener(|deck, _: &AddHorizontalGuide, _, cx| deck.add_guide(false, cx)))
        .on_action(cx.listener(|deck, _: &AddVerticalGuide, _, cx| deck.add_guide(true, cx)))
}

impl Deck {
    /// Runs an object command; the next read shows the result.
    pub fn object_command(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(tab) = self.core.active else {
            return;
        };
        self.run_in(Some(tab), id, values, cx);
        self.force_reread(tab, cx);
    }

    /// Inserts a picture, or replaces the selected one.
    pub fn pick_picture(&mut self, replace: bool, cx: &mut Context<Self>) {
        self.pick(false, cx, move |deck, path, cx| {
            let url = ely_lok::file_url(&path);
            let id = if replace {
                "image.replace"
            } else {
                "image.insert"
            };
            deck.object_command(id, &[&url], cx)
        });
    }

    /// Shows comments, or properties, in the right column.
    pub(crate) fn toggle_remarks(&mut self, cx: &mut Context<Self>) {
        self.remarks_open = !self.remarks_open;
        if self.remarks_open {
            ely_ui::settings::panels::show(false, true, cx);
        }
        self.snap_menus = None;
        self.sync_menus(cx);
        cx.notify();
    }

    /// A guide through the middle of the slide.
    fn add_guide(&mut self, vertical: bool, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (w, h) = tab.extra.shown_size();
        let (kind, x, y) = if vertical {
            ("1", w / 2, 0)
        } else {
            ("0", 0, h / 2)
        };
        self.object_command("guide.add", &[kind, &x.to_string(), &y.to_string()], cx);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn insert_media_shows_only_where_it_plays() {
        let cx = gpui::TestAppContext::single();
        cx.update(ely_gpui_component::init_for_tests);
        let (offered, total) = cx.update(|cx| {
            ely_ui::settings::install_defaults(cx);
            ely_ui::i18n::install_as("en-US", cx);
            let media = crate::deck::Deck::text(cx, "show.insert.media", &[]);
            let [insert, _] = super::menus((false, false, false), cx);
            let names = insert.items.iter().filter_map(|item| match item {
                gpui::MenuItem::Action { name, .. } => Some(name.clone()),
                _ => None,
            });
            let names: Vec<_> = names.collect();
            (names.contains(&media), names.len())
        });
        assert_eq!(offered, ely_platform::media::BUILT);
        assert!(total >= 11, "the rest stay: {total}");
    }
}
