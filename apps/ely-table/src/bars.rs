//! The name box and the formula bar (TABLE-03).

use ely_gpui_component::{
    forms::{Input, InputEvent, TextInput},
    theme::{ActiveTheme as _, ControlSize},
};
use ely_lok::LokView;
use ely_office::prelude::*;
use gpui::{
    AnyElement, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    KeyDownEvent, ParentElement as _, Styled as _, Window, div,
};

use crate::book::Book;

pub struct Bars {
    pub name: Entity<TextInput>,
    pub formula: Entity<TextInput>,
    /// Tab, address and content the bars last took.
    shown: Option<(u64, String, String)>,
    /// A bar the user is typing in keeps its text.
    editing: Option<Bar>,
}

#[derive(Clone, Copy, PartialEq)]
enum Bar {
    Name,
    Formula,
}

impl Bars {
    pub fn new(window: &mut Window, cx: &mut Context<Book>) -> Bars {
        let (name, formula) = (
            Book::text(cx, "table.name_box", &[]),
            Book::text(cx, "table.formula", &[]),
        );
        let name = cx.new(|cx| TextInput::new(window, cx).label(name));
        let formula = cx.new(|cx| TextInput::new(window, cx).label(formula));
        for (input, bar) in [(&name, Bar::Name), (&formula, Bar::Formula)] {
            cx.subscribe_in(
                input,
                window,
                move |book, input, event, window, cx| match event {
                    InputEvent::Focus => book.bars.editing = Some(bar),
                    InputEvent::Blur => {
                        book.bars.editing = None;
                        book.bars.shown = None;
                        book.refill(cx);
                    }
                    InputEvent::Submit => {
                        let text = input.read(cx).text().to_string();
                        book.submit(bar, text, window, cx);
                    }
                    InputEvent::Changed => {}
                },
            )
            .detach();
        }
        Bars {
            name,
            formula,
            shown: None,
            editing: None,
        }
    }

    pub fn render(&self, cx: &mut Context<Book>) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .on_key_down(cx.listener(|book, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    book.leave_bars(window, cx);
                }
            }))
            .child(
                div()
                    .id("table-name-box")
                    .w_32()
                    .child(Input::new(&self.name).size(ControlSize::Sm)),
            )
            .child(
                div()
                    .id("table-formula")
                    .flex_1()
                    .child(Input::new(&self.formula).size(ControlSize::Sm)),
            )
            .into_any_element()
    }
}

impl Book {
    /// Shows the active cell, except in a bar being typed.
    pub fn sync_bars(&mut self, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            return;
        };
        let cell = view.read(cx).cell();
        let key = (id, cell.address.clone(), cell.formula.clone());
        if self.bars.shown.as_ref() == Some(&key) {
            return;
        }
        let editing = self.bars.editing;
        if editing != Some(Bar::Name) {
            self.bars
                .name
                .update(cx, |input, cx| input.set_text(key.1.clone(), cx));
        }
        if editing != Some(Bar::Formula) {
            self.bars
                .formula
                .update(cx, |input, cx| input.set_text(key.2.clone(), cx));
        }
        self.bars.shown = Some(key);
    }

    /// Shows the active tab's cell again.
    pub(crate) fn refill(&mut self, cx: &mut Context<Self>) {
        if let Some(view) = self.active_tab().map(|tab| tab.view.clone()) {
            self.sync_bars(&view, cx);
        }
    }

    /// Enter goes to the address or enters the content.
    fn submit(&mut self, bar: Bar, text: String, window: &mut Window, cx: &mut Context<Self>) {
        match bar {
            Bar::Name => self.run_active("cell.goto", &[&text], cx),
            Bar::Formula => self.run_active("cell.enter", &[&text], cx),
        }
        self.leave_bars(window, cx);
    }

    /// Escape or Enter hands the keys back to the grid.
    fn leave_bars(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.bars.editing = None;
        self.bars.shown = None;
        self.refill(cx);
        if let Some(tab) = self.active_tab() {
            let focus = tab.view.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
        }
    }
}
