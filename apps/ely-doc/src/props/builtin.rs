//! Writer's built-in style names in the interface language (I-03).

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

/// Kernel, English, Chinese, Spanish and French names.
pub(super) type Builtin = (&'static str, &'static str, &'static str, &'static str, &'static str);

/// The character default: outside the pool, never suffixed.
const NO_CHARACTER_STYLE: Builtin = ("Standard", "No Character Style", "无字符样式", "Ningún estilo de carácter", "Aucun style de caractère");

/// A clashing user name carries this, as in LibreOffice.
const USER: &str = " (user)";

/// `name` less a user suffix, as LibreOffice drops it.
fn unmarked(name: &str) -> &str {
    match name.strip_suffix(USER) {
        Some(rest) if name.encode_utf16().count() > 8 => rest,
        _ => name,
    }
}

/// The list name of a state name, as LibreOffice's FillProgName.
pub(super) fn listed(styles: &[Builtin], name: &str) -> String {
    if let Some((kernel, ..)) = styles.iter().find(|(_, english, ..)| *english == name) {
        return kernel.to_string();
    }
    if styles.iter().any(|row| row.0 == name && *row != NO_CHARACTER_STYLE) {
        return format!("{name}{USER}");
    }
    unmarked(name).to_string()
}

/// How a list name reads, as LibreOffice's FillUIName.
pub(super) fn shown(styles: &[Builtin], name: &str, cx: &App) -> SharedString {
    let locale = cx.global::<I18n>().locale().tag;
    if let Some((_, english, zh, es, fr)) = styles.iter().find(|(kernel, ..)| *kernel == name) {
        return match locale { "zh-CN" => *zh, "es-ES" => *es, "fr-FR" => *fr, _ => *english }.into();
    }
    if styles.iter().any(|row| row.1 == name && *row != NO_CHARACTER_STYLE) {
        return format!("{name}{USER}").into();
    }
    unmarked(name).to_string().into()
}

// From LibreOffice 26.2.6: SwStyleNameMapper, strings.hrc, zh-CN, es, fr po.
pub(super) const PARAGRAPH: [Builtin; 126] = [
    ("Standard", "Default Paragraph Style", "默认段落样式", "Estilo de párrafo predeterminado", "Style de paragraphe par défaut"),
    ("Text body", "Body Text", "正文", "Cuerpo de texto", "Corps de texte"),
    ("First line indent", "First Line Indent", "首行缩进", "Sangría de primer renglón", "Retrait de première ligne"),
    ("Hanging indent", "Hanging Indent", "悬挂缩进", "Sangría francesa", "Alinéa négatif"),
    ("Text body indent", "Body Text, Indented", "带缩进的正文", "Cuerpo de texto con sangría", "Corps de texte avec retrait"),
    ("Salutation", "Complimentary Close", "结尾敬语", "Frase de despedida", "Formule finale"),
    ("Signature", "Signature", "签名", "Firma", "Signature"),
    ("List Indent", "List Indent", "列表缩进", "Confrontación", "Retrait de liste"),
    ("Marginalia", "Marginalia", "旁注", "Nota al margen", "Note en marge"),
    ("List", "List", "列表", "Lista", "Liste"),
    ("Numbering 1 Start", "Numbering 1 Start", "编号 1 起始", "Numeración 1 inicio", "Numérotation 1 début"),
    ("Numbering 1", "Numbering 1", "编号 1", "Numeración 1", "Numérotation 1"),
    ("Numbering 1 End", "Numbering 1 End", "编号 1 结束", "Numeración 1 fin", "Numérotation 1 fin"),
    ("Numbering 1 Cont.", "Numbering 1 Cont.", "编号 1 续", "Numeración 1 continuación", "Numérotation 1 suite"),
    ("Numbering 2 Start", "Numbering 2 Start", "编号 2 起始", "Numeración 2 inicio", "Numérotation 2 début"),
    ("Numbering 2", "Numbering 2", "编号 2", "Numeración 2", "Numérotation 2"),
    ("Numbering 2 End", "Numbering 2 End", "编号 2 结束", "Numeración 2 fin", "Numérotation 2 fin"),
    ("Numbering 2 Cont.", "Numbering 2 Cont.", "编号 2 续", "Numeración 2 continuación", "Numérotation 2 suite"),
    ("Numbering 3 Start", "Numbering 3 Start", "编号 3 起始", "Numeración 3 inicio", "Numérotation 3 début"),
    ("Numbering 3", "Numbering 3", "编号 3", "Numeración 3", "Numérotation 3"),
    ("Numbering 3 End", "Numbering 3 End", "编号 3 结束", "Numeración 3 fin", "Numérotation 3 fin"),
    ("Numbering 3 Cont.", "Numbering 3 Cont.", "编号 3 续", "Numeración 3 continuación", "Numérotation 3 suite"),
    ("Numbering 4 Start", "Numbering 4 Start", "编号 4 起始", "Numeración 4 inicio", "Numérotation 4 début"),
    ("Numbering 4", "Numbering 4", "编号 4", "Numeración 4", "Numérotation 4"),
    ("Numbering 4 End", "Numbering 4 End", "编号 4 结束", "Numeración 4 fin", "Numérotation 4 fin"),
    ("Numbering 4 Cont.", "Numbering 4 Cont.", "编号 4 续", "Numeración 4 continuación", "Numérotation 4 suite"),
    ("Numbering 5 Start", "Numbering 5 Start", "编号 5 起始", "Numeración 5 inicio", "Numérotation 5 début"),
    ("Numbering 5", "Numbering 5", "编号 5", "Numeración 5", "Numérotation 5"),
    ("Numbering 5 End", "Numbering 5 End", "编号 5 结束", "Numeración 5 fin", "Numérotation 5 fin"),
    ("Numbering 5 Cont.", "Numbering 5 Cont.", "编号 5 续", "Numeración 5 continuación", "Numérotation 5 suite"),
    ("List 1 Start", "List 1 Start", "列表 1 起始", "Lista 1 inicio", "Liste 1 début"),
    ("List 1", "List 1", "列表 1", "Lista 1", "Liste 1"),
    ("List 1 End", "List 1 End", "列表 1 结束", "Lista 1 fin", "Liste 1 fin"),
    ("List 1 Cont.", "List 1 Cont.", "列表 1 续", "Lista 1 continuación", "Liste 1 suite"),
    ("List 2 Start", "List 2 Start", "列表 2 起始", "Lista 2 inicio", "Liste 2 début"),
    ("List 2", "List 2", "列表 2", "Lista 2", "Liste 2"),
    ("List 2 End", "List 2 End", "列表 2 结束", "Lista 2 fin", "Liste 2 fin"),
    ("List 2 Cont.", "List 2 Cont.", "列表 2 续", "Lista 2 continuación", "Liste 2 suite"),
    ("List 3 Start", "List 3 Start", "列表 3 起始", "Lista 3 inicio", "Liste 3 début"),
    ("List 3", "List 3", "列表 3", "Lista 3", "Liste 3"),
    ("List 3 End", "List 3 End", "列表 3 结束", "Lista 3 fin", "Liste 3 fin"),
    ("List 3 Cont.", "List 3 Cont.", "列表 3 续", "Lista 3 continuación", "Liste 3 suite"),
    ("List 4 Start", "List 4 Start", "列表 4 起始", "Lista 4 inicio", "Liste 4 début"),
    ("List 4", "List 4", "列表 4", "Lista 4", "Liste 4"),
    ("List 4 End", "List 4 End", "列表 4 结束", "Lista 4 fin", "Liste 4 fin"),
    ("List 4 Cont.", "List 4 Cont.", "列表 4 续", "Lista 4 continuación", "Liste 4 suite"),
    ("List 5 Start", "List 5 Start", "列表 5 起始", "Lista 5 inicio", "Liste 5 début"),
    ("List 5", "List 5", "列表 5", "Lista 5", "Liste 5"),
    ("List 5 End", "List 5 End", "列表 5 结束", "Lista 5 fin", "Liste 5 fin"),
    ("List 5 Cont.", "List 5 Cont.", "列表 5 续", "Lista 5 continuación", "Liste 5 suite"),
    ("Header and Footer", "Header and Footer", "页眉与页脚", "Cabecera y pie", "En-tête et pied de page"),
    ("Header", "Header", "页眉", "Cabecera", "En-tête"),
    ("Header left", "Header Left", "页眉左侧", "Cabecera izquierda", "En-tête gauche"),
    ("Header right", "Header Right", "页眉右侧", "Cabecera derecha", "En-tête droit"),
    ("Footer", "Footer", "页脚", "Pie de página", "Pied de page"),
    ("Footer left", "Footer Left", "页脚左侧", "Pie de página izquierdo", "Pied de page gauche"),
    ("Footer right", "Footer Right", "页脚右侧", "Pie de página derecho", "Pied de page droit"),
    ("Table Contents", "Table Contents", "表格内容", "Contenido de la tabla", "Contenu de tableau"),
    ("Table Heading", "Table Heading", "表格标题", "Título de la tabla", "Titre de tableau"),
    ("Caption", "Caption", "题注", "Leyenda", "Légende"),
    ("Illustration", "Illustration", "插图", "Ilustración", "Illustration"),
    ("Table", "Table", "表格", "Tabla", "Tableau"),
    ("Text", "Text", "文字", "Texto", "Texte"),
    ("Figure", "Figure", "图示", "Figura", "Figure"),
    ("Frame contents", "Frame Contents", "框架内容", "Contenido del marco", "Contenu de cadre"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "Note de bas de page"),
    ("Addressee", "Addressee", "收件人", "Destinatario", "Destinataire"),
    ("Sender", "Sender", "发件人", "Remitente", "Expéditeur"),
    ("Endnote", "Endnote", "尾注", "Nota final", "Note de fin"),
    ("Drawing", "Drawing", "绘图", "Dibujo", "Dessin"),
    ("Comment", "Comment", "批注", "Comentario", "Commentaire"),
    ("Index", "Index", "索引", "Índice", "Index"),
    ("Index Heading", "Index Heading", "索引标题", "Título del índice", "Index lexical : titre"),
    ("Index 1", "Index 1", "索引 1", "Índice 1", "Index lexical 1"),
    ("Index 2", "Index 2", "索引 2", "Índice 2", "Index lexical 2"),
    ("Index 3", "Index 3", "索引 3", "Índice 3", "Index lexical 3"),
    ("Index Separator", "Index Separator", "索引分隔符", "Separador del índice", "Index lexical : séparateur"),
    ("Contents Heading", "Contents Heading", "目录标题", "Título del sumario", "Titre de table des matières"),
    ("Contents 1", "Contents 1", "目录 1", "Sumario 1", "Table des matières niveau 1"),
    ("Contents 2", "Contents 2", "目录 2", "Sumario 2", "Table des matières niveau 2"),
    ("Contents 3", "Contents 3", "目录 3", "Sumario 3", "Table des matières niveau 3"),
    ("Contents 4", "Contents 4", "目录 4", "Sumario 4", "Table des matières niveau 4"),
    ("Contents 5", "Contents 5", "目录 5", "Sumario 5", "Table des matières niveau 5"),
    ("User Index Heading", "User Index Heading", "自定义索引标题", "Título del índice de usuario", "Titre d'index personnalisé"),
    ("User Index 1", "User Index 1", "自定义索引 1", "Índice de usuario 1", "Index personnalisé 1"),
    ("User Index 2", "User Index 2", "自定义索引 2", "Índice de usuario 2", "Index personnalisé 2"),
    ("User Index 3", "User Index 3", "自定义索引 3", "Índice de usuario 3", "Index personnalisé 3"),
    ("User Index 4", "User Index 4", "自定义索引 4", "Índice de usuario 4", "Index personnalisé 4"),
    ("User Index 5", "User Index 5", "自定义索引 5", "Índice de usuario 5", "Index personnalisé 5"),
    ("Contents 6", "Contents 6", "目录 6", "Sumario 6", "Table des matières niveau 6"),
    ("Contents 7", "Contents 7", "目录 7", "Sumario 7", "Table des matières niveau 7"),
    ("Contents 8", "Contents 8", "目录 8", "Sumario 8", "Table des matières niveau 8"),
    ("Contents 9", "Contents 9", "目录 9", "Sumario 9", "Table des matières niveau 9"),
    ("Contents 10", "Contents 10", "目录 10", "Sumario 10", "Table des matières niveau 10"),
    ("Figure Index Heading", "Figure Index Heading", "图示索引标题", "Título del índice de figuras", "Titre de l'index des figures"),
    ("Figure Index 1", "Figure Index 1", "图示索引 1", "Índice de figuras 1", "Index des figures 1"),
    ("Object index heading", "Object Index Heading", "对象索引标题", "Título del índice de objetos", "Titre d'index d'objets"),
    ("Object index 1", "Object Index 1", "对象索引 1", "Índice de objetos 1", "Index d'objets 1"),
    ("Table index heading", "Table Index Heading", "表格索引标题", "Título del índice de tablas", "Titre d'index de tableaux"),
    ("Table index 1", "Table Index 1", "表格索引 1", "Índice de tablas 1", "Index de tableaux 1"),
    ("Bibliography Heading", "Bibliography Heading", "参考文献标题", "Título de la bibliografía", "Titre de bibliographie"),
    ("Bibliography 1", "Bibliography 1", "参考文献 1", "Bibliografía 1", "Bibliographie 1"),
    ("User Index 6", "User Index 6", "自定义索引 6", "Índice de usuario 6", "Index personnalisé 6"),
    ("User Index 7", "User Index 7", "自定义索引 7", "Índice de usuario 7", "Index personnalisé 7"),
    ("User Index 8", "User Index 8", "自定义索引 8", "Índice de usuario 8", "Index personnalisé 8"),
    ("User Index 9", "User Index 9", "自定义索引 9", "Índice de usuario 9", "Index personnalisé 9"),
    ("User Index 10", "User Index 10", "自定义索引 10", "Índice de usuario 10", "Index personnalisé 10"),
    ("Title", "Title", "标题", "Título general", "Titre principal"),
    ("Subtitle", "Subtitle", "副标题", "Subtítulo", "Sous-titre"),
    ("Appendix", "Appendix", "附录", "Apéndice", "Annexe"),
    ("Heading", "Heading", "标题样式", "Título", "Titre"),
    ("Heading 1", "Heading 1", "标题 1", "Título 1", "Titre 1"),
    ("Heading 2", "Heading 2", "标题 2", "Título 2", "Titre 2"),
    ("Heading 3", "Heading 3", "标题 3", "Título 3", "Titre 3"),
    ("Heading 4", "Heading 4", "标题 4", "Título 4", "Titre 4"),
    ("Heading 5", "Heading 5", "标题 5", "Título 5", "Titre 5"),
    ("Heading 6", "Heading 6", "标题 6", "Título 6", "Titre 6"),
    ("Heading 7", "Heading 7", "标题 7", "Título 7", "Titre 7"),
    ("Heading 8", "Heading 8", "标题 8", "Título 8", "Titre 8"),
    ("Heading 9", "Heading 9", "标题 9", "Título 9", "Titre 9"),
    ("Heading 10", "Heading 10", "标题 10", "Título 10", "Titre 10"),
    ("Quotations", "Block Quotation", "成段引文", "Cita en bloque", "Bloc de citation"),
    ("Preformatted Text", "Preformatted Text", "预设定格式的文本", "Texto preformateado", "Texte préformaté"),
    ("Horizontal Line", "Horizontal Line", "水平线", "Línea horizontal", "Ligne horizontale"),
    ("List Contents", "List Contents", "列表内容", "Contenido de lista", "Contenu de liste"),
    ("List Heading", "List Heading", "列表标题", "Título de la lista", "En-tête de liste"),
];

pub(super) const CHARACTER: [Builtin; 27] = [
    NO_CHARACTER_STYLE,
    ("Footnote Symbol", "Footnote Characters", "脚注符", "Caracteres de nota al pie", "Caractères de note de bas de page"),
    ("Page Number", "Page Number", "页码", "Número de página", "Numéro de page"),
    ("Caption characters", "Caption Characters", "题注字符", "Caracteres de leyenda", "Caractères de la légende"),
    ("Drop Caps", "Drop Caps", "首字下沉", "Letras capitulares", "Lettrines"),
    ("Numbering Symbols", "Numbering Symbols", "编号符号", "Símbolos de numeración", "Caractères de numérotation"),
    ("Bullet Symbols", "Bullets", "项目符号", "Bolos", "Puces"),
    ("Internet link", "Internet Link", "Internet 链接", "Enlace de Internet", "Lien Internet"),
    ("Visited Internet Link", "Visited Internet Link", "访问过的 Internet 链接", "Enlace de Internet visitado", "Lien Internet visité"),
    ("Placeholder", "Placeholder", "占位符", "Sustitutivo", "Substituant"),
    ("Index Link", "Index Link", "索引链接", "Enlace del índice", "Saut d'index"),
    ("Endnote Symbol", "Endnote Characters", "尾注符", "Caracteres de nota final", "Caractères de note de fin"),
    ("Line numbering", "Line Numbering", "行号", "Numeración de renglones", "Numérotation de lignes"),
    ("Main index entry", "Main Index Entry", "主索引条目", "Entrada principal del índice", "Entrée principale d'index"),
    ("Footnote anchor", "Footnote Anchor", "脚注锚点", "Ancla de nota al pie", "Ancre de note de bas de page"),
    ("Endnote anchor", "Endnote Anchor", "尾注锚点", "Ancla de nota final", "Ancre de note de fin"),
    ("Rubies", "Rubies", "字旁标音", "Texto ágata", "Rubys"),
    ("Vertical Numbering Symbols", "Vertical Numbering Symbols", "竖式编号符号", "Caracteres de numeración vertical", "Caractères de numérotation verticale"),
    ("Emphasis", "Emphasis", "强调", "Destacado", "Accentuation"),
    ("Citation", "Quotation", "引用", "Cita", "Citation"),
    ("Strong Emphasis", "Strong Emphasis", "特别强调", "Destaque mayor", "Accentuation forte"),
    ("Source Text", "Source Text", "源文本", "Texto original", "Texte source"),
    ("Example", "Example", "示例", "Ejemplo", "Exemple"),
    ("User Entry", "User Entry", "用户输入", "Entrada de usuario", "Saisie de l'utilisateur"),
    ("Variable", "Variable", "变量", "Variable", "Variable"),
    ("Definition", "Definition", "定义", "Definición", "Définition"),
    ("Teletype", "Teletype", "电传打字", "Teletipo", "Texte non proportionnel"),
];

pub(super) const PAGE: [Builtin; 10] = [
    ("Standard", "Default Page Style", "默认页面样式", "Estilo de página predeterminado", "Style de page par défaut"),
    ("First Page", "First Page", "首页", "Primera página", "Première page"),
    ("Left Page", "Left Page", "左页", "Página izquierda", "Page gauche"),
    ("Right Page", "Right Page", "右页", "Página derecha", "Page droite"),
    ("Envelope", "Envelope", "信封", "Sobre", "Enveloppe"),
    ("Index", "Index", "索引", "Índice", "Index"),
    ("HTML", "HTML", "HTML", "HTML", "HTML"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "Note de bas de page"),
    ("Endnote", "Endnote", "尾注", "Nota final", "Note de fin"),
    ("Landscape", "Landscape", "横向", "Horizontal", "Paysage"),
];
#[cfg(test)]
mod tests {
    use super::{CHARACTER, PAGE, PARAGRAPH};

    #[test]
    fn an_english_name_names_one_style() {
        for styles in [&PARAGRAPH[..], &CHARACTER, &PAGE] {
            for (kernel, english, ..) in styles {
                let other = styles.iter().find(|(name, ..)| name == english);
                assert!(other.is_none_or(|(name, ..)| name == kernel), "{english}");
            }
        }
    }

    #[test]
    fn each_language_names_each_style_once() {
        for styles in [&PARAGRAPH[..], &CHARACTER, &PAGE] {
            let names = |pick: fn(&super::Builtin) -> &str| styles.iter().map(pick).collect::<std::collections::HashSet<_>>().len();
            assert_eq!(names(|row| row.2), styles.len(), "Chinese");
            assert_eq!(names(|row| row.3), styles.len(), "Spanish");
            assert_eq!(names(|row| row.4), styles.len(), "French");
        }
    }

    #[gpui::test]
    fn spanish_builtin_names_preserve_user_and_kernel_names(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("es-ES", app);
            assert_eq!(super::shown(&PARAGRAPH, "Text body", app), "Cuerpo de texto");
            assert_eq!(super::shown(&PARAGRAPH, "Header", app), "Cabecera");
            assert_eq!(super::shown(&PARAGRAPH, "Heading", app), "Título");
            assert_eq!(super::shown(&PARAGRAPH, "Title", app), "Título general");
            assert_eq!(super::shown(&PAGE, "Standard", app), "Estilo de página predeterminado");
            assert_eq!(super::shown(&CHARACTER, "Standard", app), "Ningún estilo de carácter");
            assert_eq!(super::shown(&PARAGRAPH, "Mi estilo", app), "Mi estilo");
            assert_eq!(super::listed(&PARAGRAPH, "Body Text"), "Text body");
            assert_eq!(super::shown(&PARAGRAPH, "Text body (user)", app), "Text body");
        });
    }

    #[gpui::test]
    fn french_builtin_names_preserve_user_and_kernel_names(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("fr-FR", app);
            assert_eq!(super::shown(&PARAGRAPH, "Text body", app), "Corps de texte");
            assert_eq!(super::shown(&PARAGRAPH, "Heading", app), "Titre");
            assert_eq!(super::shown(&PARAGRAPH, "Title", app), "Titre principal");
            assert_eq!(super::shown(&PAGE, "Standard", app), "Style de page par défaut");
            assert_eq!(super::shown(&CHARACTER, "Standard", app), "Aucun style de caractère");
            assert_eq!(super::shown(&PARAGRAPH, "Mon style", app), "Mon style");
            assert_eq!(super::listed(&PARAGRAPH, "Body Text"), "Text body");
        });
    }
}
