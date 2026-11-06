//! The toolbar above the page (GitHub #67).

use ely_gpui_component::{
    buttons::IconButton,
    primitives::IconName,
    shell::{Toolbar, ToolbarGroup, ToolbarSeparator},
    theme::{ActiveTheme as _, ControlSize},
};
use ely_office::panel::{font_box, size_box, toggle};
use gpui::{
    AnyElement, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, rems,
};

use super::Props;
use crate::docs::Docs;

/// Word's Home row: the panel's commands, one click away.
pub struct Bar {
    props: Entity<Props>,
    _props: Subscription,
}

impl Bar {
    pub fn new(props: Entity<Props>, cx: &mut Context<Self>) -> Self {
        let observed = cx.observe(&props, |_, _, cx| cx.notify());
        Bar { props, _props: observed }
    }
}

impl Render for Bar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.props.update(cx, |props, cx| props.toolbar(window, cx))
    }
}

impl Props {
    fn toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(shown) = &self.shown else {
            return div().into_any_element();
        };
        let (undo, redo) = ely_office::undo_able(shown.view.read(cx));
        let why = "doc.why.selection";
        let mark = |props: &Self, command: &'static str, icon, tip, cx: &Context<Self>| {
            let button = toggle(props, command, (Some(icon), ""), (tip, why), cx);
            div().debug_selector(move || format!("doc-bar {command}")).child(button)
        };
        let history = ToolbarGroup::new()
            .child(history(("doc-bar-undo", IconName::Undo2, "office.undo"), undo, ely_office::Undo, cx))
            .child(history(("doc-bar-redo", IconName::Redo2, "office.redo"), redo, ely_office::Redo, cx));
        let name = Docs::text(cx, "doc.props.font", &[]);
        let font = font_box(self, ("doc-bar-font", name), &self.fonts, window, cx);
        let size = size_box(self, ("doc-bar-size", Docs::text(cx, "doc.props.size", &[])), window, cx);
        let text = ToolbarGroup::new()
            .child(div().w(rems(10.)).child(font))
            .child(div().w(rems(4.5)).child(size));
        let marks = ToolbarGroup::new()
            .child(mark(self, "format.bold", IconName::Bold, "doc.props.bold", cx))
            .child(mark(self, "format.italic", IconName::Italic, "doc.props.italic", cx))
            .child(mark(self, "format.underline", IconName::Underline, "doc.props.underline", cx))
            .child(mark(self, "format.strikeout", IconName::Strikethrough, "doc.props.strikeout", cx));
        let align = ToolbarGroup::new()
            .child(mark(self, "paragraph.left", IconName::AlignLeft, "doc.props.left", cx))
            .child(mark(self, "paragraph.center", IconName::AlignCenter, "doc.props.center", cx))
            .child(mark(self, "paragraph.right", IconName::AlignRight, "doc.props.right", cx))
            .child(mark(self, "paragraph.justify", IconName::AlignJustify, "doc.props.justify", cx));
        let lists = ToolbarGroup::new()
            .child(mark(self, "list.bullets", IconName::List, "doc.props.bullets", cx))
            .child(mark(self, "list.numbering", IconName::ListOrdered, "doc.props.numbering", cx));
        let bar = Toolbar::new()
            .child(history)
            .child(ToolbarSeparator)
            .child(text)
            .child(ToolbarSeparator)
            .child(marks)
            .child(ToolbarSeparator)
            .child(align)
            .child(ToolbarSeparator)
            .child(lists);
        // Narrow windows scroll it sideways.
        div()
            .id("doc-bar")
            .debug_selector(|| "doc-bar".into())
            .overflow_x_scroll()
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .child(bar)
            .into_any_element()
    }
}

/// Undo or Redo, as the Edit menu runs them.
fn history<A: gpui::Action + Clone>(
    (id, icon, tip): (&'static str, IconName, &str),
    able: bool,
    action: A,
    cx: &Context<Props>,
) -> impl IntoElement {
    let button = IconButton::new(id, icon)
        .size(ControlSize::Sm)
        .tooltip(Docs::text(cx, tip, &[]))
        .disabled(!able)
        .on_click(move |_, window, cx| window.dispatch_action(Box::new(action.clone()), cx));
    div().debug_selector(move || id.into()).child(button)
}
