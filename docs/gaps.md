# 缺口

组件库没有的，在本项目内建。`agents.md` 的“缺口”只列在哪，这里是每条为什么缺、怎么做的。

| 缺口 | 位置 |
| --- | --- |
| LOK 图块画布与输入桥 | `crates/ely-lok` |
| 富剪贴板：gpui 剪贴板只有文本与图片，无 HTML（富文本、表格） | `ely_platform::clipboard`，用 arboard（三端 HTML、PNG） |
| LOK 的无障碍事件只报当前段落（文本、插入点、选区，UTF-16 计） | 文字文档经补丁 0105 取正文各段与光标所在段，`LokView` 以 gpui 的 `a11y_synthetic_children` 交成文本串，插入点与选区换成字符（`view/reading.rs`）；Calc、Impress 只交当前段落；ElyPDF 把所显示页面的逐行文字与选区交成 `Document` 的文本串（`reading.rs`）；都只在辅助技术连上时做 |
| 选中对象的框与手柄：LOK 不画进图块 | `LokView` 按图形选区画框与 8 个手柄；按住手柄拖动经 `setGraphicSelection` 交内核（缩放；裁剪模式下裁剪） |
| 虚拟化 PDF 页面栈（`DocumentViewer` 一次铺全部页，不合 PERF-05） | `crates/ely-pdf-engine`、`crates/ely-pdf-worker`、`apps/ely-pdf` |
| 组件库的大纲项（`lists::Outline`）无无障碍角色：屏幕阅读器读不到 | 上游待办；ElyMark 的搜索失败改为结果列表里的行 |
| `documents::ZenMode` 把正文夹进 Sm 宽的列，与 ElyMark 的正文宽度设置冲突 | ElyMark 不用它，专注模式原生重做（C14.12） |
| 表格公式栏与名称框（网格是 LOK 图块，`tables::Spreadsheet` 不用） | `apps/ely-table` |
| Doc 工具栏（`documents::FixedFormatToolbar` 绑 `TextInput`，不接 LOK） | ElyDoc 页面上方 `props/bar.rs`，用 `shell::Toolbar` 组合属性面板的同一组命令；Table、Show 无 |
| CSV 导入的编码、分隔符、引号、列类型（`misc::CsvImporter` 只读逗号 CSV、映射到属主字段，文案写死英文） | `apps/ely-table` 的 `text.rs`，内核按选项导入 |
| 导出格式与说明（`misc::ExportDialog` 无 TSV，文案写死英文，不能附说明） | 各 app，用 `overlays::Dialog` 组合 |
| 幻灯片缩略图条（`documents::PageThumbnailList` 只收图片路径，无隐藏标记、右键菜单与重排） | `apps/ely-show` 的 `slides.rs`，内核按页绘制到内存图 |
| PDF 页面缩略图（同上，且一次铺全部页，不合 PERF-05） | `apps/ely-pdf` 的 `side.rs`，`uniform_list` 只画可见行，worker 按需渲染 |
| 演示的参考线：LOK 不把参考线画进图块，也无参考线接口 | 补丁 0027 读写参考线；`apps/ely-show` 的 `guides.rs` 画在画布上，拖动移动，拖出幻灯片即删除 |
| gpui 文本里不能嵌图片：行内公式无从排版 | 组件库 `render/math_text.rs`：以不断行的空格占出公式的宽（整个公式作一个词折行），排好的公式画在其上，按下处读回公式的原文 |
| Omarchy 色板要照 `A11yChecker` 的同一套对照规则 | `ely-ui` 的 `omarchy/palette.rs` 抄了对照表；组件库自 70ab3c5 起公开 `devtools::audit`，待改用 |
| 组件库与 gpui 在无障碍语义、禁用状态与内部文案上的缺口 | 上游待办，逐项的现状在 `docs/accessibility.md` 的“上游待办”一节；项目自绘控件已自带角色与名字 |
| PPTX 的章节：内核无此结构，导入即丢 | `ely_office::files::sections` 打开时读 `p14:sectionLst`，有即 `Risk::Sections`；加密的 PPTX 看不到章节，即 `Risk::Sealed`；二者皆横幅说明，保存改为另存副本，自动保存与命令行保存不写原文件 |
| LOK 的 `.uno:StyleApply` 状态给英文界面名，样式列表给程序名，两者对不上；内核界面固定英文，内置样式名到不了别的语言 | ElyDoc `props/builtin.rs` 照 LibreOffice 26.2 的译文列出内置段落、字符、页面样式的程序名与八种界面语言的名字（vi 未译的条目自拟）：状态里的界面名经它换回程序名，菜单按界面语言显示；用户自建的样式照原名 |
