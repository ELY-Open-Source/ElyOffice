//! Writer's built-in style names in the interface language (I-03).

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

/// The kernel's name, its English, its Chinese.
pub(super) type Builtin = (&'static str, &'static str, &'static str);

/// Turns the kernel's English name into its list name.
pub(super) fn listed<'a>(styles: &'static [Builtin], name: &'a str) -> &'a str {
    styles
        .iter()
        .find(|(_, english, _)| *english == name)
        .map_or(name, |(kernel, ..)| kernel)
}

/// How `name` reads; a user's own style keeps its name.
pub(super) fn shown(styles: &[Builtin], name: &str, cx: &App) -> SharedString {
    let chinese = cx.global::<I18n>().locale().tag == "zh-CN";
    match styles.iter().find(|(kernel, ..)| *kernel == name) {
        Some((_, _, zh)) if chinese => (*zh).into(),
        Some((_, english, _)) => (*english).into(),
        None => name.to_string().into(),
    }
}

// From LibreOffice 26.2.6: SwStyleNameMapper, strings.hrc, zh-CN po.
pub(super) const PARAGRAPH: [Builtin; 126] = [
    ("Standard", "Default Paragraph Style", "默认段落样式"),
    ("Text body", "Body Text", "正文"),
    ("First line indent", "First Line Indent", "首行缩进"),
    ("Hanging indent", "Hanging Indent", "悬挂缩进"),
    ("Text body indent", "Body Text, Indented", "带缩进的正文"),
    ("Salutation", "Complimentary Close", "结尾敬语"),
    ("Signature", "Signature", "签名"),
    ("List Indent", "List Indent", "列表缩进"),
    ("Marginalia", "Marginalia", "旁注"),
    ("List", "List", "列表"),
    ("Numbering 1 Start", "Numbering 1 Start", "编号 1 起始"),
    ("Numbering 1", "Numbering 1", "编号 1"),
    ("Numbering 1 End", "Numbering 1 End", "编号 1 结束"),
    ("Numbering 1 Cont.", "Numbering 1 Cont.", "编号 1 续"),
    ("Numbering 2 Start", "Numbering 2 Start", "编号 2 起始"),
    ("Numbering 2", "Numbering 2", "编号 2"),
    ("Numbering 2 End", "Numbering 2 End", "编号 2 结束"),
    ("Numbering 2 Cont.", "Numbering 2 Cont.", "编号 2 续"),
    ("Numbering 3 Start", "Numbering 3 Start", "编号 3 起始"),
    ("Numbering 3", "Numbering 3", "编号 3"),
    ("Numbering 3 End", "Numbering 3 End", "编号 3 结束"),
    ("Numbering 3 Cont.", "Numbering 3 Cont.", "编号 3 续"),
    ("Numbering 4 Start", "Numbering 4 Start", "编号 4 起始"),
    ("Numbering 4", "Numbering 4", "编号 4"),
    ("Numbering 4 End", "Numbering 4 End", "编号 4 结束"),
    ("Numbering 4 Cont.", "Numbering 4 Cont.", "编号 4 续"),
    ("Numbering 5 Start", "Numbering 5 Start", "编号 5 起始"),
    ("Numbering 5", "Numbering 5", "编号 5"),
    ("Numbering 5 End", "Numbering 5 End", "编号 5 结束"),
    ("Numbering 5 Cont.", "Numbering 5 Cont.", "编号 5 续"),
    ("List 1 Start", "List 1 Start", "列表 1 起始"),
    ("List 1", "List 1", "列表 1"),
    ("List 1 End", "List 1 End", "列表 1 结束"),
    ("List 1 Cont.", "List 1 Cont.", "列表 1 续"),
    ("List 2 Start", "List 2 Start", "列表 2 起始"),
    ("List 2", "List 2", "列表 2"),
    ("List 2 End", "List 2 End", "列表 2 结束"),
    ("List 2 Cont.", "List 2 Cont.", "列表 2 续"),
    ("List 3 Start", "List 3 Start", "列表 3 起始"),
    ("List 3", "List 3", "列表 3"),
    ("List 3 End", "List 3 End", "列表 3 结束"),
    ("List 3 Cont.", "List 3 Cont.", "列表 3 续"),
    ("List 4 Start", "List 4 Start", "列表 4 起始"),
    ("List 4", "List 4", "列表 4"),
    ("List 4 End", "List 4 End", "列表 4 结束"),
    ("List 4 Cont.", "List 4 Cont.", "列表 4 续"),
    ("List 5 Start", "List 5 Start", "列表 5 起始"),
    ("List 5", "List 5", "列表 5"),
    ("List 5 End", "List 5 End", "列表 5 结束"),
    ("List 5 Cont.", "List 5 Cont.", "列表 5 续"),
    ("Header and Footer", "Header and Footer", "页眉与页脚"),
    ("Header", "Header", "页眉"),
    ("Header left", "Header Left", "页眉左侧"),
    ("Header right", "Header Right", "页眉右侧"),
    ("Footer", "Footer", "页脚"),
    ("Footer left", "Footer Left", "页脚左侧"),
    ("Footer right", "Footer Right", "页脚右侧"),
    ("Table Contents", "Table Contents", "表格内容"),
    ("Table Heading", "Table Heading", "表格标题"),
    ("Caption", "Caption", "题注"),
    ("Illustration", "Illustration", "插图"),
    ("Table", "Table", "表格"),
    ("Text", "Text", "文字"),
    ("Figure", "Figure", "图示"),
    ("Frame contents", "Frame Contents", "框架内容"),
    ("Footnote", "Footnote", "脚注"),
    ("Addressee", "Addressee", "收件人"),
    ("Sender", "Sender", "发件人"),
    ("Endnote", "Endnote", "尾注"),
    ("Drawing", "Drawing", "绘图"),
    ("Comment", "Comment", "批注"),
    ("Index", "Index", "索引"),
    ("Index Heading", "Index Heading", "索引标题"),
    ("Index 1", "Index 1", "索引 1"),
    ("Index 2", "Index 2", "索引 2"),
    ("Index 3", "Index 3", "索引 3"),
    ("Index Separator", "Index Separator", "索引分隔符"),
    ("Contents Heading", "Contents Heading", "目录标题"),
    ("Contents 1", "Contents 1", "目录 1"),
    ("Contents 2", "Contents 2", "目录 2"),
    ("Contents 3", "Contents 3", "目录 3"),
    ("Contents 4", "Contents 4", "目录 4"),
    ("Contents 5", "Contents 5", "目录 5"),
    ("User Index Heading", "User Index Heading", "自定义索引标题"),
    ("User Index 1", "User Index 1", "自定义索引 1"),
    ("User Index 2", "User Index 2", "自定义索引 2"),
    ("User Index 3", "User Index 3", "自定义索引 3"),
    ("User Index 4", "User Index 4", "自定义索引 4"),
    ("User Index 5", "User Index 5", "自定义索引 5"),
    ("Contents 6", "Contents 6", "目录 6"),
    ("Contents 7", "Contents 7", "目录 7"),
    ("Contents 8", "Contents 8", "目录 8"),
    ("Contents 9", "Contents 9", "目录 9"),
    ("Contents 10", "Contents 10", "目录 10"),
    ("Figure Index Heading", "Figure Index Heading", "图示索引标题"),
    ("Figure Index 1", "Figure Index 1", "图示索引 1"),
    ("Object index heading", "Object Index Heading", "对象索引标题"),
    ("Object index 1", "Object Index 1", "对象索引 1"),
    ("Table index heading", "Table Index Heading", "表格索引标题"),
    ("Table index 1", "Table Index 1", "表格索引 1"),
    ("Bibliography Heading", "Bibliography Heading", "参考文献标题"),
    ("Bibliography 1", "Bibliography 1", "参考文献 1"),
    ("User Index 6", "User Index 6", "自定义索引 6"),
    ("User Index 7", "User Index 7", "自定义索引 7"),
    ("User Index 8", "User Index 8", "自定义索引 8"),
    ("User Index 9", "User Index 9", "自定义索引 9"),
    ("User Index 10", "User Index 10", "自定义索引 10"),
    ("Title", "Title", "标题"),
    ("Subtitle", "Subtitle", "副标题"),
    ("Appendix", "Appendix", "附录"),
    ("Heading", "Heading", "标题样式"),
    ("Heading 1", "Heading 1", "标题 1"),
    ("Heading 2", "Heading 2", "标题 2"),
    ("Heading 3", "Heading 3", "标题 3"),
    ("Heading 4", "Heading 4", "标题 4"),
    ("Heading 5", "Heading 5", "标题 5"),
    ("Heading 6", "Heading 6", "标题 6"),
    ("Heading 7", "Heading 7", "标题 7"),
    ("Heading 8", "Heading 8", "标题 8"),
    ("Heading 9", "Heading 9", "标题 9"),
    ("Heading 10", "Heading 10", "标题 10"),
    ("Quotations", "Block Quotation", "成段引文"),
    ("Preformatted Text", "Preformatted Text", "预设定格式的文本"),
    ("Horizontal Line", "Horizontal Line", "水平线"),
    ("List Contents", "List Contents", "列表内容"),
    ("List Heading", "List Heading", "列表标题"),
];

pub(super) const CHARACTER: [Builtin; 27] = [
    ("Standard", "No Character Style", "无字符样式"),
    ("Footnote Symbol", "Footnote Characters", "脚注符"),
    ("Page Number", "Page Number", "页码"),
    ("Caption characters", "Caption Characters", "题注字符"),
    ("Drop Caps", "Drop Caps", "首字下沉"),
    ("Numbering Symbols", "Numbering Symbols", "编号符号"),
    ("Bullet Symbols", "Bullets", "项目符号"),
    ("Internet link", "Internet Link", "Internet 链接"),
    ("Visited Internet Link", "Visited Internet Link", "访问过的 Internet 链接"),
    ("Placeholder", "Placeholder", "占位符"),
    ("Index Link", "Index Link", "索引链接"),
    ("Endnote Symbol", "Endnote Characters", "尾注符"),
    ("Line numbering", "Line Numbering", "行号"),
    ("Main index entry", "Main Index Entry", "主索引条目"),
    ("Footnote anchor", "Footnote Anchor", "脚注锚点"),
    ("Endnote anchor", "Endnote Anchor", "尾注锚点"),
    ("Rubies", "Rubies", "字旁标音"),
    ("Vertical Numbering Symbols", "Vertical Numbering Symbols", "竖式编号符号"),
    ("Emphasis", "Emphasis", "强调"),
    ("Citation", "Quotation", "引用"),
    ("Strong Emphasis", "Strong Emphasis", "特别强调"),
    ("Source Text", "Source Text", "源文本"),
    ("Example", "Example", "示例"),
    ("User Entry", "User Entry", "用户输入"),
    ("Variable", "Variable", "变量"),
    ("Definition", "Definition", "定义"),
    ("Teletype", "Teletype", "电传打字"),
];

pub(super) const PAGE: [Builtin; 10] = [
    ("Standard", "Default Page Style", "默认页面样式"),
    ("First Page", "First Page", "首页"),
    ("Left Page", "Left Page", "左页"),
    ("Right Page", "Right Page", "右页"),
    ("Envelope", "Envelope", "信封"),
    ("Index", "Index", "索引"),
    ("HTML", "HTML", "HTML"),
    ("Footnote", "Footnote", "脚注"),
    ("Endnote", "Endnote", "尾注"),
    ("Landscape", "Landscape", "横向"),
];
#[cfg(test)]
mod tests {
    use super::{CHARACTER, PAGE, PARAGRAPH};

    #[test]
    fn an_english_name_names_one_style() {
        for styles in [&PARAGRAPH[..], &CHARACTER, &PAGE] {
            for (kernel, english, _) in styles {
                let other = styles.iter().find(|(name, ..)| name == english);
                assert!(other.is_none_or(|(name, ..)| name == kernel), "{english}");
            }
        }
    }
}
