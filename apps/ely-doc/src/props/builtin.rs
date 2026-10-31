//! Writer's built-in style names in the interface language (I-03).

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

/// Kernel, English, Chinese and Spanish names.
pub(super) type Builtin = (&'static str, &'static str, &'static str, &'static str);

/// The character default: outside the pool, never suffixed.
const NO_CHARACTER_STYLE: Builtin = ("Standard", "No Character Style", "无字符样式", "Sin estilo de carácter");

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
    if let Some((_, english, zh, es)) = styles.iter().find(|(kernel, ..)| *kernel == name) {
        return match locale { "zh-CN" => *zh, "es-ES" => *es, _ => *english }.into();
    }
    if styles.iter().any(|row| row.1 == name && *row != NO_CHARACTER_STYLE) {
        return format!("{name}{USER}").into();
    }
    unmarked(name).to_string().into()
}

// From LibreOffice 26.2.6: SwStyleNameMapper, strings.hrc, zh-CN po.
pub(super) const PARAGRAPH: [Builtin; 126] = [
    ("Standard", "Default Paragraph Style", "默认段落样式", "Estilo de párrafo predeterminado"),
    ("Text body", "Body Text", "正文", "Cuerpo de texto"),
    ("First line indent", "First Line Indent", "首行缩进", "Sangría de primera línea"),
    ("Hanging indent", "Hanging Indent", "悬挂缩进", "Sangría francesa"),
    ("Text body indent", "Body Text, Indented", "带缩进的正文", "Cuerpo de texto con sangría"),
    ("Salutation", "Complimentary Close", "结尾敬语", "Despedida"),
    ("Signature", "Signature", "签名", "Firma"),
    ("List Indent", "List Indent", "列表缩进", "Sangría de lista"),
    ("Marginalia", "Marginalia", "旁注", "Nota al margen"),
    ("List", "List", "列表", "Lista"),
    ("Numbering 1 Start", "Numbering 1 Start", "编号 1 起始", "Numeración 1 inicio"),
    ("Numbering 1", "Numbering 1", "编号 1", "Numeración 1"),
    ("Numbering 1 End", "Numbering 1 End", "编号 1 结束", "Numeración 1 final"),
    ("Numbering 1 Cont.", "Numbering 1 Cont.", "编号 1 续", "Numeración 1 continuación"),
    ("Numbering 2 Start", "Numbering 2 Start", "编号 2 起始", "Numeración 2 inicio"),
    ("Numbering 2", "Numbering 2", "编号 2", "Numeración 2"),
    ("Numbering 2 End", "Numbering 2 End", "编号 2 结束", "Numeración 2 final"),
    ("Numbering 2 Cont.", "Numbering 2 Cont.", "编号 2 续", "Numeración 2 continuación"),
    ("Numbering 3 Start", "Numbering 3 Start", "编号 3 起始", "Numeración 3 inicio"),
    ("Numbering 3", "Numbering 3", "编号 3", "Numeración 3"),
    ("Numbering 3 End", "Numbering 3 End", "编号 3 结束", "Numeración 3 final"),
    ("Numbering 3 Cont.", "Numbering 3 Cont.", "编号 3 续", "Numeración 3 continuación"),
    ("Numbering 4 Start", "Numbering 4 Start", "编号 4 起始", "Numeración 4 inicio"),
    ("Numbering 4", "Numbering 4", "编号 4", "Numeración 4"),
    ("Numbering 4 End", "Numbering 4 End", "编号 4 结束", "Numeración 4 final"),
    ("Numbering 4 Cont.", "Numbering 4 Cont.", "编号 4 续", "Numeración 4 continuación"),
    ("Numbering 5 Start", "Numbering 5 Start", "编号 5 起始", "Numeración 5 inicio"),
    ("Numbering 5", "Numbering 5", "编号 5", "Numeración 5"),
    ("Numbering 5 End", "Numbering 5 End", "编号 5 结束", "Numeración 5 final"),
    ("Numbering 5 Cont.", "Numbering 5 Cont.", "编号 5 续", "Numeración 5 continuación"),
    ("List 1 Start", "List 1 Start", "列表 1 起始", "Lista 1 inicio"),
    ("List 1", "List 1", "列表 1", "Lista 1"),
    ("List 1 End", "List 1 End", "列表 1 结束", "Lista 1 final"),
    ("List 1 Cont.", "List 1 Cont.", "列表 1 续", "Lista 1 continuación"),
    ("List 2 Start", "List 2 Start", "列表 2 起始", "Lista 2 inicio"),
    ("List 2", "List 2", "列表 2", "Lista 2"),
    ("List 2 End", "List 2 End", "列表 2 结束", "Lista 2 final"),
    ("List 2 Cont.", "List 2 Cont.", "列表 2 续", "Lista 2 continuación"),
    ("List 3 Start", "List 3 Start", "列表 3 起始", "Lista 3 inicio"),
    ("List 3", "List 3", "列表 3", "Lista 3"),
    ("List 3 End", "List 3 End", "列表 3 结束", "Lista 3 final"),
    ("List 3 Cont.", "List 3 Cont.", "列表 3 续", "Lista 3 continuación"),
    ("List 4 Start", "List 4 Start", "列表 4 起始", "Lista 4 inicio"),
    ("List 4", "List 4", "列表 4", "Lista 4"),
    ("List 4 End", "List 4 End", "列表 4 结束", "Lista 4 final"),
    ("List 4 Cont.", "List 4 Cont.", "列表 4 续", "Lista 4 continuación"),
    ("List 5 Start", "List 5 Start", "列表 5 起始", "Lista 5 inicio"),
    ("List 5", "List 5", "列表 5", "Lista 5"),
    ("List 5 End", "List 5 End", "列表 5 结束", "Lista 5 final"),
    ("List 5 Cont.", "List 5 Cont.", "列表 5 续", "Lista 5 continuación"),
    ("Header and Footer", "Header and Footer", "页眉与页脚", "Encabezado y pie de página"),
    ("Header", "Header", "页眉", "Encabezado"),
    ("Header left", "Header Left", "页眉左侧", "Encabezado izquierdo"),
    ("Header right", "Header Right", "页眉右侧", "Encabezado derecho"),
    ("Footer", "Footer", "页脚", "Pie de página"),
    ("Footer left", "Footer Left", "页脚左侧", "Pie de página izquierdo"),
    ("Footer right", "Footer Right", "页脚右侧", "Pie de página derecho"),
    ("Table Contents", "Table Contents", "表格内容", "Contenido de tabla"),
    ("Table Heading", "Table Heading", "表格标题", "Encabezado de tabla"),
    ("Caption", "Caption", "题注", "Leyenda"),
    ("Illustration", "Illustration", "插图", "Ilustración"),
    ("Table", "Table", "表格", "Tabla"),
    ("Text", "Text", "文字", "Texto"),
    ("Figure", "Figure", "图示", "Figura"),
    ("Frame contents", "Frame Contents", "框架内容", "Contenido de marco"),
    ("Footnote", "Footnote", "脚注", "Nota al pie"),
    ("Addressee", "Addressee", "收件人", "Destinatario"),
    ("Sender", "Sender", "发件人", "Remitente"),
    ("Endnote", "Endnote", "尾注", "Nota al final"),
    ("Drawing", "Drawing", "绘图", "Dibujo"),
    ("Comment", "Comment", "批注", "Comentario"),
    ("Index", "Index", "索引", "Índice"),
    ("Index Heading", "Index Heading", "索引标题", "Título de índice"),
    ("Index 1", "Index 1", "索引 1", "Índice 1"),
    ("Index 2", "Index 2", "索引 2", "Índice 2"),
    ("Index 3", "Index 3", "索引 3", "Índice 3"),
    ("Index Separator", "Index Separator", "索引分隔符", "Separador de índice"),
    ("Contents Heading", "Contents Heading", "目录标题", "Título del índice de contenido"),
    ("Contents 1", "Contents 1", "目录 1", "Contenido 1"),
    ("Contents 2", "Contents 2", "目录 2", "Contenido 2"),
    ("Contents 3", "Contents 3", "目录 3", "Contenido 3"),
    ("Contents 4", "Contents 4", "目录 4", "Contenido 4"),
    ("Contents 5", "Contents 5", "目录 5", "Contenido 5"),
    ("User Index Heading", "User Index Heading", "自定义索引标题", "Título del índice de usuario"),
    ("User Index 1", "User Index 1", "自定义索引 1", "Índice de usuario 1"),
    ("User Index 2", "User Index 2", "自定义索引 2", "Índice de usuario 2"),
    ("User Index 3", "User Index 3", "自定义索引 3", "Índice de usuario 3"),
    ("User Index 4", "User Index 4", "自定义索引 4", "Índice de usuario 4"),
    ("User Index 5", "User Index 5", "自定义索引 5", "Índice de usuario 5"),
    ("Contents 6", "Contents 6", "目录 6", "Contenido 6"),
    ("Contents 7", "Contents 7", "目录 7", "Contenido 7"),
    ("Contents 8", "Contents 8", "目录 8", "Contenido 8"),
    ("Contents 9", "Contents 9", "目录 9", "Contenido 9"),
    ("Contents 10", "Contents 10", "目录 10", "Contenido 10"),
    ("Figure Index Heading", "Figure Index Heading", "图示索引标题", "Título del índice de figuras"),
    ("Figure Index 1", "Figure Index 1", "图示索引 1", "Índice de figuras 1"),
    ("Object index heading", "Object Index Heading", "对象索引标题", "Título del índice de objetos"),
    ("Object index 1", "Object Index 1", "对象索引 1", "Índice de objetos 1"),
    ("Table index heading", "Table Index Heading", "表格索引标题", "Título del índice de tablas"),
    ("Table index 1", "Table Index 1", "表格索引 1", "Índice de tablas 1"),
    ("Bibliography Heading", "Bibliography Heading", "参考文献标题", "Título de bibliografía"),
    ("Bibliography 1", "Bibliography 1", "参考文献 1", "Bibliografía 1"),
    ("User Index 6", "User Index 6", "自定义索引 6", "Índice de usuario 6"),
    ("User Index 7", "User Index 7", "自定义索引 7", "Índice de usuario 7"),
    ("User Index 8", "User Index 8", "自定义索引 8", "Índice de usuario 8"),
    ("User Index 9", "User Index 9", "自定义索引 9", "Índice de usuario 9"),
    ("User Index 10", "User Index 10", "自定义索引 10", "Índice de usuario 10"),
    ("Title", "Title", "标题", "Título"),
    ("Subtitle", "Subtitle", "副标题", "Subtítulo"),
    ("Appendix", "Appendix", "附录", "Apéndice"),
    ("Heading", "Heading", "标题样式", "Encabezado"),
    ("Heading 1", "Heading 1", "标题 1", "Título 1"),
    ("Heading 2", "Heading 2", "标题 2", "Título 2"),
    ("Heading 3", "Heading 3", "标题 3", "Título 3"),
    ("Heading 4", "Heading 4", "标题 4", "Título 4"),
    ("Heading 5", "Heading 5", "标题 5", "Título 5"),
    ("Heading 6", "Heading 6", "标题 6", "Título 6"),
    ("Heading 7", "Heading 7", "标题 7", "Título 7"),
    ("Heading 8", "Heading 8", "标题 8", "Título 8"),
    ("Heading 9", "Heading 9", "标题 9", "Título 9"),
    ("Heading 10", "Heading 10", "标题 10", "Título 10"),
    ("Quotations", "Block Quotation", "成段引文", "Cita en bloque"),
    ("Preformatted Text", "Preformatted Text", "预设定格式的文本", "Texto preformateado"),
    ("Horizontal Line", "Horizontal Line", "水平线", "Línea horizontal"),
    ("List Contents", "List Contents", "列表内容", "Contenido de lista"),
    ("List Heading", "List Heading", "列表标题", "Encabezado de lista"),
];

pub(super) const CHARACTER: [Builtin; 27] = [
    NO_CHARACTER_STYLE,
    ("Footnote Symbol", "Footnote Characters", "脚注符", "Caracteres de nota al pie"),
    ("Page Number", "Page Number", "页码", "Número de página"),
    ("Caption characters", "Caption Characters", "题注字符", "Caracteres de leyenda"),
    ("Drop Caps", "Drop Caps", "首字下沉", "Letra capitular"),
    ("Numbering Symbols", "Numbering Symbols", "编号符号", "Símbolos de numeración"),
    ("Bullet Symbols", "Bullets", "项目符号", "Viñetas"),
    ("Internet link", "Internet Link", "Internet 链接", "Enlace de Internet"),
    ("Visited Internet Link", "Visited Internet Link", "访问过的 Internet 链接", "Enlace de Internet visitado"),
    ("Placeholder", "Placeholder", "占位符", "Marcador de posición"),
    ("Index Link", "Index Link", "索引链接", "Enlace de índice"),
    ("Endnote Symbol", "Endnote Characters", "尾注符", "Caracteres de nota al final"),
    ("Line numbering", "Line Numbering", "行号", "Numeración de líneas"),
    ("Main index entry", "Main Index Entry", "主索引条目", "Entrada principal de índice"),
    ("Footnote anchor", "Footnote Anchor", "脚注锚点", "Ancla de nota al pie"),
    ("Endnote anchor", "Endnote Anchor", "尾注锚点", "Ancla de nota al final"),
    ("Rubies", "Rubies", "字旁标音", "Guías fonéticas"),
    ("Vertical Numbering Symbols", "Vertical Numbering Symbols", "竖式编号符号", "Símbolos de numeración vertical"),
    ("Emphasis", "Emphasis", "强调", "Énfasis"),
    ("Citation", "Quotation", "引用", "Cita"),
    ("Strong Emphasis", "Strong Emphasis", "特别强调", "Énfasis fuerte"),
    ("Source Text", "Source Text", "源文本", "Texto fuente"),
    ("Example", "Example", "示例", "Ejemplo"),
    ("User Entry", "User Entry", "用户输入", "Entrada del usuario"),
    ("Variable", "Variable", "变量", "Variable"),
    ("Definition", "Definition", "定义", "Definición"),
    ("Teletype", "Teletype", "电传打字", "Teletipo"),
];

pub(super) const PAGE: [Builtin; 10] = [
    ("Standard", "Default Page Style", "默认页面样式", "Estilo de página predeterminado"),
    ("First Page", "First Page", "首页", "Primera página"),
    ("Left Page", "Left Page", "左页", "Página izquierda"),
    ("Right Page", "Right Page", "右页", "Página derecha"),
    ("Envelope", "Envelope", "信封", "Sobre"),
    ("Index", "Index", "索引", "Índice"),
    ("HTML", "HTML", "HTML", "HTML"),
    ("Footnote", "Footnote", "脚注", "Nota al pie"),
    ("Endnote", "Endnote", "尾注", "Nota al final"),
    ("Landscape", "Landscape", "横向", "Horizontal"),
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
}
