//! Writer's built-in style names in the interface language (I-03).

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

/// Kernel, English, Chinese, Spanish and Japanese names.
pub(super) type Builtin = (&'static str, &'static str, &'static str, &'static str, &'static str);

/// The character default: outside the pool, never suffixed.
const NO_CHARACTER_STYLE: Builtin = ("Standard", "No Character Style", "无字符样式", "Ningún estilo de carácter", "文字スタイルなし");

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
    if let Some((_, english, zh, es, ja)) = styles.iter().find(|(kernel, ..)| *kernel == name) {
        return match locale { "zh-CN" => *zh, "es-ES" => *es, "ja-JP" => *ja, _ => *english }.into();
    }
    if styles.iter().any(|row| row.1 == name && *row != NO_CHARACTER_STYLE) {
        return format!("{name}{USER}").into();
    }
    unmarked(name).to_string().into()
}

// From LibreOffice 26.2.6: SwStyleNameMapper, strings.hrc, zh-CN, es, ja.
pub(super) const PARAGRAPH: [Builtin; 126] = [
    ("Standard", "Default Paragraph Style", "默认段落样式", "Estilo de párrafo predeterminado", "標準段落スタイル"),
    ("Text body", "Body Text", "正文", "Cuerpo de texto", "本文"),
    ("First line indent", "First Line Indent", "首行缩进", "Sangría de primer renglón", "段落字下げ"),
    ("Hanging indent", "Hanging Indent", "悬挂缩进", "Sangría francesa", "ぶら下げインデント"),
    ("Text body indent", "Body Text, Indented", "带缩进的正文", "Cuerpo de texto con sangría", "本文インデント"),
    ("Salutation", "Complimentary Close", "结尾敬语", "Frase de despedida", "結語"),
    ("Signature", "Signature", "签名", "Firma", "署名"),
    ("List Indent", "List Indent", "列表缩进", "Confrontación", "箇条書きインデント"),
    ("Marginalia", "Marginalia", "旁注", "Nota al margen", "傍注"),
    ("List", "List", "列表", "Lista", "箇条書き"),
    ("Numbering 1 Start", "Numbering 1 Start", "编号 1 起始", "Numeración 1 inicio", "番号付け 1 始め"),
    ("Numbering 1", "Numbering 1", "编号 1", "Numeración 1", "番号付け 1"),
    ("Numbering 1 End", "Numbering 1 End", "编号 1 结束", "Numeración 1 fin", "番号付け 1 終り"),
    ("Numbering 1 Cont.", "Numbering 1 Cont.", "编号 1 续", "Numeración 1 continuación", "番号付け 1 続き"),
    ("Numbering 2 Start", "Numbering 2 Start", "编号 2 起始", "Numeración 2 inicio", "番号付け 2 始め"),
    ("Numbering 2", "Numbering 2", "编号 2", "Numeración 2", "番号付け 2"),
    ("Numbering 2 End", "Numbering 2 End", "编号 2 结束", "Numeración 2 fin", "番号付け 2 終り"),
    ("Numbering 2 Cont.", "Numbering 2 Cont.", "编号 2 续", "Numeración 2 continuación", "番号付け 2 続き"),
    ("Numbering 3 Start", "Numbering 3 Start", "编号 3 起始", "Numeración 3 inicio", "番号付け 3 始め"),
    ("Numbering 3", "Numbering 3", "编号 3", "Numeración 3", "番号付け 3"),
    ("Numbering 3 End", "Numbering 3 End", "编号 3 结束", "Numeración 3 fin", "番号付け 3 終り"),
    ("Numbering 3 Cont.", "Numbering 3 Cont.", "编号 3 续", "Numeración 3 continuación", "番号付け 3 続き"),
    ("Numbering 4 Start", "Numbering 4 Start", "编号 4 起始", "Numeración 4 inicio", "番号付け 4 始め"),
    ("Numbering 4", "Numbering 4", "编号 4", "Numeración 4", "番号付け 4"),
    ("Numbering 4 End", "Numbering 4 End", "编号 4 结束", "Numeración 4 fin", "番号付け 4 終り"),
    ("Numbering 4 Cont.", "Numbering 4 Cont.", "编号 4 续", "Numeración 4 continuación", "番号付け 4 続き"),
    ("Numbering 5 Start", "Numbering 5 Start", "编号 5 起始", "Numeración 5 inicio", "番号付け 5 始め"),
    ("Numbering 5", "Numbering 5", "编号 5", "Numeración 5", "番号付け 5"),
    ("Numbering 5 End", "Numbering 5 End", "编号 5 结束", "Numeración 5 fin", "番号付け 5 終り"),
    ("Numbering 5 Cont.", "Numbering 5 Cont.", "编号 5 续", "Numeración 5 continuación", "番号付け 5 続き"),
    ("List 1 Start", "List 1 Start", "列表 1 起始", "Lista 1 inicio", "箇条書き1始め"),
    ("List 1", "List 1", "列表 1", "Lista 1", "箇条書き1"),
    ("List 1 End", "List 1 End", "列表 1 结束", "Lista 1 fin", "箇条書き1終り"),
    ("List 1 Cont.", "List 1 Cont.", "列表 1 续", "Lista 1 continuación", "箇条書き1続き"),
    ("List 2 Start", "List 2 Start", "列表 2 起始", "Lista 2 inicio", "箇条書き2始め"),
    ("List 2", "List 2", "列表 2", "Lista 2", "箇条書き2"),
    ("List 2 End", "List 2 End", "列表 2 结束", "Lista 2 fin", "箇条書き2終り"),
    ("List 2 Cont.", "List 2 Cont.", "列表 2 续", "Lista 2 continuación", "箇条書き2続き"),
    ("List 3 Start", "List 3 Start", "列表 3 起始", "Lista 3 inicio", "箇条書き3始め"),
    ("List 3", "List 3", "列表 3", "Lista 3", "箇条書き3"),
    ("List 3 End", "List 3 End", "列表 3 结束", "Lista 3 fin", "箇条書き3終り"),
    ("List 3 Cont.", "List 3 Cont.", "列表 3 续", "Lista 3 continuación", "箇条書き3続き"),
    ("List 4 Start", "List 4 Start", "列表 4 起始", "Lista 4 inicio", "箇条書き4始め"),
    ("List 4", "List 4", "列表 4", "Lista 4", "箇条書き4"),
    ("List 4 End", "List 4 End", "列表 4 结束", "Lista 4 fin", "箇条書き4終り"),
    ("List 4 Cont.", "List 4 Cont.", "列表 4 续", "Lista 4 continuación", "箇条書き4続き"),
    ("List 5 Start", "List 5 Start", "列表 5 起始", "Lista 5 inicio", "箇条書き5始め"),
    ("List 5", "List 5", "列表 5", "Lista 5", "箇条書き5"),
    ("List 5 End", "List 5 End", "列表 5 结束", "Lista 5 fin", "箇条書き5終り"),
    ("List 5 Cont.", "List 5 Cont.", "列表 5 续", "Lista 5 continuación", "箇条書き5続き"),
    ("Header and Footer", "Header and Footer", "页眉与页脚", "Cabecera y pie", "ヘッダーとフッター"),
    ("Header", "Header", "页眉", "Cabecera", "ヘッダー"),
    ("Header left", "Header Left", "页眉左侧", "Cabecera izquierda", "ヘッダー左"),
    ("Header right", "Header Right", "页眉右侧", "Cabecera derecha", "ヘッダー右"),
    ("Footer", "Footer", "页脚", "Pie de página", "フッター"),
    ("Footer left", "Footer Left", "页脚左侧", "Pie de página izquierdo", "フッター 左"),
    ("Footer right", "Footer Right", "页脚右侧", "Pie de página derecho", "フッター 右"),
    ("Table Contents", "Table Contents", "表格内容", "Contenido de la tabla", "表の内容"),
    ("Table Heading", "Table Heading", "表格标题", "Título de la tabla", "表の見出し"),
    ("Caption", "Caption", "题注", "Leyenda", "キャプション"),
    ("Illustration", "Illustration", "插图", "Ilustración", "イラストレーション"),
    ("Table", "Table", "表格", "Tabla", "表"),
    ("Text", "Text", "文字", "Texto", "テキスト"),
    ("Figure", "Figure", "图示", "Figura", "図"),
    ("Frame contents", "Frame Contents", "框架内容", "Contenido del marco", "枠の内容"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "脚注"),
    ("Addressee", "Addressee", "收件人", "Destinatario", "宛先"),
    ("Sender", "Sender", "发件人", "Remitente", "差出人"),
    ("Endnote", "Endnote", "尾注", "Nota final", "文末脚注"),
    ("Drawing", "Drawing", "绘图", "Dibujo", "図版"),
    ("Comment", "Comment", "批注", "Comentario", "コメント"),
    ("Index", "Index", "索引", "Índice", "索引"),
    ("Index Heading", "Index Heading", "索引标题", "Título del índice", "索引 見出し"),
    ("Index 1", "Index 1", "索引 1", "Índice 1", "索引 1"),
    ("Index 2", "Index 2", "索引 2", "Índice 2", "索引 2"),
    ("Index 3", "Index 3", "索引 3", "Índice 3", "索引 3"),
    ("Index Separator", "Index Separator", "索引分隔符", "Separador del índice", "索引 セパレーター"),
    ("Contents Heading", "Contents Heading", "目录标题", "Título del sumario", "目次見出し"),
    ("Contents 1", "Contents 1", "目录 1", "Sumario 1", "目次 1"),
    ("Contents 2", "Contents 2", "目录 2", "Sumario 2", "目次 2"),
    ("Contents 3", "Contents 3", "目录 3", "Sumario 3", "目次 3"),
    ("Contents 4", "Contents 4", "目录 4", "Sumario 4", "目次 4"),
    ("Contents 5", "Contents 5", "目录 5", "Sumario 5", "目次 5"),
    ("User Index Heading", "User Index Heading", "自定义索引标题", "Título del índice de usuario", "ユーザー定義の索引見出し"),
    ("User Index 1", "User Index 1", "自定义索引 1", "Índice de usuario 1", "ユーザー定義の索引 1"),
    ("User Index 2", "User Index 2", "自定义索引 2", "Índice de usuario 2", "ユーザー定義の索引 2"),
    ("User Index 3", "User Index 3", "自定义索引 3", "Índice de usuario 3", "ユーザー定義の索引 3"),
    ("User Index 4", "User Index 4", "自定义索引 4", "Índice de usuario 4", "ユーザー定義の索引 4"),
    ("User Index 5", "User Index 5", "自定义索引 5", "Índice de usuario 5", "ユーザー定義の索引 5"),
    ("Contents 6", "Contents 6", "目录 6", "Sumario 6", "目次 6"),
    ("Contents 7", "Contents 7", "目录 7", "Sumario 7", "目次 7"),
    ("Contents 8", "Contents 8", "目录 8", "Sumario 8", "目次 8"),
    ("Contents 9", "Contents 9", "目录 9", "Sumario 9", "目次 9"),
    ("Contents 10", "Contents 10", "目录 10", "Sumario 10", "目次 10"),
    ("Figure Index Heading", "Figure Index Heading", "图示索引标题", "Título del índice de figuras", "図索引見出し"),
    ("Figure Index 1", "Figure Index 1", "图示索引 1", "Índice de figuras 1", "図索引 1"),
    ("Object index heading", "Object Index Heading", "对象索引标题", "Título del índice de objetos", "オブジェクト索引見出し"),
    ("Object index 1", "Object Index 1", "对象索引 1", "Índice de objetos 1", "オブジェクト索引 1"),
    ("Table index heading", "Table Index Heading", "表格索引标题", "Título del índice de tablas", "表索引見出し"),
    ("Table index 1", "Table Index 1", "表格索引 1", "Índice de tablas 1", "表索引 1"),
    ("Bibliography Heading", "Bibliography Heading", "参考文献标题", "Título de la bibliografía", "参考文献表見出し"),
    ("Bibliography 1", "Bibliography 1", "参考文献 1", "Bibliografía 1", "参考文献表 1"),
    ("User Index 6", "User Index 6", "自定义索引 6", "Índice de usuario 6", "ユーザー定義の索引 6"),
    ("User Index 7", "User Index 7", "自定义索引 7", "Índice de usuario 7", "ユーザー定義の索引 7"),
    ("User Index 8", "User Index 8", "自定义索引 8", "Índice de usuario 8", "ユーザー定義の索引 8"),
    ("User Index 9", "User Index 9", "自定义索引 9", "Índice de usuario 9", "ユーザー定義の索引 9"),
    ("User Index 10", "User Index 10", "自定义索引 10", "Índice de usuario 10", "ユーザー定義の索引 10"),
    ("Title", "Title", "标题", "Título general", "表題"),
    ("Subtitle", "Subtitle", "副标题", "Subtítulo", "副題"),
    ("Appendix", "Appendix", "附录", "Apéndice", "付録"),
    ("Heading", "Heading", "标题样式", "Título", "見出し"),
    ("Heading 1", "Heading 1", "标题 1", "Título 1", "見出し1"),
    ("Heading 2", "Heading 2", "标题 2", "Título 2", "見出し2"),
    ("Heading 3", "Heading 3", "标题 3", "Título 3", "見出し3"),
    ("Heading 4", "Heading 4", "标题 4", "Título 4", "見出し4"),
    ("Heading 5", "Heading 5", "标题 5", "Título 5", "見出し5"),
    ("Heading 6", "Heading 6", "标题 6", "Título 6", "見出し6"),
    ("Heading 7", "Heading 7", "标题 7", "Título 7", "見出し7"),
    ("Heading 8", "Heading 8", "标题 8", "Título 8", "見出し8"),
    ("Heading 9", "Heading 9", "标题 9", "Título 9", "見出し9"),
    ("Heading 10", "Heading 10", "标题 10", "Título 10", "見出し10"),
    ("Quotations", "Block Quotation", "成段引文", "Cita en bloque", "引用文"),
    ("Preformatted Text", "Preformatted Text", "预设定格式的文本", "Texto preformateado", "整形済みテキスト"),
    ("Horizontal Line", "Horizontal Line", "水平线", "Línea horizontal", "横線"),
    ("List Contents", "List Contents", "列表内容", "Contenido de lista", "リストの内容"),
    ("List Heading", "List Heading", "列表标题", "Título de la lista", "リストの見出し"),
];

pub(super) const CHARACTER: [Builtin; 27] = [
    NO_CHARACTER_STYLE,
    ("Footnote Symbol", "Footnote Characters", "脚注符", "Caracteres de nota al pie", "脚注番号"),
    ("Page Number", "Page Number", "页码", "Número de página", "ページ番号"),
    ("Caption characters", "Caption Characters", "题注字符", "Caracteres de leyenda", "キャプションの文字"),
    ("Drop Caps", "Drop Caps", "首字下沉", "Letras capitulares", "ドロップキャップ"),
    ("Numbering Symbols", "Numbering Symbols", "编号符号", "Símbolos de numeración", "番号付け記号"),
    ("Bullet Symbols", "Bullets", "项目符号", "Bolos", "箇条書き"),
    ("Internet link", "Internet Link", "Internet 链接", "Enlace de Internet", "インターネットリンク"),
    ("Visited Internet Link", "Visited Internet Link", "访问过的 Internet 链接", "Enlace de Internet visitado", "訪れたインターネットリンク"),
    ("Placeholder", "Placeholder", "占位符", "Sustitutivo", "プレースホルダー"),
    ("Index Link", "Index Link", "索引链接", "Enlace del índice", "索引ジャンプ"),
    ("Endnote Symbol", "Endnote Characters", "尾注符", "Caracteres de nota final", "文末脚注番号"),
    ("Line numbering", "Line Numbering", "行号", "Numeración de renglones", "行番号付け"),
    ("Main index entry", "Main Index Entry", "主索引条目", "Entrada principal del índice", "索引の主項目"),
    ("Footnote anchor", "Footnote Anchor", "脚注锚点", "Ancla de nota al pie", "脚注参照番号"),
    ("Endnote anchor", "Endnote Anchor", "尾注锚点", "Ancla de nota final", "文末脚注参照記号"),
    ("Rubies", "Rubies", "字旁标音", "Texto ágata", "ふりがな"),
    ("Vertical Numbering Symbols", "Vertical Numbering Symbols", "竖式编号符号", "Caracteres de numeración vertical", "縦書き番号付け記号"),
    ("Emphasis", "Emphasis", "强调", "Destacado", "強調"),
    ("Citation", "Quotation", "引用", "Cita", "引用句"),
    ("Strong Emphasis", "Strong Emphasis", "特别强调", "Destaque mayor", "さらに強調"),
    ("Source Text", "Source Text", "源文本", "Texto original", "ソーステキスト"),
    ("Example", "Example", "示例", "Ejemplo", "例"),
    ("User Entry", "User Entry", "用户输入", "Entrada de usuario", "ユーザー入力"),
    ("Variable", "Variable", "变量", "Variable", "変数"),
    ("Definition", "Definition", "定义", "Definición", "定義"),
    ("Teletype", "Teletype", "电传打字", "Teletipo", "等幅"),
];

pub(super) const PAGE: [Builtin; 10] = [
    ("Standard", "Default Page Style", "默认页面样式", "Estilo de página predeterminado", "標準ページスタイル"),
    ("First Page", "First Page", "首页", "Primera página", "表紙ページ"),
    ("Left Page", "Left Page", "左页", "Página izquierda", "左ページ"),
    ("Right Page", "Right Page", "右页", "Página derecha", "右ページ"),
    ("Envelope", "Envelope", "信封", "Sobre", "封筒"),
    ("Index", "Index", "索引", "Índice", "索引"),
    ("HTML", "HTML", "HTML", "HTML", "HTML"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "脚注"),
    ("Endnote", "Endnote", "尾注", "Nota final", "文末脚注"),
    ("Landscape", "Landscape", "横向", "Horizontal", "横置き"),
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
            assert_eq!(names(|row| row.4), styles.len(), "Japanese");
        }
    }

    #[gpui::test]
    fn japanese_builtin_names_come_from_libreoffice(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("ja-JP", app);
            assert_eq!(super::shown(&PARAGRAPH, "Standard", app), "標準段落スタイル");
            assert_eq!(super::shown(&PARAGRAPH, "Text body", app), "本文");
            assert_eq!(super::shown(&PARAGRAPH, "Heading 1", app), "見出し1");
            assert_eq!(super::shown(&CHARACTER, "Standard", app), "文字スタイルなし");
            assert_eq!(super::shown(&PAGE, "First Page", app), "表紙ページ");
            assert_eq!(super::shown(&PARAGRAPH, "私のスタイル", app), "私のスタイル");
            assert_eq!(super::listed(&PARAGRAPH, "Body Text"), "Text body");
        });
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
}
