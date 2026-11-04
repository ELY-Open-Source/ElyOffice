//! Writer's built-in style names in the interface language (I-03).

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

/// Kernel, English, then each language's names.
pub(super) type Builtin = (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str);

/// The character default: outside the pool, never suffixed.
const NO_CHARACTER_STYLE: Builtin = ("Standard", "No Character Style", "无字符样式", "Ningún estilo de carácter", "文字スタイルなし", "문자 스타일 없음", "Không có kiểu ký tự", "Aucun style de caractère", "Keine Zeichenvorlage");

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
    if let Some((_, english, zh, es, ja, ko, vi, fr, de)) = styles.iter().find(|(kernel, ..)| *kernel == name) {
        return match locale { "zh-CN" => *zh, "es-ES" => *es, "ja-JP" => *ja, "ko-KR" => *ko, "vi-VN" => *vi, "fr-FR" => *fr, "de-DE" => *de, _ => *english }.into();
    }
    if styles.iter().any(|row| row.1 == name && *row != NO_CHARACTER_STYLE) {
        return format!("{name}{USER}").into();
    }
    unmarked(name).to_string().into()
}

// LibreOffice 26.2.6: SwStyleNameMapper, strings.hrc and its translations.
pub(super) static PARAGRAPH: [Builtin; 126] = [
    ("Standard", "Default Paragraph Style", "默认段落样式", "Estilo de párrafo predeterminado", "標準段落スタイル", "기본 단락 스타일", "Kiểu đoạn văn mặc định", "Style de paragraphe par défaut", "Standard"),
    ("Text body", "Body Text", "正文", "Cuerpo de texto", "本文", "본문", "Thân văn bản", "Corps de texte", "Fließtext"),
    ("First line indent", "First Line Indent", "首行缩进", "Sangría de primer renglón", "段落字下げ", "첫 줄 들여쓰기", "Căn lề dòng đầu tiên", "Retrait de première ligne", "Einzug erste Zeile"),
    ("Hanging indent", "Hanging Indent", "悬挂缩进", "Sangría francesa", "ぶら下げインデント", "끌어 맞춤", "Thụt đầu dòng treo", "Alinéa négatif", "Hängender Einzug"),
    ("Text body indent", "Body Text, Indented", "带缩进的正文", "Cuerpo de texto con sangría", "本文インデント", "본문 들여쓰기", "Thân văn bản, thụt lề", "Corps de texte avec retrait", "Fließtext, eingerückt"),
    ("Salutation", "Complimentary Close", "结尾敬语", "Frase de despedida", "結語", "인사말", "Lời kết", "Formule finale", "Grußformel"),
    ("Signature", "Signature", "签名", "Firma", "署名", "서명", "Chữ ký", "Signature", "Unterschrift"),
    ("List Indent", "List Indent", "列表缩进", "Confrontación", "箇条書きインデント", "목록 들여쓰기", "Căn lề danh sách", "Retrait de liste", "Gegenüberstellung"),
    ("Marginalia", "Marginalia", "旁注", "Nota al margen", "傍注", "여백", "Ghi chú ở lề", "Note en marge", "Marginalie"),
    ("List", "List", "列表", "Lista", "箇条書き", "목록", "Danh sách", "Liste", "Aufzählung"),
    ("Numbering 1 Start", "Numbering 1 Start", "编号 1 起始", "Numeración 1 inicio", "番号付け 1 始め", "번호 1 시작", "Bắt đầu đánh số 1", "Numérotation 1 début", "Nummerierung 1 Anfang"),
    ("Numbering 1", "Numbering 1", "编号 1", "Numeración 1", "番号付け 1", "번호매기기 1", "Đánh số 1", "Numérotation 1", "Nummerierung 1"),
    ("Numbering 1 End", "Numbering 1 End", "编号 1 结束", "Numeración 1 fin", "番号付け 1 終り", "번호 1 종료", "Kết thúc đánh số 1", "Numérotation 1 fin", "Nummerierung 1 Ende"),
    ("Numbering 1 Cont.", "Numbering 1 Cont.", "编号 1 续", "Numeración 1 continuación", "番号付け 1 続き", "번호 1 계속", "Tiếp tục đánh số 1", "Numérotation 1 suite", "Nummerierung 1 Fortsetzung"),
    ("Numbering 2 Start", "Numbering 2 Start", "编号 2 起始", "Numeración 2 inicio", "番号付け 2 始め", "번호 2 시작", "Bắt đầu đánh số 2", "Numérotation 2 début", "Nummerierung 2 Anfang"),
    ("Numbering 2", "Numbering 2", "编号 2", "Numeración 2", "番号付け 2", "번호매기기 2", "Đánh số 2", "Numérotation 2", "Nummerierung 2"),
    ("Numbering 2 End", "Numbering 2 End", "编号 2 结束", "Numeración 2 fin", "番号付け 2 終り", "번호 2 종료", "Kết thúc đánh số 2", "Numérotation 2 fin", "Nummerierung 2 Ende"),
    ("Numbering 2 Cont.", "Numbering 2 Cont.", "编号 2 续", "Numeración 2 continuación", "番号付け 2 続き", "번호 2 계속", "Tiếp tục đánh số 2", "Numérotation 2 suite", "Nummerierung 2 Fortsetzung"),
    ("Numbering 3 Start", "Numbering 3 Start", "编号 3 起始", "Numeración 3 inicio", "番号付け 3 始め", "번호 3 시작", "Bắt đầu đánh số 3", "Numérotation 3 début", "Nummerierung 3 Anfang"),
    ("Numbering 3", "Numbering 3", "编号 3", "Numeración 3", "番号付け 3", "번호매기기 3", "Đánh số 3", "Numérotation 3", "Nummerierung 3"),
    ("Numbering 3 End", "Numbering 3 End", "编号 3 结束", "Numeración 3 fin", "番号付け 3 終り", "번호 3 종료", "Kết thúc đánh số 3", "Numérotation 3 fin", "Nummerierung 3 Ende"),
    ("Numbering 3 Cont.", "Numbering 3 Cont.", "编号 3 续", "Numeración 3 continuación", "番号付け 3 続き", "번호 3 계속", "Tiếp tục đánh số 3", "Numérotation 3 suite", "Nummerierung 3 Fortsetzung"),
    ("Numbering 4 Start", "Numbering 4 Start", "编号 4 起始", "Numeración 4 inicio", "番号付け 4 始め", "번호 4 시작", "Bắt đầu đánh số 4", "Numérotation 4 début", "Nummerierung 4 Anfang"),
    ("Numbering 4", "Numbering 4", "编号 4", "Numeración 4", "番号付け 4", "번호매기기 4", "Đánh số 4", "Numérotation 4", "Nummerierung 4"),
    ("Numbering 4 End", "Numbering 4 End", "编号 4 结束", "Numeración 4 fin", "番号付け 4 終り", "번호 4 종료", "Kết thúc đánh số 4", "Numérotation 4 fin", "Nummerierung 4 Ende"),
    ("Numbering 4 Cont.", "Numbering 4 Cont.", "编号 4 续", "Numeración 4 continuación", "番号付け 4 続き", "번호 4 계속", "Tiếp tục đánh số 4", "Numérotation 4 suite", "Nummerierung 4 Fortsetzung"),
    ("Numbering 5 Start", "Numbering 5 Start", "编号 5 起始", "Numeración 5 inicio", "番号付け 5 始め", "번호 5 시작", "Bắt đầu đánh số 5", "Numérotation 5 début", "Nummerierung 5 Anfang"),
    ("Numbering 5", "Numbering 5", "编号 5", "Numeración 5", "番号付け 5", "번호매기기 5", "Đánh số 5", "Numérotation 5", "Nummerierung 5"),
    ("Numbering 5 End", "Numbering 5 End", "编号 5 结束", "Numeración 5 fin", "番号付け 5 終り", "번호 5 종료", "Kết thúc đánh số 5", "Numérotation 5 fin", "Nummerierung 5 Ende"),
    ("Numbering 5 Cont.", "Numbering 5 Cont.", "编号 5 续", "Numeración 5 continuación", "番号付け 5 続き", "번호 5 계속", "Tiếp tục đánh số 5", "Numérotation 5 suite", "Nummerierung 5 Fortsetzung"),
    ("List 1 Start", "List 1 Start", "列表 1 起始", "Lista 1 inicio", "箇条書き1始め", "목록 1 시작", "Bắt đầu danh sách 1", "Liste 1 début", "Aufzählung 1 Anfang"),
    ("List 1", "List 1", "列表 1", "Lista 1", "箇条書き1", "목록 1", "Danh sách 1", "Liste 1", "Aufzählung 1"),
    ("List 1 End", "List 1 End", "列表 1 结束", "Lista 1 fin", "箇条書き1終り", "목록 1 종료", "Kết thúc danh sách 1", "Liste 1 fin", "Aufzählung 1 Ende"),
    ("List 1 Cont.", "List 1 Cont.", "列表 1 续", "Lista 1 continuación", "箇条書き1続き", "목록 1 계속", "Tiếp tục danh sách 1", "Liste 1 suite", "Aufzählung 1 Fortsetzung"),
    ("List 2 Start", "List 2 Start", "列表 2 起始", "Lista 2 inicio", "箇条書き2始め", "목록 2 시작", "Bắt đầu danh sách 2", "Liste 2 début", "Aufzählung 2 Anfang"),
    ("List 2", "List 2", "列表 2", "Lista 2", "箇条書き2", "목록 2", "Danh sách 2", "Liste 2", "Aufzählung 2"),
    ("List 2 End", "List 2 End", "列表 2 结束", "Lista 2 fin", "箇条書き2終り", "목록 2 종료", "Kết thúc danh sách 2", "Liste 2 fin", "Aufzählung 2 Ende"),
    ("List 2 Cont.", "List 2 Cont.", "列表 2 续", "Lista 2 continuación", "箇条書き2続き", "목록 2 계속", "Tiếp tục danh sách 2", "Liste 2 suite", "Aufzählung 2 Fortsetzung"),
    ("List 3 Start", "List 3 Start", "列表 3 起始", "Lista 3 inicio", "箇条書き3始め", "목록 3 시작", "Bắt đầu danh sách 3", "Liste 3 début", "Aufzählung 3 Anfang"),
    ("List 3", "List 3", "列表 3", "Lista 3", "箇条書き3", "목록 3", "Danh sách 3", "Liste 3", "Aufzählung 3"),
    ("List 3 End", "List 3 End", "列表 3 结束", "Lista 3 fin", "箇条書き3終り", "목록 3 종료", "Kết thúc danh sách 3", "Liste 3 fin", "Aufzählung 3 Ende"),
    ("List 3 Cont.", "List 3 Cont.", "列表 3 续", "Lista 3 continuación", "箇条書き3続き", "목록 3 계속", "Tiếp tục danh sách 3", "Liste 3 suite", "Aufzählung 3 Fortsetzung"),
    ("List 4 Start", "List 4 Start", "列表 4 起始", "Lista 4 inicio", "箇条書き4始め", "목록 4 시작", "Bắt đầu danh sách 4", "Liste 4 début", "Aufzählung 4 Anfang"),
    ("List 4", "List 4", "列表 4", "Lista 4", "箇条書き4", "목록 4", "Danh sách 4", "Liste 4", "Aufzählung 4"),
    ("List 4 End", "List 4 End", "列表 4 结束", "Lista 4 fin", "箇条書き4終り", "목록 4 종료", "Kết thúc danh sách 4", "Liste 4 fin", "Aufzählung 4 Ende"),
    ("List 4 Cont.", "List 4 Cont.", "列表 4 续", "Lista 4 continuación", "箇条書き4続き", "목록 4 계속", "Tiếp tục danh sách 4", "Liste 4 suite", "Aufzählung 4 Fortsetzung"),
    ("List 5 Start", "List 5 Start", "列表 5 起始", "Lista 5 inicio", "箇条書き5始め", "목록 5 시작", "Bắt đầu danh sách 5", "Liste 5 début", "Aufzählung 5 Anfang"),
    ("List 5", "List 5", "列表 5", "Lista 5", "箇条書き5", "목록 5", "Danh sách 5", "Liste 5", "Aufzählung 5"),
    ("List 5 End", "List 5 End", "列表 5 结束", "Lista 5 fin", "箇条書き5終り", "목록 5 종료", "Kết thúc danh sách 5", "Liste 5 fin", "Aufzählung 5 Ende"),
    ("List 5 Cont.", "List 5 Cont.", "列表 5 续", "Lista 5 continuación", "箇条書き5続き", "목록 5 계속", "Tiếp tục danh sách 5", "Liste 5 suite", "Aufzählung 5 Fortsetzung"),
    ("Header and Footer", "Header and Footer", "页眉与页脚", "Cabecera y pie", "ヘッダーとフッター", "머리글 및 바닥글", "Đầu trang và chân trang", "En-tête et pied de page", "Kopf-/Fußzeile"),
    ("Header", "Header", "页眉", "Cabecera", "ヘッダー", "머리글", "Đầu trang", "En-tête", "Kopfzeile"),
    ("Header left", "Header Left", "页眉左侧", "Cabecera izquierda", "ヘッダー左", "머리글 왼쪽", "Đầu trang bên trái", "En-tête gauche", "Kopfzeile links"),
    ("Header right", "Header Right", "页眉右侧", "Cabecera derecha", "ヘッダー右", "머리글 오른쪽", "Đầu trang bên phải", "En-tête droit", "Kopfzeile rechts"),
    ("Footer", "Footer", "页脚", "Pie de página", "フッター", "바닥글", "Chân trang", "Pied de page", "Fußzeile"),
    ("Footer left", "Footer Left", "页脚左侧", "Pie de página izquierdo", "フッター 左", "바닥글 왼쪽", "Chân trang bên trái", "Pied de page gauche", "Fußzeile links"),
    ("Footer right", "Footer Right", "页脚右侧", "Pie de página derecho", "フッター 右", "바닥글 오른쪽", "Chân trang bên phải", "Pied de page droit", "Fußzeile rechts"),
    ("Table Contents", "Table Contents", "表格内容", "Contenido de la tabla", "表の内容", "표 내용", "Nội dung bảng", "Contenu de tableau", "Tabelleninhalt"),
    ("Table Heading", "Table Heading", "表格标题", "Título de la tabla", "表の見出し", "표제목", "Tiêu đề bảng", "Titre de tableau", "Tabellenüberschrift"),
    ("Caption", "Caption", "题注", "Leyenda", "キャプション", "캡션", "Phụ đề", "Légende", "Beschriftung"),
    ("Illustration", "Illustration", "插图", "Ilustración", "イラストレーション", "사진", "Hình", "Illustration", "Abbildung"),
    ("Table", "Table", "表格", "Tabla", "表", "표", "Bảng", "Tableau", "Tabelle"),
    ("Text", "Text", "文字", "Texto", "テキスト", "텍스트", "Văn bản", "Texte", "Text"),
    ("Figure", "Figure", "图示", "Figura", "図", "그림", "Hình minh họa", "Figure", "Schaubild"),
    ("Frame contents", "Frame Contents", "框架内容", "Contenido del marco", "枠の内容", "프레임 내용", "Nội dung khung", "Contenu de cadre", "Rahmeninhalt"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "脚注", "각주", "Cước chú", "Note de bas de page", "Fußnote"),
    ("Addressee", "Addressee", "收件人", "Destinatario", "宛先", "받는 사람", "Người nhận", "Destinataire", "Empfänger"),
    ("Sender", "Sender", "发件人", "Remitente", "差出人", "보내는 사람", "Người gửi", "Expéditeur", "Absender"),
    ("Endnote", "Endnote", "尾注", "Nota final", "文末脚注", "미주", "Chú thích cuối", "Note de fin", "Endnote"),
    ("Drawing", "Drawing", "绘图", "Dibujo", "図版", "그리기", "Bản vẽ", "Dessin", "Zeichnung"),
    ("Comment", "Comment", "批注", "Comentario", "コメント", "메모", "Bình luận", "Commentaire", "Kommentar"),
    ("Index", "Index", "索引", "Índice", "索引", "색인", "Chỉ mục", "Index", "Verzeichnis"),
    ("Index Heading", "Index Heading", "索引标题", "Título del índice", "索引 見出し", "색인 제목", "Tiêu đề chỉ mục", "Index lexical : titre", "Stichwortverzeichnis Überschrift"),
    ("Index 1", "Index 1", "索引 1", "Índice 1", "索引 1", "색인 1", "Chỉ mục 1", "Index lexical 1", "Stichwortverzeichnis 1"),
    ("Index 2", "Index 2", "索引 2", "Índice 2", "索引 2", "색인 2", "Chỉ mục 2", "Index lexical 2", "Stichwortverzeichnis 2"),
    ("Index 3", "Index 3", "索引 3", "Índice 3", "索引 3", "색인 3", "Chỉ mục 3", "Index lexical 3", "Stichwortverzeichnis 3"),
    ("Index Separator", "Index Separator", "索引分隔符", "Separador del índice", "索引 セパレーター", "색인 구분자", "Dấu tách chỉ mục", "Index lexical : séparateur", "Stichwortverzeichnis Trennzeichen"),
    ("Contents Heading", "Contents Heading", "目录标题", "Título del sumario", "目次見出し", "목차 제목", "Tiêu đề nội dung", "Titre de table des matières", "Inhaltsverzeichnis Überschrift"),
    ("Contents 1", "Contents 1", "目录 1", "Sumario 1", "目次 1", "목차 1", "Nội dung 1", "Table des matières niveau 1", "Inhaltsverzeichnis 1"),
    ("Contents 2", "Contents 2", "目录 2", "Sumario 2", "目次 2", "목차 2", "Nội dung 2", "Table des matières niveau 2", "Inhaltsverzeichnis 2"),
    ("Contents 3", "Contents 3", "目录 3", "Sumario 3", "目次 3", "목차 3", "Nội dung 3", "Table des matières niveau 3", "Inhaltsverzeichnis 3"),
    ("Contents 4", "Contents 4", "目录 4", "Sumario 4", "目次 4", "목차 4", "Nội dung 4", "Table des matières niveau 4", "Inhaltsverzeichnis 4"),
    ("Contents 5", "Contents 5", "目录 5", "Sumario 5", "目次 5", "목차 5", "Nội dung 5", "Table des matières niveau 5", "Inhaltsverzeichnis 5"),
    ("User Index Heading", "User Index Heading", "自定义索引标题", "Título del índice de usuario", "ユーザー定義の索引見出し", "사용자 색인 제목", "Tiêu đề chỉ mục người dùng", "Titre d'index personnalisé", "Benutzerdefiniertes Verzeichnis Überschrift"),
    ("User Index 1", "User Index 1", "自定义索引 1", "Índice de usuario 1", "ユーザー定義の索引 1", "사용자 색인 1", "Chỉ mục người dùng 1", "Index personnalisé 1", "Benutzerdefiniertes Verzeichnis 1"),
    ("User Index 2", "User Index 2", "自定义索引 2", "Índice de usuario 2", "ユーザー定義の索引 2", "사용자 색인 2", "Chỉ mục người dùng 2", "Index personnalisé 2", "Benutzerdefiniertes Verzeichnis 2"),
    ("User Index 3", "User Index 3", "自定义索引 3", "Índice de usuario 3", "ユーザー定義の索引 3", "사용자 색인 3", "Chỉ mục người dùng 3", "Index personnalisé 3", "Benutzerdefiniertes Verzeichnis 3"),
    ("User Index 4", "User Index 4", "自定义索引 4", "Índice de usuario 4", "ユーザー定義の索引 4", "사용자 색인 4", "Chỉ mục người dùng 4", "Index personnalisé 4", "Benutzerdefiniertes Verzeichnis 4"),
    ("User Index 5", "User Index 5", "自定义索引 5", "Índice de usuario 5", "ユーザー定義の索引 5", "사용자 색인 5", "Chỉ mục người dùng 5", "Index personnalisé 5", "Benutzerdefiniertes Verzeichnis 5"),
    ("Contents 6", "Contents 6", "目录 6", "Sumario 6", "目次 6", "목차 6", "Nội dung 6", "Table des matières niveau 6", "Inhaltsverzeichnis 6"),
    ("Contents 7", "Contents 7", "目录 7", "Sumario 7", "目次 7", "목차 7", "Nội dung 7", "Table des matières niveau 7", "Inhaltsverzeichnis 7"),
    ("Contents 8", "Contents 8", "目录 8", "Sumario 8", "目次 8", "목차 8", "Nội dung 8", "Table des matières niveau 8", "Inhaltsverzeichnis 8"),
    ("Contents 9", "Contents 9", "目录 9", "Sumario 9", "目次 9", "목차 9", "Nội dung 9", "Table des matières niveau 9", "Inhaltsverzeichnis 9"),
    ("Contents 10", "Contents 10", "目录 10", "Sumario 10", "目次 10", "목차 10", "Nội dung 10", "Table des matières niveau 10", "Inhaltsverzeichnis 10"),
    ("Figure Index Heading", "Figure Index Heading", "图示索引标题", "Título del índice de figuras", "図索引見出し", "그림 색인 머릿말", "Tiêu đề chỉ mục hình minh họa", "Titre de l'index des figures", "Abbildungsverzeichnis Überschriften"),
    ("Figure Index 1", "Figure Index 1", "图示索引 1", "Índice de figuras 1", "図索引 1", "그림 색인 1", "Chỉ mục hình minh họa 1", "Index des figures 1", "Abbildungsverzeichnis 1"),
    ("Object index heading", "Object Index Heading", "对象索引标题", "Título del índice de objetos", "オブジェクト索引見出し", "개체 색인 제목", "Tiêu đề chỉ mục đối tượng", "Titre d'index d'objets", "Objektverzeichnis Überschrift"),
    ("Object index 1", "Object Index 1", "对象索引 1", "Índice de objetos 1", "オブジェクト索引 1", "개체 색인 1", "Chỉ mục đối tượng 1", "Index d'objets 1", "Objektverzeichnis 1"),
    ("Table index heading", "Table Index Heading", "表格索引标题", "Título del índice de tablas", "表索引見出し", "표 색인 제목", "Tiêu đề chỉ mục bảng", "Titre d'index de tableaux", "Tabellenverzeichnis Überschrift"),
    ("Table index 1", "Table Index 1", "表格索引 1", "Índice de tablas 1", "表索引 1", "표 색인 1", "Chỉ mục bảng 1", "Index de tableaux 1", "Tabellenverzeichnis 1"),
    ("Bibliography Heading", "Bibliography Heading", "参考文献标题", "Título de la bibliografía", "参考文献表見出し", "참고문헌 제목", "Tiêu đề Thư tịch", "Titre de bibliographie", "Literaturverzeichnis Überschrift"),
    ("Bibliography 1", "Bibliography 1", "参考文献 1", "Bibliografía 1", "参考文献表 1", "참고 문헌 1", "Thư tịch 1", "Bibliographie 1", "Literaturverzeichnis 1"),
    ("User Index 6", "User Index 6", "自定义索引 6", "Índice de usuario 6", "ユーザー定義の索引 6", "사용자 색인 6", "Chỉ mục người dùng 6", "Index personnalisé 6", "Benutzerdefiniertes Verzeichnis 6"),
    ("User Index 7", "User Index 7", "自定义索引 7", "Índice de usuario 7", "ユーザー定義の索引 7", "사용자 색인 7", "Chỉ mục người dùng 7", "Index personnalisé 7", "Benutzerdefiniertes Verzeichnis 7"),
    ("User Index 8", "User Index 8", "自定义索引 8", "Índice de usuario 8", "ユーザー定義の索引 8", "사용자 색인 8", "Chỉ mục người dùng 8", "Index personnalisé 8", "Benutzerdefiniertes Verzeichnis 8"),
    ("User Index 9", "User Index 9", "自定义索引 9", "Índice de usuario 9", "ユーザー定義の索引 9", "사용자 색인 9", "Chỉ mục người dùng 9", "Index personnalisé 9", "Benutzerdefiniertes Verzeichnis 9"),
    ("User Index 10", "User Index 10", "自定义索引 10", "Índice de usuario 10", "ユーザー定義の索引 10", "사용자 색인 10", "Chỉ mục người dùng 10", "Index personnalisé 10", "Benutzerdefiniertes Verzeichnis 10"),
    ("Title", "Title", "标题", "Título general", "表題", "문서 제목", "Tựa đề", "Titre principal", "Titel"),
    ("Subtitle", "Subtitle", "副标题", "Subtítulo", "副題", "부제", "Tiêu đề phụ", "Sous-titre", "Untertitel"),
    ("Appendix", "Appendix", "附录", "Apéndice", "付録", "부록", "Phụ lục", "Annexe", "Anhang"),
    ("Heading", "Heading", "标题样式", "Título", "見出し", "제목", "Tiêu đề", "Titre", "Überschrift"),
    ("Heading 1", "Heading 1", "标题 1", "Título 1", "見出し1", "제목 1", "Tiêu đề 1", "Titre 1", "Überschrift 1"),
    ("Heading 2", "Heading 2", "标题 2", "Título 2", "見出し2", "제목 2", "Tiêu đề 2", "Titre 2", "Überschrift 2"),
    ("Heading 3", "Heading 3", "标题 3", "Título 3", "見出し3", "제목 3", "Tiêu đề 3", "Titre 3", "Überschrift 3"),
    ("Heading 4", "Heading 4", "标题 4", "Título 4", "見出し4", "제목 4", "Tiêu đề 4", "Titre 4", "Überschrift 4"),
    ("Heading 5", "Heading 5", "标题 5", "Título 5", "見出し5", "제목 5", "Tiêu đề 5", "Titre 5", "Überschrift 5"),
    ("Heading 6", "Heading 6", "标题 6", "Título 6", "見出し6", "제목 6", "Tiêu đề 6", "Titre 6", "Überschrift 6"),
    ("Heading 7", "Heading 7", "标题 7", "Título 7", "見出し7", "제목 7", "Tiêu đề 7", "Titre 7", "Überschrift 7"),
    ("Heading 8", "Heading 8", "标题 8", "Título 8", "見出し8", "제목 8", "Tiêu đề 8", "Titre 8", "Überschrift 8"),
    ("Heading 9", "Heading 9", "标题 9", "Título 9", "見出し9", "제목 9", "Tiêu đề 9", "Titre 9", "Überschrift 9"),
    ("Heading 10", "Heading 10", "标题 10", "Título 10", "見出し10", "제목 10", "Tiêu đề 10", "Titre 10", "Überschrift 10"),
    ("Quotations", "Block Quotation", "成段引文", "Cita en bloque", "引用文", "인용 블록", "Trích dẫn khối", "Bloc de citation", "Blockzitat"),
    ("Preformatted Text", "Preformatted Text", "预设定格式的文本", "Texto preformateado", "整形済みテキスト", "사전 서식 지정된 텍스트", "Văn bản định dạng sẵn", "Texte préformaté", "Vorformatierter Text"),
    ("Horizontal Line", "Horizontal Line", "水平线", "Línea horizontal", "横線", "수평 선", "Đường nằm ngang", "Ligne horizontale", "Horizontale Linie"),
    ("List Contents", "List Contents", "列表内容", "Contenido de lista", "リストの内容", "목록 내용", "Nội dung danh sách", "Contenu de liste", "Listeninhalt"),
    ("List Heading", "List Heading", "列表标题", "Título de la lista", "リストの見出し", "목록 제목", "Tiêu đề danh sách", "En-tête de liste", "Listenkopf"),
];

pub(super) const CHARACTER: [Builtin; 27] = [
    NO_CHARACTER_STYLE,
    ("Footnote Symbol", "Footnote Characters", "脚注符", "Caracteres de nota al pie", "脚注番号", "각주 기호", "Ký tự cước chú", "Caractères de note de bas de page", "Fußnotenzeichen"),
    ("Page Number", "Page Number", "页码", "Número de página", "ページ番号", "페이지 번호", "Số thứ tự trang", "Numéro de page", "Seitennummer"),
    ("Caption characters", "Caption Characters", "题注字符", "Caracteres de leyenda", "キャプションの文字", "캡션 문자", "Ký tự phụ đề", "Caractères de la légende", "Beschriftungszeichen"),
    ("Drop Caps", "Drop Caps", "首字下沉", "Letras capitulares", "ドロップキャップ", "단락의 첫문자 장식", "Chữ hoa trang trí", "Lettrines", "Initialenzeichen"),
    ("Numbering Symbols", "Numbering Symbols", "编号符号", "Símbolos de numeración", "番号付け記号", "번호 매기기 기호", "Ký hiệu đánh số", "Caractères de numérotation", "Nummerierungszeichen"),
    ("Bullet Symbols", "Bullets", "项目符号", "Bolos", "箇条書き", "글머리 기호", "Chấm điểm", "Puces", "Aufzählungszeichen"),
    ("Internet link", "Internet Link", "Internet 链接", "Enlace de Internet", "インターネットリンク", "인터넷 링크", "Liên kết Internet", "Lien Internet", "Internetverknüpfung"),
    ("Visited Internet Link", "Visited Internet Link", "访问过的 Internet 链接", "Enlace de Internet visitado", "訪れたインターネットリンク", "방문한 인터넷 링크", "Liên kết Internet đã thăm", "Lien Internet visité", "Besuchte Internetverknüpfung"),
    ("Placeholder", "Placeholder", "占位符", "Sustitutivo", "プレースホルダー", "플레이스홀더", "Bộ giữ chỗ", "Substituant", "Platzhalter"),
    ("Index Link", "Index Link", "索引链接", "Enlace del índice", "索引ジャンプ", "색인 연결", "Liên kết chỉ mục", "Saut d'index", "Verzeichnissprung"),
    ("Endnote Symbol", "Endnote Characters", "尾注符", "Caracteres de nota final", "文末脚注番号", "미주 기호", "Ký tự chú thích cuối", "Caractères de note de fin", "Endnotenzeichen"),
    ("Line numbering", "Line Numbering", "行号", "Numeración de renglones", "行番号付け", "행 번호 매기기", "Đánh số dòng", "Numérotation de lignes", "Zeilennummerierung"),
    ("Main index entry", "Main Index Entry", "主索引条目", "Entrada principal del índice", "索引の主項目", "주 항목 색인", "Mục nhập chỉ mục chính", "Entrée principale d'index", "Stichwortverzeichnis Haupteintrag"),
    ("Footnote anchor", "Footnote Anchor", "脚注锚点", "Ancla de nota al pie", "脚注参照番号", "각주 기준 위치", "Neo cước chú", "Ancre de note de bas de page", "Fußnotenanker"),
    ("Endnote anchor", "Endnote Anchor", "尾注锚点", "Ancla de nota final", "文末脚注参照記号", "미주 고정 기준", "Neo chú thích cuối", "Ancre de note de fin", "Endnotenanker"),
    ("Rubies", "Rubies", "字旁标音", "Texto ágata", "ふりがな", "윗주 달기", "Ruby", "Rubys", "Rubys"),
    ("Vertical Numbering Symbols", "Vertical Numbering Symbols", "竖式编号符号", "Caracteres de numeración vertical", "縦書き番号付け記号", "세로 번호 매기기 기호", "Ký hiệu đánh số nằm dọc", "Caractères de numérotation verticale", "Vertikale Nummerierungszeichen"),
    ("Emphasis", "Emphasis", "强调", "Destacado", "強調", "강조", "Nhấn mạnh", "Accentuation", "Betont"),
    ("Citation", "Quotation", "引用", "Cita", "引用句", "인용", "Trích dẫn", "Citation", "Zitat"),
    ("Strong Emphasis", "Strong Emphasis", "特别强调", "Destaque mayor", "さらに強調", "강한 강조", "Rất nhấn mạnh", "Accentuation forte", "Stark betont"),
    ("Source Text", "Source Text", "源文本", "Texto original", "ソーステキスト", "원본 텍스트", "Văn bản nguồn", "Texte source", "Quelltext"),
    ("Example", "Example", "示例", "Ejemplo", "例", "예", "Ví dụ", "Exemple", "Beispiel"),
    ("User Entry", "User Entry", "用户输入", "Entrada de usuario", "ユーザー入力", "사용자 항목", "Mục nhập người dùng", "Saisie de l'utilisateur", "Benutzereingabe"),
    ("Variable", "Variable", "变量", "Variable", "変数", "변수", "Biến", "Variable", "Variable"),
    ("Definition", "Definition", "定义", "Definición", "定義", "정의", "Định nghĩa", "Définition", "Definition"),
    ("Teletype", "Teletype", "电传打字", "Teletipo", "等幅", "텔레타이프", "Máy điện báo đánh chữ", "Texte non proportionnel", "Nichtproportionaler Text"),
];

pub(super) const PAGE: [Builtin; 10] = [
    ("Standard", "Default Page Style", "默认页面样式", "Estilo de página predeterminado", "標準ページスタイル", "기본 페이지 스타일", "Kiểu trang mặc định", "Style de page par défaut", "Standard"),
    ("First Page", "First Page", "首页", "Primera página", "表紙ページ", "첫 페이지", "Trang đầu", "Première page", "Erste Seite"),
    ("Left Page", "Left Page", "左页", "Página izquierda", "左ページ", "왼쪽 페이지", "Trang trái", "Page gauche", "Linke Seite"),
    ("Right Page", "Right Page", "右页", "Página derecha", "右ページ", "오른쪽 페이지", "Trang phải", "Page droite", "Rechte Seite"),
    ("Envelope", "Envelope", "信封", "Sobre", "封筒", "봉투", "Phong bì", "Enveloppe", "Briefumschlag"),
    ("Index", "Index", "索引", "Índice", "索引", "색인", "Chỉ mục", "Index", "Verzeichnis"),
    ("HTML", "HTML", "HTML", "HTML", "HTML", "HTML", "HTML", "HTML", "HTML"),
    ("Footnote", "Footnote", "脚注", "Nota al pie", "脚注", "각주", "Cước chú", "Note de bas de page", "Fußnote"),
    ("Endnote", "Endnote", "尾注", "Nota final", "文末脚注", "미주", "Chú thích cuối", "Note de fin", "Endnote"),
    ("Landscape", "Landscape", "横向", "Horizontal", "横置き", "가로 방향", "Phong cảnh", "Paysage", "Querformat"),
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
            assert_eq!(names(|row| row.5), styles.len(), "Korean");
            assert_eq!(names(|row| row.6), styles.len(), "Vietnamese");
            assert_eq!(names(|row| row.7), styles.len(), "French");
            assert_eq!(names(|row| row.8), styles.len(), "German");
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

    #[gpui::test]
    fn german_builtin_names_come_from_libreoffice(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("de-DE", app);
            assert_eq!(super::shown(&PARAGRAPH, "Text body", app), "Fließtext");
            assert_eq!(super::shown(&PARAGRAPH, "Heading", app), "Überschrift");
            assert_eq!(super::shown(&PARAGRAPH, "Title", app), "Titel");
            assert_eq!(super::shown(&PARAGRAPH, "Header", app), "Kopfzeile");
            assert_eq!(super::shown(&CHARACTER, "Standard", app), "Keine Zeichenvorlage");
            assert_eq!(super::shown(&PARAGRAPH, "Meine Vorlage", app), "Meine Vorlage");
            assert_eq!(super::listed(&PARAGRAPH, "Body Text"), "Text body");
        });
    }
}
