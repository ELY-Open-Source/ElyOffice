//! What a conversation was doing when the server went quiet.

use std::fmt;

/// The step a timeout names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Connecting,
    ConnectingThroughProxy,
    StartingTls,
    WaitingForGreeting,
    Greeting,
    LoggingIn,
    LoggingOut,
    ListingMailboxes,
    ReadingCapabilities,
    OpeningMailbox,
    OpeningDrafts,
    OpeningMailboxToChange,
    SearchingByDate,
    FindingMovedMessage,
    ReadingFlags,
    FetchingHeaders,
    FetchingMessages,
    ReadingStructures,
    FetchingMessageText,
    CheckingMessages,
    ChangingFlags,
    MovingMessages,
    RemovingMessages,
    MarkingOldDraft,
    RemovingOldDraft,
    SavingDraft,
    SavingSentCopy,
    Sending,
    WaitingToAccept,
    ListingMessages,
    FetchingMessage,
    DeletingMessage,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Connecting => "connecting",
            Self::ConnectingThroughProxy => "connecting through the proxy",
            Self::StartingTls => "starting TLS",
            Self::WaitingForGreeting => "waiting for the greeting",
            Self::Greeting => "greeting",
            Self::LoggingIn => "logging in",
            Self::LoggingOut => "logging out",
            Self::ListingMailboxes => "listing mailboxes",
            Self::ReadingCapabilities => "reading capabilities",
            Self::OpeningMailbox => "opening the mailbox",
            Self::OpeningDrafts => "opening the drafts folder",
            Self::OpeningMailboxToChange => "opening the mailbox to change it",
            Self::SearchingByDate => "searching by date",
            Self::FindingMovedMessage => "finding a moved message",
            Self::ReadingFlags => "reading flags",
            Self::FetchingHeaders => "fetching headers",
            Self::FetchingMessages => "fetching messages",
            Self::ReadingStructures => "reading structures",
            Self::FetchingMessageText => "fetching message text",
            Self::CheckingMessages => "checking messages",
            Self::ChangingFlags => "changing flags",
            Self::MovingMessages => "moving messages",
            Self::RemovingMessages => "removing messages",
            Self::MarkingOldDraft => "marking the old draft",
            Self::RemovingOldDraft => "removing the old draft",
            Self::SavingDraft => "saving the draft",
            Self::SavingSentCopy => "saving the sent copy",
            Self::Sending => "sending",
            Self::WaitingToAccept => "waiting for the server to accept",
            Self::ListingMessages => "listing messages",
            Self::FetchingMessage => "fetching the message",
            Self::DeletingMessage => "deleting the message",
        })
    }
}
