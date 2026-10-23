use ely_gpui_component::i18n::I18n;

mod analysis;
mod calc;
mod data;
mod doc;
mod mail;
mod mail_drafts;
mod mail_organize;
mod mail_contacts;
mod mail_migrate;
mod mail_notify;
mod mail_storage;
mod mail_rules;
mod mail_search;
mod mail_outbox;
mod mail_write;
mod mark;
mod mark_menus;
mod motion;
mod office;
mod output;
mod pdf;
mod play;
mod refs;
mod secure;
mod settings;
mod show;
mod table;
mod tools;
mod tree;
mod tree_ops;
mod tree_places;
use gpui::{App, SharedString};

const EN: &[(&str, &str)] = &[
    ("menu.app.quit", "Quit {app}"),
    ("tray.open", "Open {app}"),
    ("updates.failed", "The update did not finish: {error}"),
    ("updates.latest", "{app} {version} is the latest version."),
    ("menu.app.updates", "Check for Updates…"),
    ("updates.package", "{app} {version} is out. Download its package from the release page, then run: {command}"),
    ("updates.copy", "Copy Command"),
    ("menu.app.switch", "Open Ely App"),
    ("toast.open", "Open"),
    ("toast.open_failed", "The file did not open: {error}"),
    ("menu.app.switch_failed", "{app} did not open: {error}"),
    ("menu.file", "File"),
    ("menu.insert", "Insert"),
    ("command.file.new", "New"),
    ("command.file.open", "Open…"),
    ("command.file.save", "Save"),
    ("command.file.save_as", "Save As…"),
    ("command.file.rename", "Rename…"),
    ("command.file.export", "Export…"),
    ("command.file.print", "Print…"),
    ("command.blocked.no_document", "No open document"),
    ("palette.group.file", "File"),
    ("gallery.ready", "Ready"),
    ("gallery.nav", "Navigation"),
    ("gallery.inbox", "Inbox"),
    ("gallery.archive", "Archive"),
    ("gallery.props", "Properties"),
    ("gallery.content", "Content"),
    ("gallery.mixed", "Mixed"),
    ("gallery.content_of", "Content of {page}"),
    ("gallery.rename", "Rename…"),
    ("gallery.delete", "Delete…"),
    ("gallery.group.selection", "Selection"),
    ("gallery.row.format", "Format"),
    ("gallery.row.pages", "Pages"),
    ("gallery.empty.pages", "No pages"),
    ("gallery.task.done", "Export done"),
    ("gallery.task.cancelled", "Export cancelled"),
    ("gallery.task.progress", "Export {state} · Cancel"),
    ("gallery.task.retry", "{error} · Retry"),
    ("gallery.error.disk_full", "The disk is full"),
    ("gallery.rename.title", "Rename Page"),
    ("gallery.rename.submit", "Rename"),
    ("gallery.rename.empty", "A page needs a name"),
    ("gallery.rename.placeholder", "Page name"),
    (
        "gallery.rename.failed",
        "The disk is full; the name was not saved",
    ),
    ("gallery.delete.title", "Delete {title}?"),
    (
        "gallery.delete.message",
        "{title} and its items are gone for good.",
    ),
    ("gallery.delete.confirm", "Delete Page"),
];

const ZH_CN: &[(&str, &str)] = &[
    ("menu.app.quit", "退出 {app}"),
    ("tray.open", "打开 {app}"),
    ("updates.failed", "更新没有完成：{error}"),
    ("updates.latest", "{app} {version} 已是最新版本。"),
    ("menu.app.updates", "检查更新…"),
    ("updates.package", "{app} {version} 已发布。从发布页下载其安装包后运行：{command}"),
    ("updates.copy", "复制命令"),
    ("menu.app.switch", "打开 Ely 应用"),
    ("toast.open", "打开"),
    ("toast.open_failed", "文件没能打开：{error}"),
    ("menu.app.switch_failed", "{app} 没能打开：{error}"),
    ("menu.file", "文件"),
    ("menu.insert", "插入"),
    ("command.file.new", "新建"),
    ("command.file.open", "打开…"),
    ("command.file.save", "保存"),
    ("command.file.save_as", "另存为…"),
    ("command.file.rename", "重命名…"),
    ("command.file.export", "导出…"),
    ("command.file.print", "打印…"),
    ("command.blocked.no_document", "没有打开的文档"),
    ("palette.group.file", "文件"),
    ("gallery.ready", "就绪"),
    ("gallery.nav", "导航"),
    ("gallery.inbox", "收件箱"),
    ("gallery.archive", "归档"),
    ("gallery.props", "属性"),
    ("gallery.content", "内容"),
    ("gallery.mixed", "混合"),
    ("gallery.content_of", "{page}的内容"),
    ("gallery.rename", "重命名…"),
    ("gallery.delete", "删除…"),
    ("gallery.group.selection", "所选"),
    ("gallery.row.format", "格式"),
    ("gallery.row.pages", "页数"),
    ("gallery.empty.pages", "没有页面"),
    ("gallery.task.done", "导出完成"),
    ("gallery.task.cancelled", "导出已取消"),
    ("gallery.task.progress", "导出 {state} · 取消"),
    ("gallery.task.retry", "{error} · 重试"),
    ("gallery.error.disk_full", "磁盘已满"),
    ("gallery.rename.title", "重命名页面"),
    ("gallery.rename.submit", "重命名"),
    ("gallery.rename.empty", "页面需要名称"),
    ("gallery.rename.placeholder", "页面名称"),
    ("gallery.rename.failed", "磁盘已满，名称未保存"),
    ("gallery.delete.title", "删除{title}？"),
    ("gallery.delete.message", "{title}及其内容将被永久删除。"),
    ("gallery.delete.confirm", "删除页面"),
];

/// zh-* maps to zh-CN; anything else to en-US.
pub fn resolve(tag: &str) -> &'static str {
    if tag.to_ascii_lowercase().starts_with("zh") {
        "zh-CN"
    } else {
        "en-US"
    }
}

/// Installs the catalogs and resolves the locale.
pub fn install(cx: &mut App) {
    let tag = locale_tag(cx);
    let locale = resolve(&tag);
    log::info!("i18n: locale {tag} -> {locale}");
    install_as(locale, cx);
}

/// Installs the catalogs speaking `locale` (en-US or zh-CN).
pub fn install_as(locale: &'static str, cx: &mut App) {
    #[cfg(target_os = "windows")]
    gpui_windows::set_text_language(locale);
    let (en, zh) = catalogs();
    cx.set_global(
        I18n::new(locale)
            .catalog("en-US", &en)
            .catalog("zh-CN", &zh),
    );
}

/// Keys and messages of one language.
type Catalog = Vec<(&'static str, &'static str)>;

/// The English and Chinese catalogs, every module's messages.
fn catalogs() -> (Catalog, Catalog) {
    let en: Vec<_> = EN
        .iter()
        .chain(office::EN)
        .chain(doc::EN)
        .chain(refs::EN)
        .chain(mail::EN)
        .chain(mail_write::EN)
        .chain(mail_drafts::EN)
        .chain(mail_outbox::EN)
        .chain(mail_organize::EN)
        .chain(mail_contacts::EN)
        .chain(mail_migrate::EN)
        .chain(mail_notify::EN)
        .chain(mail_storage::EN)
        .chain(mail_rules::EN)
        .chain(mail_search::EN)
        .chain(tools::EN)
        .chain(table::EN)
        .chain(calc::EN)
        .chain(data::EN)
        .chain(analysis::EN)
        .chain(output::EN)
        .chain(show::EN)
        .chain(motion::EN)
        .chain(play::EN)
        .chain(pdf::EN)
        .chain(secure::EN)
        .chain(tree::EN)
        .chain(tree_ops::EN)
        .chain(tree_places::EN)
        .chain(mark::EN)
        .chain(mark_menus::EN)
        .chain(settings::EN)
        .copied()
        .collect();
    let zh: Vec<_> = ZH_CN
        .iter()
        .chain(office::ZH_CN)
        .chain(doc::ZH_CN)
        .chain(refs::ZH_CN)
        .chain(mail::ZH_CN)
        .chain(mail_write::ZH_CN)
        .chain(mail_drafts::ZH_CN)
        .chain(mail_outbox::ZH_CN)
        .chain(mail_organize::ZH_CN)
        .chain(mail_contacts::ZH_CN)
        .chain(mail_migrate::ZH_CN)
        .chain(mail_notify::ZH_CN)
        .chain(mail_storage::ZH_CN)
        .chain(mail_rules::ZH_CN)
        .chain(mail_search::ZH_CN)
        .chain(tools::ZH_CN)
        .chain(table::ZH_CN)
        .chain(calc::ZH_CN)
        .chain(data::ZH_CN)
        .chain(analysis::ZH_CN)
        .chain(output::ZH_CN)
        .chain(show::ZH_CN)
        .chain(motion::ZH_CN)
        .chain(play::ZH_CN)
        .chain(pdf::ZH_CN)
        .chain(secure::ZH_CN)
        .chain(tree::ZH_CN)
        .chain(tree_ops::ZH_CN)
        .chain(tree_places::ZH_CN)
        .chain(mark::ZH_CN)
        .chain(mark_menus::ZH_CN)
        .chain(settings::ZH_CN)
        .copied()
        .collect();
    (en, zh)
}

/// `ELY_LOCALE`, then the language setting, then the system's.
fn locale_tag(cx: &App) -> String {
    use ely_platform::settings::{Language, word};
    match crate::settings::current(cx).language {
        Language::System => ely_platform::locale::system(),
        _ if ely_platform::env::value("ELY_LOCALE").is_some() => ely_platform::locale::system(),
        chosen => word(&Language::ALL, &chosen).into(),
    }
}

pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
    cx.global::<I18n>().text(key, args)
}

/// The message for `n` things: `key.one` when one, else `key`.
pub fn counted(cx: &App, key: &str, n: u64, args: &[(&str, &str)]) -> SharedString {
    text(cx, &count_key(key, n), args)
}

/// Words and characters; with a selection, its words.
pub fn word_count(cx: &App, words: u64, characters: u64, selected: Option<u64>) -> SharedString {
    let count = |key, n: u64| counted(cx, key, n, &[("n", &n.to_string())]);
    let Some(selected) = selected else {
        let (words, characters) = (count("count.word", words), count("count.character", characters));
        return text(cx, "count.words", &[("words", &words), ("characters", &characters)]);
    };
    let args = [("selected", selected.to_string()), ("words", words.to_string())];
    let args: Vec<_> = args.iter().map(|(key, value)| (*key, value.as_str())).collect();
    counted(cx, "count.selected", words, &args)
}

fn count_key(key: &str, n: u64) -> String {
    match n {
        1 => format!("{key}.one"),
        _ => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{catalogs, count_key, resolve};
    use std::collections::BTreeSet;

    #[test]
    fn one_thing_takes_the_singular() {
        assert_eq!(count_key("tree.changed", 1), "tree.changed.one");
        for n in [0, 2, 21] {
            assert_eq!(count_key("tree.changed", n), "tree.changed", "{n}");
        }
    }

    #[gpui::test]
    fn counted_reads_one_and_many_in_both_languages(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            super::install_as("en-US", cx);
            let said = |n: u64, cx: &gpui::App| super::counted(cx, "tree.changed", n, &[("n", &n.to_string())]);
            assert_eq!((said(1, cx), said(2, cx)), ("1 file changed".into(), "2 files changed".into()));
            super::install_as("zh-CN", cx);
            assert_eq!((said(1, cx), said(2, cx)), ("1 个文件有改动".into(), "2 个文件有改动".into()));
        });
    }

    #[test]
    fn every_english_singular_reads_unlike_its_plural_at_one() {
        let (en, _) = catalogs();
        let text = |key: &str| en.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).expect("the key");
        for (key, one) in en.iter().filter(|(key, _)| key.ends_with(".one")) {
            let many = text(key.trim_end_matches(".one")).replace("{n}", "1").replace("{count}", "1");
            assert_ne!(*one, many, "{key} reads like its plural");
            assert!(!one.contains("{n}") && !one.contains("{count}"), "{key} names its one");
        }
    }

    #[test]
    fn both_catalogs_hold_the_same_keys_and_singulars() {
        let (en, zh) = catalogs();
        let keys = |list: &[(&str, &str)]| list.iter().map(|(key, _)| key.to_string()).collect::<BTreeSet<_>>();
        let (en, zh) = (keys(&en), keys(&zh));
        assert_eq!(en.symmetric_difference(&zh).collect::<Vec<_>>(), Vec::<&String>::new());
        for key in en.iter().filter_map(|key| key.strip_suffix(".one")) {
            assert!(en.contains(key), "{key}.one has no plural");
        }
    }

    #[test]
    fn zh_variants_pick_zh_cn_others_pick_en() {
        for tag in ["zh-CN", "zh-Hans", "zh", "ZH-TW"] {
            assert_eq!(resolve(tag), "zh-CN", "{tag}");
        }
        for tag in ["en", "en-US", "fr", "ja", "", "x9!"] {
            assert_eq!(resolve(tag), "en-US", "{tag}");
        }
    }
}
