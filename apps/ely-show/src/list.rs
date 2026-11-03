//! The shown slide's objects, front last (SHOW-06).

use ely_gpui_component::{
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName},
};
use ely_lok::show::Object;
use ely_office::prelude::*;
use gpui::{App, Context, IntoElement, SharedString, WeakEntity};

use crate::deck::Deck;
use crate::design::{Panel, on_deck};

/// What the list calls `object`.
fn title(object: &Object, cx: &App) -> SharedString {
    if !object.name.is_empty() {
        return object.name.clone().into();
    }
    Deck::text(cx, &format!("show.kind.{}", object.kind), &[])
}

impl Panel {
    pub(crate) fn object_list(
        &self,
        objects: &[Object],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut list = SelectableList::new("show-objects")
            .label(Deck::text(cx, "show.objects", &[]))
            .multiple();
        let mut chosen = Vec::new();
        for (at, object) in objects.iter().enumerate() {
            let key = SharedString::from(at.to_string());
            let mut item = ListItem::new(
                SharedString::from(format!("show-object-{at}")),
                title(object, cx),
            );
            if !object.placeholder.is_empty() {
                let key = format!("show.placeholder.{}", object.placeholder);
                item = item.description(Deck::text(cx, &key, &[]));
            }
            if object.locked {
                item = item.trailing(Icon::new(IconName::Lock));
            }
            if object.selected {
                chosen.push(key.clone());
            }
            list = list.row(key, item);
        }
        let deck: WeakEntity<Deck> = self.deck.clone();
        list.selected(chosen).on_change(move |keys, _, cx| {
            let picked: Vec<String> = keys.iter().map(ToString::to_string).collect();
            on_deck(&deck, cx, |deck, cx| deck.select_objects(&picked, cx))
        })
    }
}

impl Deck {
    /// Selects the objects at `picked`, in order.
    pub fn select_objects(&mut self, picked: &[String], cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return;
        };
        if picked.is_empty() {
            self.run_in(Some(tab), "object.deselect", &["false"], cx);
        }
        let adds: Vec<[&str; 2]> =
            picked.iter().enumerate().map(|(n, at)| [at.as_str(), if n > 0 { "true" } else { "false" }]).collect();
        let commands: Vec<(&str, &[&str])> = adds.iter().map(|values| ("object.select", values.as_slice())).collect();
        self.run_all_in(tab, &commands, cx);
        self.force_reread(tab, cx);
    }
}
