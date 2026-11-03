//! Japanese contacts for ElyMail.

pub(super) const JA: &[(&str, &str)] = &[
    ("mail.boxes.contacts", "連絡先"),
    ("mail.contacts.name", "名前"),
    ("mail.contacts.address", "メールアドレス"),
    ("mail.contacts.add", "連絡先を追加"),
    ("mail.contacts.remove", "連絡先を削除"),
    ("mail.contacts.import", "vCard をインポート…"),
    ("mail.contacts.export", "vCard をエクスポート…"),
    ("mail.contacts.empty", "連絡先はまだありません。メールを送った相手がここに追加されます。"),
    ("mail.contacts.unread", "連絡先を読み取れませんでした: {error}"),
    ("mail.contacts.unsaved", "連絡先を保存できませんでした: {error}"),
    ("mail.contacts.imported", "インポートしました: 追加 {added} 件、名前の変更 {renamed} 件、既存 {unchanged} 件。アドレスのないカード {skipped} 件"),
    ("mail.contacts.unimported", "vCard ファイルをインポートできませんでした: {error}"),
    ("mail.contacts.unexported", "連絡先をエクスポートできませんでした: {error}"),
];
