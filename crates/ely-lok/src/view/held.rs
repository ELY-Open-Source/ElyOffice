//! Input typed before the view existed (R15.57).

use std::rc::Rc;

use gpui::{App, Context, Div, InteractiveElement as _, KeyContext, Keystroke, Modifiers, Window};

use super::{
    LokView,
    input::{CONTEXT, Copy, Cut, NextField, Paste, PreviousField},
};

/// Makes `hold` keep the keys the view binds, for `replay`.
pub fn bound_keys(hold: Div, keep: impl Fn(Keystroke, &mut App) + 'static) -> Div {
    let keep = Rc::new(keep);
    let stroke = |modifiers: fn(&App) -> Modifiers, key: &'static str| {
        let keep = Rc::clone(&keep);
        move |cx: &mut App| keep(Keystroke { modifiers: modifiers(cx), key: key.into(), key_char: None }, cx)
    };
    let paste = stroke(Modifiers::secondary_key, "v");
    let (next, previous) = (stroke(|_| Modifiers::none(), "tab"), stroke(|_| Modifiers::shift(), "tab"));
    // Nothing is selected yet: copy and cut have nothing.
    hold.key_context(CONTEXT)
        .on_action(|_: &Copy, _, _| log::info!("lok: copy while the document loads copies nothing"))
        .on_action(|_: &Cut, _, _| log::info!("lok: cut while the document loads cuts nothing"))
        .on_action(move |_: &Paste, _, cx| paste(cx))
        .on_action(move |_: &NextField, _, cx| next(cx))
        .on_action(move |_: &PreviousField, _, cx| previous(cx))
}

impl LokView {
    /// Takes `keys` in order, as typed; text has no key.
    pub fn replay(&mut self, keys: Vec<Keystroke>, window: &mut Window, cx: &mut Context<Self>) {
        let mut context = KeyContext::default();
        context.add(CONTEXT);
        let context = [context];
        for key in keys {
            if key.key.is_empty() {
                if let Some(text) = &key.key_char {
                    self.commit(None, text);
                }
                continue;
            }
            // The view's own bindings, as its actions run them.
            let (bound, _) = cx.key_bindings().borrow().bindings_for_input(std::slice::from_ref(&key), &context);
            let action = bound.first().map(|binding| binding.action().boxed_clone());
            match action.as_ref().map(|action| action.as_any()) {
                Some(any) if any.is::<Paste>() => self.on_paste(&Paste, window, cx),
                Some(any) if any.is::<NextField>() => self.on_next_field(&NextField, window, cx),
                Some(any) if any.is::<PreviousField>() => self.on_previous_field(&PreviousField, window, cx),
                _ => {
                    if !self.walk(&key, cx) && !self.turn_key(&key) {
                        self.key_down(&key);
                    }
                }
            }
        }
        cx.notify();
    }
}
