//! Each engine stage's words, in the reader's language.

use ely_mail_engine::Stage;
use gpui::{App, SharedString};

use crate::mail::text;

/// The server went quiet during `stage`, in the reader's words.
pub fn quiet(stage: Stage, cx: &App) -> SharedString {
    let key = match stage {
        // Behind a proxy this is the step that stalls.
        Stage::StartingTls => return text(cx, "mail.error.timeout_tls", &[]),
        Stage::Connecting => "mail.stage.connecting",
        Stage::ConnectingThroughProxy => "mail.stage.proxy",
        Stage::WaitingForGreeting => "mail.stage.greeting_wait",
        Stage::Greeting => "mail.stage.greeting",
        Stage::LoggingIn => "mail.stage.login",
        Stage::LoggingOut => "mail.stage.logout",
        Stage::ListingMailboxes => "mail.stage.mailboxes",
        Stage::ReadingCapabilities => "mail.stage.capabilities",
        Stage::OpeningMailbox => "mail.stage.open",
        Stage::OpeningDrafts => "mail.stage.open_drafts",
        Stage::OpeningMailboxToChange => "mail.stage.open_change",
        Stage::SearchingByDate => "mail.stage.search_date",
        Stage::FindingMovedMessage => "mail.stage.find_moved",
        Stage::ReadingFlags => "mail.stage.flags",
        Stage::FetchingHeaders => "mail.stage.headers",
        Stage::FetchingMessages => "mail.stage.messages",
        Stage::ReadingStructures => "mail.stage.structures",
        Stage::FetchingMessageText => "mail.stage.text",
        Stage::CheckingMessages => "mail.stage.check",
        Stage::ChangingFlags => "mail.stage.change_flags",
        Stage::MovingMessages => "mail.stage.move",
        Stage::RemovingMessages => "mail.stage.remove",
        Stage::MarkingOldDraft => "mail.stage.mark_draft",
        Stage::RemovingOldDraft => "mail.stage.remove_draft",
        Stage::SavingDraft => "mail.stage.save_draft",
        Stage::SavingSentCopy => "mail.stage.save_sent",
        Stage::Sending => "mail.stage.send",
        Stage::WaitingToAccept => "mail.stage.accept",
        Stage::ListingMessages => "mail.stage.list",
        Stage::FetchingMessage => "mail.stage.fetch_one",
        Stage::DeletingMessage => "mail.stage.delete_one",
    };
    text(cx, "mail.error.timeout", &[("detail", &text(cx, key, &[]))])
}
