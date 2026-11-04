use ely_gpui_component::i18n::I18n;

mod analysis;
mod de;
#[cfg(test)]
mod german_tests;
mod es;
mod ko;
#[cfg(test)]
mod korean_tests;
mod fr;
#[cfg(test)]
mod french_tests;
#[cfg(test)]
mod spanish_tests;
mod ja;
#[cfg(test)]
mod japanese_tests;
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
mod mail_stages;
mod mail_outbox;
mod mail_write;
mod mark;
mod mark_menus;
mod motion;
mod office;
mod output;
mod pdf;
mod pdf_steps;
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
mod vi;
#[cfg(test)]
mod vietnamese_tests;
use ely_ipc::{Then, Worded};
use ely_platform::settings::{LANGUAGES, Language};
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
    ("menu.edit", "Edit"),
    ("menu.view", "View"),
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
    ("error.no_documents", "there is no Documents folder"),
    ("error.no_home", "there is no home folder"),
    ("error.no_store", "the mail store is not open"),
    ("error.closed", "the document closed"),
    ("error.open_there", "an open document is there"),
    ("error.no_pixels", "the picture has no pixels"),
    ("error.then", "{error}; removing the files already written failed too: {left}"),
    ("error.no_key", "the recovery key is not in the key store"),
    ("error.not_installed", "{app} is not installed"),
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
    ("menu.edit", "编辑"),
    ("menu.view", "视图"),
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
    ("error.no_documents", "没有“文档”文件夹"),
    ("error.no_home", "没有主文件夹"),
    ("error.no_store", "邮件存储没有打开"),
    ("error.closed", "文档已关闭"),
    ("error.open_there", "那里有一个打开着的文档"),
    ("error.no_pixels", "图片没有像素"),
    ("error.then", "{error}；已写出的文件也没能删除：{left}"),
    ("error.no_key", "密钥存储里没有恢复密钥"),
    ("error.not_installed", "{app} 没有安装"),
];

/// The listed language of `tag`'s primary subtag; else en-US.
pub fn resolve(tag: &str) -> &'static str {
    let primary = |tag: &str| tag.split(['-', '_', '.', '@']).next().unwrap_or_default().to_ascii_lowercase();
    LANGUAGES.iter().find(|(listed, _)| primary(listed) == primary(tag)).map_or("en-US", |(listed, _)| listed)
}

/// Installs the catalogs and resolves the locale.
pub fn install(cx: &mut App) {
    let tag = locale_tag(cx);
    let locale = resolve(&tag);
    log::info!("i18n: locale {tag} -> {locale}");
    install_as(locale, cx);
}

/// Installs the catalogs speaking `locale`, a listed tag.
pub fn install_as(locale: &'static str, cx: &mut App) {
    #[cfg(target_os = "windows")]
    gpui_windows::set_text_language(locale);
    let [en, zh, es, ja, ko, vi, fr, de] = catalogs();
    cx.set_global(
        I18n::new(locale)
            .catalog("en-US", &en)
            .catalog("zh-CN", &zh)
            .catalog("es-ES", &es)
            .catalog("ja-JP", &ja)
            .catalog("ko-KR", &ko)
            .catalog("vi-VN", &vi)
            .catalog("fr-FR", &fr)
            .catalog("de-DE", &de),
    );
}

/// Keys and messages of one language.
type Catalog = Vec<(&'static str, &'static str)>;

const EN_MODULES: [&[(&str, &str)]; 34] = [
    EN,
    office::EN,
    doc::EN,
    refs::EN,
    mail::EN,
    mail_write::EN,
    mail_drafts::EN,
    mail_outbox::EN,
    mail_organize::EN,
    mail_contacts::EN,
    mail_migrate::EN,
    mail_notify::EN,
    mail_storage::EN,
    mail_rules::EN,
    mail_search::EN,
    mail_stages::EN,
    tools::EN,
    table::EN,
    calc::EN,
    data::EN,
    analysis::EN,
    output::EN,
    show::EN,
    motion::EN,
    play::EN,
    pdf::EN,
    pdf_steps::EN,
    secure::EN,
    tree::EN,
    tree_ops::EN,
    tree_places::EN,
    mark::EN,
    mark_menus::EN,
    settings::EN,
];

const ZH_MODULES: [&[(&str, &str)]; 34] = [
    ZH_CN,
    office::ZH_CN,
    doc::ZH_CN,
    refs::ZH_CN,
    mail::ZH_CN,
    mail_write::ZH_CN,
    mail_drafts::ZH_CN,
    mail_outbox::ZH_CN,
    mail_organize::ZH_CN,
    mail_contacts::ZH_CN,
    mail_migrate::ZH_CN,
    mail_notify::ZH_CN,
    mail_storage::ZH_CN,
    mail_rules::ZH_CN,
    mail_search::ZH_CN,
    mail_stages::ZH_CN,
    tools::ZH_CN,
    table::ZH_CN,
    calc::ZH_CN,
    data::ZH_CN,
    analysis::ZH_CN,
    output::ZH_CN,
    show::ZH_CN,
    motion::ZH_CN,
    play::ZH_CN,
    pdf::ZH_CN,
    pdf_steps::ZH_CN,
    secure::ZH_CN,
    tree::ZH_CN,
    tree_ops::ZH_CN,
    tree_places::ZH_CN,
    mark::ZH_CN,
    mark_menus::ZH_CN,
    settings::ZH_CN,
];

/// Every language's catalog, every module's messages.
fn catalogs() -> [Catalog; 8] {
    [EN_MODULES, ZH_MODULES, es::ES, ja::JA, ko::KO, vi::VI, fr::FR, de::DE].map(|modules| modules.iter().flat_map(|module| module.iter().copied()).collect())
}

/// `ELY_LOCALE`, then the language setting, then the system's.
fn locale_tag(cx: &App) -> String {
    match crate::settings::current(cx).language {
        Language::System => ely_platform::locale::system(),
        _ if ely_platform::env::value("ELY_LOCALE").is_some() => ely_platform::locale::system(),
        Language::Listed(tag) => tag.into(),
    }
}

pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
    cx.global::<I18n>().text(key, args)
}

/// `error` in the window's language when Ely worded it.
pub fn error(cx: &App, error: &std::io::Error) -> SharedString {
    if let Some(then) = error.get_ref().and_then(|inner| inner.downcast_ref::<Then>()) {
        let (first, left) = (self::error(cx, &then.error), self::error(cx, &then.left));
        return text(cx, "error.then", &[("error", &first), ("left", &left)]);
    }
    let Some(said) = error.get_ref().and_then(|inner| inner.downcast_ref::<Worded>()) else {
        return error.to_string().into();
    };
    let args: Vec<_> = said.args.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
    text(cx, said.key, &args)
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
    use super::{I18n, LANGUAGES, catalogs, count_key, resolve};
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

    #[gpui::test]
    fn errors_ely_words_read_in_each_language(cx: &mut gpui::TestAppContext) {
        use std::io;
        let locked = super::Worded::error(io::ErrorKind::PermissionDenied, "error.no_key", &[]);
        let pixels = || super::Worded::error(io::ErrorKind::InvalidData, "error.no_pixels", &[]);
        let then = super::Worded::then(pixels(), io::Error::other("B"));
        cx.update(|cx| {
            for (tag, key, both) in [
                ("zh-CN", "密钥存储里没有恢复密钥", "图片没有像素；已写出的文件也没能删除：B"),
                ("es-ES", "la clave de recuperación no está en el almacén de claves", "la imagen no tiene píxeles; tampoco se pudieron eliminar los archivos ya escritos: B"),
            ] {
                super::install_as(tag, cx);
                assert_eq!((super::error(cx, &locked), super::error(cx, &then)), (key.into(), both.into()), "{tag}");
                assert_eq!(super::error(cx, &io::Error::other("the system's")), "the system's", "{tag}");
            }
        });
        assert_eq!((locked.to_string(), then.to_string()), ("error.no_key".into(), "error.then".into()), "logs name the key");
    }

    #[test]
    fn every_english_singular_reads_unlike_its_plural_at_one() {
        let [en, ..] = catalogs();
        let text = |key: &str| en.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).expect("the key");
        for (key, one) in en.iter().filter(|(key, _)| key.ends_with(".one")) {
            let many = text(key.trim_end_matches(".one")).replace("{n}", "1").replace("{count}", "1");
            assert_ne!(*one, many, "{key} reads like its plural");
            assert!(!one.contains("{n}") && !one.contains("{count}"), "{key} names its one");
        }
    }

    #[test]
    fn every_catalog_holds_the_same_keys_and_singulars() {
        let keys = |list: &[(&str, &str)]| list.iter().map(|(key, _)| key.to_string()).collect::<BTreeSet<_>>();
        let [en, others @ ..] = catalogs().map(|catalog| keys(&catalog));
        for other in others {
            assert_eq!(en.symmetric_difference(&other).collect::<Vec<_>>(), Vec::<&String>::new());
        }
        for key in en.iter().filter_map(|key| key.strip_suffix(".one")) {
            assert!(en.contains(key), "{key}.one has no plural");
        }
    }

    #[test]
    fn zh_variants_pick_zh_cn_others_pick_en() {
        for tag in ["zh-CN", "zh-Hans", "zh", "ZH-TW", "zh_CN"] {
            assert_eq!(resolve(tag), "zh-CN", "{tag}");
        }
        for tag in ["en", "en-US", "it", "", "x9!", "zhx"] {
            assert_eq!(resolve(tag), "en-US", "{tag}");
        }
    }

    #[gpui::test]
    fn every_listed_language_has_its_catalog(cx: &mut gpui::TestAppContext) {
        let primaries: BTreeSet<_> = LANGUAGES.iter().map(|(tag, _)| tag.split('-').next()).collect();
        assert_eq!(primaries.len(), LANGUAGES.len(), "the system's language picks one");
        cx.update(|cx| {
            for (tag, _) in LANGUAGES {
                assert_eq!(resolve(tag), tag);
                assert_eq!(resolve(tag.split('-').next().unwrap()), tag);
                super::install_as(tag, cx);
                assert_eq!(cx.global::<I18n>().locale().tag, tag);
                super::text(cx, "settings.language", &[]);
            }
        });
    }
}
