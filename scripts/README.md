# 脚本与命令

日常用的几条在 `agents.md` 的“命令”；这里是全部。测试脚本逐个的说明在 `tests/scripts/README.md`。加了、改了脚本，改这里。

```text
scripts/doctor.sh              本系统构建所需的工具与系统包（与 README 的 Build 逐项对应）
scripts/fetch-references.sh [名字...]  取上游源码到 reference/；给了名字只取那几个（pr.yml 只取 mupdf），名字不认识即报错、不取
scripts/fetch-tessdata.sh      取 OCR 语言数据到 engines/tessdata（钉提交，核散列）
scripts/vendor-gpui.sh [write]  vendor 的 gpui 四个 crate 须是 crates.io 原版（核散列）加 patches/gpui；write 重建（pr.yml 核对）
scripts/build-git.sh           构建钉住的 Git 到 engines/git（随 ElyTree）
scripts/fetch-git-lfs.sh       取 git-lfs 放进 engines/git（核散列）
scripts/build-openssl.sh       构建钉住的静态 OpenSSL 至 engines/openssl（Intel Mac 与 Linux 的 MuPDF 用，核散列；Mac 核架构）
scripts/build-bash.sh [目录]   构建钉住的 GNU bash 5.3 为 x86_64，默认装到 ~/ely-share/bash-x86_64（Intel 版发布在 Rosetta 下以它跑 ci.sh；核散列与架构）
scripts/review.sh ID 轮次 路径  审查一条任务：herdr 里另一个 Claude Code 审查，结论进 `tasks/reviews/`；细则见 `tests/scripts/README.md` 的 review-rules.sh
scripts/apply-lo-patches.sh engines/lo-src  源码已到补丁序列第几个，补上其余（构建脚本调用）
packaging/linux/container.sh 命令  在 AlmaLinux 8 镜像里运行（glibc 2.28，gcc-toolset-15 把新的 libstdc++ 符号静态链入；目标在 target/alma8）
packaging/linux/package.sh 应用  出该应用的 AppImage、deb、rpm 与 Arch 包到 dist/（在容器里运行）；包的依赖带 glibc 的版本下限（包内文件所需的最高版本）；libdbus 与它载入的库用系统的；`bundle.sh` 再出装全部应用的 `ely-office` 包（deb、rpm、Arch），三个文档应用共用一份内核
packaging/sign.sh 清单...       以 ELY_UPDATE_KEY 写出每份更新清单的 .sig（ci.sh 上传前调）
packaging/nix.sh 标签          由该发布各 AppImage 的更新清单写 nix/release.json（flake.nix 读它；发后提交）
packaging/winget.sh 标签       由该发布 Windows 的更新清单写 dist/winget 下首批各应用的 WinGet 清单（`ZacharyZhang.<名字>`；发布后交 winget-pkgs）
packaging/source.sh dist        本提交的源码包、随包的 copyleft 上游源码（打过补丁的 MuPDF、Git）与 LibreOffice 构建所取的外部库（`packaging/lo-tarballs.tsv`，散列不符即失败），文件名带提交号；有未提交改动即拒绝；发布只上传 Linux 的
scripts/release-tests.sh         发布的门：建好两个 worker 并点名，严格模式跑 `cargo test --workspace`（ci.sh 打包前调）
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
scripts/check.sh [根]            规则检查：入库代码文件不超 500 行（vendor、reference、patches 除外）、agents.md 不超 30000 字节、注释（comments.py）、测试的等待（waits.py）、行尾（eol.sh）；JS、TS、TSX 与 HTML 的注释经 `tests/scripts/comments.mjs` 由官网的 TypeScript、parse5 解析，先在 `site/` 跑 `pnpm install`
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
