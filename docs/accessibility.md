# 无障碍与国际化复核

首批七个应用。Windows 11 与 Linux（AT-SPI）于 2026-10-27 复核七个（R14.8）；macOS 27 于 2026-10-30 复核七个（R14.53，含 ElyMail；2026-10-08 那次只六个）。每条写依据；做不到或未验证的写明。

## 键盘

| 要求 | 依据 | 缺口 |
| --- | --- | --- |
| 键盘完成打开、编辑、保存、搜索与主要工具 | 共用命令（`ely_ui::commands`：新建、打开、保存、另存为、导出、打印、关闭标签 Cmd+W、命令面板、设置），各应用的查找（Cmd+F）与模式键；`shell.sh`、`doc-shell.sh` 以按键驱动；ElyMark 的块以 Enter、空格打开，由组件库的 gpui 窗口测试覆盖；查找与编辑工具随 C14.9 原生重做 | 拖动类操作（图片手柄、透视表字段、PDF 区域标记）没有键盘替代 |
| 焦点框不依赖悬停 | 组件库可按元素都是 Tab 停靠点并带 `focus_ring`（7fab072 起 git 行带透明边框使焦点框可见） | — |
| 内容区与界面间焦点可预测 | Tab 顺序导航→内容→属性→状态（C05.4，`shell.sh`）；浮层关闭后焦点回到打开前之处（组件库 8658999）；文档（LOK 视图）把 Tab 留给自己（制表符、跳单元格、缩进），F6 与 Shift+F6 照 Tab 的次序移出与移回（组件库 029632b；原先没有键盘出口）；ElyPDF 的页面是 Tab 停靠点（原先离开后 Tab 回不去）；`windows-names.sh`、`linux-names.sh`、`macos-names.sh` 在七个应用里走完 Tab 一圈，每个文档都走得到、F6 都出得去（三端都到页面即按 F6，焦点须出页面，F6 的落点也核名字）；ElyMark 的文档每块是一个停靠点，F6 原先只在块间走，组件库 63543a0 起直接出文档；Windows 上在别的窗口之后打开、再点到前面的窗口，焦点照报给辅助技术（gpui-pre-windows 的 `a11y_init` 改读 `GetActiveWindow`，原先读前台窗口，读屏听不到 Tab） | ElyMark 原生组件的焦点次序待 C14.16 核对 |
| 对比度 4.5:1 | 主题与 Omarchy 色板按组件库 `A11yChecker` 规则收口（正文、状态色 4.5:1，焦点与图表色 3:1），核对不过的主题整套不用（C05.1） | 文档内容的颜色取自文件，照原样显示 |

## 辅助技术

| 要求 | 依据 | 缺口 |
| --- | --- | --- |
| 控件有角色与名称 | 组件库核心控件、对话框、列表、树、提交与差异行、设置行（组与说明）、提示（Toast 有角色与名称）；真窗口测试都经 AX 树操作（`ax-values`、`ax-press`）；图标按钮的名字只取提示文字，不再以图标编号（x、zoom-out）充数（组件库 76047a6，六种语言）；文档视图名为“文档”“电子表格”“演示文稿”“PDF 文档”；属性栏与对话框的数字框、下拉框取其行的标签；`windows-names.sh`（UI Automation）与 `linux-names.sh`（AT-SPI）核对七个应用每个可操作节点与每个 Tab 停靠点都有名字，`macos-names.sh`（AX，VoiceOver 所读）在 macOS 上同；macOS 上辅助技术问焦点原先总得到窗口（gpui-pre-macos 把 AccessKit 挂在内容视图上，第一响应者却是其中的 gpui 视图），改挂到 gpui 视图 | `lists::Outline` 与 `feedback::Alert` 无角色（上游待办）；gpui 无 aria-disabled，以说明文字表达 |
| 文档内容可达 | LOK 视图把当前段落（Calc 为当前单元格与输入行）以 `MultilineTextInput` 交给 AX，段落是文本串，插入点与选区随之（R14.7，gpui 0.3.10 的合成子节点）；ElyPDF 以 `Document` 交出所显示页面的逐行文字与拖选的选区；`windows-a11y.sh` 经 UI Automation 的 TextPattern（NVDA 所读）读到段落、插入点、跨 emoji 的选区与 PDF 的页文与选区 | LOK 只报当前段落，整篇文档到不了辅助技术；VoiceOver 朗读不测（用户定）；AT-SPI 的树、名字与焦点（`linux-names.sh`）与经其 Text 接口读文字、插入点与选区（`linux-a11y.sh`，R14.52）已实测；ElyMark 原生组件的 AX 待 C14.16 核对 |
| 编辑状态可达 | 保存状态、风险、引擎停止以提示条与状态栏文字表达 | 状态栏文字是否进 AX 树未逐项核对 |

## 国际化

| 要求 | 依据 | 缺口 |
| --- | --- | --- |
| 简中与英文界面 | `ely_ui::i18n` 缺键即失败；组件库面板族、对话框、批注栏等内建中英目录；设置可切语言（下次启动生效） | 内核给出的固定文字由 Ely 重写：ElyTable 的状态、错误与单元格样式，ElyDoc 的内置样式名与纸张名（R14.6，`chinese.rs` 在中文下画出排序、筛选与属性栏，无英文）；文件与主题自带的名字照原样 |
| 中英文输入、组合输入、标点 | LOK 视图与 ElyMark 的编辑器都经输入法回调处理组字（`lokview.sh` 组“你好”；组件库的文档测试）；往返用例含中文（`independent.rs`） | 系统拼音输入法实机只做过人工检查，测试以输入法回调模拟 |
| 混合字体 | 内核按字体回退排版；缺失字体在 ElyDoc 页面上方列出 | 字体缺失环境的分页对照未做（见 `docs/compatibility.md`） |
| 非 ASCII 路径 | `paths_keep_spaces_cjk_and_case`（保存）；启动参数按当前目录取绝对路径（C20.1 修） | — |
| 日期、数字、纸张用显式区域 | 内核载入选项固定 `Language=en-US`（`ely-lok/src/worker.rs` 的 `LOAD_OPTIONS`），公式为 Excel A1 语法；纸张取自文档本身并原样打印（C19.2） | — |
| 改界面语言不重解释已存数值 | 同上：内核的数字区域不随界面语言，界面语言只改 Ely 的文案与选区统计的名称 | — |


## Spanish interface

ElyDoc and ElyTable offer **Español** in Settings → General → Language.
Restart each app after changing it. **System** recognizes regional Spanish tags,
including `es-PE`, `es-MX`, `es-419` and POSIX `es_PE.UTF-8`.
The translation includes their shared settings and document dialogs. Other apps'
dedicated messages are still English when this shared preference is Spanish.
LibreOffice formula names and numeric input conventions are unchanged; this adds
an interface language, not Spanish spelling dictionaries.
