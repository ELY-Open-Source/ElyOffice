//! The style group: apply, create, update (DOC-04).

use ely_gpui_component::primitives::Tooltip;
use ely_gpui_component::{
    buttons::Button,
    data_display::PropertyGroup,
    forms::{Choice, Select},
    theme::ControlSize,
};
use ely_office::panel::{Panel as _, known, row, send};
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, div,
};

use super::Props;
use super::builtin::{self, Builtin, CHARACTER, PARAGRAPH};
use crate::docs::Docs;

impl Props {
    pub(super) fn style_group(&self, cx: &mut Context<Self>) -> PropertyGroup {
        let tip = |key: &str| Docs::text(cx, key, &[]);
        let (paragraphs, characters) = self
            .shown
            .as_ref()
            .map(|shown| (shown.paragraph.clone(), shown.character.clone()))
            .unwrap_or_default();
        let current = self.state(".uno:StyleApply", cx);
        let paragraph = self.style_menu(
            "doc-props-paragraph",
            paragraphs,
            current.clone(),
            ("ParagraphStyles", &PARAGRAPH),
            cx,
        );
        let character_style = self.state(".uno:CharStyle", cx);
        let character = self.style_menu(
            "doc-props-character",
            characters,
            character_style.clone(),
            ("CharacterStyles", &CHARACTER),
            cx,
        );
        PropertyGroup::new(tip("doc.props.style"))
            .row(
                tip("doc.props.paragraph"),
                row([
                    paragraph,
                    self.new_style("2", cx),
                    self.update_style("2", current, cx),
                ]),
            )
            .row(
                tip("doc.props.character"),
                row([
                    character,
                    self.new_style("1", cx),
                    self.update_style("1", character_style, cx),
                ]),
            )
    }

    /// A style list; unlisted or mixed names show as placeholder.
    fn style_menu(
        &self,
        id: &'static str,
        names: Vec<SharedString>,
        current: Option<SharedString>,
        (family, styles): (&'static str, &'static [Builtin]),
        cx: &Context<Self>,
    ) -> AnyElement {
        let this = cx.entity();
        let empty = names.is_empty();
        let current = current.map(|name| SharedString::from(builtin::listed(styles, &name).to_string()));
        let placeholder = match current.as_deref() {
            Some(ely_lok::MIXED) => Docs::text(cx, "office.props.mixed", &[]),
            Some(name) if known(name) => builtin::shown(styles, name, cx),
            _ => Docs::text(cx, "doc.props.choose", &[]),
        };
        let mut menu = Select::new(
            id,
            names.into_iter().map(|name| {
                let label = builtin::shown(styles, &name, cx);
                Choice::new(name, label)
            }),
        )
        .size(ControlSize::Sm)
        .disabled(empty)
        .placeholder(placeholder)
        .on_change(move |name, _, cx| {
            this.update(cx, |props, cx| {
                props.run("format.style", &[name.as_ref(), family], cx)
            })
        });
        if let Some(name) = current.filter(|name| known(name)) {
            menu = menu.selected(name);
        }
        menu.into_any_element()
    }

    fn new_style(&self, family: &'static str, cx: &Context<Self>) -> AnyElement {
        let this = cx.entity();
        Button::new(
            SharedString::from(format!("doc-props-new-style-{family}")),
            Docs::text(cx, "doc.props.new_style", &[]),
        )
        .size(ControlSize::Sm)
        .on_click(move |_, _, cx| {
            this.update(cx, |props, cx| {
                props.naming = Some(family);
                cx.notify();
            })
        })
        .into_any_element()
    }

    fn update_style(
        &self,
        family: &'static str,
        name: Option<SharedString>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let this = cx.entity();
        let id = format!("doc-props-update-style-{family}");
        let name = name.filter(|name| known(name));
        let button = Button::new(
            SharedString::from(id.clone()),
            Docs::text(cx, "doc.props.update_style", &[]),
        )
        .size(ControlSize::Sm)
        .disabled(name.is_none())
        .on_click({
            let name = name.clone();
            move |_, _, cx| {
                if let Some(name) = &name {
                    send(&this, "style.update", &[name.as_ref(), family], cx);
                }
            }
        });
        if name.is_some() {
            return button.into_any_element();
        }
        div()
            .id(SharedString::from(format!("{id}-why")))
            .child(button)
            .tooltip(Tooltip::text(Docs::text(cx, "doc.props.no_style", &[])))
            .into_any_element()
    }
}
