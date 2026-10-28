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
| Mail | mail-send、mail-parser、mail-builder，IMAP（imap-proto 解析）与 POP3 由引擎自写（`crates/ely-mail-engine`，应用进程内的后台任务，无 worker）+ SQLite（FTS5）；网络随系统代理（HTTP CONNECT、SOCKS5）；正文以 html5ever 解析、净化后按块的子集由 GPUI 绘制；写信用 `documents::MarkdownDocument`，发出纯文本与 HTML；Microsoft 走 OAuth，Gmail 与 iCloud 走应用专用密码 |
| Tree | Git CLI，参数数组启动 |
| Mark | 组件库 `documents::MarkdownDocument`（GPUI，原文即文本，无序列化）；组件库的 Markdown 树（`documents::mdast`，pulldown-cmark 建）解析与导出 HTML，RaTeX 公式，mermaid-rs-renderer 图；PDF 经 `ely-pdf-worker` 由 MuPDF 排版 |
| Agent | Kigi CLI 的 Rust 代码直接嵌入（不用 ACP），沿用其提供商登录与模型配置 |

## 目录

```text
apps/        八个应用，各自一个二进制
crates/      ely-ui  ely-platform  ely-ipc（应用与引擎 worker 之间的帧）；ely-files ely-lok ely-office（文档应用共用的外壳：标签、会话、保存、关闭询问、冲突、密码；引擎为 LOK 或 MuPDF）ely-pdf-engine（PDF worker 的协议与客户端，纯 Rust）ely-pdf-worker（MuPDF worker，独立包，只它编 MuPDF）；ely-mail-engine（邮件协议、同步与本地存储）；ely-mail-fake（测试用的脚本邮件服务器，真 TLS）
reference/   上游源码，只读，不属于本项目，不审查
engines/     下载的二进制与引擎构建产物，不入库，不审查；mupdf-src 是 reference/mupdf 加 patches/mupdf 的构建副本
patches/     对上游的补丁，随源码发行（libreoffice、mupdf）
vendor/      crates.io 上游 crate 的副本加本项目的修复，经工作区 `[patch.crates-io]` 换入（gpui-pre、gpui-pre-windows、mail-parser）；首个提交为原样，其后是修复
packaging/   三端打包脚本，apps.tsv 是八个应用的二进制名、名字、桌面分类、发布批次与许可（Linux 包与 Nix 的许可字段）；icons/ 由图标母版渲染各端格式；libreoffice/ely.xcd 是内核的默认配置层
assets/      icons/ 是八个应用的图标母版（SVG）与渲染出的 icns、png、ico，规则见其 README
dist/        打包产物，不入库
docs/        兼容清单、无障碍、帮助、许可、性能记录；engineering-risks.md、qa.md 等内部笔记只在本机，不入库
scripts/     fetch-references.sh 取上游  doctor.sh 查工具  review.sh 开分栏审查  build-*.sh 构建引擎
tests/       scripts 与 engines 下是脚本测试；engines 缺二进制时报 SKIP；fixtures 是测试共用的样本（media/clip.mp4：1 秒 H.264/AAC；pdf/aes128.pdf：AES-128 加密，打开密码 user、所有者密码 owner；pdf/signer.p12 与 signer.der：自签名测试证书，密码 ely；pdf/chain-root.der 与 chain-leaf.der：测试用的根与其签发的证书；show/sections.pptx：t.pptx 加两个章节（p14:sectionLst），sections-utf16.pptx 同而 presentation.xml 为 UTF-16LE，sections-locked.pptx 为其加密版，打开密码 ely，sections-locked-tail.pptx 同而目录移到文件末尾、`EncryptedPackage` 在最后一格；tree/launcher-fails、launcher-held、launcher-pwd、launcher-session：ElyTree 终端启动器测试用的可执行脚本（Git 里 100755）；git/git-2.39、git-2.40：只报版本的假 Git（版本下限测试，Git 里 100755）；mail/sample.eml：带编码主题、分组抄送、纯文本加 HTML、cid 图片与附件的邮件，LF 行尾，测试换成 CRLF）
tasks/       TASKS.md 进度真相
site/        官网（Vite 8、pnpm、TypeScript，无框架；等轴 SVG 积木；中英两种文案在 src/text.ts）；site/video 是发布视频的分镜、配乐、录屏与合成脚本，产物在 site/video/out（不入库）
```

## 组件映射

依赖 `ely-gpui-component`（pin 392a037，gpui 为 crates.io 的 `gpui-pre` =0.3.10，Rust 1.98.1）。名字取自 `reference/Ely-GPUI-Components`：总包 `src/lib.rs` 列出各模块在哪个 crate。gpui 用法规则见该仓库 `AGENTS.md`。`tests/scripts/agents_md.py` 核对下表每个名字都是组件库公开项。

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
| Mail | `mail::{MailboxList, MailList, MailThreadView, MailReader, MailComposer, SignatureEditor, ScheduleSend}` |
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
| LOK 的无障碍事件只报当前段落（文本、插入点、选区，UTF-16 计），整篇文档到不了辅助技术 | `LokView` 以 gpui 的 `a11y_synthetic_children` 把段落交成文本串，插入点与选区换成字符（`view/render/a11y.rs`）；ElyPDF 把所显示页面的逐行文字与选区交成 `Document` 的文本串（`reading.rs`）；都只在辅助技术连上时做 |
| 选中对象的框与手柄：LOK 不画进图块 | `LokView` 按图形选区画框与 8 个手柄；按住手柄拖动经 `setGraphicSelection` 交内核（缩放；裁剪模式下裁剪） |
| 虚拟化 PDF 页面栈（`DocumentViewer` 一次铺全部页，不合 PERF-05） | `crates/ely-pdf-engine`、`crates/ely-pdf-worker`、`apps/ely-pdf` |
| 组件库的大纲项（`lists::Outline`）无无障碍角色：屏幕阅读器读不到 | 上游待办；ElyMark 的搜索失败改为结果列表里的行。`feedback::Banner` 已修（fac20ab：危险与警告为 alert，其余为 status，名为其文字），`feedback::Alert` 同（105bf94，名为标题与正文） |
| `documents::ZenMode` 把正文夹进 Sm 宽的列，与 ElyMark 的正文宽度设置冲突 | ElyMark 不用它，专注模式原生重做（C14.12） |
| 表格公式栏与名称框（网格是 LOK 图块，`tables::Spreadsheet` 不用） | `apps/ely-table` |
| Doc/Table/Show 工具栏（`documents::FixedFormatToolbar` 绑 `TextInput`，不接 LOK） | 各 app，用 `shell::Toolbar` 组合 |
| CSV 导入的编码、分隔符、引号、列类型（`misc::CsvImporter` 只读逗号 CSV、映射到属主字段，文案写死英文） | `apps/ely-table` 的 `text.rs`，内核按选项导入 |
| 导出格式与说明（`misc::ExportDialog` 无 TSV，文案写死英文，不能附说明） | 各 app，用 `overlays::Dialog` 组合 |
| 幻灯片缩略图条（`documents::PageThumbnailList` 只收图片路径，无隐藏标记、右键菜单与重排） | `apps/ely-show` 的 `slides.rs`，内核按页绘制到内存图 |
| PDF 页面缩略图（同上，且一次铺全部页，不合 PERF-05） | `apps/ely-pdf` 的 `side.rs`，`uniform_list` 只画可见行，worker 按需渲染 |
| 演示的参考线：LOK 不把参考线画进图块，也无参考线接口 | 补丁 0027 读写参考线；`apps/ely-show` 的 `guides.rs` 画在画布上，拖动移动，拖出幻灯片即删除 |
| gpui 文本里不能嵌图片：行内公式无从排版 | 组件库 `render/math_text.rs`：以不断行的空格占出公式的宽（整个公式作一个词折行），排好的公式画在其上，按下处读回公式的原文 |
| Omarchy 色板要照 `A11yChecker` 的同一套对照规则 | `ely-ui` 的 `omarchy/palette.rs` 抄了对照表；组件库自 70ab3c5 起公开 `devtools::audit`，待改用 |
| 组件无 role/aria 语义：菜单、列表等控件在 AX 树仍不可见 | 上游已修核心控件（7380afb：Button、TextInput、Tabs、面板行、对话框等）；其余上游待办。项目自绘控件已自带 |
| gpui 无 aria-disabled API（`AriaProperties` 无 disabled 字段） | 组件 7380afb 起以本地化 aria_description 表达；上游待办：gpui 补 API |
| 组件内部文案未接 i18n（Ely 用到的组件已接：提示、默认占位、补全空行、字号示例，6df9ab0；查找框、排序与筛选、密码开关，a4bf696；`i18n/tests/shipped.rs` 防回退；其余组件之外：欢迎页标题、菜单项等） | 面板族文案已接 i18n（7380afb，内建 en/zh-CN），`CommentSidebar` 与 `RelativeTime` 同（408f2b0）；其余上游待办 |
| PPTX 的章节：内核无此结构，导入即丢 | `ely_office::files::sections` 打开时读 `ppt/presentation.xml` 的 `p14:sectionLst`，有即 `Risk::Sections`（XML 按 BOM 与字节辨 UTF-16，UTF-32 报错）；加密的 PPTX（OLE 里的 `EncryptedPackage`）看不到章节，即 `Risk::Sealed`；二者皆横幅说明，保存改为另存副本（原文件不动），自动保存与命令行保存不写原文件 |
| LOK 的 `.uno:StyleApply` 状态给英文界面名（`Default Paragraph Style`），样式列表给程序名（`Standard`），两者对不上；内核界面固定英文，内置样式名到不了中文 | ElyDoc `props/builtin.rs`：照 LibreOffice 26.2 的 SwStyleNameMapper、strings.hrc 与 zh-CN 译文列出内置的段落、字符、页面样式（程序名、英文、中文），状态里的界面名经它换回程序名，菜单按界面语言显示；用户自建的样式照原名。更新样式仍用状态里的界面名 |

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

```text
scripts/doctor.sh              工具链版本
scripts/fetch-references.sh    取上游源码到 reference/
scripts/fetch-tessdata.sh      取 OCR 语言数据到 engines/tessdata（钉提交，核对散列）
scripts/build-git.sh           构建钉住的 Git 到 engines/git（ElyTree 随包）
scripts/fetch-git-lfs.sh       取 git-lfs 放进 engines/git（核对散列）
scripts/build-openssl.sh       构建钉住的静态 OpenSSL 到 engines/openssl（Intel Mac 与 Linux 的 MuPDF 用，核对散列；Mac 核对架构）
tests/scripts/engine-arch.sh 架构  macOS 引擎里每个 Mach-O 都是该架构（ci.sh 发布前调用）
scripts/review.sh ID 轮次 路径  审查一条任务（herdr 右侧分栏，codex gpt-6-astra，effort xhigh，fast）
scripts/apply-lo-patches.sh engines/lo-src  找出源码已到补丁序列第几个，补上其余（构建脚本调用）
tests/scripts/lo-patches.sh    补丁序列从中途接着打、再打一次为空操作
tests/scripts/media-targets.sh 媒体播放接口在 Linux、Windows 目标可编译（缺目标时 SKIP）
cargo test -p ely-platform --test media  系统播放器真放 1 秒片段：可播、出帧、播完、循环、暂停守住、坏文件说原因（macOS、Windows 与 Linux）
tests/scripts/linux-idle.sh      真窗口 ElyTree：打开仓库后、Omarchy 换主题与改设置后，空闲时不读主题、设置，不跑 git status；照样跟随换主题、改设置与改文件，经内存映射的写入也跟随
tests/scripts/linux-backends.sh  Linux 构建带 Wayland 与 X11；Hyprland 上 ElyTree 在两者上都映射出窗口、画出页面与起始页文字（截窗口量色，tesseract 读字）
tests/scripts/wayland-clipboard.sh  无头 sway（无 Xwayland，X11 剪贴板为空）里与 wl-copy、wl-paste 互相复制粘贴文字、HTML、PNG（容器内构建与运行，缺 docker 时 SKIP）
packaging/linux/container.sh 命令  在 AlmaLinux 8 镜像里运行（glibc 2.28，gcc-toolset-15 把新的 libstdc++ 符号静态链入；引擎、应用、打包；目标在 target/alma8）
tests/scripts/linux-glibc.sh     Linux 的应用、worker 与引擎要求的 glibc 不高于 2.31，GLIBCXX、CXXABI 不高于 GCC 10（Debian 11、Ubuntu 20.04），读版本需求表，`GLIBC_ABI_DT_RELR` 这类 ABI 标记也拒
tests/scripts/linux-glibc-rules.sh  linux-glibc.sh 对 glibc 2.36 的符号与紧凑重定位（`GLIBC_ABI_DT_RELR`）报错，只要 2.2.5 的通过（现编的 ELF，只在 Linux）
packaging/linux/package.sh 应用  出该应用的 AppImage、deb、rpm 与 Arch 包到 dist/（在容器里运行）
packaging/sign.sh 清单...       以 ELY_UPDATE_KEY 写出每份更新清单的 .sig（ci.sh 上传前调用）
packaging/nix.sh 标签          由该发布各 AppImage 的更新清单写 nix/release.json（flake.nix 读它；发布后提交）
tests/scripts/nix.sh           nix.sh 按假清单写出、拒绝版本不符的清单；nix/release.json 与 apps.tsv 一致；nixos/nix 容器里 flake 的许可与表相同、ElyMark 构建并在 Xvfb 下画出（缺 docker 时 SKIP 容器部分）
packaging/source.sh dist        本提交的源码包与随包的 copyleft 上游源码（打过补丁的 MuPDF、Git），文件名带提交号；工作区有未提交改动即拒绝（macOS 的 release.sh 与 CI 的 Linux 调用，发布只上传 Linux 的；Windows 的检出把符号链接写成文件，不出源码包）
tests/scripts/ci-share.sh        ci.sh 只经指向 ELY_SHARE 的链接用共享引擎：自建目录、指向别处或目标带换行的链接即拒绝
tests/scripts/ci-upload.sh       ci.sh 的上传：macOS 建源码包而不上传，Linux 上传，重跑只换本平台的同名文件、别的平台的不动（假的 GitHub API，在 Linux 或 WSL 运行）
tests/scripts/linux-packages.sh  deb、rpm、Arch 包在干净的 Debian 11、Ubuntu 26.04、Fedora 44、Arch 容器里装上，每个 ELF 都找得到库
tests/scripts/linux-pins.sh      Linux 打包工具（appimagetool、linuxdeploy、运行时、nfpm）的下载与钉的散列相符
tests/scripts/linux-installed.sh installed|appimage  Hyprland 上装好的六个应用或 AppImage：一个窗口、随 Omarchy（Catppuccin）、读出页面文字（PDF、Markdown 经各自引擎），ElyTree 随换主题到 Latte（grim 按窗口截取）
tests/scripts/linux-distros.sh   deb 在 Ubuntu 26.04、rpm 在 Fedora 44 容器里装上，各应用在 Xvfb（Mesa 软件 Vulkan）下画出窗口、随 Omarchy、读出页面文字（与 linux-installed.sh 共用 linux-common.sh）
cargo test -p ely-show --test compose  切换半程帧经 Metal 取像素（主线程，仅 macOS）
tests/scripts/*.sh *.py        脚本测试（缺工具时报 SKIP）
packaging/macos/bundle.sh ely-doc debug    出 dist/ElyDoc.app（带图标、文档类型与该应用的引擎）
packaging/macos/release.sh [批次]          该批各应用的 release 包与 dmg，另有装全部的 ElyOffice dmg；ELY_SIGN_IDENTITY 加 ELY_NOTARY_PROFILE 即公证
packaging/windows/bundle.sh   由 dist 里各应用的安装程序出合集 ElyOffice 安装程序（先跑 installer.sh）
packaging/windows/templates.sh 以本构建的内核重写 packaging/windows/new 的空白 DOCX、XLSX、PPTX（右键“新建”用）
packaging/icons/masters.py     写出 assets/icons 的全部 SVG 母版（图标设计只改这里）
packaging/icons/build.sh       由 assets/icons 的母版渲染 icns、png、ico（只在 macOS）
tests/scripts/icons.sh         八个图标齐全、未过期，macOS 为 squircle，其余无底形
tests/scripts/release-macos.sh dist 里首批六个 .app：以访达的方式打开文件（启动与运行中），引擎都取自包内，带齐许可，不含系统会登记的内嵌包，每个二进制要求的系统不高于应用所写，可执行文件的 `@rpath` 依赖都在包内；源码包齐全
tests/engines/*.sh             引擎探测（LOK、MuPDF）；LOK 探测本构建的内核，不挂载官方镜像
scripts/inert-lo.sh 路径.app    构建出的 LibreOffice.app 去掉文档类型、URL 类型、快速查看与聚焦插件并注销（build-libreoffice.sh 调用）
scripts/extract-official-lo.sh 用 7zz 把官方镜像的 Frameworks 与 Resources 解到 engines/lo-official（不挂载，无包，“未打补丁即拒绝”的单测用）
tests/scripts/lo-unregistered.sh  “打开方式”里没有 LibreOffice，构建出的包不声明任何文件类型
tests/scripts/lokview.sh       真窗口里的 LokView：输入法、键入、点击、拖动，AX 树里的段落文本；ELY_CLIPBOARD_TEST=1 再测复制粘贴（会覆盖系统剪贴板）
tests/scripts/doc-shell.sh     真窗口 ElyDoc：空白标签进 AX 树，启动后 Cmd+N 打开新建对话框（需辅助功能权限）
tests/scripts/doc-props.sh     真窗口 ElyDoc 属性栏：段前段后连续键入、回车、失焦，读 AX 值与命令数（需辅助功能权限）
tests/scripts/doc-review.sh    真窗口 ElyDoc：无日期的批注逐条写明作者与原因（AX 可读），多条时批注与修订列表仍在窗口内（需辅助功能权限）
tests/scripts/theme.sh         真窗口 ElyTable 以深、浅两种外观启动，截窗口量网格亮度（需屏幕录制权限）
tests/scripts/lag.sh ely-doc   真窗口键入，从调试日志算输入落地、图块、按键到新图块的分位数（需辅助功能权限）；ELY_LAG_LOG=路径 留下日志
tests/scripts/pdf-idle.sh      真窗口 ElyPDF 无文件启动时无 worker 子进程
tests/scripts/comments-rules.sh  comments.py 对长注释与跨行注释报错
tests/scripts/rpath-rules.sh     rpath.sh（release-macos.sh 逐个调用）对找不到或在包外（含 `..` 与链接）的 `@rpath` 依赖、otool 读不出的文件报错（替身 otool，任意系统可跑）
tests/scripts/bad-settings.sh  通用设置读不出时 ElyDoc 照样启动，文件不被覆盖
tests/scripts/table-cli.sh     开着 ElyTable 与一个工作簿，另一进程经 `ely-table cell` 列出、读、写（公式）、保存，错误地址与未开的文件被拒；保存答复时读磁盘核对；六万行 xlsx 上保存、写入、再保存交叠，第二次保存答复时写入已在盘上
tests/scripts/doc-cli.sh       开着 ElyDoc 与一个 docx，另一进程经 `ely-doc text` 列出、读正文、替换（含中文）、保存，缺的词、未开的文件与短命令被拒；保存答复时读磁盘核对
tests/scripts/show-cli.sh      开着 ElyShow 与一个 pptx，另一进程经 `ely-show text` 列出、读各页文字、替换（含中文）、保存，缺的词、未开的文件与短命令被拒；保存答复时读磁盘核对
tests/scripts/mark-cli.sh      开着 ElyMark 与一个 md，另一进程经 `ely-mark text` 列出、读原文、替换（含中文）、保存，缺的词、未开的文件与短命令被拒；保存答复时读磁盘核对（`CARGO_TARGET_DIR` 设了即取其中的构建）
tests/scripts/eol.sh            入库文本皆 LF（vendor 照上游字节），新文件按 LF 归一
tests/scripts/mark-build.sh     由不带 engines 与 reference 的 `git archive HEAD` 构建 ElyMark，`ELY_MUPDF_DIR` 指空目录
tests/scripts/worker-rebuild.sh 改了 MuPDF 垫片（只动时间），worker 的构建脚本即重跑
tests/scripts/data-folder.sh   指向文件要求搬动时，ElyMark 启动即把数据搬进所选文件夹、切换并记日志
tests/scripts/one-instance.sh  第二份 ElyDoc 把文件交给第一份后退出，第一份开出它、工作目录不动
tests/scripts/update.sh        装好的 ElyTree 从本机服务的新版下载、替换、重新打开（需辅助功能权限）
tests/scripts/settings.sh      真窗口：ElyPDF 设置里选深色，ElyTree 即刻变深；作者名写入、不丢外观；ElyTree 拒绝非 Git 程序；设置文件读不出时提示且不覆盖；主窗口重开回到原处（需辅助功能与屏幕录制权限）
tests/scripts/privacy.sh       真窗口 ElyDoc：键入的文字不进日志；worker 的日志进应用日志；设置里的诊断先列出所含内容，导出的文件去掉家目录与姓名、不含正文（需辅助功能权限）
tests/scripts/panels.sh        真窗口 ElyDoc：左侧栏默认不宽于 230、拖动内侧边缘变宽、宽度存入设置、重启后照旧（需辅助功能权限）
tests/scripts/storage.sh       真窗口 ElyMark：设置里的存储列出各类大小，日志当场清空，恢复副本下次启动前清掉，恢复点数写入设置（需辅助功能权限）
tests/scripts/recovery.sh      真窗口 ElyDoc：恢复点、崩溃后副本、连续两次崩溃、只杀引擎后重新打开（需辅助功能权限）
tests/scripts/pdf-recovery.sh  真窗口 ElyPDF：加锁文件的封存恢复点、空闲引擎退出后重开时再问密码（需辅助功能权限）
tests/scripts/autosave.sh      真窗口 ElyDoc：自动保存开关、停手即写、外部改动不覆盖（需辅助功能权限）
tests/scripts/print.sh         真窗口 ElyDoc：打印用文档自己的纸张，取消不动文档（需辅助功能权限）
cargo test -p ely-platform --test print  Windows 经“Microsoft Print to PDF”出作业，核对纸张、方向与页面位置；Linux 起私有会话总线与替身打印门户，核对页面设置、双面、令牌与交出的字节、取消（缺 dbus-daemon 时 SKIP）
tests/scripts/show-fullscreen.sh  真窗口 ElyShow：F5 放映的窗口盖住整块屏幕（菜单栏与程序坞之上，需辅助功能权限）
tests/scripts/dock-macos.sh      dist 里 ElyDoc、ElyTable、ElyShow 与包外的 target/debug/ely-doc 各启动数次：LOK worker 从未登记为程序坞应用，图块照出
tests/scripts/windows-bundle.sh  合集安装程序逐个静默运行所选应用的安装程序；某个被拒即停下并报出其名与退出码（缺 makensis 时 SKIP）
tests/scripts/windows-installer.sh  假应用的安装程序：所选目录原有的文件经两次安装与卸载都在，旧版按注册表清理，文件被占用或程序在运行时什么也不改，写不进时放回旧版（缺 makensis 时 SKIP）
tests/scripts/types.sh         packaging/types.tsv 的文件类型与 URL 方案在 Linux 桌面项、Windows 安装程序与 macOS 的 plist 里一致，macOS 的 UTI 照 MIME 类型、级别为 Alternate，URL 方案从不被接管
tests/scripts/windows-a11y.sh   真窗口 ElyDoc 与 ElyPDF：UI Automation 的 TextPattern 读到段落、插入点与选区（含 emoji），PDF 的页文与拖选（缺引擎时 SKIP）
tests/scripts/windows-names.sh  真窗口七个应用：UI Automation 里每个可操作节点与 Tab 停靠点都有名字，Tab 走得到文档、F6 出得去（样本取自 a11y-samples.sh；缺引擎时 SKIP）
tests/scripts/linux-names.sh    同上，经 AT-SPI：Xvfb 在容器里经 TCP，应用与私有 D-Bus、AT-SPI 总线在本机（缺 docker、AT-SPI 的 Python 绑定或引擎时 SKIP）
tests/scripts/windows-types.sh  真安装程序以假类型 `.elyprobe` 写 HKCU：能打开、只在无处理程序时设默认与“新建”、假方案 `elyprobe:` 进“默认应用”而不接管、别的程序的登记与“新建”不动、换了文件夹后旧卸载不删新的、写不进即安装失败（缺 makensis 时 SKIP）
tests/scripts/layout.sh        真窗口：六个应用在 640、800、1100 宽下无元素越出窗口左右，留截图（需辅助功能与屏幕录制权限）
tests/scripts/ooxml.py <用例> <文件>  不经 LibreOffice 读 DOCX、XLSX、PPTX 并按自算的预期核对（往返用例调用）
cd site && pnpm run deploy    官网构建并经 wrangler 发到 Cloudflare Workers（静态资源，另有 `site/worker` 只答 `/release.json`：GitHub API 的最新版本、其中三个完整安装包（Mac arm64、Mac x86_64、Windows）的地址与全部发布的下载总数，加上 Gitea 上迁移前的下载（Gitea 不通时只计 GitHub；都不计更新清单 `.json`），缓存 5 分钟；下载按钮按访客的平台直接下载完整安装包，Linux 与单独的应用指向 `releases/latest`），地址 https://office.elygpui.com（workers.dev 地址 https://ely-office.zhangyanghaha0407.workers.dev 同站）
cd site && pnpm test && pnpm e2e   官网单测（几何、道路连通）与 Playwright 端到端（悬停、聚焦、按压、命中归属）
cd site/video/scenes && npm run verify   发布视频：群像与离场两个渲染器在切场处屏幕角点一致，离场全程各屏播自己的录屏帧（真页面渲染）
cargo check --workspace
cargo test --workspace
ELY_GIT=路径 cargo test -p ely-tree  ElyTree 测试用该 Git（系统 Git 须 2.40 起）；不设即用 engines/git 的随包 Git，都没有即失败
rm -rf target/debug/incremental   每次构建或测试后都删（用户定：增量缓存会占满磁盘）
cargo build -p ely-lok --bins   LOK 单测前先建 worker；单测不会自动重建它
cargo build -p ely-pdf-worker  ElyPDF 与 ElyMark 的 worker（链接 engines/mupdf-build，ELY_MUPDF_DIR 可改；运行时 ELY_PDF_WORKER 可指定）
cargo run -p ely-doc          启动一个应用（ElyDoc 先 cargo build -p ely-lok --bins，LOK 取 ELY_LOK_DIR 或 engines/ 下的构建）；ELY_LOG 设日志级别；ELY_SMOKE=1 开窗口、画一帧即退出，ELY_SMOKE=hold 开着不抢焦点；ELY_NO_DIALOG=1 启动失败不弹对话框
ELY_FRAMES=1                  以 `--features frames` 构建的应用每 2 秒（有绘制时）记一行帧时间：绘制与“弄脏到呈现”的分位数、超过 8.3 毫秒的帧数（gpui 的 profiler 日志）；ELY_DATA_ROOT=绝对路径 换掉数据的默认位置，指向文件也在其中（只供基准与测试，不碰用户数据）
```

## 常设决定

完整记录（按日期，含实现细节与实测）在 `docs/decisions.md`。新的决定追加到那里；改变下列常规的，同时改本节。

- 上游：LibreOffice 26.2.6、MuPDF 1.28.5、Git 2.56.0 加 git-lfs 3.8.0、`gpui-pre` =0.3.10，以 `scripts/fetch-references.sh` 为准；组件库跟随其 main，修复在组件库仓库里做并推上去，再改 pin。
- 仓库与发布在 GitHub 组织 ELY-Open-Source（ElyOffice、Ely-GPUI-Components）。推 `v*` 标签即运行 `.github/workflows/release.yml`：自托管机器上以 `scripts/ci.sh` 构建 macOS（arm64，Rosetta 下的 Intel）、Windows、Linux（WSL 里的 AlmaLinux 8 容器）并上传。引擎经 `ELY_SHARE` 共用，新的 LibreOffice 补丁须先在三台机器的共享引擎上重建。
- Git：main 的历史不改写；提交签名，不写任何署名行。
- 许可：自有代码 MIT；随包引擎照各自的许可，Linux 包的许可字段取 `packaging/apps.tsv`。
- macOS 发布以 Developer ID 签名并公证（`ELY_SIGN_IDENTITY`、`ELY_NOTARY_PROFILE`），缺即失败，不出未公证的包。
- 首批发布 ElyDoc、ElyTable、ElyShow、ElyPDF、ElyMail、ElyTree、ElyMark；ElyAgent 第二批。
- 数据只在 `ely_platform::data_dir` 给出的目录；用户文档永不碰；卸载先问。根默认在系统的本地数据目录；设置“存储”可换到所选文件夹下的 `ElyOffice`，指向文件 `folder.json` 留在默认位置，下次启动、无 Ely 应用在运行时搬动（`ely_platform::folder`）。
- 文件类型只写在 `packaging/types.tsv`，只进“打开方式”，从不设为默认（Windows 的“新建”只在该类型无任何处理程序时设）。URL 方案也写在那里，第二列以冒号结尾（`mailto:`）：Linux 的 `x-scheme-handler` 与 `%U`，Windows 的 `Capabilities\URLAssociations` 与 `RegisteredApplications`（只进“默认应用”供选，从不写 `Classes\mailto`），macOS 的 `CFBundleURLTypes`；链接经 `ely_ui::mail_link` 原样交到应用，ElyMail 以引擎的 `mailto` 照 RFC 6068 填好新信（引号里的逗号不切、域名转 IDNA、主题解 MIME 编码词）。“打开方式”里不得出现 LibreOffice，任何脚本都不挂载 LibreOffice 镜像。
- 语言：界面英文与简体中文；内核界面固定英文，内核给出的固定文字（ElyTable 的状态与错误、单元格样式，ElyDoc 的内置样式名）由 Ely 按界面语言重写，文件与主题资源自带的名字（母版、主题色集、用户样式、字体）照原样；ElyTable 公式为 Excel A1、数字按 en-US；拼写词典只随英文。
- 外观：Doc、Table、Show、PDF 的页面照文件原色，只有四周的界面随深浅色；邮件正文、Mark、Agent 随深浅色。深色底 `#252525`，不用纯黑。有 Omarchy 时随 Omarchy。
- 性能：八个应用都不许卡顿（120 FPS 手感，树莓派 3 也要流畅），输入延迟与帧时间实测验收；界面线程不等内核、不持 worker 锁，空闲不做事。
- LOK：worker 启动时关掉内核的 OpenCL（`SAL_DISABLE_OPENCL=1`：其启动自检在 AMD 驱动里会卡死，Calc 在 CPU 上算）；界面只经 `ely_lok::commands::COMMANDS` 发 UNO 命令；内核弹出的对话框即刻关闭并报错，需要的对话框由 Ely 自建；内核改动只以 `patches/libreoffice` 的补丁序列进入。
- 更新：每个应用每个平台一份清单，读 GitHub 的最新发布（`releases/latest/download`，不调 API）；清单以 ed25519 签名（`<清单>.sig`，签的是文件名加内容），公钥编进应用（`ely_platform::update::KEY`），缺签名或不符即拒；种子是组织机密 `ELY_UPDATE_KEY`，本机副本 `update-key.txt` 不入库，丢失即旧版再也收不到更新；自动检查默认关，从不静默安装；包管理器与 Nix 装的不自更新，只提示。
- 每个应用只跑一份，再次启动把文件交给运行中的那份。
- 本机别的程序经 `ely-table cell docs|get|set|save` 读写 ElyTable 已打开工作簿的单元格并保存：请求走实例门（本机回环、令牌文件只给同一用户），由应用以 `ely_ui::on_ask` 作答；始终开着；按地址读写由内核补丁 0058 完成，不动用户的光标与选区。ElyDoc 同样经 `ely-doc text docs|get|replace|save`：读的是内核导出的 fodt 副本里正文的段落（`ely_office::remote::body_text`，不含脚注、批注、文本框与删除的修订），替换是 Writer 的全部替换（精确、区分大小写、静默，光标照旧，一步撤销，一处没有即报错）。ElyShow 经 `ely-show text docs|get|replace|save`：读 fodp 副本里每张幻灯片的文本对象（`ely_office::remote::slide_text`，一段一行，幻灯片之间换页符，不含备注与批注，正是 LOK 下 Impress 全部替换所及），替换同为 `text.replace`，照 Impress 每个改到的文本框一步撤销；Impress 有选中对象时只在其中替换，故先取消选中、替换后按序号重选（重读对象核对未变，再以新版本号选），所在幻灯片与选中照旧。ElyMark 经 `ely-mark text docs|get|replace|save`：读的是文档此刻的 Markdown 原文（含未写回的键入），替换是原文里精确、区分大小写的全部替换，经组件库 `MarkdownDocument::replace` 先写回开着的块、一步撤销；预览模式拒绝。命令行、`docs`、`save` 与按文件找标签在 `ely_office::remote`，各应用只写自己的动词。
- ElyMark 原生实现（组件库 Markdown 组件与其 Markdown 树），不用 CEF、Electron；Typora 只作参照，读同一文件不同时随 GitHub（GFM）。
- ElyMail：IMAP 与 POP3 由引擎自写，SMTP 用 mail-send；Microsoft 走 OAuth，客户端 ID 构建时注入（本机 `cred.txt`，CI 为组织机密），缺即构建失败；Gmail 与 iCloud 走应用专用密码。网络随系统代理。
- ElyAgent 以 Rust 依赖直接嵌入 Kigi CLI，不用 ACP。
- 审查：每条任务由 `scripts/review.sh` 交 codex 审查，至多三轮。
