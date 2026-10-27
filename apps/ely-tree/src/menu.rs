//! The Repository menu, and File's New and Open as repositories.

use ely_gpui_component::secondary;
use ely_ui::commands;
use std::path::PathBuf;

use gpui::{App, Context, KeyBinding, Menu, MenuItem, PathPromptOptions, Window, actions};

use crate::tree::{Tree, text};

actions!(
    tree,
    [
        CloneRepository,
        CloseRepository,
        Fetch,
        Pull,
        Push,
        ForcePush,
        OpenTerminal,
        Reveal,
        CleanUntracked,
        ChooseEditor
    ]
);

pub const CONTEXT: &str = "tree";

pub fn bind(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        secondary!("w"),
        CloseRepository,
        Some(CONTEXT),
    )]);
}

pub fn menus(cx: &App) -> Vec<Menu> {
    let terminal = crate::tools::TERMINAL
        .then(|| MenuItem::action(text(cx, "tree.tool.terminal", &[]), OpenTerminal));
    let items = [
        MenuItem::action(text(cx, "tree.clone", &[]), CloneRepository),
        MenuItem::action(text(cx, "tree.close", &[]), CloseRepository),
        MenuItem::separator(),
        MenuItem::action(text(cx, "tree.net.fetch", &[]), Fetch),
        MenuItem::action(text(cx, "tree.net.pull", &[]), Pull),
        MenuItem::action(text(cx, "tree.net.push", &[]), Push),
        MenuItem::action(text(cx, "tree.force.open", &[]), ForcePush),
        MenuItem::separator(),
    ]
    .into_iter()
    .chain(terminal)
    .chain([
        MenuItem::action(text(cx, crate::tools::REVEAL, &[]), Reveal),
        MenuItem::action(text(cx, "tree.clean.open", &[]), CleanUntracked),
        MenuItem::action(text(cx, "tree.tool.editor", &[]), ChooseEditor),
    ]);
    vec![Menu::new(text(cx, "tree.menu.repository", &[])).items(items)]
}

impl Tree {
    /// Asks for folders; `then` gets each one picked.
    fn pick(
        &mut self,
        multiple: bool,
        then: fn(&mut Tree, PathBuf, &mut Context<Tree>),
        cx: &mut Context<Self>,
    ) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            let picked = picked
                .await
                .map_err(|error| error.to_string())
                .and_then(|picked| picked.map_err(|error| error.to_string()));
            this.update(cx, |tree, cx| match picked {
                Ok(Some(paths)) => paths.into_iter().for_each(|path| then(tree, path, cx)),
                Ok(None) => log::info!("tree: the folder panel was cancelled"),
                Err(error) => tree.failed("tree.error.panel", &error, cx),
            })
            .ok();
        })
        .detach();
    }

    pub fn on_open(&mut self, _: &commands::Open, _: &mut Window, cx: &mut Context<Self>) {
        self.pick(true, Tree::open_path, cx);
    }

    pub fn on_new(&mut self, _: &commands::New, _: &mut Window, cx: &mut Context<Self>) {
        self.pick(false, Tree::init_at, cx);
    }

    pub fn on_clone(&mut self, _: &CloneRepository, window: &mut Window, cx: &mut Context<Self>) {
        self.ask_url(window, cx);
    }

    pub fn on_close(&mut self, _: &CloseRepository, _: &mut Window, cx: &mut Context<Self>) {
        self.close_active(cx);
    }
}
