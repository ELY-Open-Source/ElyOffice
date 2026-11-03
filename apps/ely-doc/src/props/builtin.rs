//! Writer's built-in style names in the interface language (I-03).

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

/// Kernel, English, Chinese, Spanish and Vietnamese names.
pub(super) type Builtin = (&'static str, &'static str, &'static str, &'static str, &'static str);

/// The character default: outside the pool, never suffixed.
const NO_CHARACTER_STYLE: Builtin = ("Standard", "No Character Style", "无字符样式", "Ningún estilo de carácter", "Không có kiểu ký tự");

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
    if let Some((_, english, zh, es, vi)) = styles.iter().find(|(kernel, ..)| *kernel == name) {
        return match locale { "zh-CN" => *zh, "es-ES" => *es, "vi-VN" => *vi, _ => *english }.into();
    }
    if styles.iter().any(|row| row.1 == name && *row != NO_CHARACTER_STYLE) {
        return format!("{name}{USER}").into();
    }
    unmarked(name).to_string().into()
}

// From LibreOffice 26.2.6: SwStyleNameMapper, strings.hrc, zh-CN, es, vi po.
pub(super) const PARAGRAPH: [Builtin; 126] = [
    ("Standard", "Default Paragraph Style", "默认段落样式", "Estilo de párrafo predeterminado", "Kiểu đoạn văn mặc định"),
    ("Text body", "Body Text", "正文", "Cuerpo de texto", "Thân văn bản"),
    ("First line indent", "First Line Indent", "首行缩进", "Sangría de primer renglón", "Căn lề dòng đầu tiên"),
    ("Hanging indent", "Hanging Indent", "悬挂缩进", "Sangría francesa", "Thụt đầu dòng treo"),
    ("Text body indent", "Body Text, Indented", "带缩进的正文", "Cuerpo de texto con sangría", "Thân văn bản, thụt lề"),
    ("Salutation", "Complimentary Close", "结尾敬语", "Frase de despedida", "Lời kết"),
    ("Signature", "Signature", "签名", "Firma", "Chữ ký"),
    ("List Indent", "List Indent", "列表缩进", "Confrontación", "Căn lề danh sách"),
    ("Marginalia", "Marginalia", "旁注", "Nota al margen", "Ghi chú ở lề"),
    ("List", "List", "列表", "Lista", "Danh sách"),
    ("Numbering 1 Start", "Numbering 1 Start", "编号 1 起始", "Numeración 1 inicio", "Bắt đầu đánh số 1"),
    ("Numbering 1", "Numbering 1", "编号 1", "Numeración 1", "Đánh số 1"),
    ("Numbering 1 End", "Numbering 1 End", "编号 1 结束", "Numeración 1 fin", "Kết thúc đánh số 1"),
    ("Numbering 1 Cont.", "Numbering 1 Cont.", "编号 1 续", "Numeración 1 continuación", "Tiếp tục đánh số 1"),
    ("Numbering 2 Start", "Numbering 2 Start", "编号 2 起始", "Numeración 2 inicio", "Bắt đầu đánh số 2"),
    ("Numbering 2", "Numbering 2", "编号 2", "Numeración 2", "Đánh số 2"),
    ("Numbering 2 End", "Numbering 2 End", "编号 2 结束", "Numeración 2 fin", "Kết thúc đánh số 2"),
    ("Numbering 2 Cont.", "Numbering 2 Cont.", "编号 2 续", "Numeración 2 continuación", "Tiếp tục đánh số 2"),
    ("Numbering 3 Start", "Numbering 3 Start", "编号 3 起始", "Numeración 3 inicio", "Bắt đầu đánh số 3"),
    ("Numbering 3", "Numbering 3", "编号 3", "Numeración 3", "Đánh số 3"),
    ("Numbering 3 End", "Numbering 3 End", "编号 3 结束", "Numeración 3 fin", "Kết thúc đánh số 3"),
    ("Numbering 3 Cont.", "Numbering 3 Cont.", "编号 3 续", "Numeración 3 continuación", "Tiếp tục đánh số 3"),
    ("Numbering 4 Start", "Numbering 4 Start", "编号 4 起始", "Numeración 4 inicio", "Bắt đầu đánh số 4"),
    ("Numbering 4", "Numbering 4", "编号 4", "Numeración 4", "Đánh số 4"),
    ("Numbering 4 End", "Numbering 4 End", "编号 4 结束", "Numeración 4 fin", "Kết thúc đánh số 4"),
    ("Numbering 4 Cont.", "Numbering 4 Cont.", "编号 4 续", "Numeración 4 continuación", "Tiếp tục đánh số 4"),
    ("Numbering 5 Start", "Numbering 5 Start", "编号 5 起始", "Numeración 5 inicio", "Bắt đầu đánh số 5"),
    ("Numbering 5", "Numbering 5", "编号 5", "Numeración 5", "Đánh số 5"),
    ("Numbering 5 End", "Numbering 5 End", "编号 5 结束", "Numeración 5 fin", "Kết thúc đánh số 5"),
    ("Numbering 5 Cont.", "Numbering 5 Cont.", "编号 5 续", "Numeración 5 continuación", "Tiếp tục đánh số 5"),
    ("List 1 Start", "List 1 Start", "列表 1 起始", "Lista 1 inicio", "Bắt đầu danh sách 1"),
    ("List 1", "List 1", "列表 1", "Lista 1", "Danh sách 1"),
    ("List 1 End", "List 1 End", "列表 1 结束", "Lista 1 fin", "Kết thúc danh sách 1"),
    ("List 1 Cont.", "List 1 Cont.", "列表 1 续", "Lista 1 continuación", "Tiếp tục danh sách 1"),
    ("List 2 Start", "List 2 Start", "列表 2 起始", "Lista 2 inicio", "Bắt đầu danh sách 2"),
    ("List 2", "List 2", "列表 2", "Lista 2", "Danh sách 2"),
    ("List 2 End", "List 2 End", "列表 2 结束", "Lista 2 fin", "Kết thúc danh sách 2"),
    ("List 2 Cont.", "List 2 Cont.", "列表 2 续", "Lista 2 continuación", "Tiếp tục danh sách 2"),
    ("List 3 Start", "List 3 Start", "列表 3 起始", "Lista 3 inicio", "Bắt đầu danh sách 3"),
    ("List 3", "List 3", "列表 3", "Lista 3", "Danh sách 3"),
    ("List 3 End", "List 3 End", "列表 3 结束", "Lista 3 fin", "Kết thúc danh sách 3"),
    ("List 3 Cont.", "List 3 Cont.", "列表 3 续", "Lista 3 continuación", "Tiếp tục danh sách 3"),
    ("List 4 Start", "List 4 Start", "列表 4 起始", "Lista 4 inicio", "Bắt đầu danh sách 4"),
    ("List 4", "List 4", "列表 4", "Lista 4", "Danh sách 4"),
    ("List 4 End", "List 4 End", "列表 4 结束", "Lista 4 fin", "Kết thúc danh sách 4"),
    ("List 4 Cont.", "List 4 Cont.", "列表 4 续", "Lista 4 continuación", "Tiếp tục danh sách 4"),
    ("List 5 Start", "List 5 Start", "列表 5 起始", "Lista 5 inicio", "Bắt đầu danh sách 5"),
    ("List 5", "List 5", "列表 5", "Lista 5", "Danh sách 5"),
    ("List 5 End", "List 5 End", "列表 5 结束", "Lista 5 fin", "Kết thúc danh sách 5"),
    ("List 5 Cont.", "List 5 Cont.", "列表 5 续", "Lista 5 continuación", "Tiếp tục danh sách 5"),
    ("Header and Footer", "Header and Footer", "页眉与页脚", "Cabecera y pie", "Đầu trang và chân trang"),
    ("Header", "Header", "页眉", "Cabecera", "Đầu trang"),
    ("Header left", "Header Left", "页眉左侧", "Cabecera izquierda", "Đầu trang bên trái"),
    ("Header right", "Header Right", "页眉右侧", "Cabecera derecha", "Đầu trang bên phải"),
    ("Footer", "Footer", "页脚", "Pie de página", "Chân trang"),
    ("Footer left", "Footer Left", "页脚左侧", "Pie de página izquierdo", "Chân trang bên trái"),
    ("Footer right", "Footer Right", "页脚右侧", "Pie de página derecho", "Chân trang bên phải"),
    ("Table Contents", "Table Contents", "表格内容", "Contenido de la tabla", "Nội dung bảng"),
    ("Table Heading", "Table Heading", "表格标题", "Título de la tabla", "Tiêu đề bảng"),
    ("Caption", "Caption", "题注", "Leyenda", "Phụ đề"),
    ("Illustration", "Illustration", "插图", "Ilustración", "Hình"),
    ("Table", "Table", "表格", "Tabla", "Bảng"),
    ("Text", "Text", "文字", "Texto", "Văn bản"),
    ("Figure", "Figure", "图示", "Figura", "Hình minh họa"),
    ("Frame contents", "Frame Contents", "框架内容", "Contenido del marco", "Nội dung khung"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "Cước chú"),
    ("Addressee", "Addressee", "收件人", "Destinatario", "Người nhận"),
    ("Sender", "Sender", "发件人", "Remitente", "Người gửi"),
    ("Endnote", "Endnote", "尾注", "Nota final", "Chú thích cuối"),
    ("Drawing", "Drawing", "绘图", "Dibujo", "Bản vẽ"),
    ("Comment", "Comment", "批注", "Comentario", "Bình luận"),
    ("Index", "Index", "索引", "Índice", "Chỉ mục"),
    ("Index Heading", "Index Heading", "索引标题", "Título del índice", "Tiêu đề chỉ mục"),
    ("Index 1", "Index 1", "索引 1", "Índice 1", "Chỉ mục 1"),
    ("Index 2", "Index 2", "索引 2", "Índice 2", "Chỉ mục 2"),
    ("Index 3", "Index 3", "索引 3", "Índice 3", "Chỉ mục 3"),
    ("Index Separator", "Index Separator", "索引分隔符", "Separador del índice", "Dấu tách chỉ mục"),
    ("Contents Heading", "Contents Heading", "目录标题", "Título del sumario", "Tiêu đề nội dung"),
    ("Contents 1", "Contents 1", "目录 1", "Sumario 1", "Nội dung 1"),
    ("Contents 2", "Contents 2", "目录 2", "Sumario 2", "Nội dung 2"),
    ("Contents 3", "Contents 3", "目录 3", "Sumario 3", "Nội dung 3"),
    ("Contents 4", "Contents 4", "目录 4", "Sumario 4", "Nội dung 4"),
    ("Contents 5", "Contents 5", "目录 5", "Sumario 5", "Nội dung 5"),
    ("User Index Heading", "User Index Heading", "自定义索引标题", "Título del índice de usuario", "Tiêu đề chỉ mục người dùng"),
    ("User Index 1", "User Index 1", "自定义索引 1", "Índice de usuario 1", "Chỉ mục người dùng 1"),
    ("User Index 2", "User Index 2", "自定义索引 2", "Índice de usuario 2", "Chỉ mục người dùng 2"),
    ("User Index 3", "User Index 3", "自定义索引 3", "Índice de usuario 3", "Chỉ mục người dùng 3"),
    ("User Index 4", "User Index 4", "自定义索引 4", "Índice de usuario 4", "Chỉ mục người dùng 4"),
    ("User Index 5", "User Index 5", "自定义索引 5", "Índice de usuario 5", "Chỉ mục người dùng 5"),
    ("Contents 6", "Contents 6", "目录 6", "Sumario 6", "Nội dung 6"),
    ("Contents 7", "Contents 7", "目录 7", "Sumario 7", "Nội dung 7"),
    ("Contents 8", "Contents 8", "目录 8", "Sumario 8", "Nội dung 8"),
    ("Contents 9", "Contents 9", "目录 9", "Sumario 9", "Nội dung 9"),
    ("Contents 10", "Contents 10", "目录 10", "Sumario 10", "Nội dung 10"),
    ("Figure Index Heading", "Figure Index Heading", "图示索引标题", "Título del índice de figuras", "Tiêu đề chỉ mục hình minh họa"),
    ("Figure Index 1", "Figure Index 1", "图示索引 1", "Índice de figuras 1", "Chỉ mục hình minh họa 1"),
    ("Object index heading", "Object Index Heading", "对象索引标题", "Título del índice de objetos", "Tiêu đề chỉ mục đối tượng"),
    ("Object index 1", "Object Index 1", "对象索引 1", "Índice de objetos 1", "Chỉ mục đối tượng 1"),
    ("Table index heading", "Table Index Heading", "表格索引标题", "Título del índice de tablas", "Tiêu đề chỉ mục bảng"),
    ("Table index 1", "Table Index 1", "表格索引 1", "Índice de tablas 1", "Chỉ mục bảng 1"),
    ("Bibliography Heading", "Bibliography Heading", "参考文献标题", "Título de la bibliografía", "Tiêu đề Thư tịch"),
    ("Bibliography 1", "Bibliography 1", "参考文献 1", "Bibliografía 1", "Thư tịch 1"),
    ("User Index 6", "User Index 6", "自定义索引 6", "Índice de usuario 6", "Chỉ mục người dùng 6"),
    ("User Index 7", "User Index 7", "自定义索引 7", "Índice de usuario 7", "Chỉ mục người dùng 7"),
    ("User Index 8", "User Index 8", "自定义索引 8", "Índice de usuario 8", "Chỉ mục người dùng 8"),
    ("User Index 9", "User Index 9", "自定义索引 9", "Índice de usuario 9", "Chỉ mục người dùng 9"),
    ("User Index 10", "User Index 10", "自定义索引 10", "Índice de usuario 10", "Chỉ mục người dùng 10"),
    ("Title", "Title", "标题", "Título general", "Tựa đề"),
    ("Subtitle", "Subtitle", "副标题", "Subtítulo", "Tiêu đề phụ"),
    ("Appendix", "Appendix", "附录", "Apéndice", "Phụ lục"),
    ("Heading", "Heading", "标题样式", "Título", "Tiêu đề"),
    ("Heading 1", "Heading 1", "标题 1", "Título 1", "Tiêu đề 1"),
    ("Heading 2", "Heading 2", "标题 2", "Título 2", "Tiêu đề 2"),
    ("Heading 3", "Heading 3", "标题 3", "Título 3", "Tiêu đề 3"),
    ("Heading 4", "Heading 4", "标题 4", "Título 4", "Tiêu đề 4"),
    ("Heading 5", "Heading 5", "标题 5", "Título 5", "Tiêu đề 5"),
    ("Heading 6", "Heading 6", "标题 6", "Título 6", "Tiêu đề 6"),
    ("Heading 7", "Heading 7", "标题 7", "Título 7", "Tiêu đề 7"),
    ("Heading 8", "Heading 8", "标题 8", "Título 8", "Tiêu đề 8"),
    ("Heading 9", "Heading 9", "标题 9", "Título 9", "Tiêu đề 9"),
    ("Heading 10", "Heading 10", "标题 10", "Título 10", "Tiêu đề 10"),
    ("Quotations", "Block Quotation", "成段引文", "Cita en bloque", "Trích dẫn khối"),
    ("Preformatted Text", "Preformatted Text", "预设定格式的文本", "Texto preformateado", "Văn bản định dạng sẵn"),
    ("Horizontal Line", "Horizontal Line", "水平线", "Línea horizontal", "Đường nằm ngang"),
    ("List Contents", "List Contents", "列表内容", "Contenido de lista", "Nội dung danh sách"),
    ("List Heading", "List Heading", "列表标题", "Título de la lista", "Tiêu đề danh sách"),
];

pub(super) const CHARACTER: [Builtin; 27] = [
    NO_CHARACTER_STYLE,
    ("Footnote Symbol", "Footnote Characters", "脚注符", "Caracteres de nota al pie", "Ký tự cước chú"),
    ("Page Number", "Page Number", "页码", "Número de página", "Số thứ tự trang"),
    ("Caption characters", "Caption Characters", "题注字符", "Caracteres de leyenda", "Ký tự phụ đề"),
    ("Drop Caps", "Drop Caps", "首字下沉", "Letras capitulares", "Chữ hoa trang trí"),
    ("Numbering Symbols", "Numbering Symbols", "编号符号", "Símbolos de numeración", "Ký hiệu đánh số"),
    ("Bullet Symbols", "Bullets", "项目符号", "Bolos", "Chấm điểm"),
    ("Internet link", "Internet Link", "Internet 链接", "Enlace de Internet", "Liên kết Internet"),
    ("Visited Internet Link", "Visited Internet Link", "访问过的 Internet 链接", "Enlace de Internet visitado", "Liên kết Internet đã thăm"),
    ("Placeholder", "Placeholder", "占位符", "Sustitutivo", "Bộ giữ chỗ"),
    ("Index Link", "Index Link", "索引链接", "Enlace del índice", "Liên kết chỉ mục"),
    ("Endnote Symbol", "Endnote Characters", "尾注符", "Caracteres de nota final", "Ký tự chú thích cuối"),
    ("Line numbering", "Line Numbering", "行号", "Numeración de renglones", "Đánh số dòng"),
    ("Main index entry", "Main Index Entry", "主索引条目", "Entrada principal del índice", "Mục nhập chỉ mục chính"),
    ("Footnote anchor", "Footnote Anchor", "脚注锚点", "Ancla de nota al pie", "Neo cước chú"),
    ("Endnote anchor", "Endnote Anchor", "尾注锚点", "Ancla de nota final", "Neo chú thích cuối"),
    ("Rubies", "Rubies", "字旁标音", "Texto ágata", "Ruby"),
    ("Vertical Numbering Symbols", "Vertical Numbering Symbols", "竖式编号符号", "Caracteres de numeración vertical", "Ký hiệu đánh số nằm dọc"),
    ("Emphasis", "Emphasis", "强调", "Destacado", "Nhấn mạnh"),
    ("Citation", "Quotation", "引用", "Cita", "Trích dẫn"),
    ("Strong Emphasis", "Strong Emphasis", "特别强调", "Destaque mayor", "Rất nhấn mạnh"),
    ("Source Text", "Source Text", "源文本", "Texto original", "Văn bản nguồn"),
    ("Example", "Example", "示例", "Ejemplo", "Ví dụ"),
    ("User Entry", "User Entry", "用户输入", "Entrada de usuario", "Mục nhập người dùng"),
    ("Variable", "Variable", "变量", "Variable", "Biến"),
    ("Definition", "Definition", "定义", "Definición", "Định nghĩa"),
    ("Teletype", "Teletype", "电传打字", "Teletipo", "Máy điện báo đánh chữ"),
];

pub(super) const PAGE: [Builtin; 10] = [
    ("Standard", "Default Page Style", "默认页面样式", "Estilo de página predeterminado", "Kiểu trang mặc định"),
    ("First Page", "First Page", "首页", "Primera página", "Trang đầu"),
    ("Left Page", "Left Page", "左页", "Página izquierda", "Trang trái"),
    ("Right Page", "Right Page", "右页", "Página derecha", "Trang phải"),
    ("Envelope", "Envelope", "信封", "Sobre", "Phong bì"),
    ("Index", "Index", "索引", "Índice", "Chỉ mục"),
    ("HTML", "HTML", "HTML", "HTML", "HTML"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "Cước chú"),
    ("Endnote", "Endnote", "尾注", "Nota final", "Chú thích cuối"),
    ("Landscape", "Landscape", "横向", "Horizontal", "Phong cảnh"),
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
            assert_eq!(names(|row| row.4), styles.len(), "Vietnamese");
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
    fn vietnamese_builtin_names_come_from_libreoffice(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("vi-VN", app);
            assert_eq!(super::shown(&PARAGRAPH, "Heading", app), "Tiêu đề");
            assert_eq!(super::shown(&PARAGRAPH, "Title", app), "Tựa đề");
            assert_eq!(super::shown(&PARAGRAPH, "Header", app), "Đầu trang");
            assert_eq!(super::shown(&PAGE, "Standard", app), "Kiểu trang mặc định");
            assert_eq!(super::shown(&PARAGRAPH, "Kiểu của tôi", app), "Kiểu của tôi");
        });
    }
}
