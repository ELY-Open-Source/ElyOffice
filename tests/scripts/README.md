# 测试脚本

每个脚本测什么、要什么条件。加了、改了脚本，同步改本文件。缺工具时报 SKIP，另有说明的除外。

```text
tests/scripts/references-rules.sh  fetch-references.sh 离线：只取所给的上游、不碰别的（MuPDF 的子模块），未知名字先报错
tests/scripts/engine-arch.sh 架构  macOS 引擎里每个 Mach-O 都是该架构（ci.sh 发布前调用）
tests/scripts/review-rules.sh   review.sh 对替身 herdr：审查者以 Kimi K3.1 Pro、effort high 起，claude 起不来、2 分钟未开工、阻塞、状态读不出、空结论、窗格关不掉皆败，起不来与开工前读不出状态报出 herdr 的错误码，Claude 经 argv 取提示（替身里打进去的提示一律卡住）、herdr 晚认出 Claude 照等、状态按窗格号读，审查者空闲而结论晚到照等，CRLF 的 jq 照常，首个窗格在调用者右侧、其后分在该列最高的窗格下面，两半的终端都留 10 行才分（框占 2 行，24 行分得、23 行不分），列满即另开标签页、读完结论关掉、开不出即报，两个调用者同时放也不压到 10 行以下、锁 30 秒不放即报，阻塞 10 次即报出屏上文字并关窗格、短暂阻塞照等，提示要求一切留在 `.work/<ID>-r<轮次>/`，读完结论即关，失败的留到本条下一轮、别条不关（三端可跑）
tests/scripts/lo-patches.sh    补丁序列从中途接着打、再打一次为空操作
tests/scripts/media-targets.sh 媒体播放接口在 Linux、Windows 目标可编译（缺目标时 SKIP）
cargo test -p ely-platform --test media  系统播放器真放 1 秒片段：可播、出帧、播完、循环、暂停守住、坏文件说原因（macOS、Windows 与 Linux）
tests/scripts/linux-idle.sh      真窗口 ElyTree：打开仓库后、Omarchy 换主题与改设置后，空闲时不读主题、设置，不跑 git status；照样跟随换主题、改设置与改文件，经内存映射的写入也跟随
tests/scripts/linux-backends.sh  Linux 构建带 Wayland 与 X11；Hyprland 上 ElyTree 在两者上都映射出窗口、画出页面与起始页文字（截窗口量色，tesseract 读字）
tests/scripts/linux-xim.sh       X11 上的 ElyMark 与 fcitx5 经 XIM 建成输入上下文（`CreateIcReply`，无 XIM 错误）；fcitx5 没在跑才自起、用完即停（缺 X 显示或 fcitx5 时 SKIP；`ELY_MARK_BIN` 指定要测的构建，不设即跑 Cargo 刚建出的那个；握手前退出或超时即报退出码与应用最后的输出）
tests/scripts/xim-rules.sh       linux-xim.sh 对替身（cargo、fcitx5、应用）：跑 Cargo 报出的构建而非 target 里的旧程序，应用握手前退出即报其退出码与错误（只在 Linux）
tests/scripts/wayland-clipboard.sh  无头 sway（无 Xwayland，X11 剪贴板为空）里与 wl-copy、wl-paste 互相复制粘贴文字、HTML、PNG（容器内构建与运行，缺 docker 时 SKIP）
tests/scripts/linux-glibc.sh     Linux 的应用、worker 与引擎要求的 glibc 不高于 2.28，GLIBCXX、CXXABI 不高于 GCC 8（AlmaLinux 8，即发布的构建容器），读版本需求表，`GLIBC_ABI_DT_RELR` 这类 ABI 标记也拒，缺 readelf 即失败、不跳过；ci.sh 在 Linux 打包后于构建容器里调用，不过即不上传
tests/scripts/linux-glibc-rules.sh  linux-glibc.sh 对 glibc 2.30、2.36 的符号与紧凑重定位（`GLIBC_ABI_DT_RELR`）报错，PATH 里没有 readelf 也报错，只要 2.2.5 的通过（现编的 ELF，只在 Linux）
tests/scripts/elfs-rules.sh      `packaging/linux/elfs.sh`（package.sh 调用）只放行指向包内的 `$ORIGIN` 路径：首尾换行、绝对路径、出包、空项与带符号的 ELF 皆拒，报出包外所需的库（现编的 ELF，须 patchelf，在 container.sh 里跑）
tests/scripts/winget.sh        winget.sh 按假发布写出清单，地址、散列、卸载键、许可与简介照 apps.tsv 与安装程序，`winget validate` 通过（无 winget 时 SKIP），版本不符、缺清单即拒
tests/scripts/nix.sh           nix.sh 按假清单写出、拒绝版本不符的清单；nix/release.json 与 apps.tsv 一致；nixos/nix 容器里 flake 的许可与表相同、ElyMark 构建并在 Xvfb 下画出（缺 docker 时 SKIP 容器部分）
tests/scripts/ci-share.sh        ci.sh 只经指向 ELY_SHARE 的链接用共享引擎：自建目录、指向别处或目标带换行的链接即拒绝
tests/scripts/records.py         TASKS、BUGS、BugList、decisions 引用的库内文件都在，否则标“（已移除）”；引用 .work/ 须标“（本机，未入库）”（pr.yml 调用）
tests/scripts/workflows.py       release.yml 的任务互不等待、都经 ci.sh；ci.sh 先跑 release-tests.sh 再打包；pr.yml 的 runs-on 恰为 ubuntu-24.04、以整行的 `[sudo] apt-get install` 装齐 README 的库、严格模式、panic 数比基线提交（各带反例）
tests/scripts/lo-tarballs.sh     `packaging/lo-tarballs.tsv` 的散列与 `download.lst` 一致、地址是 LibreOffice 的镜像、本机构建所取都在其中（Windows 经 Cygwin）
tests/scripts/source-rules.sh    source.sh 把已取与下载的外部库按内容打包，散列不符或取不到即失败、不出包（假 curl）
tests/scripts/ci-upload.sh       ci.sh 的上传：测试失败即一个不传，Linux 的包过不了 glibc 的上限也不传；发布只建一次且为预发布；macOS 建源码包而不上传，Linux 上传，重跑只换本平台的同名文件、别的平台的不动（假的 GitHub API，在 Linux 或 WSL 运行）
tests/scripts/linux-packages.sh  deb、rpm、Arch 包在干净的 Debian 11、Ubuntu 26.04、Fedora 44、Arch 容器里装上，每个 ELF 都找得到库
tests/scripts/linux-pins.sh      Linux 打包工具（appimagetool、linuxdeploy、运行时、nfpm）的下载与钉的散列相符
tests/scripts/linux-installed.sh installed|appimage  Hyprland 上装好的六个应用或 AppImage：一个窗口、随 Omarchy（Catppuccin）、读出页面文字（PDF、Markdown 经各自引擎），ElyTree 随换主题到 Latte（grim 按窗口截取）
tests/scripts/linux-distros.sh   deb 在 Ubuntu 26.04、rpm 在 Fedora 44 容器里装上，各应用在 Xvfb（Mesa 软件 Vulkan）下画出窗口、随 Omarchy、读出页面文字（与 linux-installed.sh 共用 linux-common.sh）
cargo test -p ely-show --test compose  切换半程帧经 Metal 取像素（主线程，仅 macOS）
tests/scripts/*.sh *.py        脚本测试（缺工具时报 SKIP）
tests/scripts/icons.sh         八个图标齐全、未过期，macOS 为 squircle，其余无底形
tests/scripts/release-macos.sh dist 里首批七个 .app：以访达的方式打开文件（启动与运行中，ElyMail 打开 .eml），引擎都取自包内，带齐许可，不含系统会登记的内嵌包，每个二进制要求的系统不高于应用所写，可执行文件的 `@rpath` 依赖都在包内；源码包齐全
tests/engines/*.sh             引擎探测（LOK、MuPDF）；LOK 探测本构建的内核，不挂载官方镜像
tests/scripts/lo-unregistered.sh  “打开方式”里没有 LibreOffice，构建出的包不声明任何文件类型
tests/scripts/lokview.sh       真窗口里的 LokView：输入法、键入、点击、拖动，AX 树里的段落文本；ELY_CLIPBOARD_TEST=1 再测复制粘贴（会覆盖系统剪贴板）
tests/scripts/doc-shell.sh     真窗口 ElyDoc：空白标签进 AX 树，启动后 Cmd+N 打开新建对话框（需辅助功能权限）
tests/scripts/doc-props.sh     真窗口 ElyDoc 属性栏：段前段后连续键入、回车、失焦，读 AX 值与命令数（需辅助功能权限）
tests/scripts/doc-review.sh    真窗口 ElyDoc：无日期的批注逐条写明作者与原因（AX 可读），多条时批注与修订列表仍在窗口内（需辅助功能权限）
tests/scripts/theme.sh         真窗口 ElyTable 以深、浅两种外观启动，截窗口量网格亮度（需屏幕录制权限）
tests/scripts/lag.sh ely-doc   真窗口键入，从调试日志算输入落地、图块、按键到新图块的分位数（需辅助功能权限）；ELY_LAG_LOG=路径 留下日志
tests/scripts/pdf-idle.sh      真窗口 ElyPDF 无文件启动时无 worker 子进程
tests/scripts/build-deps.sh      干净的 Ubuntu 24.04、Fedora 44、Arch 容器只照 README 的 Linux 行安装：doctor.sh 装前报缺、装后整体通过、移走 make 即报缺，`ely-mail-engine`、整个工作区与官网建得成（缺 docker 时 SKIP）
tests/scripts/doctor-rules.sh    doctor.sh 判过旧的工具为失败（替身 node，看退出码）；Cygwin 里跑也找得到其工具；Windows 上照 gpui 构建脚本的次序找 fxc：`GPUI_FXC_PATH`、PATH、SDK 注册表，皆无即报缺（替身 reg，各例自去调用者 PATH 里的 fxc）
tests/scripts/mac-bash-rules.sh  macOS 上 ci.sh 在 bash 3.2 下不做事即失败、在 bash 5 下照常往下；doctor.sh 判 PATH 里的 bash 3.2 为过旧、5.3 照过，Intel 用的 bash 只认在 Rosetta 下跑得起的 bash 5（arm64 的即缺；只在 macOS）
tests/scripts/check-rules.sh    check.sh 只数代码文件（末行无 LF 也计）、报出超长文件全名、30001 字节的 agents.md 即败、注释与行尾的失败也使其失败
tests/scripts/comments-rules.sh  comments.py 对长注释与跨行注释报错（含 JS、HTML 及其脚本、Python 行尾注释与文档字符串），字符串里的 # 不算，相邻的行尾注释不算跨行，未入库与 vendor 的文件不查
tests/scripts/rpath-rules.sh     rpath.sh（release-macos.sh 逐个调用）对找不到或在包外（含 `..` 与链接）的 `@rpath` 依赖、otool 读不出的文件报错（替身 otool，任意系统可跑）
tests/scripts/bad-settings.sh  通用设置读不出时 ElyDoc 照样启动，文件不被覆盖
tests/scripts/windows-headless-start.sh  无可见桌面（服务、SSH 登录）时 ElyDoc 只记一条启动错误、不弹对话框、以 1 退出（启动失败的对话框在自己的线程，不在 gpui 借用中跑消息循环）；有可见桌面时窗口照开即 SKIP（非 Windows 时 SKIP）
tests/scripts/table-cli.sh     开着 ElyTable 与一个工作簿，另一进程经 `ely-table cell` 列出、读、写（公式）、保存，错误地址与未开的文件被拒；保存答复时读磁盘核对；六万行 xlsx 上保存、写入、再保存交叠，第二次保存答复时写入已在盘上
tests/scripts/doc-cli.sh       开着 ElyDoc 与一个 docx，另一进程经 `ely-doc text` 列出、读正文、替换（含中文）、保存，缺的词、未开的文件与短命令被拒；保存答复时读磁盘核对
tests/scripts/show-cli.sh      开着 ElyShow 与一个 pptx，另一进程经 `ely-show text` 列出、读各页文字、替换（含中文）、保存，缺的词、未开的文件与短命令被拒；保存答复时读磁盘核对
tests/scripts/mark-cli.sh      开着 ElyMark 与一个 md，另一进程经 `ely-mark text` 列出、读原文、替换（含中文）、保存，缺的词、未开的文件与短命令被拒；保存答复时读磁盘核对（`CARGO_TARGET_DIR` 设了即取其中的构建）
tests/scripts/eol.sh            入库文本皆 LF（vendor 照上游字节），新文件按 LF 归一
tests/scripts/eol-rules.sh      eol.sh 读整条路径：带空格的 vendor 路径照放，vendor 之外的报出全名，git 列不出文件即失败
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
tests/scripts/show-fullscreen.sh  真窗口 ElyShow：F5 放映的窗口盖住整块屏幕，按 F5 前看得到的菜单栏与程序坞窗口放映中都不在（看不到菜单栏即 SKIP；需辅助功能与屏幕录制权限）
tests/scripts/dock-macos.sh      dist 里第一批各 .app 与包外的 target/debug/ely-doc 各启动数次：应用自身是一个程序坞图标、观察期内不退出，子进程从未登记为程序坞应用；带内核的三个另核 worker 为 UIElement、图块照出
tests/scripts/dock-rules.sh      dock-macos.sh 对假应用（替身 lsappinfo）：worker 类型张冠李戴、提前退出、查询失败、无用例的应用皆败（只在 macOS）
tests/scripts/windows-bundle.sh  合集安装程序逐个静默运行所选应用的安装程序；某个被拒即停下并报出其名与退出码（缺 makensis 时 SKIP）
tests/scripts/windows-installer.sh  假应用的安装程序：所选目录原有的文件经两次安装与卸载都在，旧版按注册表清理，文件被占用或程序在运行时什么也不改，写不进时放回旧版（缺 makensis 时 SKIP）
tests/scripts/types.sh         packaging/types.tsv 的文件类型与 URL 方案在 Linux 桌面项、Windows 安装程序与 macOS 的 plist 里一致，macOS 的 UTI 照 MIME 类型、级别为 Alternate，URL 方案从不被接管
tests/scripts/windows-a11y.sh   真窗口 ElyDoc 与 ElyPDF：UI Automation 的 TextPattern 读到段落、插入点与选区（含 emoji），PDF 的页文与拖选（缺引擎时 SKIP）
tests/scripts/linux-a11y.sh     同 windows-a11y.sh，经 AT-SPI 的 Text 接口（Orca 所读）：Xvfb 在容器里，私有 AT-SPI 总线，键鼠经 XTest（缺 docker、AT-SPI 的 Python 绑定或引擎时 SKIP）
tests/scripts/windows-names.sh  真窗口七个应用：UI Automation 里每个可操作节点与 Tab 停靠点都有名字，Tab 走得到文档、F6 出得去（样本取自 a11y-samples.sh；缺引擎时 SKIP）
tests/scripts/linux-names.sh    同上，并核对报给辅助技术的窗口位置与 X 窗口相符，经 AT-SPI：Xvfb 在容器里经 TCP，应用与私有 D-Bus、AT-SPI 总线在本机（缺 docker、AT-SPI 的 Python 绑定或引擎时 SKIP）
tests/scripts/macos-names.sh    同上，经 macOS 的 AX（VoiceOver 所读，`ax-names.swift`）：应用空闲后才接入，窗口按钮不算（系统自命名），Tab 停靠点至少两个（窗口按钮是原生的）（需辅助功能权限）
tests/scripts/windows-save-as.py  真窗口 ElyDoc：另存为里键入无扩展名的名字即存为 .docx，同名先问再替换（键鼠真输入，须桌面空闲；非 Windows 时 SKIP）
tests/scripts/windows-tab-keys.py  真窗口 ElyDoc 与 ElyShow 各开四份：真鼠标点标签、Ctrl+W、从外面再开一份，每次第一键只落在前台文档（经 `text get` 核对；键鼠真输入，须桌面空闲；非 Windows 时 SKIP）
tests/scripts/windows-table-save-edit.py  真窗口 ElyTable：单元格键入中经命令行保存，编辑不断、其后的键接着输入（公式栏读回），盘上是保存时已输入的格，回车后再存即有全文（键盘真输入，须桌面空闲；非 Windows 时 SKIP）
tests/scripts/windows-table-zoom.py 目录  真窗口 ElyTable：键、状态栏缩放钮（真点击）与 Ctrl+滚轮在 A1 上缩到 67%、125%、400%，首个行列标题是 A、1（UI Automation），截图里网格线量出 A1 与 B2 一般宽高、光标框正落在 A1 的线上，日志的视图起点为 (0, 0)；滚轮在 C7 上时它的角到指针的像素不变；全部报出；截图写入该目录（键鼠真输入，须桌面空闲；非 Windows 时 SKIP）
tests/scripts/windows-types.sh  真安装程序以假类型 `.elyprobe` 写 HKCU：能打开、只在无处理程序时设默认与“新建”、假方案 `elyprobe:` 进“默认应用”而不接管、别的程序的登记与“新建”不动、换了文件夹后旧卸载不删新的、写不进即安装失败（缺 makensis 时 SKIP）
tests/scripts/layout.sh        真窗口：六个应用在 640、800、1100 宽下无元素越出窗口左右，留截图（需辅助功能与屏幕录制权限）
tests/perf/bench.py [--runs N] [--only PERF-..]  性能基准（PERF-01 至 08），release 构建，结果写 docs/perf；macOS、Windows 直接跑（驱动 tests/perf/drive.py、drive.ps1，Windows 上不在前台即拒发输入）
tests/perf/load.py [--docs N] [--rounds R] [--keys K] [--apps ...]  PERF-10 极限负载：Doc、Table、Show、Mark 各开 N 份（默认 100；每十份一份大的，Doc 另有外部链接的，大工作簿含复杂公式与跨工作簿引用；样本由 loadfiles.py 以 Python 写，不经 Office），前台文档空闲时与其余文档同时经命令行编辑时各按键量反馈、帧与主线程停顿，全部保存后读盘核对编辑、按键与链接，结果写 docs/perf/<日期>-<机型>-load-N.md（frames 发行构建，只在 Windows 实测过）；可用内存低于 12 GB 即停；有人点击或挪动窗口即作废重来，至多三次
tests/perf/extreme.py [--cases ...]  PERF-10 的极端情形，真窗口逐项核对：链接源缺失、源已改（关着与开着）、三份文档切标签时键入与同时编辑、键入中保存、关闭未保存（询问、取消、保存）、编辑中杀掉引擎后从恢复点重开、缩放 25% 至 400% 与滚轮（截图留看）、20 份同时交来再逐个关闭、单个超大文件（500 页 docx、6 万行 xlsx、189 页 pptx、7 MB md）；结果写 docs/perf/<日期>-<机型>-extreme.md（只在 Windows）
tests/perf/bigtable.py 输出 行数 宽 高 jpeg|png inline|anchor|tight [英寸 [cantsplit]]  一张大表格的 DOCX：一列图片一列中文（R14.48 的样本）
tests/perf/linux.sh [参数]      Linux 上的 bench.py：容器里的 Xvfb、私有 AT-SPI 总线（atspi.py）
tests/scripts/ooxml.py <用例> <文件>  不经 LibreOffice 读 DOCX、XLSX、PPTX 并按自算的预期核对（往返用例调用）；ooxml_pivot.py 同法核对透视表的标签与标题属性，ooxml_motion.py 核对视频的淡入与触发序列，ooxml_formula.py 核对公式文字，ooxml_word.py 核对 Word 公式（减号、双线体）与目录
tests/office/compare.ps1 目录   本机 Microsoft Office 经 COM 写出大而复杂的 DOCX、XLSX、PPTX，Ely 内核打开、写入、另存，新引擎重开核对；Office 读回两份，比内容摘要（文字、公式与重算的值、批注、备注、域的目标、动画，Word 更新域后每页的起始），不符即失败（已知差异钉住前后值；须 ELY_LOK_DIR，只在 Windows）
tests/scripts/deny-rules.sh    deny.sh：列着的重复组过、多余或无着落的 skip 败、去掉的旧版回来败（临时 crate）
tests/scripts/panics-rules.sh  panics.py：列着的过、测试模块与 build.rs 不算、新处、增多、消失、无理由皆败（临时 crate）
cd site && pnpm test && pnpm e2e   官网单测（几何、道路连通；更新日志与 git 的 v* 标签一一对应，每次发布加 `site/changelog/<版本>.md` 并部署；命令行页含各应用 USAGE 的每条命令）与 Playwright 端到端（悬停、聚焦、按压、命中归属）
cd site/video/scenes && npm run verify   发布视频：群像与离场两个渲染器在切场处屏幕角点一致，离场全程各屏播自己的录屏帧（真页面渲染）
```
