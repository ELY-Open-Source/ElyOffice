//! A save starts at the next typing pause (R15.37).

use std::time::{Duration, Instant};

use gpui::{App, Context, TaskExt as _, Window};

use crate::{Engine, Tabs, Waiting, in_window};

/// No key this long: typing paused.
const PAUSE: Duration = Duration::from_millis(500);
/// The longest a save waits for a pause.
const CAP: Duration = Duration::from_secs(2);

/// How long a save asked at `asked` still waits.
fn wait(typed: Option<Instant>, asked: Instant, now: Instant) -> Option<Duration> {
    let due = (typed? + PAUSE).min(asked + CAP);
    due.checked_duration_since(now).filter(|wait| !wait.is_zero())
}

fn due<H: Tabs>(host: &H, id: u64, asked: Instant, cx: &App) -> Option<Duration> {
    let typed = host.tab(id).and_then(|tab| H::Engine::typed_at(tab.view.read(cx)));
    wait(typed, asked, cx.background_executor().now())
}

/// Runs `then` once tab `id`'s typing pauses, `CAP` at most.
pub(crate) fn at_pause<H: Tabs>(host: &mut H, id: u64, then: Waiting<H>, window: &mut Window, cx: &mut Context<H>) {
    let asked = cx.background_executor().now();
    let Some(wait) = due(host, id, asked, cx) else {
        return then(host, window, cx);
    };
    log::info!("office: tab {id}'s save waits for a typing pause");
    if let Some(tab) = host.tab_mut(id) {
        tab.pausing += 1;
    }
    cx.notify();
    after(id, asked, wait, then, cx);
}

fn after<H: Tabs>(id: u64, asked: Instant, wait: Duration, then: Waiting<H>, cx: &mut Context<H>) {
    cx.spawn(async move |this, cx| {
        cx.background_executor().timer(wait).await;
        in_window(&this, cx, |host, window, cx| {
            if let Some(wait) = due(host, id, asked, cx) {
                return after(id, asked, wait, then, cx);
            }
            let waited = cx.background_executor().now() - asked;
            let why = if waited >= CAP { "typing went on" } else { "typing paused" };
            log::info!("office: tab {id}'s save starts {} ms after it was asked: {why}", waited.as_millis());
            if let Some(tab) = host.tab_mut(id) {
                tab.pausing -= 1;
            }
            then(host, window, cx)
        })
    })
    .detach_and_log_err(cx);
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{CAP, PAUSE, wait};

    #[test]
    fn a_save_waits_for_a_pause_and_no_longer_than_the_cap() {
        let asked = Instant::now();
        let ms = Duration::from_millis;
        assert_eq!(wait(None, asked, asked), None, "nothing typed: at once");
        assert_eq!(wait(Some(asked - PAUSE), asked, asked), None, "already paused");
        assert_eq!(wait(Some(asked - ms(100)), asked, asked), Some(PAUSE - ms(100)), "the rest of the pause");
        let late = asked + ms(1900);
        assert_eq!(wait(Some(late), asked, late), Some(ms(100)), "typing on: the cap");
        assert_eq!(wait(Some(asked + CAP), asked, asked + CAP), None, "the cap reached");
    }
}
