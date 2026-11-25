# 脚本与命令

日常用的几条在 `agents.md` 的“命令”；这里是全部。测试脚本逐个的说明在 `tests/scripts/README.md`。加了、改了脚本，改这里。

```text
scripts/doctor.sh              本系统构建所需的工具与系统包（与 README 的 Build 逐项对应）
scripts/fetch-references.sh [名字...]  取上游源码到 reference/；给了名字只取那几个（pr.yml 只取 mupdf），名字不认识即报错、不取
scripts/fetch-tessdata.sh      取 OCR 语言数据到 engines/tessdata（钉提交，核散列）
scripts/vendor-gpui.sh [write]  vendor 的 gpui 四个 crate 须是 crates.io 原版（核散列）加 patches/gpui；write 重建（pr.yml 核对）
scripts/build-libreoffice.sh [--symbols]  构建钉住的 LibreOffice 加 patches/libreoffice 到 engines/lo-src；Linux 上把 `packaging/libreoffice/ely-fonts.conf` 的字体别名插进内核 `share/fonts/truetype/fc_local.conf` 的 `</fontconfig>` 前（已有 Ely 的别名即换成当前的，该文件缺即失败），在写内核编号之前；先跑 fetch-fonts.sh，把 `packaging/libreoffice/fonts/SOURCES.tsv` 列的字体放进内核的 `fonts/truetype`；--symbols 另建带符号的一份到 engines/lo-sym，只供采样，不进发布
scripts/fetch-fonts.sh             内核随附的字体：照 `packaging/libreoffice/fonts/SOURCES.tsv`（文件夹、文件、地址、该文件的 SHA-256、上游原件的 SHA-256；地址带 `#包内路径` 即从该 zip 里取）下载到 engines/fonts；上游原件按散列存在 engines/fonts/.upstream，坏的重取；任一散列不符、取不到或包里没有该成员即失败，不留半成品；已有且相符的跳过
scripts/build-git.sh           构建钉住的 Git 到 engines/git（随 ElyTree）
scripts/fetch-git-lfs.sh       取 git-lfs 放进 engines/git（核散列）
scripts/build-openssl.sh       构建钉住的静态 OpenSSL 至 engines/openssl（Intel Mac 与 Linux 的 MuPDF 用，核散列；Mac 核架构）
scripts/build-bash.sh [目录]   构建钉住的 GNU bash 5.3 为 x86_64，默认装到 ~/ely-share/bash-x86_64（Intel 版发布在 Rosetta 下以它跑 ci.sh；核散列与架构）
scripts/review.sh ID 轮次 路径  审查一条任务：herdr 里的 Pi（K3.1 Pro，思考 high）审查，结论进 `tasks/reviews/`；审查者的窗格设 `ELY_DATA_ROOT=.work/<ID>-r<轮次>/data/ElyOffice`，有 `.work/bin/cargo` 时把它排在 PATH 最前；细则见 `tests/scripts/README.md` 的 review-rules.sh
scripts/apply-lo-patches.sh [--check] engines/lo-src  源码已到补丁序列第几个，补上其余（构建脚本调用）；--check 只核不打：须在钉住的提交上且带全序列，否则说缺哪几个并失败（ci.sh 核共享引擎）
scripts/lo-extras.sh 内核目录  列出构建会裁掉、却在内核里的扩展（dict-en 以外的词典，dict-en 里的 Lightproof 等）；build-libreoffice.sh 照它裁，kernel-id.sh 见有即拒
scripts/lo-configured.sh engines/lo-src  该树是否在此处配置的：`config_host.mk` 记的 SRCDIR 不是它的实际路径（树搬过、配置时的目录已删）即失败并点名两处（构建脚本调用；手动 make 前也可先跑）
packaging/linux/container.sh 命令  在 AlmaLinux 8 镜像里运行（glibc 2.28，gcc-toolset-15 把新的 libstdc++ 符号静态链入；cargo 是镜像里从源码建的 huacnlee/cargo，其发布版要 glibc 2.34；产物与本机同在 target/release，镜像里再建即换回）
packaging/linux/package.sh 应用  出该应用的 AppDir 与包的依赖（在容器里运行）；依赖带 glibc 的版本下限（包内文件所需的最高版本）；libdbus 与它载入的库用系统的；`bundle.sh` 再出装全部应用的 `ely-office` 包（deb、rpm、Arch）与一个 `ElyOffice-版本-linux-架构.AppImage`，三个文档应用共用一份内核
packaging/sign.sh 清单...       以 ELY_UPDATE_KEY 写出每份更新清单的 .sig（ci.sh 上传前调）
packaging/nix.sh 标签          由该发布的每应用清单写 nix/release.json（flake.nix 读它；发后提交）；首个统一发布前须换成读 `ely-office-linux-x86_64.json`，否则对它即失败
packaging/winget.sh 标签       由该发布的 `ely-office-windows-x86_64.json` 写 dist/winget 下一份 WinGet 清单（`ZacharyZhang.ElyOffice`，即统一安装程序；发布后交 winget-pkgs）
packaging/aur.sh 标签 [--push]  由该发布的 Arch 包 `ely-office-版本-1-x86_64.pkg.tar.zst` 写 dist/aur/ely-office-bin（PKGBUILD、.SRCINFO、安装脚本；描述、依赖、许可、provides 与安装脚本都取自包内的 .PKGINFO、.INSTALL，conflicts 另加 `ely-office`；源地址是 GitHub 发布，散列为整包的 SHA-256）；包名或版本不符即拒；要 zstd；--push 经 `ssh://aur@aur.archlinux.org/ely-office-bin.git` 推上 AUR，要已登记到 AUR 账号的 SSH 密钥（发布后在 Arch 上跑）
packaging/source.sh dist        本提交的源码包、随包的 copyleft 上游源码（打过补丁的 MuPDF、Git）与 LibreOffice 构建所取的外部库（`packaging/lo-tarballs.tsv`，散列不符即失败）与随内核字体的上游原件（`packaging/libreoffice/fonts/SOURCES.tsv`，先用 engines/fonts/.upstream 的，散列不符即失败），文件名带提交号；有未提交改动即拒绝；发布只上传 Linux 的
scripts/mac-ready.sh 凭据名     Mac 发布前：`coreaudiod` 20 秒内答 `system_profiler SPAudioDataType`（停应时放媒体的测试会挂住），`notarytool` 读得到该公证凭据（登录钥匙串在本会话里锁着即读不到）；任一不行即说明原因与出路并失败（ci.sh 在 macOS 上构建前调）
scripts/release-tests.sh         发布的门：建好两个 worker 并点名，严格模式跑 `cargo test --workspace`，再跑 vendor 下各 crate 自己的测试（`tests/scripts/vendor-tests.sh`，本平台的 gpui-pre 一行缺失或零测试即败；Linux 经 `dbus-run-session`）（ci.sh 打包前调）
scripts/compatibility.py [--check]  写 docs/compatibility.md 的版本与内核两行；--check 不符即败（ci.sh、pr.yml）
scripts/test-map.py            写 docs/test-map.md：PRD 每个编号的任务与其测试（pr.yml 核对与提交一致）
scripts/lo-tarballs.sh lo-src     列出该 LibreOffice 构建所取的压缩包（名字、SHA-256、地址），由其 `Makefile.fetch` 得出
packaging/macos/bundle.sh ely-doc debug    出 dist/ElyDoc.app（带图标、文档类型与该应用的 worker；worker 在辅助包 `Contents/Helpers/Ely Engine.app`，有自己的包标识；内核不在包内）
packaging/macos/kernel.sh 目录             出三个文档应用共用的内核（Frameworks 与 Resources，去掉图库与同义词库，逐个签名后写内核编号）
packaging/macos/release.sh [批次]          该批各应用与 dist/Kernel，合成一个 `ElyOffice-<版本>-<架构>.pkg`：“自定”页每个应用一个勾，不可重定位；内核是隐藏的选项，选了任一文档应用即装到 `/Library/Application Support/Ely Office/Kernel`，装前以旧的 `ely-login stop` 停引擎、再删去旧的；同一选项装 `ely-login` 与登录项（LaunchAgent），装完预热；写指向它的 `ely-office-macos-<架构>.json`；ELY_INSTALLER_IDENTITY 即签名，再加 ELY_SIGN_IDENTITY 与 ELY_NOTARY_PROFILE 即公证
packaging/windows/installer.sh 出 dist 里唯一的 Windows 安装程序 ElyOffice-<版本>-windows-x86_64-setup.exe（首批各应用一个勾，内核旁放 `ely-login.exe`），另写指向它的 `ely-office-windows-x86_64.json` 与各应用的更新清单（0.1.16 读每应用的清单）
packaging/windows/templates.sh 以本构建的内核重写 packaging/windows/new 的空白 DOCX、XLSX、PPTX（右键“新建”用）
packaging/icons/masters.py     写出 assets/icons 的 SVG 母版（图标设计只改这里）
packaging/icons/build.sh       由母版渲染 icns、png、ico（只在 macOS）
scripts/inert-lo.sh 路径.app    构建出的 LibreOffice.app 去掉文档类型、URL 类型、快速查看与聚焦插件并注销（build-libreoffice.sh 调用）
scripts/bench.ps1 -Exe 程序 -File 文件 [-Log ely.log] [-Runs 5]  Windows 测试机上的热打开：每轮关掉程序（引擎留着）再开，给了 -Log 即量到日志里的“first tile drawn”（按日志的时刻），不给即量到 Office 窗口标题带上文件名；末行是中位；须在桌面会话里跑（`schtasks /IT`）
scripts/kernel-id.sh 内核 文件   内核目录各文件的 SHA-256 与各链接的指向合成内核编号，写进 `ely-kernel-id`（在版本文件旁）；应用与引擎握手时核对（build-libreoffice.sh、stage.sh、macOS 的 kernel.sh 调用；已建的引擎跑一次）
scripts/extract-official-lo.sh 用 7zz 把官方镜像的 Frameworks 与 Resources 解到 engines/lo-official 并写其内核编号（不挂载；“未打补丁即拒绝”的单测用）
scripts/check.sh [根]            规则检查：入库代码文件不超 500 行（vendor、reference、patches 除外）、agents.md 不超 30000 字节、注释（comments.py）、测试的等待（waits.py）、横幅只有 `doc-fonts`（banners.py）、行尾（eol.sh）；JS、TS、TSX 与 HTML 的注释经 `tests/scripts/comments.mjs` 由官网的 TypeScript、parse5 解析，先在 `site/` 跑 `pnpm install`
cd site && pnpm run deploy    官网构建并经 wrangler 发到 Cloudflare Workers：另有 `site/worker` 答 `/release.json`（最新版本、安装包地址与下载总数，缓存 5 分钟）并转发 `/releases/` 下的发布文件；地址 https://office.elygpui.com（workers.dev 上同站）
scripts/deny.sh [根] [检查...]  cargo-deny：安全公告、yanked、许可与来源；重复版本只许 deny.toml 里列着的组，组消失而 skip 还在即败
scripts/panics.py [根]          非测试代码的 expect、unwrap、panic!、unreachable!（clippy 量，构建脚本、ely-mail-fake 除外）与 scripts/panics.tsv 逐条相符、各带理由，任一种类不多于 HEAD 的表；clippy 由它自设 `CARGO_CHECK_SHARED_DEPS=0`，编不过时打出编译错；`--sites` 列出本机各处
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
