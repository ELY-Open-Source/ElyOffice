# Ely Office

单一事实来源。技术栈或产品方向变更时，同步改本文件。决策记录在 `docs/decisions.md`。需求以 `PRD.md` 为准。进度以 `tasks/TASKS.md` 为准。

## 产品

八个桌面应用：ElyDoc、ElyTable、ElyShow、ElyPDF、ElyMail、ElyTree、ElyMark、ElyAgent。离线优先，文件在用户目录。

## 技术栈

| 层 | 选型 |
| --- | --- |
| 界面 | Rust + GPUI + Ely-GPUI-Components |
| Doc / Table / Show | LibreOfficeKit，独立工作进程 |
| Show 媒体 | 各平台原生：macOS AVFoundation，Windows Media Foundation，Linux GStreamer（LOK 不放媒体） |
| PDF | MuPDF（C 接口），Tesseract OCR，独立工作进程 |
| Mail | mail-send、mail-parser、mail-builder；IMAP（imap-proto 解析）与 POP3 由引擎自写（`crates/ely-mail-engine`，应用进程内的后台任务，无 worker）+ SQLite（FTS5）；正文解析、净化后由 GPUI 绘制；写信用 `documents::MarkdownDocument` |
| Tree | Git CLI，参数数组启动 |
| Mark | 组件库 `documents::MarkdownDocument`（原文即文本）与其 Markdown 树（pulldown-cmark）；RaTeX 公式，mermaid-rs-renderer 图；PDF 经 `ely-pdf-worker` 由 MuPDF 排版 |
| Agent | Kigi CLI 的 Rust 代码直接嵌入（不用 ACP），沿用其提供商登录与模型配置 |

## 目录

```text
apps/        八个应用，各自一个二进制
crates/      ely-ui  ely-platform  ely-ipc（应用与 worker 之间的帧）  ely-files  ely-lok  ely-office（文档应用共用的外壳）  ely-pdf-engine  ely-pdf-worker（只它编 MuPDF）  ely-mail-engine  ely-mail-fake（测试用的邮件服务器）  ely-start（Windows 启动画面）
reference/   上游源码，只读，不属于本项目，不审查
engines/     下载的二进制与引擎构建产物，不入库，不审查
patches/     对上游的补丁，随源码发行（libreoffice、mupdf、gpui）
vendor/      crates.io 上游 crate 的副本加本项目的修复，经 `[patch.crates-io]` 换入；首个提交原样，其后修复
packaging/   三端打包脚本；apps.tsv 是八个应用的名字、批次与许可，types.tsv 是文件类型
assets/      icons/ 是图标母版与渲染出的各端格式
dist/        打包产物，不入库
docs/        决定（decisions.md）、其细则（standing.md）、缺口（gaps.md）、兼容、无障碍、帮助、许可、性能记录、test-map.md（由 scripts/test-map.py 生成）
scripts/     构建、检查、发布的脚本，逐个的说明在 scripts/README.md
tests/       scripts 与 engines 下是脚本测试（说明在 tests/scripts/README.md）；engines 缺二进制时报 SKIP；fixtures 是共用样本（说明在 tests/fixtures/README.md）
tasks/       TASKS.md 进度真相
site/        官网（Vite、pnpm、TypeScript，无框架）；site/video 是发布视频
```

## 组件映射

依赖 `ely-gpui-component`（pin 46f26e8，gpui 为 crates.io 的 `gpui-pre` =0.3.11，Rust 1.98.1）。名字取自与本仓库并列的组件库检出 `../Ely-GPUI-Components`（不在 `reference/`；总包 `src/lib.rs` 列出各模块在哪个 crate，gpui 用法规则见其 `AGENTS.md`）；`tests/scripts/agents_md.py` 核对下表每个名字都是组件库公开项，没有那个检出即失败。

| 需要 | 用 |
| --- | --- |
| 窗口外壳 | `shell::{TitleBar, TabBar, StatusBar, Toolbar, ToolbarGroup, WindowSwitcher, ZoomControl}` `layout::{AppShell, SplitPane, Sidebar, Dock}` |
| 命令与菜单 | `menus::{MenuBar, ContextMenu, DropdownMenu}` `navigation::{CommandPalette, QuickOpen}` |
| 导航 | `navigation::{EditorTabs, Tabs, Breadcrumb, TableOfContents}` `lists::{Tree, Outline, FileTree, SelectableList}` |
| 属性与表单 | `data_display::{PropertyGrid, DescriptionList}` `forms::{Select, FontPicker, ColorPicker, NumberInput, ScrubInput, Switch, SearchInput, TextInput}` `buttons::{Button, IconButton, ToggleButton, SegmentedControl}` |
| 对话框 | `overlays::{Dialog, ConfirmDialog, AlertDialog, PromptDialog, Popover}` |
| 反馈 | `feedback::{Toaster, ToastViewport, SaveState, SavingIndicator, SyncStatus, Alert, Banner, EmptyState, ErrorView}` `shell::{OfflineIndicator, CrashReporter, UpdateBanner, UpdateDialog}` |
| 设置 | `settings::{SettingsLayout, SettingsSection, SettingsRow, ThemeSelector, KeyboardShortcutsList, StorageSettings, PrivacySettings}` |
| Doc | `documents::{PrintPreview, TemplatePicker, WordCount, PageThumbnailList}` `editor::{FindWidget}` `collab::{CommentSidebar, Thread, Comment, Peer}` `lists::{List, ListItem}` |
| Table | `tables::{PivotTable, FilterBuilder, SortBuilder}` `misc::{CsvImporter, ExportDialog}` `editor::{CompletionMenu, SignatureHelp, FindWidget}` `forms::{CheckboxGroup, ColorPalette}` |
| Mail | `mail::{MailboxList, MailList, MailThreadView, MailReader, MailComposer, SignatureEditor, ScheduleSend}`；正文以 `documents::{MarkdownRenderer, RightPress}` 的 `selectable` 选中复制，右键菜单由 ElyMail 建 |
| Tree | `git::{ChangesList, CommitInput, CommitList, lanes, DiffViewer, ConflictResolver, BlameView, BranchList, TagList, StashList, FileHistory}` |
| Mark | `documents::{MarkdownDocument, MarkdownRenderer, ZenMode, MarkdownReader, MarkdownBlock, alert, is_toc, BlockCommand, InlineFormat, TableEdit, EditRefusal, TableRefusal, DocumentTarget, TargetBlock}` `editor::{CodeEditor, FindWidget}`；导出与复制调用可视同一组读法；段落、格式与表格菜单经文档的 `block`、`format`、`edit_table`；右键菜单听 `DocumentEvent::Pointed` |
| Agent | `agent::{ToolCallCard, PermissionPrompt, ToolApprovalDialog, AgentStepList, FileChangeCard, MultiFileDiffReview, ArtifactPanel, CostBreakdown}` `chat::{MessageList, PromptInput, StreamingMarkdown, ConversationList}` |
| 动效 | `motion::{duration, Flip, Reorder, Transition, Stagger, ProgressBar, Spinner, Skeleton}` |
| 主题 | `theme::{Theme, Mode}`；Omarchy 色板由 `ely-ui` 映射成组件库的调色板 |

## 缺口

组件库没有的，在本项目内建；每条为什么缺、怎么做的在 `docs/gaps.md`。

| 缺口 | 位置 |
| --- | --- |
| LOK 图块画布与输入桥 | `crates/ely-lok` |
| 富剪贴板（gpui 无 HTML） | `ely_platform::clipboard`（arboard） |
| LOK 的无障碍只报当前段落 | `LokView` 的 `view/reading.rs`（补丁 0105）；ElyPDF 的 `reading.rs` |
| 选中对象的框与手柄 | `LokView` |
| 虚拟化 PDF 页面栈 | `crates/ely-pdf-engine`、`crates/ely-pdf-worker`、`apps/ely-pdf` |
| `lists::Outline` 无无障碍角色 | 上游待办 |
| `documents::ZenMode` 与正文宽度设置冲突 | ElyMark 原生重做 |
| 表格公式栏与名称框 | `apps/ely-table` |
| Doc 工具栏 | ElyDoc 的 `props/bar.rs` |
| CSV 导入的编码、分隔符、引号、列类型 | `apps/ely-table` 的 `text.rs` |
| 导出格式与说明 | 各 app，用 `overlays::Dialog` 组合 |
| 幻灯片缩略图条 | `apps/ely-show` 的 `slides.rs` |
| PDF 页面缩略图 | `apps/ely-pdf` 的 `side.rs` |
| 演示的参考线 | 补丁 0027，`apps/ely-show` 的 `guides.rs` |
| 行内公式（gpui 文本里不能嵌图片） | 组件库 `render/math_text.rs` |
| Omarchy 色板的对照规则 | `ely-ui` 的 `omarchy/palette.rs` |
| 无障碍语义、禁用状态与内部文案 | 上游待办，见 `docs/accessibility.md` |
| PPTX 的章节 | `ely_office::files::sections` |
| 内置样式名的各语言名字 | ElyDoc 的 `props/builtin.rs` |

## gpui 规则

取自组件库的 `AGENTS.md`、`src/lib.rs` 与 `examples/docs/hello.rs`，应用必须遵守。

- 启动：`gpui_platform::application().with_assets(Assets).run(..)`，闭包里先调 `ely_gpui_component::init(cx)`。没接 `Assets` 会 panic。
- 首帧用 `Theme::set_mode_now`。之后 `Theme::set_mode` 渐变。
- 根元素包进 `FocusScope`，调 `.root()`，聚焦它的句柄。
- 渲染之外改了状态，调 `cx.notify()`。绘制中改的，再调 `window.request_animation_frame()`。不用 `request_animation_frame` 轮询布局。
- 颜色、控件尺寸、圆角、阴影取自 `cx.theme()`。间距用 gpui 的 rem 阶梯。边框用 `border_1`。
- 盖在页面上的东西走 `primitives::raise`。对话框走 `overlays::Dialog`。每个窗口一个 `Toaster`，一个 `ToastViewport`。
- 可按压的元素都是 Tab 停靠点：`tab_index(0)` 加 `focus_ring`。`hover()` 需要元素 id。
- 动效时长取 `motion::duration()`，它遵守减少动效。重复动画在减少动效下静止。动画只包住移动的部分：动画 id 传给子孙，换 id 会重建它们的焦点与拖动状态。
- 文本框是 `forms::TextInput` 实体。文案走 `i18n::I18n`，缺键即失败。
- 关窗前的询问经组件库 `shell::guard_close`，不直接用 `on_window_should_close`：后者只交给系统，标题栏自绘的关闭（Windows、Linux）会绕过它。
- 可复用组件内的 id 带上属主的 id。`uniform_list` 的行设 `w_full`。盖在内容上的层按需用 `block_mouse_except_scroll`。

## 规则

1. 界面优先用 Ely-GPUI-Components。通用交互回组件库，不在应用里复制。
2. 出错就暴露。不吞错，不写兜底，不用默认值遮挡缺失数据。
3. 修根因。不打补丁式小修。
4. 关键路径有日志（`log`，与 gpui 一致）。日志不含正文、附件、密钥、提示词。
5. 文件不超过 500 行。注释一行，十字以内，只在必要时写。
6. 无 dead code，无 v1/v2 接口，无过期代码。
7. 禁止大框套小框。禁止无用说明文字和小标题。
8. 不过度设计。不加预留扩展，不加无第二个使用者的抽象。
9. 动效只在交互时运行，每段不超过 220 毫秒，遵守系统“减少动效”；等待中的进度动画可循环到等待结束，减少动效下静止。空闲时不重绘。目标设备含 potato computer。
10. 未验证的能力不写成已完成。

## 命令

日常用的在下面；全部脚本（取上游、建引擎、打包、发布）在 `scripts/README.md`，测试脚本在 `tests/scripts/README.md`。加了、改了脚本，改那里。

```text
scripts/doctor.sh              本系统构建所需的工具与系统包
scripts/check.sh [根]          规则检查：文件行数、本文件字节数、注释、测试的等待、行尾；先在 site/ 跑 pnpm install
scripts/deny.sh                cargo-deny：安全公告、yanked、许可、来源、重复版本
scripts/panics.py              非测试代码的 expect、unwrap、panic! 与 scripts/panics.tsv 相符
scripts/test-map.py            写 docs/test-map.md（勾任务的提交前跑）
scripts/review.sh ID 轮次 路径  审查一条任务，结论进 tasks/reviews/
scripts/release-tests.sh       发布的门：建好两个 worker，严格模式跑全部测试
CARGO_CHECK_SHARED_DEPS=0 cargo check --workspace   clippy 同
cargo test --workspace         `-p` 的 ely-files、ely-mail、ely-agent 与组件库的包重名，写 `ely-mail@0.1`
cargo build -p ely-lok --bins  LOK 单测前先建 worker；单测不会自动重建它
cargo build -p ely-pdf-worker  ElyPDF 与 ElyMark 的 worker
ELY_LOK_WORKER、ELY_PDF_WORKER  带内核的单测须设：huacnlee/cargo 的测试程序不在 ./target 下
ELY_REQUIRE_ENGINES=1 cargo test  缺引擎的测试即失败（不设时打 SKIP 后通过）
ELY_GIT=路径 cargo test -p ely-tree  用该 Git；不设即用 engines/git 的，都没有即失败
cargo run -p ely-doc           启动一个应用；ELY_LOG 设日志级别；ELY_SMOKE=1 画一帧即退出
rm -rf target/debug/incremental   标准 Cargo 下每次构建或测试后都删
```

## 常设决定

每条只写规定；全文（实现、范围、任务编号）在 `docs/standing.md`，按日期的完整记录在 `docs/decisions.md`。新的决定追加到 decisions；改变下列常规的，同时改本节与 standing.md。

- 上游：LibreOffice 26.2.6、MuPDF 1.28.5、Git 2.56.0 加 git-lfs 3.8.0、`gpui-pre` =0.3.11，以 `scripts/fetch-references.sh` 为准；组件库跟随其 main，修复在组件库仓库里做并推上去，再改 pin。
- 仓库与发布：GitHub 组织 ELY-Open-Source。推 `v*` 标签即由 `release.yml` 的四个任务（macOS arm64、Intel，Windows，Linux x86_64，不出 Linux arm64）在自托管机器上以 `scripts/ci.sh` 构建并上传；cargo 不是 huacnlee/cargo 即不建；`ci.sh` 先过门（更新日志、基准记录、该提交的 `pr.yml`、`release-tests.sh`），不过即不建；各端都上传并核对后才设为最新。新的 LibreOffice 补丁须先在各机的共享引擎（`ELY_SHARE`）上重建。
- 构建一律用 [huacnlee/cargo](https://huacnlee.github.io/cargo/)；带内核的单测须设 `ELY_LOK_WORKER`、`ELY_PDF_WORKER`，`cargo check` 与 clippy 须设 `CARGO_CHECK_SHARED_DEPS=0`；PR 的机器（GitHub 托管）是标准 Cargo，脚本两者都得过。
- Git：main 的历史不改写；提交签名，不写任何署名行。
- 许可：自有代码 MIT；随包引擎照各自的许可，Linux 包的许可字段取 `packaging/apps.tsv`。
- macOS 发布以 Developer ID 签名并公证，缺即失败，不出未公证的包。
- 首批发布 ElyDoc、ElyTable、ElyShow、ElyPDF、ElyMail、ElyTree、ElyMark；ElyAgent 第二批，加进同一个包。
- 安装包：每个平台一个 ElyOffice 安装包（macOS 为 `.pkg`），不出单个应用的包；Windows、macOS 可选装哪几个，Linux 全装；Doc、Table、Show 共用一份内核，内核、常驻引擎与登录项是隐藏组件，选了任一文档应用即装。
- 常驻引擎：登录后每个已装的文档应用常驻一个已过自检的空闲引擎，三端都做，不给开关；按物理内存少留或不留；空闲不占 CPU；不在时应用自己起。
- 横幅：只留 ElyDoc 的字体缺失；其余信息去状态栏、出后果时的对话框、错误页或 toast。
- 数据只在 `ely_platform::data_dir` 给出的目录；用户文档永不碰；卸载先问；数据文件夹可在设置“存储”里换。
- 文件类型与 URL 方案只写在 `packaging/types.tsv`，只进“打开方式”，从不设为默认。“打开方式”里不得出现 LibreOffice，任何脚本都不挂载 LibreOffice 镜像。
- 语言：英文、简体中文、西班牙语、法语、德语、日语、韩语、越南语，每个应用各语言都全，缺键即失败，不以英文补；英文、中文以外的六种标“预览”。内核界面固定英文，其固定文字由 Ely 按界面语言重写。ElyTable 公式为 Excel A1、数字按 en-US；拼写词典只随英文。
- 外观：Doc、Table、Show、PDF 的页面照文件原色，只有四周的界面随深浅色；邮件正文、Mark、Agent 随深浅色。深色底 `#252525`，不用纯黑。有 Omarchy 时随 Omarchy。
- 性能：八个应用都不许卡顿（120 FPS 手感，potato computer 也要流畅），输入延迟与帧时间实测验收；界面线程不等内核、不持 worker 锁，空闲不做事；保存与恢复点给键入让路。
- LOK：界面只经 `ely_lok::commands::COMMANDS` 发 UNO 命令；外部链接打开时显示保存的值，用户点“更新链接”才读源；内核弹出的对话框即刻关闭并报错，需要的由 Ely 自建；内核改动只以 `patches/libreoffice` 的补丁序列进入；worker 启动时关掉 OpenCL；应用与 worker 只经本用户的端点相连（Windows 命名管道，macOS、Linux 0600 套接字），握手核对版本与内核编号（`ely-kernel-id`）。
- 更新：每个平台一份清单，更新整套，读 GitHub 的最新发布，不通则读官网转发的同一路径；清单以 ed25519 签名，缺签名或不符即拒；种子 `ELY_UPDATE_KEY`（本机 `update-key.txt`，不入库）丢失即旧版再也收不到更新；自动检查默认关，从不静默安装；包管理器与 Nix 装的只提示。
- 每个应用只跑一份，再次启动把文件交给运行中的那份。
- 命令行读写已打开的文档：`ely-table cell`，`ely-doc text`、`ely-show text`、`ely-mark text`；请求走实例门（本机回环、令牌只给同一用户），不动用户的光标与选区；范围见 `docs/help.md`。
- ElyTable 的 R 宏（实验性）：只在用户点运行时起 `Rscript`，不是沙箱，从不代装；细则见 `docs/r-macros.md`。
- ElyMark 原生实现（组件库 Markdown 组件与其 Markdown 树），不用 CEF、Electron；Typora 只作参照，读同一文件不同时随 GitHub（GFM）。
- ElyMail：IMAP 与 POP3 由引擎自写，SMTP 用 mail-send；Microsoft 走 OAuth，客户端 ID 构建时注入（本机 `cred.txt`，CI 为组织机密），缺即构建失败；Gmail 与 iCloud 走应用专用密码，QQ 邮箱、163 邮箱、126 邮箱走授权码。网络随系统代理。
- ElyAgent 以 Rust 依赖直接嵌入 Kigi CLI，不用 ACP。
- 合并前：`pr.yml` 在每个 PR 与 main 的推送上跑 check.sh、deny.sh、panics.py、整个工作区的 clippy（告警即败）与不需引擎的测试；PR 从不上自托管机器，带引擎的全部测试是发布的门。
- 审查：每条任务交另一个模型审查，至多三轮（`scripts/review.sh`；R18 起 Cindy reviewer）。
