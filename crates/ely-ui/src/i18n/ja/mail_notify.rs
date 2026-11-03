//! Japanese notices of new mail, and its files, for ElyMail.

pub(super) const JA: &[(&str, &str)] = &[
    ("mail.notify.title", "新着メール"),
    ("mail.notify.body.one", "差出人: {senders}"),
    ("mail.notify.body", "{senders} から新着メッセージ {count} 件"),
    ("mail.notify.more", "{senders} ほか"),
    ("mail.file.downloading", "メッセージとファイルをダウンロードしています…"),
    ("mail.file.download_failed", "ファイルをダウンロードできませんでした: {error}"),
    ("mail.file.unread", "ファイルを読み取れませんでした: {error}"),
    ("mail.file.unsaved", "ファイルを保存できませんでした: {error}"),
    ("mail.file.saved", "{name} を保存しました"),
    ("mail.file.unopened", "ファイルを開けませんでした: {error}"),
    ("mail.file.unsafe", "この種類のファイルはプログラムを実行できるため、ここでは開きません。保存してから、差出人を信頼できる場合はご自身で開いてください。"),
];
