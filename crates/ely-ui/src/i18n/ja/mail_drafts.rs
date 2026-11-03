//! Japanese ElyMail drafts, here and on the server.

pub(super) const JA: &[(&str, &str)] = &[
    ("mail.boxes.drafts", "下書き"),
    ("mail.drafts.pick", "表示する下書きを選んでください"),
    ("mail.drafts.to", "宛先"),
    ("mail.drafts.cc", "Cc"),
    ("mail.drafts.bcc", "Bcc"),
    ("mail.drafts.saved", "保存日時"),
    ("mail.drafts.continue", "続きを書く"),
    ("mail.drafts.delete", "下書きを削除…"),
    ("mail.drafts.kept", "メッセージは下書きに保存されています"),
    ("mail.drafts.unkept", "下書きを保存できませんでした: {error}"),
    ("mail.drafts.undropped", "下書きを削除できませんでした: {error}"),
    ("mail.drafts.unread", "下書きを読み取れませんでした: {error}"),
    ("mail.drafts.up", "サーバーの下書きフォルダーにも下書きを保存する"),
    ("mail.drafts.unsaved_choice", "下書きの設定を保存できませんでした: {error}"),
    ("mail.drafts.no_folder", "このコンピューターに保存しました。サーバーにはまだ下書きフォルダーがありません。"),
    ("mail.drafts.server_failed", "このコンピューターに保存しました。サーバーのコピーは保存できませんでした: {error}"),
    ("mail.drafts.server_stale", "サーバーの下書きに古いコピーが残っている可能性があります: {error}"),
];
