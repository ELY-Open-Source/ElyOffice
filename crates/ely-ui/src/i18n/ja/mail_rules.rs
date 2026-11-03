//! Japanese local rules for ElyMail.

pub(super) const JA: &[(&str, &str)] = &[
    ("mail.rules.field", "対象"),
    ("mail.rules.from", "差出人"),
    ("mail.rules.to", "宛先または Cc"),
    ("mail.rules.subject", "件名"),
    ("mail.rules.words", "探す語句"),
    ("mail.rules.action", "処理"),
    ("mail.rules.flag", "フラグを付ける"),
    ("mail.rules.archive", "アーカイブする"),
    ("mail.rules.move", "{name} に移動する"),
    ("mail.rules.add", "ルールを追加"),
    ("mail.rules.remove", "ルールを削除"),
    ("mail.rules.said", "{field} に「{words}」を含む新着メール: {action}"),
    ("mail.rules.unsaved", "ルールを保存できませんでした: {error}"),
    ("mail.rules.unread", "ルールを読み取れませんでした: {error}"),
    ("mail.rules.refused", "ルールを実行できませんでした: {error}"),
];
