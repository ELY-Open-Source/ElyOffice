//! Writer's built-in style names in the interface language (I-03).

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

/// Kernel, English, Chinese, Spanish and Korean names.
pub(super) type Builtin = (&'static str, &'static str, &'static str, &'static str, &'static str);

/// The character default: outside the pool, never suffixed.
const NO_CHARACTER_STYLE: Builtin = ("Standard", "No Character Style", "无字符样式", "Ningún estilo de carácter", "문자 스타일 없음");

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
    if let Some((_, english, zh, es, ko)) = styles.iter().find(|(kernel, ..)| *kernel == name) {
        return match locale { "zh-CN" => *zh, "es-ES" => *es, "ko-KR" => *ko, _ => *english }.into();
    }
    if styles.iter().any(|row| row.1 == name && *row != NO_CHARACTER_STYLE) {
        return format!("{name}{USER}").into();
    }
    unmarked(name).to_string().into()
}

// From LibreOffice 26.2.6: SwStyleNameMapper, strings.hrc, zh-CN, es, ko po.
pub(super) const PARAGRAPH: [Builtin; 126] = [
    ("Standard", "Default Paragraph Style", "默认段落样式", "Estilo de párrafo predeterminado", "기본 단락 스타일"),
    ("Text body", "Body Text", "正文", "Cuerpo de texto", "본문"),
    ("First line indent", "First Line Indent", "首行缩进", "Sangría de primer renglón", "첫 줄 들여쓰기"),
    ("Hanging indent", "Hanging Indent", "悬挂缩进", "Sangría francesa", "끌어 맞춤"),
    ("Text body indent", "Body Text, Indented", "带缩进的正文", "Cuerpo de texto con sangría", "본문 들여쓰기"),
    ("Salutation", "Complimentary Close", "结尾敬语", "Frase de despedida", "인사말"),
    ("Signature", "Signature", "签名", "Firma", "서명"),
    ("List Indent", "List Indent", "列表缩进", "Confrontación", "목록 들여쓰기"),
    ("Marginalia", "Marginalia", "旁注", "Nota al margen", "여백"),
    ("List", "List", "列表", "Lista", "목록"),
    ("Numbering 1 Start", "Numbering 1 Start", "编号 1 起始", "Numeración 1 inicio", "번호 1 시작"),
    ("Numbering 1", "Numbering 1", "编号 1", "Numeración 1", "번호매기기 1"),
    ("Numbering 1 End", "Numbering 1 End", "编号 1 结束", "Numeración 1 fin", "번호 1 종료"),
    ("Numbering 1 Cont.", "Numbering 1 Cont.", "编号 1 续", "Numeración 1 continuación", "번호 1 계속"),
    ("Numbering 2 Start", "Numbering 2 Start", "编号 2 起始", "Numeración 2 inicio", "번호 2 시작"),
    ("Numbering 2", "Numbering 2", "编号 2", "Numeración 2", "번호매기기 2"),
    ("Numbering 2 End", "Numbering 2 End", "编号 2 结束", "Numeración 2 fin", "번호 2 종료"),
    ("Numbering 2 Cont.", "Numbering 2 Cont.", "编号 2 续", "Numeración 2 continuación", "번호 2 계속"),
    ("Numbering 3 Start", "Numbering 3 Start", "编号 3 起始", "Numeración 3 inicio", "번호 3 시작"),
    ("Numbering 3", "Numbering 3", "编号 3", "Numeración 3", "번호매기기 3"),
    ("Numbering 3 End", "Numbering 3 End", "编号 3 结束", "Numeración 3 fin", "번호 3 종료"),
    ("Numbering 3 Cont.", "Numbering 3 Cont.", "编号 3 续", "Numeración 3 continuación", "번호 3 계속"),
    ("Numbering 4 Start", "Numbering 4 Start", "编号 4 起始", "Numeración 4 inicio", "번호 4 시작"),
    ("Numbering 4", "Numbering 4", "编号 4", "Numeración 4", "번호매기기 4"),
    ("Numbering 4 End", "Numbering 4 End", "编号 4 结束", "Numeración 4 fin", "번호 4 종료"),
    ("Numbering 4 Cont.", "Numbering 4 Cont.", "编号 4 续", "Numeración 4 continuación", "번호 4 계속"),
    ("Numbering 5 Start", "Numbering 5 Start", "编号 5 起始", "Numeración 5 inicio", "번호 5 시작"),
    ("Numbering 5", "Numbering 5", "编号 5", "Numeración 5", "번호매기기 5"),
    ("Numbering 5 End", "Numbering 5 End", "编号 5 结束", "Numeración 5 fin", "번호 5 종료"),
    ("Numbering 5 Cont.", "Numbering 5 Cont.", "编号 5 续", "Numeración 5 continuación", "번호 5 계속"),
    ("List 1 Start", "List 1 Start", "列表 1 起始", "Lista 1 inicio", "목록 1 시작"),
    ("List 1", "List 1", "列表 1", "Lista 1", "목록 1"),
    ("List 1 End", "List 1 End", "列表 1 结束", "Lista 1 fin", "목록 1 종료"),
    ("List 1 Cont.", "List 1 Cont.", "列表 1 续", "Lista 1 continuación", "목록 1 계속"),
    ("List 2 Start", "List 2 Start", "列表 2 起始", "Lista 2 inicio", "목록 2 시작"),
    ("List 2", "List 2", "列表 2", "Lista 2", "목록 2"),
    ("List 2 End", "List 2 End", "列表 2 结束", "Lista 2 fin", "목록 2 종료"),
    ("List 2 Cont.", "List 2 Cont.", "列表 2 续", "Lista 2 continuación", "목록 2 계속"),
    ("List 3 Start", "List 3 Start", "列表 3 起始", "Lista 3 inicio", "목록 3 시작"),
    ("List 3", "List 3", "列表 3", "Lista 3", "목록 3"),
    ("List 3 End", "List 3 End", "列表 3 结束", "Lista 3 fin", "목록 3 종료"),
    ("List 3 Cont.", "List 3 Cont.", "列表 3 续", "Lista 3 continuación", "목록 3 계속"),
    ("List 4 Start", "List 4 Start", "列表 4 起始", "Lista 4 inicio", "목록 4 시작"),
    ("List 4", "List 4", "列表 4", "Lista 4", "목록 4"),
    ("List 4 End", "List 4 End", "列表 4 结束", "Lista 4 fin", "목록 4 종료"),
    ("List 4 Cont.", "List 4 Cont.", "列表 4 续", "Lista 4 continuación", "목록 4 계속"),
    ("List 5 Start", "List 5 Start", "列表 5 起始", "Lista 5 inicio", "목록 5 시작"),
    ("List 5", "List 5", "列表 5", "Lista 5", "목록 5"),
    ("List 5 End", "List 5 End", "列表 5 结束", "Lista 5 fin", "목록 5 종료"),
    ("List 5 Cont.", "List 5 Cont.", "列表 5 续", "Lista 5 continuación", "목록 5 계속"),
    ("Header and Footer", "Header and Footer", "页眉与页脚", "Cabecera y pie", "머리글 및 바닥글"),
    ("Header", "Header", "页眉", "Cabecera", "머리글"),
    ("Header left", "Header Left", "页眉左侧", "Cabecera izquierda", "머리글 왼쪽"),
    ("Header right", "Header Right", "页眉右侧", "Cabecera derecha", "머리글 오른쪽"),
    ("Footer", "Footer", "页脚", "Pie de página", "바닥글"),
    ("Footer left", "Footer Left", "页脚左侧", "Pie de página izquierdo", "바닥글 왼쪽"),
    ("Footer right", "Footer Right", "页脚右侧", "Pie de página derecho", "바닥글 오른쪽"),
    ("Table Contents", "Table Contents", "表格内容", "Contenido de la tabla", "표 내용"),
    ("Table Heading", "Table Heading", "表格标题", "Título de la tabla", "표제목"),
    ("Caption", "Caption", "题注", "Leyenda", "캡션"),
    ("Illustration", "Illustration", "插图", "Ilustración", "사진"),
    ("Table", "Table", "表格", "Tabla", "표"),
    ("Text", "Text", "文字", "Texto", "텍스트"),
    ("Figure", "Figure", "图示", "Figura", "그림"),
    ("Frame contents", "Frame Contents", "框架内容", "Contenido del marco", "프레임 내용"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "각주"),
    ("Addressee", "Addressee", "收件人", "Destinatario", "받는 사람"),
    ("Sender", "Sender", "发件人", "Remitente", "보내는 사람"),
    ("Endnote", "Endnote", "尾注", "Nota final", "미주"),
    ("Drawing", "Drawing", "绘图", "Dibujo", "그리기"),
    ("Comment", "Comment", "批注", "Comentario", "메모"),
    ("Index", "Index", "索引", "Índice", "색인"),
    ("Index Heading", "Index Heading", "索引标题", "Título del índice", "색인 제목"),
    ("Index 1", "Index 1", "索引 1", "Índice 1", "색인 1"),
    ("Index 2", "Index 2", "索引 2", "Índice 2", "색인 2"),
    ("Index 3", "Index 3", "索引 3", "Índice 3", "색인 3"),
    ("Index Separator", "Index Separator", "索引分隔符", "Separador del índice", "색인 구분자"),
    ("Contents Heading", "Contents Heading", "目录标题", "Título del sumario", "목차 제목"),
    ("Contents 1", "Contents 1", "目录 1", "Sumario 1", "목차 1"),
    ("Contents 2", "Contents 2", "目录 2", "Sumario 2", "목차 2"),
    ("Contents 3", "Contents 3", "目录 3", "Sumario 3", "목차 3"),
    ("Contents 4", "Contents 4", "目录 4", "Sumario 4", "목차 4"),
    ("Contents 5", "Contents 5", "目录 5", "Sumario 5", "목차 5"),
    ("User Index Heading", "User Index Heading", "自定义索引标题", "Título del índice de usuario", "사용자 색인 제목"),
    ("User Index 1", "User Index 1", "自定义索引 1", "Índice de usuario 1", "사용자 색인 1"),
    ("User Index 2", "User Index 2", "自定义索引 2", "Índice de usuario 2", "사용자 색인 2"),
    ("User Index 3", "User Index 3", "自定义索引 3", "Índice de usuario 3", "사용자 색인 3"),
    ("User Index 4", "User Index 4", "自定义索引 4", "Índice de usuario 4", "사용자 색인 4"),
    ("User Index 5", "User Index 5", "自定义索引 5", "Índice de usuario 5", "사용자 색인 5"),
    ("Contents 6", "Contents 6", "目录 6", "Sumario 6", "목차 6"),
    ("Contents 7", "Contents 7", "目录 7", "Sumario 7", "목차 7"),
    ("Contents 8", "Contents 8", "目录 8", "Sumario 8", "목차 8"),
    ("Contents 9", "Contents 9", "目录 9", "Sumario 9", "목차 9"),
    ("Contents 10", "Contents 10", "目录 10", "Sumario 10", "목차 10"),
    ("Figure Index Heading", "Figure Index Heading", "图示索引标题", "Título del índice de figuras", "그림 색인 머릿말"),
    ("Figure Index 1", "Figure Index 1", "图示索引 1", "Índice de figuras 1", "그림 색인 1"),
    ("Object index heading", "Object Index Heading", "对象索引标题", "Título del índice de objetos", "개체 색인 제목"),
    ("Object index 1", "Object Index 1", "对象索引 1", "Índice de objetos 1", "개체 색인 1"),
    ("Table index heading", "Table Index Heading", "表格索引标题", "Título del índice de tablas", "표 색인 제목"),
    ("Table index 1", "Table Index 1", "表格索引 1", "Índice de tablas 1", "표 색인 1"),
    ("Bibliography Heading", "Bibliography Heading", "参考文献标题", "Título de la bibliografía", "참고문헌 제목"),
    ("Bibliography 1", "Bibliography 1", "参考文献 1", "Bibliografía 1", "참고 문헌 1"),
    ("User Index 6", "User Index 6", "自定义索引 6", "Índice de usuario 6", "사용자 색인 6"),
    ("User Index 7", "User Index 7", "自定义索引 7", "Índice de usuario 7", "사용자 색인 7"),
    ("User Index 8", "User Index 8", "自定义索引 8", "Índice de usuario 8", "사용자 색인 8"),
    ("User Index 9", "User Index 9", "自定义索引 9", "Índice de usuario 9", "사용자 색인 9"),
    ("User Index 10", "User Index 10", "自定义索引 10", "Índice de usuario 10", "사용자 색인 10"),
    ("Title", "Title", "标题", "Título general", "문서 제목"),
    ("Subtitle", "Subtitle", "副标题", "Subtítulo", "부제"),
    ("Appendix", "Appendix", "附录", "Apéndice", "부록"),
    ("Heading", "Heading", "标题样式", "Título", "제목"),
    ("Heading 1", "Heading 1", "标题 1", "Título 1", "제목 1"),
    ("Heading 2", "Heading 2", "标题 2", "Título 2", "제목 2"),
    ("Heading 3", "Heading 3", "标题 3", "Título 3", "제목 3"),
    ("Heading 4", "Heading 4", "标题 4", "Título 4", "제목 4"),
    ("Heading 5", "Heading 5", "标题 5", "Título 5", "제목 5"),
    ("Heading 6", "Heading 6", "标题 6", "Título 6", "제목 6"),
    ("Heading 7", "Heading 7", "标题 7", "Título 7", "제목 7"),
    ("Heading 8", "Heading 8", "标题 8", "Título 8", "제목 8"),
    ("Heading 9", "Heading 9", "标题 9", "Título 9", "제목 9"),
    ("Heading 10", "Heading 10", "标题 10", "Título 10", "제목 10"),
    ("Quotations", "Block Quotation", "成段引文", "Cita en bloque", "인용 블록"),
    ("Preformatted Text", "Preformatted Text", "预设定格式的文本", "Texto preformateado", "사전 서식 지정된 텍스트"),
    ("Horizontal Line", "Horizontal Line", "水平线", "Línea horizontal", "수평 선"),
    ("List Contents", "List Contents", "列表内容", "Contenido de lista", "목록 내용"),
    ("List Heading", "List Heading", "列表标题", "Título de la lista", "목록 제목"),
];

pub(super) const CHARACTER: [Builtin; 27] = [
    NO_CHARACTER_STYLE,
    ("Footnote Symbol", "Footnote Characters", "脚注符", "Caracteres de nota al pie", "각주 기호"),
    ("Page Number", "Page Number", "页码", "Número de página", "페이지 번호"),
    ("Caption characters", "Caption Characters", "题注字符", "Caracteres de leyenda", "캡션 문자"),
    ("Drop Caps", "Drop Caps", "首字下沉", "Letras capitulares", "단락의 첫문자 장식"),
    ("Numbering Symbols", "Numbering Symbols", "编号符号", "Símbolos de numeración", "번호 매기기 기호"),
    ("Bullet Symbols", "Bullets", "项目符号", "Bolos", "글머리 기호"),
    ("Internet link", "Internet Link", "Internet 链接", "Enlace de Internet", "인터넷 링크"),
    ("Visited Internet Link", "Visited Internet Link", "访问过的 Internet 链接", "Enlace de Internet visitado", "방문한 인터넷 링크"),
    ("Placeholder", "Placeholder", "占位符", "Sustitutivo", "플레이스홀더"),
    ("Index Link", "Index Link", "索引链接", "Enlace del índice", "색인 연결"),
    ("Endnote Symbol", "Endnote Characters", "尾注符", "Caracteres de nota final", "미주 기호"),
    ("Line numbering", "Line Numbering", "行号", "Numeración de renglones", "행 번호 매기기"),
    ("Main index entry", "Main Index Entry", "主索引条目", "Entrada principal del índice", "주 항목 색인"),
    ("Footnote anchor", "Footnote Anchor", "脚注锚点", "Ancla de nota al pie", "각주 기준 위치"),
    ("Endnote anchor", "Endnote Anchor", "尾注锚点", "Ancla de nota final", "미주 고정 기준"),
    ("Rubies", "Rubies", "字旁标音", "Texto ágata", "윗주 달기"),
    ("Vertical Numbering Symbols", "Vertical Numbering Symbols", "竖式编号符号", "Caracteres de numeración vertical", "세로 번호 매기기 기호"),
    ("Emphasis", "Emphasis", "强调", "Destacado", "강조"),
    ("Citation", "Quotation", "引用", "Cita", "인용"),
    ("Strong Emphasis", "Strong Emphasis", "特别强调", "Destaque mayor", "강한 강조"),
    ("Source Text", "Source Text", "源文本", "Texto original", "원본 텍스트"),
    ("Example", "Example", "示例", "Ejemplo", "예"),
    ("User Entry", "User Entry", "用户输入", "Entrada de usuario", "사용자 항목"),
    ("Variable", "Variable", "变量", "Variable", "변수"),
    ("Definition", "Definition", "定义", "Definición", "정의"),
    ("Teletype", "Teletype", "电传打字", "Teletipo", "텔레타이프"),
];

pub(super) const PAGE: [Builtin; 10] = [
    ("Standard", "Default Page Style", "默认页面样式", "Estilo de página predeterminado", "기본 페이지 스타일"),
    ("First Page", "First Page", "首页", "Primera página", "첫 페이지"),
    ("Left Page", "Left Page", "左页", "Página izquierda", "왼쪽 페이지"),
    ("Right Page", "Right Page", "右页", "Página derecha", "오른쪽 페이지"),
    ("Envelope", "Envelope", "信封", "Sobre", "봉투"),
    ("Index", "Index", "索引", "Índice", "색인"),
    ("HTML", "HTML", "HTML", "HTML", "HTML"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "각주"),
    ("Endnote", "Endnote", "尾注", "Nota final", "미주"),
    ("Landscape", "Landscape", "横向", "Horizontal", "가로 방향"),
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
            assert_eq!(names(|row| row.4), styles.len(), "Korean");
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
    fn korean_builtin_names_take_writers_korean(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("ko-KR", app);
            assert_eq!(super::shown(&PARAGRAPH, "Heading 1", app), "제목 1");
            assert_eq!(super::shown(&PARAGRAPH, "Header", app), "머리글");
            assert_eq!(super::shown(&PARAGRAPH, "Title", app), "문서 제목");
            assert_eq!(super::shown(&CHARACTER, "Standard", app), "문자 스타일 없음");
            assert_eq!(super::shown(&PARAGRAPH, "내 스타일", app), "내 스타일");
        });
    }
}
