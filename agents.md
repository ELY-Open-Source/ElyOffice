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
| Mail | mail-send、mail-parser、mail-builder，IMAP（imap-proto 解析）与 POP3 由引擎自写（`crates/ely-mail-engine`，应用进程内的后台任务，无 worker）+ SQLite（FTS5）；网络随系统代理（HTTP CONNECT、SOCKS5）；正文以 html5ever 解析、净化后按块的子集由 GPUI 绘制；写信用 `documents::MarkdownDocument`，发出纯文本与 HTML（ElyMail 的 `markup.rs` 由 `documents::markdown_tree` 写出，链接照引擎读信的 `page::link`）；Microsoft 走 OAuth，Gmail 与 iCloud 走应用专用密码，QQ 邮箱、163 邮箱、126 邮箱走授权码 |
| Tree | Git CLI，参数数组启动 |
| Mark | 组件库 `documents::MarkdownDocument`（GPUI，原文即文本，无序列化）；组件库的 Markdown 树（`documents::mdast`，pulldown-cmark 建）解析与导出 HTML，RaTeX 公式，mermaid-rs-renderer 图；PDF 经 `ely-pdf-worker` 由 MuPDF 排版 |
| Agent | Kigi CLI 的 Rust 代码直接嵌入（不用 ACP），沿用其提供商登录与模型配置 |

## 目录

```text
apps/        八个应用，各自一个二进制
crates/      ely-ui  ely-platform  ely-ipc（应用与引擎 worker 之间的帧）；ely-files ely-lok ely-office（文档应用共用的外壳：标签、会话、保存、关闭询问、冲突、密码；引擎为 LOK 或 MuPDF）ely-pdf-engine（PDF worker 的协议与客户端，纯 Rust）ely-pdf-worker（MuPDF worker，独立包，只它编 MuPDF）；ely-mail-engine（邮件协议、同步与本地存储）；ely-mail-fake（测试用的脚本邮件服务器，真 TLS）
reference/   上游源码，只读，不属于本项目，不审查
engines/     下载的二进制与引擎构建产物，不入库，不审查；mupdf-src 是 reference/mupdf 加 patches/mupdf 的构建副本
patches/     对上游的补丁，随源码发行（libreoffice、mupdf、gpui）
vendor/      crates.io 上游 crate 的副本加本项目的修复，经工作区 `[patch.crates-io]` 换入（accesskit_atspi_common、accesskit_unix、gpui-pre、gpui-pre-linux、gpui-pre-macos、gpui-pre-windows、mail-parser、zbus）；首个提交为原样，其后是修复；gpui 四个即 crates.io 原版加 `patches/gpui`
packaging/   三端打包脚本，apps.tsv 是八个应用的二进制名、名字、桌面分类、发布批次、许可（Linux 包、Nix 与 WinGet 的许可字段）与一句话简介（WinGet）；icons/ 由图标母版渲染各端格式；libreoffice/ely.xcd 是内核的默认配置层
assets/      icons/ 是八个应用的图标母版（SVG）与渲染出的 icns、png、ico，规则见其 README
dist/        打包产物，不入库
docs/        兼容清单、无障碍、帮助、许可、性能记录、功能编号与测试的对照（test-map.md，由 scripts/test-map.py 生成）；验收的证据都在库里
scripts/     fetch-references.sh 取上游  doctor.sh 查工具  review.sh 审查  build-*.sh 建引擎
tests/       scripts 与 engines 下是脚本测试，逐个的说明在 tests/scripts/README.md；engines 缺二进制时报 SKIP；fixtures 是测试共用的样本，逐个的说明在 tests/fixtures/README.md
tasks/       TASKS.md 进度真相
site/        官网（Vite 8、pnpm、TypeScript，无框架；`/docs` 命令行页与 `/changelog` 更新日志在构建时写成，后者每版一个 `changelog/<版本>.md`；中英两种文案在 src/text.ts）；site/video 是发布视频的分镜、配乐、录屏与合成脚本，产物在 site/video/out（不入库）
```

## 组件映射

依赖 `ely-gpui-component`（pin e0f355a，gpui 为 crates.io 的 `gpui-pre` =0.3.11，Rust 1.98.1）。名字取自与本仓库并列的组件库检出 `../Ely-GPUI-Components`（不在 `reference/`；总包 `src/lib.rs` 列出各模块在哪个 crate，gpui 用法规则见其 `AGENTS.md`）；`tests/scripts/agents_md.py` 核对下表每个名字都是组件库公开项，没有那个检出即失败。

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

组件库没有的，在本项目内建：

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
9. 动效只在交互时运行，每段不超过 220 毫秒，遵守系统“减少动效”。空闲时不重绘。目标设备含树莓派。
10. 未验证的能力不写成已完成。

## 命令

测试脚本逐个的用途与条件在 `tests/scripts/README.md`；加了、改了测试脚本，改那里。

```text
scripts/doctor.sh              本系统构建所需的工具与系统包（与 README 的 Build 逐项对应）
scripts/fetch-references.sh [名字...]  取上游源码到 reference/；给了名字只取那几个（pr.yml 只取 mupdf），名字不认识即报错、不取
scripts/fetch-tessdata.sh      取 OCR 语言数据到 engines/tessdata（钉提交，核散列）
scripts/vendor-gpui.sh [write]  vendor 的 gpui 四个 crate 须是 crates.io 原版（核散列）加 patches/gpui；write 重建（pr.yml 核对）
scripts/build-git.sh           构建钉住的 Git 到 engines/git（随 ElyTree）
scripts/fetch-git-lfs.sh       取 git-lfs 放进 engines/git（核散列）
scripts/build-openssl.sh       构建钉住的静态 OpenSSL 至 engines/openssl（Intel Mac 与 Linux 的 MuPDF 用，核散列；Mac 核架构）
scripts/build-bash.sh [目录]   构建钉住的 GNU bash 5.3 为 x86_64，默认装到 ~/ely-share/bash-x86_64（Intel 版发布在 Rosetta 下以它跑 ci.sh；核散列与架构）
scripts/review.sh ID 轮次 路径  审查一条任务：herdr 里另一个 Claude Code（Kimi K3.1 Pro）审查，结论进 `tasks/reviews/`，构建与日志在 `.work/<ID>-r<轮次>/`；细则见 `tests/scripts/README.md` 的 review-rules.sh
scripts/apply-lo-patches.sh engines/lo-src  源码已到补丁序列第几个，补上其余（构建脚本调用）
packaging/linux/container.sh 命令  在 AlmaLinux 8 镜像里运行（glibc 2.28，gcc-toolset-15 把新的 libstdc++ 符号静态链入；目标在 target/alma8）
packaging/linux/package.sh 应用  出该应用的 AppImage、deb、rpm 与 Arch 包到 dist/（在容器里运行）；包的依赖带 glibc 的版本下限（包内文件所需的最高版本）；libdbus 与它载入的库用系统的；`bundle.sh` 再出装全部应用的 `ely-office` 包（deb、rpm、Arch），三个文档应用共用一份内核
packaging/sign.sh 清单...       以 ELY_UPDATE_KEY 写出每份更新清单的 .sig（ci.sh 上传前调）
packaging/nix.sh 标签          由该发布各 AppImage 的更新清单写 nix/release.json（flake.nix 读它；发后提交）
packaging/winget.sh 标签       由该发布 Windows 的更新清单写 dist/winget 下首批各应用的 WinGet 清单（`ZacharyZhang.<名字>`；发布后交 winget-pkgs）
packaging/source.sh dist        本提交的源码包、随包的 copyleft 上游源码（打过补丁的 MuPDF、Git）与 LibreOffice 构建所取的外部库（`packaging/lo-tarballs.tsv`，散列不符即失败），文件名带提交号；有未提交改动即拒绝；发布只上传 Linux 的
scripts/release-tests.sh         发布的门：建好两个 worker、以 `ELY_LOK_WORKER` 与 `ELY_PDF_WORKER` 点名它们，严格模式跑 `cargo test --workspace`（ci.sh 打包前调）
scripts/compatibility.py [--check]  写 docs/compatibility.md 的版本与内核两行；--check 不符即败（ci.sh、pr.yml）
scripts/test-map.py            写 docs/test-map.md：PRD 每个编号的任务与其测试（pr.yml 核对与提交一致）
scripts/lo-tarballs.sh lo-src     列出该 LibreOffice 构建所取的压缩包（名字、SHA-256、地址），由其 `Makefile.fetch` 得出
packaging/macos/bundle.sh ely-doc debug    出 dist/ElyDoc.app（带图标、文档类型与该应用的引擎）
packaging/macos/release.sh [批次]          该批各应用的 release 包与 dmg，另有装全部的 dmg；ELY_SIGN_IDENTITY 加 ELY_NOTARY_PROFILE 即公证
packaging/windows/bundle.sh   由 dist 里各应用的安装程序出合集 ElyOffice 安装程序（先跑 installer.sh）
packaging/windows/templates.sh 以本构建的内核重写 packaging/windows/new 的空白 DOCX、XLSX、PPTX（右键“新建”用）
packaging/icons/masters.py     写出 assets/icons 的 SVG 母版（图标设计只改这里）
packaging/icons/build.sh       由母版渲染 icns、png、ico（只在 macOS）
scripts/inert-lo.sh 路径.app    构建出的 LibreOffice.app 去掉文档类型、URL 类型、快速查看与聚焦插件并注销（build-libreoffice.sh 调用）
scripts/extract-official-lo.sh 用 7zz 把官方镜像的 Frameworks 与 Resources 解到 engines/lo-official（不挂载；“未打补丁即拒绝”的单测用）
scripts/check.sh [根]            规则检查：入库代码文件不超 500 行（vendor、reference、patches 除外）、本文件不超 30000 字节、注释（comments.py，只查入库文件）、行尾（eol.sh）；JS、TS、TSX 与 HTML 脚本的注释经 `tests/scripts/comments.mjs` 由官网的 TypeScript 解析，先在 `site/` 执行 `pnpm install`
cd site && pnpm run deploy    官网构建并经 wrangler 发到 Cloudflare Workers：另有 `site/worker` 答 `/release.json`（最新版本、安装包地址与下载总数，缓存 5 分钟）并转发 `/releases/` 下的发布文件；地址 https://office.elygpui.com（workers.dev 上同站）
scripts/deny.sh [根] [检查...]  cargo-deny：安全公告、yanked、许可与来源；重复版本只许 deny.toml 里列着的组，组消失而 skip 还在即败
scripts/panics.py [根]          非测试代码的 expect、unwrap、panic!、unreachable!（clippy 量，构建脚本、ely-mail-fake 除外）与 scripts/panics.tsv 逐条相符、各带理由，任一种类不多于 HEAD 的表；`--sites` 列出本机各处
cargo check --workspace
cargo test --workspace     `-p` 的 ely-files、ely-mail、ely-agent 与组件库的包重名，写 `ely-mail@0.1`
ELY_GIT=路径 cargo test -p ely-tree  ElyTree 测试用该 Git（系统 Git 须 2.40 起）；不设即用 engines/git 的随包 Git，都没有即失败
rm -rf target/debug/incremental   标准 Cargo 下每次构建或测试后都删（用户定：增量缓存会占满磁盘）；huacnlee/cargo 下无此目录
cargo build -p ely-lok --bins   LOK 单测前先建 worker；单测不会自动重建它
ELY_REQUIRE_ENGINES=1 cargo test  缺 LOK 构建的测试即失败（不设时打 SKIP 后通过）
cargo build -p ely-pdf-worker  ElyPDF 与 ElyMark 的 worker（链接 engines/mupdf-build，ELY_MUPDF_DIR 可改；ELY_PDF_WORKER 指定运行的）
cargo run -p ely-doc          启动一个应用（ElyDoc 先 cargo build -p ely-lok --bins，LOK 取 ELY_LOK_DIR 或 engines/ 下的构建）；ELY_LOG 设日志级别；ELY_SMOKE=1 画一帧即退出，ELY_SMOKE=hold 开着不抢焦点；ELY_NO_DIALOG=1 启动失败不弹对话框
ELY_FRAMES=1                  以 `--features frames` 构建的应用每 2 秒（有绘制时）记一行帧时间：绘制与“弄脏到呈现”的分位数、超过 8.3 毫秒的帧数（gpui 的 profiler 日志）；ELY_DATA_ROOT=绝对路径 换掉数据的默认位置，指向文件也在其中（只供基准与测试）
```

## 常设决定

完整记录（按日期，含实现细节与实测）在 `docs/decisions.md`。新的决定追加到那里；改变下列常规的，同时改本节。

- 上游：LibreOffice 26.2.6、MuPDF 1.28.5、Git 2.56.0 加 git-lfs 3.8.0、`gpui-pre` =0.3.11，以 `scripts/fetch-references.sh` 为准；组件库跟随其 main，修复在组件库仓库里做并推上去，再改 pin。
- 仓库与发布在 GitHub 组织 ELY-Open-Source（ElyOffice、Ely-GPUI-Components）。推 `v*` 标签即运行 `.github/workflows/release.yml`：自托管机器上以 `scripts/ci.sh` 构建 macOS（arm64，Rosetta 下的 Intel）、Windows、Linux x86_64（WSL 里的 AlmaLinux 8 容器）并上传，Linux arm64 在 GitHub 托管的 `ubuntu-24.04-arm` 上用同一个 AlmaLinux 8 容器构建并上传（自托管机队没有 arm64 Linux）；五个任务互不等待，`ci.sh` 打包前先以 `scripts/release-tests.sh` 跑全部测试（`ELY_REQUIRE_ENGINES=1`），一个失败即不出包；最先上传的平台把发布建成预发布，五端都上传、核对过后再改为正式并设为最新（`releases/latest` 是官网下载与应用更新所读）；源码包只由 x86_64 的 Linux 上传。引擎经 `ELY_SHARE` 共用，新的 LibreOffice 补丁须先在三台机器的共享引擎上重建；GitHub 的 arm64 机没有共享引擎，每次发布现场建。macOS 的两个任务以 bash 5 起 `ci.sh`，`ci.sh` 在 macOS 上拒绝 bash 3.2（R15.61）。
- 本机构建用 [huacnlee/cargo](https://huacnlee.github.io/cargo/)（替换 `cargo`，无配置）：中间产物在 `~/.cargo/targets/`，各工作树与项目共用、自动限额，二进制仍在 `./target`；它忽略 `--target-dir`、`CARGO_TARGET_DIR` 并告警，故 `tests/perf/bench.py`、`load.py`（frames 构建另放 `target/frames`）须在标准 Cargo 下跑。它的测试程序不在 `./target` 下，找不到旁边的 worker：带内核的单测须设 `ELY_LOK_WORKER`、`ELY_PDF_WORKER`（`release-tests.sh` 自己设）；`cargo check` 与 clippy 须设 `CARGO_CHECK_SHARED_DEPS=0`，否则两份 gpui 混用、类型对不上。发布与 PR 的机器仍是标准 Cargo，脚本两者都得过。
- Git：main 的历史不改写；提交签名，不写任何署名行。
- 许可：自有代码 MIT；随包引擎照各自的许可，Linux 包的许可字段取 `packaging/apps.tsv`。
- macOS 发布以 Developer ID 签名并公证（`ELY_SIGN_IDENTITY`、`ELY_NOTARY_PROFILE`），缺即失败，不出未公证的包。
- 首批发布 ElyDoc、ElyTable、ElyShow、ElyPDF、ElyMail、ElyTree、ElyMark；ElyAgent 第二批。
- 数据只在 `ely_platform::data_dir` 给出的目录；用户文档永不碰；卸载先问。根默认在系统的本地数据目录；设置“存储”可换到所选文件夹下的 `ElyOffice`，指向文件 `folder.json` 留在默认位置，下次启动、无 Ely 应用在运行时搬动（`ely_platform::folder`）。
- 文件类型只写在 `packaging/types.tsv`，只进“打开方式”，从不设为默认（Windows 的“新建”只在该类型无任何处理程序时设）。URL 方案也写在那里，第二列以冒号结尾（`mailto:`）：Linux 的 `x-scheme-handler` 与 `%U`，Windows 的 `Capabilities\URLAssociations` 与 `RegisteredApplications`（只进“默认应用”供选，从不写 `Classes\mailto`），macOS 的 `CFBundleURLTypes`；链接经 `ely_ui::mail_link` 原样交到应用，ElyMail 以引擎的 `mailto` 照 RFC 6068 填好新信（引号里的逗号不切、域名转 IDNA、主题解 MIME 编码词）。“打开方式”里不得出现 LibreOffice，任何脚本都不挂载 LibreOffice 镜像。
- 语言：界面英文、简体中文、西班牙语（es-ES，系统 es-* 同用）、法语（fr-FR，系统 fr-* 同用）、德语（de-DE，“Deutsch”，系统 de-* 同用）、日语（ja-JP，系统 ja-* 同用）、韩语（ko-KR，系统 ko-* 同用）与越南语（vi-VN，“Tiếng Việt”，系统 vi-* 同用），每个应用各语言都全，任一目录缺键即失败，不以英文补；英文、中文以外的六种未经母语者校对，设置里标“预览”（`docs/accessibility.md` 的“译文校对”）；内核界面固定英文，内核给出的固定文字（ElyTable 的状态与错误、单元格样式，ElyDoc 的内置样式名）由 Ely 按界面语言重写，译文照 LibreOffice 26.2.6.3 该语言的译文，文件与主题资源自带的名字（母版、主题色集、用户样式、字体）照原样；ElyTable 导入非 UTF-8 的文本时预选界面语言的 Windows 代码页（中文 GB18030，日文 Windows-932，韩文 Windows-949，西欧各语 Windows-1252，越南文 Windows-1258）；ElyTable 公式为 Excel A1、数字按 en-US；拼写词典只随英文。
- 外观：Doc、Table、Show、PDF 的页面照文件原色，只有四周的界面随深浅色；邮件正文、Mark、Agent 随深浅色。深色底 `#252525`，不用纯黑。有 Omarchy 时随 Omarchy。
- 性能：八个应用都不许卡顿（120 FPS 手感，树莓派 3 也要流畅），输入延迟与帧时间实测验收；界面线程不等内核、不持 worker 锁，空闲不做事；ElyDoc、ElyTable、ElyShow 的就地保存在下一次打字停顿（500 毫秒无键）开始，至多等 2 秒，期间打的键一并存下（R15.37）；恢复点给等着的键与图块让路（补丁 0090、0106），同一标签让路满 60 秒后下一次不让（R15.56）；命令行读的副本、用户的保存、另存为与自动保存从不让路；前台以外的用户文件首次显示（或别的程序问到）才载入，视图的输入与其图块先于排队的后台工作（R15.57）。
- LOK：worker 启动时关掉内核的 OpenCL（`SAL_DISABLE_OPENCL=1`：其启动自检在 AMD 驱动里会卡死，Calc 在 CPU 上算）；界面只经 `ely_lok::commands::COMMANDS` 发 UNO 命令；外部链接打开时显示保存的值，用户点横幅的“更新链接”才经 `.uno:ElyUpdateLinks` 读源，源之后再存即重现横幅（`ely_office::Linking`）；内核弹出的对话框即刻关闭并报错，需要的对话框由 Ely 自建；内核改动只以 `patches/libreoffice` 的补丁序列进入，其日志用 `ELY_WARN` 等（补丁 0108），经 worker 的 stderr 进应用日志。
- 更新：每个应用每个平台一份清单，读 GitHub 的最新发布（`releases/latest/download`，不调 API），不通则读官网转发的同一路径；清单以 ed25519 签名（`<清单>.sig`，签的是文件名加内容），公钥编进应用（`ely_platform::update::KEY`），缺签名或不符即拒；种子是组织机密 `ELY_UPDATE_KEY`，本机副本 `update-key.txt` 不入库，丢失即旧版再也收不到更新；自动检查默认关，从不静默安装；包管理器与 Nix 装的不自更新，只提示。
- 每个应用只跑一份，再次启动把文件交给运行中的那份。
- 本机别的程序经命令行读写已打开的文档并保存：`ely-table cell docs|get|set|save`，`ely-doc text`、`ely-show text`、`ely-mark text` 的 `docs|get|replace|save`。请求走实例门（本机回环、令牌文件只给同一用户），由应用以 `ely_ui::on_ask` 作答，始终开着；ElyTable、ElyDoc、ElyShow 的读写不动用户的光标、选区与所在幻灯片；替换是精确、区分大小写的全部替换，一步撤销（Impress 每个改到的文本框一步）；ElyMark 读写此刻的 Markdown 原文（含未写回的键入），预览模式拒绝；ElyTable 按地址读写由内核补丁 0058 完成。命令行、`docs`、`save` 与按文件找标签在 `ely_office::remote`，各应用只写自己的动词；各命令所读所改的范围见 `docs/help.md`。
- ElyTable 的 R 宏（实验性，TABLE-17）：只在用户点运行时（首次与载入 .R 后先警告）以参数数组起 `Rscript --vanilla`，处理所选区域的类型化副本；R 不是沙箱，以用户权限运行，从不代装；停止、关对话框或 30 秒即整树结束；内核（补丁 0081）在一个撤销组里写回；日志不含代码、单元格值与 R 的输出。细则见 `docs/r-macros.md`。
- ElyMark 原生实现（组件库 Markdown 组件与其 Markdown 树），不用 CEF、Electron；Typora 只作参照，读同一文件不同时随 GitHub（GFM）。
- ElyMail：IMAP 与 POP3 由引擎自写，SMTP 用 mail-send；Microsoft 走 OAuth，客户端 ID 构建时注入（本机 `cred.txt`，CI 为组织机密），缺即构建失败；Gmail 与 iCloud 走应用专用密码，QQ 邮箱、163 邮箱、126 邮箱走授权码。网络随系统代理。
- ElyAgent 以 Rust 依赖直接嵌入 Kigi CLI，不用 ACP。
- 合并前：`.github/workflows/pr.yml` 在每个 PR 与 main 的推送上，以 GitHub 托管的 Ubuntu 建 MuPDF（PDF worker 才编得过）后跑 check.sh、deny.sh、panics.py（数量比 PR 的基线或推送前的提交）、整个工作区的 clippy（告警即败）与不需引擎的测试（严格模式）；PR 从不上自托管机器，带引擎的全部测试是发布的门（release-tests.sh）。
- 审查：每条任务交另一个模型审查，至多三轮（`scripts/review.sh`；R18 起 Cindy reviewer）。
