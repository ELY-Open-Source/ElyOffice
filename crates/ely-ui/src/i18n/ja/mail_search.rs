//! Japanese search of the mail held here, for ElyMail.

pub(super) const JA: &[(&str, &str)] = &[
    ("mail.search", "メールを検索"),
    ("mail.search.in", "検索対象"),
    ("mail.search.all", "すべて"),
    ("mail.search.from", "差出人"),
    ("mail.search.to", "宛先"),
    ("mail.search.subject", "件名"),
    ("mail.search.body", "本文"),
    ("mail.search.files", "添付ファイル名"),
    ("mail.search.any_time", "すべての日付"),
    ("mail.search.local", "このコンピューターに保存しているすべてのアカウントのメールを検索しました。"),
    ("mail.search.headers", "このコンピューターに保存しているすべてのアカウントのメールを検索しました。{count} 件はヘッダーしかないため、本文は検索していません。"),
    ("mail.search.none", "一致するメッセージはありません"),
];
