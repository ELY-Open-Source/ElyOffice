use ely_platform::AppId;
use std::rc::Rc;

use gpui::{Action, App, Global, KeyBinding, Menu, MenuItem, SharedString, Window, actions};

use crate::i18n;

actions!(
    office,
    [
        New,
        Open,
        Save,
        SaveAs,
        Rename,
        Export,
        Print,
        CloseTab,
        ToggleCommandPalette,
        OpenSettings,
        CheckForUpdates,
        ToggleLeftPanel,
        ToggleRightPanel,
        Quit
    ]
);

actions!(
    apps,
    [OpenElyDoc, OpenElyTable, OpenElyShow, OpenElyPdf, OpenElyMail, OpenElyTree, OpenElyMark, OpenElyAgent]
);

/// The main window's own quit, run from any window.
struct QuitRoute(Rc<dyn Fn(&mut App)>);

impl Global for QuitRoute {}

/// Quit from every window, settings too, runs `quit`.
pub fn route_quit(cx: &mut App, quit: impl Fn(&mut App) + 'static) {
    cx.set_global(QuitRoute(Rc::new(quit)));
}

/// Quit, from any window, goes through `quit`.
pub fn bind_quit(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| quit(cx));
}

/// Quits through the main window's route, if one is set.
pub fn quit(cx: &mut App) {
    match cx.try_global::<QuitRoute>().map(|route| Rc::clone(&route.0)) {
        Some(route) => route(cx),
        None => cx.quit(),
    }
}

/// The action that starts `app` (§16 app switching).
pub fn switch_to(app: AppId) -> Box<dyn Action> {
    match app {
        AppId::Doc => Box::new(OpenElyDoc),
        AppId::Table => Box::new(OpenElyTable),
        AppId::Show => Box::new(OpenElyShow),
        AppId::Pdf => Box::new(OpenElyPdf),
        AppId::Mail => Box::new(OpenElyMail),
        AppId::Tree => Box::new(OpenElyTree),
        AppId::Mark => Box::new(OpenElyMark),
        AppId::Agent => Box::new(OpenElyAgent),
    }
}

/// One command: an id and a chord.
pub struct Command {
    pub id: &'static str,
    pub keys: &'static str,
    pub blocked: Option<&'static str>,
    pub action: fn() -> Box<dyn Action>,
}

const BLOCKED: &str = "command.blocked.no_document";
const NEW: Command = Command {
    id: "file.new",
    keys: "secondary-n",
    blocked: None,
    action: || Box::new(New),
};
const OPEN: Command = Command {
    id: "file.open",
    keys: "secondary-o",
    blocked: None,
    action: || Box::new(Open),
};
const SAVE: Command = Command {
    id: "file.save",
    keys: "secondary-s",
    blocked: Some(BLOCKED),
    action: || Box::new(Save),
};
const SAVE_AS: Command = Command {
    id: "file.save_as",
    keys: "secondary-shift-s",
    blocked: Some(BLOCKED),
    action: || Box::new(SaveAs),
};
const RENAME: Command = Command {
    id: "file.rename",
    keys: "f2",
    blocked: Some(BLOCKED),
    action: || Box::new(Rename),
};
const EXPORT: Command = Command {
    id: "file.export",
    keys: "secondary-e",
    blocked: Some(BLOCKED),
    action: || Box::new(Export),
};
const PRINT: Command = Command {
    id: "file.print",
    keys: "secondary-p",
    blocked: Some(BLOCKED),
    action: || Box::new(Print),
};

pub const FILE: [&Command; 6] = [&NEW, &OPEN, &SAVE, &SAVE_AS, &EXPORT, &PRINT];
/// A document app that also renames its files.
pub const RENAMING: [&Command; 7] = [&NEW, &OPEN, &SAVE, &SAVE_AS, &RENAME, &EXPORT, &PRINT];
/// A tool that starts and opens, but saves nothing itself.
pub const OPENING: [&Command; 2] = [&NEW, &OPEN];
pub const TOGGLE_PALETTE: &str = "secondary-shift-p";
pub const CONTEXT: &str = "office";

fn label(command: &Command, cx: &App) -> SharedString {
    i18n::text(cx, &format!("command.{}", command.id), &[])
}

/// Why `id` cannot run; a document lifts the no-document block.
pub fn blocked(id: &str, document: bool, cx: &App) -> Option<SharedString> {
    let command = RENAMING.iter().find(|command| command.id == id)?;
    let key = command.blocked.filter(|_| !document)?;
    Some(i18n::text(cx, key, &[]))
}

/// The app menu, File with `file`'s commands, then `extra`.
pub fn menus(
    app: AppId,
    file: &[&Command],
    document: bool,
    extra: Vec<Menu>,
    cx: &App,
) -> Vec<Menu> {
    let items = file.iter().map(|command| {
        let off = blocked(command.id, document, cx).is_some();
        MenuItem::Action {
            name: label(command, cx),
            action: (command.action)(),
            os_action: None,
            checked: false,
            disabled: off,
        }
    });
    // The other installed Ely apps, to switch to.
    let others: Vec<MenuItem> = AppId::ALL
        .into_iter()
        .filter(|other| *other != app && ely_platform::apps::installed(*other))
        .map(|other| MenuItem::Action {
            name: other.name().into(),
            action: switch_to(other),
            os_action: None,
            checked: false,
            disabled: false,
        })
        .collect();
    let mut first = Vec::new();
    if !others.is_empty() {
        first.push(MenuItem::submenu(Menu::new(i18n::text(cx, "menu.app.switch", &[])).items(others)));
        first.push(MenuItem::separator());
    }
    first.push(MenuItem::action(i18n::text(cx, "menu.app.settings", &[]), OpenSettings));
    // Only an installed app can update itself.
    if crate::updates::updater(cx).is_some() {
        first.push(MenuItem::action(i18n::text(cx, "menu.app.updates", &[]), CheckForUpdates));
    }
    first.push(MenuItem::separator());
    first.push(MenuItem::action(i18n::text(cx, "menu.app.quit", &[("app", app.name())]), Quit));
    let mut menus = vec![
        Menu::new(app.name()).items(first),
        Menu::new(i18n::text(cx, "menu.file", &[])).items(items.collect::<Vec<_>>()),
    ];
    menus.extend(extra);
    menus
}

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(PRINT.keys, Print, Some(CONTEXT)),
        KeyBinding::new(NEW.keys, New, Some(CONTEXT)),
        KeyBinding::new(OPEN.keys, Open, Some(CONTEXT)),
        KeyBinding::new(SAVE.keys, Save, Some(CONTEXT)),
        KeyBinding::new(SAVE_AS.keys, SaveAs, Some(CONTEXT)),
        KeyBinding::new(RENAME.keys, Rename, Some(CONTEXT)),
        KeyBinding::new(EXPORT.keys, Export, Some(CONTEXT)),
        KeyBinding::new("secondary-w", CloseTab, Some(CONTEXT)),
        KeyBinding::new(TOGGLE_PALETTE, ToggleCommandPalette, Some(CONTEXT)),
        // Shift+L is Calc's AutoFilter; B as in ElyDoc.
        KeyBinding::new("secondary-alt-s", ToggleLeftPanel, Some(CONTEXT)),
        KeyBinding::new("secondary-alt-b", ToggleRightPanel, Some(CONTEXT)),
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("secondary-,", OpenSettings, None),
    ]);
}

/// The View menu's item for the left panel.
pub fn left_panel_item(cx: &App) -> MenuItem {
    MenuItem::action(i18n::text(cx, "menu.view.left", &[]), ToggleLeftPanel)
}

/// The View menu's item for the right panel.
pub fn right_panel_item(cx: &App) -> MenuItem {
    MenuItem::action(i18n::text(cx, "menu.view.right", &[]), ToggleRightPanel)
}

pub fn run(id: &str, window: &mut Window, cx: &mut App) {
    match RENAMING.iter().find(|command| command.id == id) {
        Some(command) => window.dispatch_action((command.action)(), cx),
        None => log::warn!("commands: {id} is not a command"),
    }
}
