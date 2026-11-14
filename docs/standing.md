# 常设决定的细则

`agents.md` 的“常设决定”每条只写规定，这里是每条的全文（实现、范围、任务编号）。按日期的完整记录在 `docs/decisions.md`。改了一条，两处一起改。

## 常设决定

- 上游：LibreOffice 26.2.6、MuPDF 1.28.5、Git 2.56.0 加 git-lfs 3.8.0、`gpui-pre` =0.3.11，以 `scripts/fetch-references.sh` 为准；组件库跟随其 main，修复在组件库仓库里做并推上去，再改 pin。
- 仓库与发布在 GitHub 组织 ELY-Open-Source（ElyOffice、Ely-GPUI-Components）。推 `v*` 标签即运行 `.github/workflows/release.yml`：自托管机器上以 `scripts/ci.sh` 构建 macOS（arm64，Rosetta 下的 Intel）、Windows、Linux x86_64（Omarchy 的 AlmaLinux 8 容器）并上传，Linux arm64 在 GitHub 托管的 `ubuntu-24.04-arm` 上用同一容器（自托管机队无 arm64 Linux）；五个任务互不等待。`ci.sh` 先过门，不过即不建：更新日志写明 `[~]` 的数目、距上一版不足七天的原因（`scripts/release-gate.py`），本系统有这一版的基准记录、慢过三成的各有原因（`tests/perf/baseline.py`），该提交的 `pr.yml` 已通过；打包前再以 `scripts/release-tests.sh` 跑全部测试（`ELY_REQUIRE_ENGINES=1`）；最先上传的平台把发布建成预发布，五端都上传、核对过后再改为正式并设为最新（`releases/latest` 是官网下载与应用更新所读）；源码包只由 x86_64 的 Linux 上传。引擎经 `ELY_SHARE` 共用，新的 LibreOffice 补丁须先在三台机器的共享引擎上重建；GitHub 的 arm64 机无共享引擎，现场建。macOS 上 `ci.sh` 须 bash 5，拒绝 3.2（R15.61）。
- 构建一律用 [huacnlee/cargo](https://huacnlee.github.io/cargo/)（替换 `cargo`）：中间产物在 `~/.cargo/targets/`，各工作树与项目共用、自动限额，二进制仍在 `./target`；它忽略 `--target-dir`、`CARGO_TARGET_DIR` 并告警，故基准工具先建 frames 版、复制到 `target/frames`，再建发行版。它的测试程序不在 `./target` 下，找不到旁边的 worker：带内核的单测须设 `ELY_LOK_WORKER`、`ELY_PDF_WORKER`（`release-tests.sh` 自己设）；`cargo check` 与 clippy 须设 `CARGO_CHECK_SHARED_DEPS=0`，否则两份 gpui 混用、类型对不上。PR 的机器（GitHub 托管）是标准 Cargo，脚本两者都得过。
- Git：main 的历史不改写；提交签名，不写任何署名行。
- 许可：自有代码 MIT；随包引擎照各自的许可，Linux 包的许可字段取 `packaging/apps.tsv`。
- macOS 发布以 Developer ID 签名并公证（`ELY_SIGN_IDENTITY`、`ELY_NOTARY_PROFILE`），缺即失败，不出未公证的包。
- 首批发布 ElyDoc、ElyTable、ElyShow、ElyPDF、ElyMail、ElyTree、ElyMark；ElyAgent 第二批，加进同一个包。
- 安装包（2026-11-14 用户定，SF.6、SF.7、SF.19、SF.21）：每个平台一个 ElyOffice 安装包，不出单个应用的包；Windows 是 `ElyOffice-<版本>-windows-x86_64-setup.exe`，组件页七个应用各一个勾、默认全选，“应用和功能”只一项；macOS 是签名并公证的 `.pkg`（Developer ID Installer），“自定”页七个勾，关掉可重定位；Linux 只有 `ely-office` 包与统一的 AppImage，全部装上；WinGet 一份 `ZacharyZhang.ElyOffice`。Doc、Table、Show 共用一份内核（Windows `%LOCALAPPDATA%\Programs\Ely Office\Kernel`，macOS `/Library/Application Support/Ely Office/Kernel`，Linux `/opt/ely-office`，AppImage 解到 `~/.local/lib/ely-office/<编号>`）；内核、常驻引擎与登录项是隐藏组件，三个文档应用都没选时不装。
- 常驻引擎（2026-11-14 用户定，SF.3、SF.4、SF.8、SF.20、SF.22）：登录后每个已装的文档应用常驻一个已过自检的空闲引擎，三端都做，不给开关；Windows 用当前用户的 `Run` 值，macOS 用 LaunchAgent，Linux 用 systemd 用户服务；物理内存 8 GB 起每个应用一个，4 至 8 GB 只给最近用过的，不到 4 GB 不常驻；空闲引擎只在端点上阻塞，不占 CPU；不在时应用自己起，日志记一行。
- 横幅（2026-11-14 用户定，SF.13–SF.15）：只留 ElyDoc 的字体缺失（`doc-fonts`）；其余信息去状态栏的一项、出后果那一刻的对话框、页面位置的错误页或 toast，规则 2 不变。
- 数据只在 `ely_platform::data_dir` 给出的目录；用户文档永不碰；卸载先问。根默认在系统的本地数据目录；设置“存储”可换到所选文件夹下的 `ElyOffice`，指向文件 `folder.json` 留在默认位置，下次启动、无 Ely 应用在运行时搬动（`ely_platform::folder`）。
- 文件类型只写在 `packaging/types.tsv`，只进“打开方式”，从不设为默认（Windows 的“新建”只在该类型无任何处理程序时设）。URL 方案也写在那里，第二列以冒号结尾（`mailto:`）：Linux 的 `x-scheme-handler` 与 `%U`，Windows 的 `Capabilities\URLAssociations` 与 `RegisteredApplications`（只进“默认应用”供选，从不写 `Classes\mailto`），macOS 的 `CFBundleURLTypes`；链接经 `ely_ui::mail_link` 原样交到应用，ElyMail 以引擎的 `mailto` 照 RFC 6068 填好新信（引号里的逗号不切、域名转 IDNA、主题解 MIME 编码词）。“打开方式”里不得出现 LibreOffice，任何脚本都不挂载 LibreOffice 镜像。
- 语言：界面英文、简体中文、西班牙语（es-ES，系统 es-* 同用）、法语（fr-FR，系统 fr-* 同用）、德语（de-DE，“Deutsch”，系统 de-* 同用）、日语（ja-JP，系统 ja-* 同用）、韩语（ko-KR，系统 ko-* 同用）与越南语（vi-VN，“Tiếng Việt”，系统 vi-* 同用），每个应用各语言都全，任一目录缺键即失败，不以英文补；英文、中文以外的六种未经母语者校对，设置里标“预览”（`docs/accessibility.md` 的“译文校对”）；内核界面固定英文，内核给出的固定文字（ElyTable 的状态与错误、单元格样式，ElyDoc 的内置样式名）由 Ely 按界面语言重写，译文照 LibreOffice 26.2.6.3 该语言的译文，文件与主题资源自带的名字（母版、主题色集、用户样式、字体）照原样；ElyTable 导入非 UTF-8 的文本时预选界面语言的 Windows 代码页（中文 GB18030，日文 Windows-932，韩文 Windows-949，西欧各语 Windows-1252，越南文 Windows-1258）；ElyTable 公式为 Excel A1、数字按 en-US；拼写词典只随英文。
- 外观：Doc、Table、Show、PDF 的页面照文件原色，只有四周的界面随深浅色；邮件正文、Mark、Agent 随深浅色。深色底 `#252525`，不用纯黑。有 Omarchy 时随 Omarchy。
- 性能：八个应用都不许卡顿（120 FPS 手感，potato computer 也要流畅），输入延迟与帧时间实测验收；界面线程不等内核、不持 worker 锁，空闲不做事；ElyDoc、ElyTable、ElyShow 的就地保存在下一次打字停顿（500 毫秒无键）开始，至多等 2 秒，期间打的键一并存下（R15.37）；恢复点给等着的键与图块让路（补丁 0090、0106），同一标签让路满 60 秒后下一次不让（R15.56）；命令行读的副本、用户的保存、另存为与自动保存从不让路；前台以外的用户文件首次显示（或别的程序问到）才载入，视图的输入与其图块先于排队的后台工作（R15.57）。
- LOK：worker 启动时关掉内核的 OpenCL（`SAL_DISABLE_OPENCL=1`：其启动自检在 AMD 驱动里会卡死，Calc 在 CPU 上算）；界面只经 `ely_lok::commands::COMMANDS` 发 UNO 命令；外部链接打开时显示保存的值，用户点状态栏或菜单的“更新链接”才经 `.uno:ElyUpdateLinks` 读源，源之后再存即在状态栏说明（`ely_office::Linking`）；内核弹出的对话框即刻关闭并报错，需要的对话框由 Ely 自建；内核改动只以 `patches/libreoffice` 的补丁序列进入，其日志用 `ELY_WARN` 等（补丁 0108），经 worker 的 stderr 进应用日志。
- 更新：每个平台一份清单，更新整套（应用与内核一起换），读 GitHub 的最新发布（`releases/latest/download`，不调 API），不通则读官网转发的同一路径；清单以 ed25519 签名（`<清单>.sig`，签的是文件名加内容），公钥编进应用（`ely_platform::update::KEY`），缺签名或不符即拒；种子是组织机密 `ELY_UPDATE_KEY`，本机副本 `update-key.txt` 不入库，丢失即旧版再也收不到更新；自动检查默认关，从不静默安装；包管理器与 Nix 装的不自更新，只提示。
- 每个应用只跑一份，再次启动把文件交给运行中的那份。
- 本机别的程序经命令行读写已打开的文档并保存：`ely-table cell docs|get|set|save`，`ely-doc text`、`ely-show text`、`ely-mark text` 的 `docs|get|replace|save`。请求走实例门（本机回环、令牌文件只给同一用户），由应用以 `ely_ui::on_ask` 作答，始终开着；ElyTable、ElyDoc、ElyShow 的读写不动用户的光标、选区与所在幻灯片；替换是精确、区分大小写的全部替换，一步撤销（Impress 每个改到的文本框一步）；ElyMark 读写此刻的 Markdown 原文（含未写回的键入），预览模式拒绝；ElyTable 按地址读写由内核补丁 0058 完成。命令行、`docs`、`save` 与按文件找标签在 `ely_office::remote`，各应用只写自己的动词；各命令所读所改的范围见 `docs/help.md`。
- ElyTable 的 R 宏（实验性，TABLE-17）：只在用户点运行时（首次与载入 .R 后先警告）以参数数组起 `Rscript --vanilla`，处理所选区域的类型化副本；R 不是沙箱，以用户权限运行，从不代装；停止、关对话框或 30 秒即整树结束；内核（补丁 0081）在一个撤销组里写回；日志不含代码、单元格值与 R 的输出。细则见 `docs/r-macros.md`。
- ElyMark 原生实现（组件库 Markdown 组件与其 Markdown 树），不用 CEF、Electron；Typora 只作参照，读同一文件不同时随 GitHub（GFM）。
- ElyMail：IMAP 与 POP3 由引擎自写，SMTP 用 mail-send；Microsoft 走 OAuth，客户端 ID 构建时注入（本机 `cred.txt`，CI 为组织机密），缺即构建失败；Gmail 与 iCloud 走应用专用密码，QQ 邮箱、163 邮箱、126 邮箱走授权码。网络随系统代理。
- ElyAgent 以 Rust 依赖直接嵌入 Kigi CLI，不用 ACP。
- 合并前：`.github/workflows/pr.yml` 在每个 PR 与 main 的推送上，以 GitHub 托管的 Ubuntu 建 MuPDF（PDF worker 才编得过）后跑 check.sh、deny.sh、panics.py（数量比 PR 的基线或推送前的提交）、整个工作区的 clippy（告警即败）、发布规则的脚本与不需引擎的测试（严格模式）；PR 从不上自托管机器，带引擎的全部测试是发布的门（release-tests.sh）。
- 审查：每条任务交另一个模型审查，至多三轮（`scripts/review.sh`；R18 起 Cindy reviewer）。

## 技术栈的细节

| 层 | 选型 |
| --- | --- |
| Mail | mail-send、mail-parser、mail-builder，IMAP（imap-proto 解析）与 POP3 由引擎自写（`crates/ely-mail-engine`，应用进程内的后台任务，无 worker）+ SQLite（FTS5）；网络随系统代理（HTTP CONNECT、SOCKS5）；正文以 html5ever 解析、净化后按块的子集由 GPUI 绘制；写信用 `documents::MarkdownDocument`，发出纯文本与 HTML（ElyMail 的 `markup.rs` 由 `documents::markdown_tree` 写出，链接照引擎读信的 `page::link`）；Microsoft 走 OAuth，Gmail 与 iCloud 走应用专用密码，QQ 邮箱、163 邮箱、126 邮箱走授权码 |
| Mark | 组件库 `documents::MarkdownDocument`（GPUI，原文即文本，无序列化）；组件库的 Markdown 树（`documents::mdast`，pulldown-cmark 建）解析与导出 HTML，RaTeX 公式，mermaid-rs-renderer 图；PDF 经 `ely-pdf-worker` 由 MuPDF 排版 |

## 目录的细节

```text
apps/        八个应用，各自一个二进制
crates/      ely-ui  ely-platform  ely-ipc（应用与引擎 worker 之间的帧）；ely-files ely-lok ely-office（文档应用共用的外壳：标签、会话、保存、关闭询问、冲突、密码；引擎为 LOK 或 MuPDF）ely-pdf-engine（PDF worker 的协议与客户端，纯 Rust）ely-pdf-worker（MuPDF worker，独立包，只它编 MuPDF）；ely-mail-engine（邮件协议、同步与本地存储）；ely-mail-fake（测试用的脚本邮件服务器，真 TLS）；ely-start（Windows：双击文件即出启动画面）
reference/   上游源码，只读，不属于本项目，不审查
engines/     下载的二进制与引擎构建产物，不入库，不审查；mupdf-src 是 reference/mupdf 加 patches/mupdf 的构建副本
patches/     对上游的补丁，随源码发行（libreoffice、mupdf、gpui）
vendor/      crates.io 上游 crate 的副本加本项目的修复，经工作区 `[patch.crates-io]` 换入（accesskit_atspi_common、accesskit_unix、gpui-pre、gpui-pre-linux、gpui-pre-macos、gpui-pre-windows、mail-parser、pulldown-cmark、zbus）；首个提交原样，其后修复；gpui 四个即 crates.io 原版加 `patches/gpui`
packaging/   三端打包脚本，apps.tsv 是八个应用的二进制名、名字、桌面分类、发布批次、许可（Linux 包、Nix 与 WinGet 的许可字段）与一句话简介（WinGet）；icons/ 由图标母版渲染各端格式；libreoffice/ely.xcd 是内核的默认配置层
assets/      icons/ 是八个应用的图标母版（SVG）与渲染出的 icns、png、ico，规则见其 README
dist/        打包产物，不入库
docs/        兼容清单、无障碍、帮助、许可、性能记录、功能编号与测试的对照（test-map.md，由 scripts/test-map.py 生成）；验收的证据都在库里
scripts/     fetch-references.sh 取上游  doctor.sh 查工具  review.sh 审查  build-*.sh 建引擎
tests/       scripts 与 engines 下是脚本测试，逐个的说明在 tests/scripts/README.md；engines 缺二进制时报 SKIP；fixtures 是测试共用的样本，逐个的说明在 tests/fixtures/README.md
tasks/       TASKS.md 进度真相
site/        官网（Vite 8、pnpm、TypeScript，无框架；`/docs` 命令行页与 `/changelog` 更新日志在构建时写成，后者每版一个 `changelog/<版本>.md`；中英两种文案在 src/text.ts）；site/video 是发布视频的分镜、配乐、录屏与合成脚本，产物在 site/video/out（不入库）
```
