# 验收核对表

R14.49：2026-10-27 的审计（`OPTIMIZATION.md`（本机，未入库），五十项）逐条的结论，三个新命令行与 ElyMark 的跨段选取。2026-10-30 据提交 8760766 写成，2026-11-06 据 9fcfa05f 补入 R16 加审的结论；状态以 `tasks/TASKS.md` 为准，“过审”是末轮审查 PASS，“已修，末轮后未复审”是三轮用尽、末轮的缺陷已修并有回归测试。

## 五十项

| 编号 | 级 | 问题 | 结论与证据（任务、首个提交、末轮审查） |
| --- | --- | --- | --- |
| A-01 | P1 | PRD 与发布范围矛盾 | R14.1 过审（738dc9e，审查 r1 PASS） |
| A-02 | P0 | `[x]` 语义被违反 | R14.46 过审（18f5029；R16.26 加审 r4 PASS，07402fef） |
| A-03 | P1 | 未通过复审即关闭 | 不修：每条至多三轮是用户定的规则；R14.46 把末轮 FAIL 的 `[x]` 如实改标 |
| A-04 | P0 | BUGS.md 状态陈旧 | R14.46 过审（18f5029；R16.26 加审 r4 PASS，07402fef） |
| A-05 | P2 | 同一问题双重登记 | R14.46 过审（18f5029；R16.26 加审 r4 PASS，07402fef） |
| A-06 | P0 | 验收证据不入库 | R14.47 过审（18f5029，审查 r3 PASS） |
| A-07 | P1 | 结论性文档过期 | R14.8 过审（c17f7df，审查 r3 PASS）；R14.47 过审（18f5029，审查 r3 PASS） |
| A-08 | P2 | 过程文档体积 | 不修：体积不是缺陷，审查按编号找条目 |
| A-09 | P2 | 历史悬空引用 | R14.46 过审（18f5029；R16.26 加审 r4 PASS，07402fef） |
| B-01 | P0 | 没有合并前 CI | R14.44 过审（12a2166；R16.25 加审 r4 PASS，acc22a9f）；main 不设分支保护（用户 2026-10-30 定） |
| B-02 | P0 | 发布不以测试为门 | R14.43 过审（cfcd05e，审查 r2 PASS） |
| B-03 | P0 | 测试静默跳过 | R14.40 过审（c3a0625，审查 r1 PASS） |
| B-04 | P1 | ElyTree 测试依赖随包 Git | R14.28 过审（e16a6de，审查 r2 PASS） |
| B-05 | P2 | 权限测试不兼容 root | R14.12 过审（a2f09a8，审查 r1 PASS） |
| B-06 | P1 | 构建依赖未写全 | R14.41 过审（d950854；R16.24 加审 r4 PASS，179bb17b） |
| B-07 | P1 | 发布 runner 单点 | R14.43 过审（cfcd05e，审查 r2 PASS）；Linux 任务改上 GitHub 托管机器未做：每次要重建或缓存十余 GB 的内核 |
| B-08 | P2 | 行尾没有约束 | R14.33 过审（fee9074，审查 r3 PASS） |
| B-09 | P2 | 项目规则无自动检查 | R14.34 未过审（0e883f5；R16.21 加审 r4、r5 FAIL 各修，r6 FAIL 五处未修，367ba304；根治须以真 HTML 解析器读） |
| C-01 | P1 | ElyShow 媒体 | R14.17 过审（8ea2251，审查 r2 PASS）；R14.18 过审（e6ee7a2，审查 r2 PASS） |
| C-02 | P1 | ElyPDF 证书信任 | R14.21 未过审（de49701；R16.12 加审 r4–r6 皆 FAIL，r6 之修 3ed9bbca 未审） |
| C-03 | P1 | 自动更新安装 | R14.4 过审（2102edb，审查 r1 PASS） |
| C-04 | P2 | ElyTree 系统集成 | R14.27 过审（1c34485；R16.17 加审 r4 PASS，bb00c3eb） |
| C-05 | P1 | 验证覆盖不对称 | R14.36 过审（af23267，审查 r2 PASS） |
| C-06 | P2 | 会报错的入口仍可见 | R14.27 过审（1c34485；R16.17 加审 r4 PASS，bb00c3eb） |
| D-01 | P0 | 未与 Microsoft Office 交叉核对 | R14.36 过审（af23267，审查 r2 PASS）；与 Microsoft Office 的交叉核对未做：没有 Office |
| D-02 | P0 | 保存时静默丢数据 | R14.20 过审（88a7055，审查 r3 PASS）；R14.37 过审（2bb2a5a；R16.22 加审 r4 PASS，0f9e2b0c） |
| D-03 | P1 | ElyMail 不能成为默认邮件程序 | R14.23 过审（52dd148，审查 r3 PASS） |
| D-04 | P1 | ElyMail 不在窗口测试中 | R14.45 过审（b2320dd，审查 r3 PASS） |
| D-05 | P2 | Gmail 依赖应用专用密码 | R14.24 过审（ddcd63e，审查 r2 PASS） |
| D-06 | P1 | 用户提出的问题长期未修 | R14.5 过审（bf6c963，审查 r3 PASS） |
| D-07 | P2 | ElyMail 协议与显示限制 | 不修：四项是兼容清单写明的限制，无用户反馈 |
| D-08 | P2 | ElyAgent 占位 | 不修：ElyAgent 属第二批，发布按批次排除 |
| E-01 | P1 | 应用之间的编译耦合 | R14.32 过审（ddf394c，审查 r3 PASS） |
| E-02 | P0 | vendor 的 gpui 无记录且阻塞升级 | 不修：vendor 首个提交为原样、其后每个修补一个提交即记录，修补已随 0.3.10 移过（R14.5a）；向 Zed 提交属对外动作 |
| E-03 | P1 | 上游补丁的维护负担 | 不修：补丁头部的上游状态不改行为，改补丁即改引擎的键、三台机器重建内核 |
| E-04 | P1 | 依赖治理 | R14.35 过审（6ba0b26，审查 r2 PASS） |
| E-05 | P1 | 更新通道写死 | R14.2 过审（56c2c43，审查 r1 PASS） |
| E-06 | P2 | 官网与产品同仓 | 不修：官网同仓是目录约定，拆仓库属对外动作 |
| F-01 | P0 | 更新包的真实性 | R14.3 过审（66351d4，审查 r2 PASS）；R14.49 过审（22efdb3，审查 r3 PASS）；Windows 安装程序的 Authenticode 签名未做：没有代码签名证书 |
| F-02 | P1 | ElyMark 被 AGPL 传染 | 不修：ElyMark 自身代码仍是 MIT，MuPDF worker 是单独进程，许可字段照实列出随包组件，源码随发行 |
| F-03 | P1 | unsafe 缺说明 | 不修：无 SAFETY 注释不是缺陷，健全与否要逐处审 |
| G-01 | P1 | panic 点 | R14.38 过审（71988f7；R16.23 加审 r4、r5 FAIL 各修，r6 PASS，71e15ae1） |
| G-02 | P1 | 审计出的缺陷积压 | R14.49 过审（22efdb3，审查 r3 PASS） |
| H-01 | P1 | 性能基线过期 | R14.39 过审（c4a0cb8，审查 r3 PASS） |
| H-02 | P1 | ElyMark 内存 | R14.31 过审（ee77c0b，审查 r2 PASS） |
| H-03 | P2 | 性能项覆盖不全 | R14.39 过审（c4a0cb8，审查 r3 PASS） |
| H-04 | P2 | 120 fps 未实测 | R14.39 过审（c4a0cb8，审查 r3 PASS）；120 Hz 未实测：本机显示器最高 75 Hz |
| I-01 | P1 | 文档内容对读屏不可达 | R14.7 过审（2fce780，审查 r2 PASS）；R14.52 过审（3cf3652，审查 r2 PASS）；R14.49 过审（22efdb3，审查 r3 PASS）；VoiceOver 不测（用户 2026-10-30 定）；AT-SPI 的文本读取由 R14.52 实测（`linux-a11y.sh`）；ElyDoc 的正文整篇交出，Orca 从插入点连读到文末（R18.7，`linux-a11y.sh`）；NVDA 的连读未测，表格与演示仍只到当前单元格、当前段落 |
| I-02 | P1 | 无障碍复核范围 | R14.8 过审（c17f7df，审查 r3 PASS）；R14.53 过审（e2929f7；R16.6 加审 r4 PASS，dcfd7e27）；R14.49 过审（22efdb3，审查 r3 PASS）；macOS 七个应用（含 ElyMail）由 R14.53 复核（`macos-names.sh`），修 VoiceOver 焦点总报窗口（gpui-pre-macos）与 ElyMark 的 F6 只在块间走（组件库 63543a0）；三端的名字测试到页面即核 F6 出得去 |
| I-03 | P2 | 国际化缺口 | R14.6 过审（a52d706，审查 r2 PASS） |


## 三个命令行

2026-10-30 在 Windows 上据提交 dfa1b8a 重跑，皆过：开着应用与一个文件，另一进程列出、读、替换（含中文）、保存，保存答复时读磁盘核对；缺的词、未开的文件与短命令被拒。

| 命令 | 任务 | 测试 | 结果 |
| --- | --- | --- | --- |
| `ely-doc text docs\|get\|replace\|save` | R14.15 过审 | `tests/scripts/doc-cli.sh` | 过 |
| `ely-show text docs\|get\|replace\|save` | R14.19 过审 | `tests/scripts/show-cli.sh` | 过 |
| `ely-mark text docs\|get\|replace\|save` | R14.30 过审 | `tests/scripts/mark-cli.sh` | 过 |

## ElyMark 的跨段选取

R14.29（过审：R16.18 加审 r4 PASS，bda2e9a0）：可视模式拖动与 Shift 加上下越出所开的块，所跨的块合成一个原文文本框，复制、删除、键入随之，一步撤销，行内格式逐块施加。修在组件库；2026-10-30 在 pin 9d922a3 上重跑组件库 `ely-documents` 的 `document::tests::span`、`document::tests::format` 与 `blocks::tests::span`，49 个皆过；当时的 pin 01fe68d 推送前组件库全部库测试皆过。

## GitHub issue

| issue | 任务 | 结论 | 测试 |
| --- | --- | --- | --- |
| #49 腾讯文档导出的 docx 永远停在“正在打开” | R14.50 过审（r3 PASS） | 未复现：二十余个变体在 0.1.12 与今日的内核上 1 至 4 秒打开，根因要报告者的文件，用户定暂缓、等其回复。已修“卡住无路可走”：载入过 30 秒，空页或标签之上都出横幅与“停止文档引擎”（写明会关闭全部文档、丢失恢复点之后的改动），停止后载入失败、接上“重新打开文档” | `ely-lok` 的 `client::tests::stop`；ElyDoc 的 `tests::slow`（真窗口里按下两个按钮） |
| #50 ElyTable 自动筛选加搜索 | R14.51 过审（r3 PASS） | 值列表上方加搜索框（有名），按显示的名字、完整大小写折叠（unicase 2.10）；搜索只收窄所见，未见项的勾选照旧，“全部”只作用于所见，无匹配时说明 | `data::dialogs` 单测；`tests::data::the_column_search_narrows_the_list_and_keeps_hidden_ticks`（真窗口里键入、点单项与“全部”） |
| R14.48 23 MB 的大表格 DOCX 打不开 | R14.48 过审（r2 PASS）；用户定：视同已修 | 用户判定与 90 MB 的公开样本同一问题（.docx 名而非 Office 包），载入前查文件头已修；八种真表格样本（`tests/perf/bigtable.py`）在三端 0.1 至 3.6 秒打开；原文件不再等 | `ely-office` 的 `files::tests::a_package_name_over_other_bytes_fails_before_loading` |

## 第二次审计（2026-11-10，二十四项）

审计（`OPTIMIZATION-New.md`（本机，未入库））的 R-01 至 R-12、N-01 至 N-12，任务在 `tasks/TASKS.md` 的 R18。“过审”是末轮审查 PASS（`tasks/reviews/`）。

| 编号 | 级 | 问题 | 任务与结论 |
| --- | --- | --- | --- |
| R-01 | P0 | 与 Microsoft Office 交叉核对 | R18.27 过审（r1）：三十份 Ely 存出的文件在 Office 网页版里以编辑模式打开，无一报修复；R18.42 过审（r2）：另十八份大、怪、极限的文件（400 行带图的表、300 页、10 万行、1000 列、207 张幻灯片、套 12 层的表与 10 层的组合、加密件）全部打开，无一报修复，查出并修好加密件保存后不再加密；桌面版 Office 与内容逐项比对未做 |
| R-02 | P0 | Windows 安装包未签名 | 用户定：不做（等 SignPath） |
| R-03 | P1 | 十一项 `[x]` 末轮是 FAIL | R18.31 过审（r2）：九项本有通过的合并复审，R7.7、R15.11c 改 `[~]`；`records.py` 守住 |
| R-04 | P1 | 过期条目 | R18.31 过审（r2）：六项关闭或改写 |
| R-05 | P1 | 兼容清单落后 | R18.26 过审（r2）：`compatibility.py` 写出版本与补丁范围，不符即发布失败 |
| R-06 | P1 | 未复审的项更多 | R18.32：说明公布 `[~]` 的数目（门 r1 过审；2026-11-20 起写在不公开的 `docs/releases/<版本>.md`，R20.30）；涉及保存、恢复、更新、发信的十一项各补审一轮，全部过审 |
| R-07 | P1 | 证书信任未过审 | R18.19 三轮用尽（`[~]`）：r3 查出的 Unicode 3.2 规范化差异已修；自签名不受信的测试三端都有 |
| R-08 | P1 | 上游补丁 | R18.11 过审（r2）：`patches/UPSTREAM.md` 列出 53 处纯缺陷修复与复现；一个都没交上游（要所有者的账号与贡献协议），验收的“一半有链接”未达 |
| R-09 | P2 | gpui 修改压成一个提交 | R18.23 过审（r2）：`patches/gpui` 十三个补丁，`vendor-gpui.sh` 由原版重建并逐字节核对 |
| R-10 | P1 | 性能基线过期 | R18.30 三轮用尽（`[!]`）：门的一半过审（r2）；基准进发布的门，缺项与变慢三成都须写原因；两处退步查明并修好一处；ElyTree 的“内存上涨”是 malloc 分区，不是泄漏（`MALLOC_ARENA_MAX=2`）；本机 Linux 全量记录已写（`7937f129`），全部达标；三行按键到帧较 WSL2 下的上一份慢过三成，原因未查明（环境与代码同时变了）；r3 查出 PERF-08 的两处漏报，末轮后已修、未复审 |
| R-11 | P2 | 更新只依赖 github.com | R18.25 过审（r1）：屏蔽 GitHub 后经官网转发检查与下载都成 |
| R-12 | P1 | 读屏只读到当前段落 | R18.7 过审（r3）：ElyDoc 交出正文全部段落，Orca 从插入点连读到文末；表格与演示仍只到当前格、当前段 |
| N-01 | P0 | Windows 内核开着 DDE 服务 | R18.9 过审（r1）：补丁 0107，Windows 实测带补丁过、撤补丁败 |
| N-02 | P1 | 发布节奏 | R18.33 过审（r2）：间隔不足七天须写原因（2026-11-20 起写在 `docs/releases/<版本>.md`，R20.30），首行缺日期即败 |
| N-03 | P1 | CI 是事后检查 | R18.33 过审（r2）：标签所在提交的 `pr.yml` 未成功即不建不传 |
| N-04 | P1 | 六种语言无母语者校对 | R18.6 过审（r2）：标“预览”，会造成数据后果的文案逐条对照 LibreOffice 译文；母语者校对仍无，如实写“无” |
| N-05 | P1 | glibc 三处不一致 | R18.2 过审（r2） |
| N-06 | P2 | R 宏以用户权限运行 | R18.14 过审（r1）：首次运行与载入 `.R` 后必先警告 |
| N-07 | P1 | 切标签等恢复点 | R18.8 过审（r3）：补丁 0114 至 0123；Windows 三份切换里 ElyShow 出图 P95 27 毫秒（验收要不过 100），恢复点最长间隔 44 毫秒；ElyShow 27 键里 1 键过 100 毫秒（一次进入文本编辑的回车 584 毫秒，原因未查明）；早先一次运行 ElyShow 一份文件少存了 9 个键，原因未查明 |
| N-08 | P2 | 内核补丁的日志不存在 | R18.10 过审（r3） |
| N-09 | P2 | 不稳定的测试 | R18.15 过审（r1，连接线拖动两处根因）；R18.34 三轮用尽（`[!]`）：等待写明所等（`waits.py`），三轮各查出检查的漏判（范围、出口归属、闭包与字符串），末轮的三处已修、未复审；Mac 十次查出一处内核崩溃（补丁 0119）、一处错序（0120）、worker 激活策略与两个只等时间的测试，已修；`d972b376` 上 Mac 十次全过；其后两端再跑，查出缩进、换屏两个测试丢掉事件与导航窗格漏读大纲（应用的缺陷），已修；`4377e68e` 上发布测试 Linux、Mac 各连跑十次全过（每次 2478、2445 项） |
| N-10 | P2 | 无 IPv6 即失败 | R18.21 过审（r1） |
| N-11 | P2 | crate 重名 | R18.22 外部阻塞（`[!]`）：另一方是组件库已发布的包，改名要所有者的 crates.io 账号；现用 `-p ely-files@0.1` |
| N-12 | P2 | 过程文档膨胀 | R18.24 过审（r2）：`agents.md` 不过 30000 字节 |

同批的议题与两项实测：

| 项 | 任务与结论 |
| --- | --- |
| #75 拖文件进窗口即打开 | R18.5 过审（r2） |
| #76 首行缩进按字符 | R18.12 过审（r3）：补丁 0109 |
| #77、#42 合集包与体积 | R18.3 过审（r3） |
| #79 XLSX 打开慢 | R18.13 过审（r1）：文件本身不慢，慢在内核冷启动；启动自检改用本应用的类型，少读 28 MB |
| #80 ElyPDF 新建无反应 | R18.17 过审（r2） |
| #81 图片拖进 ElyDoc | R18.36 过审（r1） |
| #82 装着的字体报“本机缺少字体” | R18.39：补丁 0124，字体按任一语言的家族名找到（思源宋体即 Source Han Serif SC）；Mac、Windows 上新测试撤补丁败、带补丁过（含带脚本后缀的名字），Linux 上 fontconfig 两名皆认；过审（r3） |
| #48 ElyMail 未译的按钮 | R18.20 过审（r2） |
| #73、#74 | R18.18 过审（r1）：0.1.14 起已修，已关闭 |
| #14 换版式后的灰色箭头 | R18.16 阻塞（`[!]`）：照步骤复现不出，等报告者的截图或文件 |
| #16 WinGet、#37 Windows arm64 | R18.4 外部阻塞（`[!]`）：七个提交等微软人工审核；无 arm64 的 Windows 可验 |
| 极限测试（五个应用各 20 份大文件同时开着并编辑） | R18.28 过审（r2）：无崩溃、无互锁、键与改动全部读回；命令行忙时 Doc 7/30、Table 3/30、Show 14/30 个键过 100 毫秒 |
| 人与命令行同时改一份文件 | R18.29 过审（r3）：Doc、Table、Show、Mark 各一轮，两边的改动都在；查出的丢键与撤销次序已修 |
| 双击文件先出透明窗口、再等很久（用户报） | R18.37 过审（r2）：worker 进程改在自己的线程上起，Windows 实测窗口 104 毫秒即画出；余下的长等是内核冷读与 Defender 扫描，根治要签名（R-02 暂缓）；三端实测见下一行 |
| 启动画面（用户要求，照 Word） | R18.41 过审（r3）：三端实测首帧 Linux 0.2、macOS 0.2 至 0.3 秒（刚建好后首次 0.7）；Windows 的空白在程序代码之前，是 Defender 扫描新的可执行文件（40 MB 约 2 秒）。新包 `ely-start`（370 KB）：Windows 的“打开”经它；直接起它，启动画面 115 至 191 毫秒出现，应用首帧后 10 至 25 毫秒换下；经系统的“打开”，新类型、新文件第一次 1521 毫秒出现（多出的约 1.3 秒在系统起它之前，原因未查明），再开 84 与 115 毫秒；装好的完整安装包上未双击核对（测试机建不出安装包）；macOS、Linux 不加 |
| 测试在临时目录留下内核配置 | R18.40 过审（r2）：worker 活得比测试的临时目录久，退出时重建配置目录；改为先关 worker 再删目录。Linux、Mac 的发布测试（2482、2449 项）新添 0 个；Windows 只跑了 ely-lok、ely-office、ely-doc 三个包（487、24、135 项），新添 0 个 |
| 官网与旧视频工程的依赖告警 | R18.43 过审（r2）：undici、fast-uri、zod 钉到修复版，GitHub 的 13 个告警全部关闭 |
| 外部 PR #70、#71、#72 与组件库 #13、#14 | R18.38：五个都已合并并答复（作者保留）；#70、#72、组件库 #14 过审（r1），#71 与组件库 #13 过审（r2）；合并前后量了 ElyTree 的历史首屏与搜索，P50 不变 |
| 内核测试与负载下查出的问题 | 补丁 0118（PPTX 导入多加的撤销锁，导出时版式临时页记成“删除幻灯片”，撤销一次多出一页）、0119（Mac 上 PDF 导出给已销毁的焦点窗口加等待光标而崩溃）、0120（选中文本框后第一个键异步进入编辑，排在后面的键先到）、0121（恢复点记下未检查输入的各段时长）、0122（并记下最长间隔两端的位置）、0123（PPTX 恢复点在母版比对、形状、图片处查输入，本机最长间隔 46 → 3 毫秒）；macOS worker 以“常规”激活策略起来时改为转附属；`zbus` 日志默认只记警告 |

## 未完成

| 任务 | 状态 | 缘由 |
| --- | --- | --- |
| D-01 与 Microsoft Office 交叉核对 | 已做（网页版，2026-11-10 至 11，R18.27、R18.42）：三十份 Ely 存出的 DOCX、XLSX、PPTX 与十八份大、怪、极限的文件在 Office 网页版里全部打开，无一报修复 | `docs/compatibility.md` 的“与 Microsoft Office 的交叉核对”；桌面版 Office 与内容逐项比对未做 |
| F-01 Windows 安装程序的 Authenticode 签名 | 用户定：暂缓 | 没有代码签名证书；已申请 SignPath Foundation，批下后再接 |
| H-04 120 Hz 实测 | 用户定：不做 | 本机显示器最高 75 Hz |
| B-07 Linux 发布任务上 GitHub 托管机器 | 用户定：不做 | Linux 照旧在 WSL 里构建、在 Omarchy 上实测；每次上托管机器要重建或缓存十余 GB 的内核；Linux 任务已不等 Windows |

## R15 议题、流畅与多语言

R15.24：R15 的九十条逐条核对，2026-11-19 据 b72d8e35 写成，读每条的全文与其各轮审查。“过审”即 `tasks/TASKS.md` 里为 `[x]` 且末轮审查 PASS（R15.66、R15.70 由 R18.8、R18.9 做完，列其审查）；“部分”为 `[~]`，“未完”为 `[!]`。证据列该条为此修所加、今日树里仍在的回归测试（`#[test]`，至多两个）或测试文件与脚本，再列末轮审查；只因负载或别的缘故在该条里提到的测试不列。“未验证或余下”照该条最后的写法与末轮审查所记；该条后文已解决的不列，数字取最新的。本表不重跑测试。

| 编号 | 状态 | 事 | 证据 | 未验证或余下 |
| --- | --- | --- | --- | --- |
| R15.0 | 过审 | `review.sh` 改用 herdr 桌面应用的 `hc` 开分栏 | 无专属测试；`tasks/reviews/R15.0-r3.md` PASS | — |
| R15.1 | 过审 | Linux 安装包减重：剥符号、去多余资源、deb/rpm 改 xz | 无专属测试（`packaging/linux/package.sh` 遇带 `.symtab`、`.debug_*` 的 ELF 即失败）；`tasks/reviews/R15.1-r1.md` PASS | — |
| R15.2 | 过审 | ElyPDF 缩放与换模式后留在原处，不跳回首页 | `a_zoom_beside_a_short_page_keeps_its_place_in_the_row`、`single_mode_keeps_the_spot_on_the_tall_page_of_a_pair`；`tasks/reviews/R15.2-r5.md` PASS | — |
| R15.3 | 过审 | 核实 ElyPDF 左侧栏可收起（0.1.11 已做） | 只做了核实，无专属测试；`tasks/reviews/R15.3-r1.md` PASS | — |
| R15.4 | 过审 | 缩略图拖动排页，拖到边缘自动滚动 | `a_drag_held_at_the_list_foot_rolls_to_the_last_page`、`a_drop_mid_roll_lands_on_the_line`；`tasks/reviews/R15.4-r3.md` PASS | — |
| R15.5 | 过审 | PDF 文本段落可拖动移位，Delete 删除选中对象 | `a_moved_block_keeps_fonts_spacing_colour_and_hidden_text`、`delete_removes_the_aimed_object_and_nothing_else`；`tasks/reviews/R15.5-r8.md` PASS | MuPDF 补丁 0007、0008 待重建进 `engines/mupdf-build`、macOS 构建与三台机器的共享引擎；r8：Windows 无 MuPDF，原生 PDF 测试未跑 |
| R15.6 | 过审 | ElyPDF 能打开 EPUB，只读，可阅读、搜索、缩放 | `crates/ely-pdf-worker/tests/epub.rs`、`apps/ely-pdf/src/tests/epub.rs`；`tasks/reviews/R15.6-r2.md` PASS | macOS 未跑（只记 Windows、Linux 通过） |
| R15.7 | 过审 | Gmail 发信 TLS 超时：按语言逐步报出所在步骤 | `a_quiet_server_is_named_in_the_readers_language`、`mail_sends_own_timeout_keeps_the_step`；`tasks/reviews/R15.7-r3.md` PASS | 根因未查明：须报告者日志（代理来源、连接行）与 587 端口对照；发件箱仍存英文原文错误，未动 |
| R15.8 | 过审 | Outlook.com 收发失败的提示照微软步骤写 | `outlooks_held_account_says_what_to_do`；`tasks/reviews/R15.8-r2.md` PASS | #48 的账号属哪种情形未查明；个人账号 SMTP 被关时 ElyMail 无法发信，只能说明 |
| R15.9 | 过审 | ElyMark 正文两侧空白处滚轮也滚动正文 | 组件库 `visual_margins_scroll`；ElyMark `a_wheel_beside_the_text_scrolls_it`；`tasks/reviews/R15.9-r6.md` PASS | 真机 Windows 滚轮未手试（记录里的“待推送改 pin”已解：bc8e5fb 在今 pin 49270f2 之内） |
| R15.10 | 过审 | 设置项：滚动条常显 | 组件库 `an_area_shows_its_thumb_unhovered_when_bars_stay`；`scrollbars_stay_shown_when_chosen`；`tasks/reviews/R15.10-r2.md` PASS | — |
| R15.10a | 过审 | zip 条目名用反斜杠的 docx/xlsx 能打开，坏包报错不卡 | `backslash_folder_entries_open`、`a_broken_package_fails_instead_of_asking`；`tasks/reviews/R15.10a-r4.md` PASS | Linux、macOS 与三台机器的共享引擎待以 0063、0064 重建；“无 0064 即卡住”只按源码推断，未实测 |
| R15.11 | 部分 | Office 三份大样本经 Ely 编辑、保存，再由 Office 读回核对 | `tests/office/compare.ps1`、`tests/office/read-back.ps1`；`tasks/reviews/R15.11-r3.md` FAIL | r3 两处修复（按域的位置去掉域结果、逐系列核对类型与坐标轴组）未经 Office 实跑：测试机无 Office，装 Office 的机器离线；0065、0066 待在 Linux、macOS 与共享引擎上重建；已知差异：目录里 55 个 `PAGEREF` 页码成了文字，PPTX 的节 |
| R15.11a | 过审 | XLSX 动态数组存回仍是动态数组（`cm`、metadata） | `dynamic_arrays_save_as_dynamic_and_plain_ones_as_plain`、`pasted_and_undone_dynamic_arrays_stay_dynamic`；`tasks/reviews/R15.11a-r3.md` PASS | — |
| R15.11b | 过审 | DOCX 公式里的减号存回仍是 U+2212 | `equation_minus_signs_save_as_math`；`tasks/reviews/R15.11b-r2.md` PASS | — |
| R15.11c | 部分 | DOCX 分节空段落与目录末段照原样存回 | `titled_toc_field_ends_in_its_own_paragraph`；`tasks/reviews/R15.11c-r3.md` FAIL | 分节空段落照上游丢弃（接受）；r3 查出的有标题目录两处交 R15.11h，无标题目录多一行交 R15.11k（皆已勾），r3 后未复审；复核须 Word |
| R15.12 | 过审 | Windows 启动时首帧前不再冻结出空白窗口 | `docs_build_while_the_engine_holds_its_lock`；`tasks/reviews/R15.12-r2.md` PASS | 建 worker 进程仍在主线程上（热启动约 5 毫秒）；窗口出现前 GPU 与 DirectWrite 初始化约 210 毫秒，属 gpui，未处理 |
| R15.13a | 过审 | ElyShow 打开带视频的演示，内核不再崩溃（Windows） | `painting_a_video_slide_starts_no_player`；`tasks/reviews/R15.13a-r1.md` PASS | — |
| R15.14a | 过审 | 缩放后图块裁切、选框错位：worker 拒绝 DPI 感知 | `two_tiles_match_one_tile_over_both`；`tasks/reviews/R15.14a-r1.md` PASS | 单元格编辑中 Calc 的 `CELL_CURSOR` 报编辑区，选框内缩、随键入变宽，未改 |
| R15.13 | 过审 | 分段量出打开耗时，缩短“正在打开”的时间 | `a_view_opens_while_the_engine_is_busy`、`a_panel_that_comes_with_content_takes_its_width_at_once`；`tasks/reviews/R15.13-r2.md` PASS | 余下是内核本身的载入：big.docx 1.31 至 1.45 秒、big.pptx 0.94 至 0.97 秒、big.xlsx 0.44 至 0.57 秒；启动等引擎约 0.33 秒 |
| R15.13b | 过审 | 未改动的带 PAGEREF 的 docx 打开后不算已修改 | `a_docx_with_page_references_opens_unmodified`；`tasks/reviews/R15.13b-r1.md` PASS | — |
| R15.13c | 过审 | 同一引擎开多份 docx，无样式定义时不再崩溃 | `three_documents_edit_save_and_close`；`tasks/reviews/R15.13c-r1.md` PASS | — |
| R15.13d | 过审 | Windows 引擎不再与内核 Python 互锁，重启沿用原配置目录 | `a_restart_signs_with_the_newest_name_and_survives_a_failure`（`crates/ely-office/tests/restart.rs`）；行内另提的 `a_slow_load_offers_the_stop_and_the_stop_ends_the_engine` 已随 SF.13 删去（740b0d0a）；`tasks/reviews/R15.13d-r2.md` PASS | — |
| R15.13e | 过审 | 按出 Writer 旧式对话框不再令引擎退出 | `writer_dialog_keys_leave_every_document_working`；`tasks/reviews/R15.13e-r2.md` PASS | — |
| R15.14 | 过审 | Doc、Show 翻页时整屏一次呈现，不逐块出现 | `a_zoomed_sheet_maps_by_the_screen_it_holds`、`a_held_screen_keeps_its_border_reach`；`tasks/reviews/R15.14-r8.md` PASS | 真按键下按键到整屏的耗时（Doc 30 至 603 毫秒、Show 44 至 169 毫秒）是满载时测的，待安静时重跑；r8 未跑 macOS |
| R15.14b | 过审 | ElyPDF 翻页整页一次呈现并预画下一屏 | `apps/ely-pdf/src/tests/turn.rs`；`tasks/reviews/R15.14b-r3.md` PASS | — |
| R15.11d | 过审 | XLSX 透视表存回保留 Excel 的行、列、总计标签 | `crates/ely-lok/src/client/tests/pivot.rs`、`tests/scripts/ooxml_pivot.py`；`tasks/reviews/R15.11d-r3.md` PASS | big.xlsx 有 35 格求和末位不同（相对差 1.8e-14，钉为已知差异）；只有列字段时的版式交 R15.11i，`dataCaption` 交 R15.11j（皆已勾） |
| R15.11e | 过审 | PPTX 视频的淡入与点击触发序列存回照旧 | `crates/ely-lok/src/client/tests/trigger.rs`、`tests/scripts/ooxml_motion.py`；`tasks/reviews/R15.11e-r2.md` PASS | — |
| R15.11f | 过审 | XLSX 公式里的 TRUE、FALSE 不存成 `FALSE()` | `crates/ely-lok/src/client/tests/booleans.rs`、`tests/scripts/ooxml_formula.py`；`tasks/reviews/R15.11f-r1.md` PASS | — |
| R15.11g | 部分 | DOCX 公式的双线体字母与数字存回照旧 | `tests/scripts/ooxml_word.py`（`equation`）；`tasks/reviews/R15.11g-r3.md` FAIL（无 r2 文件） | r3 所指 `m:lit` 里的 `x oper y`、`x sum y` 被当作 StarMath 命令而丢内容，行内未记修复；r3 所指字面减号自测的失败，今在 Linux 上不再出现（2026-11-19 的 ely-lok 全量里 `equation_minus_signs_save_as_math` 照过）；复核须 Word |
| R15.11h | 过审 | DOCX 目录域结束后的正文存回仍在域外 | `text_after_a_toc_field_end_stays_outside_the_field`、`an_inline_content_control_keeps_the_toc_end_alone`；`tasks/reviews/R15.11h-r3.md` PASS | Word 更新目录多一空行，交 R15.11k；上游 `testTdf119200` 有无 0078 都报错 |
| R15.11i | 过审 | XLSX 只有列字段的透视表照 Excel 版式存回 | `a_pivot_with_only_columns_keeps_excel_s_layout`；`tasks/reviews/R15.11i-r2.md` PASS | 无值字段时多一行空数据行，交 R15.11l；上游透视表单测 4 例败于 0071 |
| R15.11j | 过审 | 透视表数据布局表头照 `dataCaption` 存回 | `values_on_rows_keep_the_file_s_data_caption`；`tasks/reviews/R15.11j-r1.md` PASS | Excel 存的 XLSB 由 Ely 打开时列表头仍是 Product，不是 `colHeaderCaption`（0071 的 XLSB 一侧），未处理 |
| R15.11k | 过审 | 目录域结束导出在索引节后一段段首 | `an_untitled_toc_ends_in_the_paragraph_after_it`、`a_writer_toc_keeps_the_text_after_it`；`tasks/reviews/R15.11k-r2.md` PASS | `compare.ps1` 的 `body` 仍差分节空段落（R15.11c）；上游 `testMathMso2k7`（0068）、`testWatermark` 失败 |
| R15.11l | 过审 | 无值字段透视表不留空数据行、不清邻格 | `a_pivot_with_no_values_leaves_its_neighbour`；`tasks/reviews/R15.11l-r1.md` PASS | — |
| R15.25 | 过审 | ElyDoc 新建文档的默认纸张可设、随地区 | `a_new_document_starts_on_the_chosen_paper`、`the_region_is_the_two_letter_subtag`；`tasks/reviews/R15.25-r3.md` PASS | — |
| R15.26 | 过审 | Windows 安装程序声明高 DPI（PR #55） | 行内未点名专测，只核了编出的清单含 `dpiAware`、`PerMonitorV2`（今见 `packaging/windows/installer.nsi`）；`tasks/reviews/R15.26-r1.md` PASS | 只核清单，未在缩放下实看；`bundle.nsi` 与 `windows-bundle.sh` 已由 SF.6（6ec3b60c）删去 |
| R15.27 | 过审 | ElyTable 粘贴制表符文字按行列分格、不弹框 | `pasted_lines_of_tabbed_text_fill_cells_without_a_dialog`、`a_skipped_trailing_column_keeps_its_cell`；`tasks/reviews/R15.27-r3.md` PASS | — |
| R15.28 | 过审 | ElyTable 单击超链接跳转或交系统打开 | `a_plain_click_reports_the_link_and_a_jump_follows_it`、`links_jump_or_refuse_and_dialogs_say_why`；`tasks/reviews/R15.28-r3.md` PASS | — |
| R15.29 | 过审 | ElyDoc 右侧栏加标签，可从邮件合并返回 | `the_side_tabs_lead_back_from_mail_merge`；`tasks/reviews/R15.29-r1.md` PASS | — |
| R15.30 | 过审 | 界面字体到达对话框、tooltip 与拖动预览 | `unstyled_text_takes_the_theme_overlays_too`、组件库 `a_ghost_wears_the_theme_font`；`tasks/reviews/R15.30-r1.md` PASS | — |
| R15.15 | 过审 | 合并 PR #51：ElyDoc、ElyTable 西班牙语 | `each_language_names_each_style_once`；`tasks/reviews/R15.15-r2.md` PASS | — |
| R15.16 | 过审 | 西班牙语覆盖全部应用 | 行内未点名新测，反证用的是 `tree.net.push` 缺键；`tasks/reviews/R15.16-r2.md` PASS | — |
| R15.17 | 过审 | 设置里的界面语言改为下拉列表 | `the_language_row_lists_each_language_to_pick`、`every_listed_language_has_its_catalog`；`tasks/reviews/R15.17-r1.md` PASS | — |
| R15.18 | 过审 | 日语覆盖全部应用 | `japanese_builtin_names_come_from_libreoffice`、`shift_jis_reads_as_windows_932`；`tasks/reviews/R15.18-r1.md` PASS | ElyTree 术语自拟（Git 无日文） |
| R15.19 | 过审 | 韩语覆盖全部应用 | `a_korean_windows_csv_imports_beyond_euc_kr`、组件库 `every_listed_locale_can_be_picked`；`tasks/reviews/R15.19-r3.md` PASS | 未经韩语母语者审读；sw 未译的 36 个样式名自拟 |
| R15.20 | 过审 | 越南语覆盖全部应用 | `vietnamese_builtin_names_come_from_libreoffice`、`csv_imports_read_vietnamese_windows_1258`；`tasks/reviews/R15.20-r2.md` PASS | — |
| R15.21 | 过审 | 法语覆盖全部应用 | `a_french_excel_csv_opens_in_its_code_page`、`french_translates_every_module_once_with_its_arguments`；`tasks/reviews/R15.21-r2.md` PASS | 法文的 0 该用单数，现显示复数“0 mots”，未改，交协调者定 |
| R15.22 | 过审 | 德语覆盖全部应用 | `a_german_excel_csv_opens_in_its_code_page`、`crates/ely-ui/src/i18n/german_tests.rs`；`tasks/reviews/R15.22-r1.md` PASS | ElyMark 菜单未对照 Typora de-DE 的 `Menu.json` 核对 |
| R15.23 | 过审 | 合并 PR #53：ElyTable 的 R 宏 | `a_stop_reaches_the_helpers_r_started`（原名 `an_end_reaches_the_helpers_r_started`，bf80be48 改名）、`a_refusal_mid_batch_undoes_its_writes_and_leaves_no_redo`；`tasks/reviews/R15.23-r6.md` PASS | r6 时 Linux、Windows 都无 R，真 R 四例未执行，以此前 Mac、Windows 的记录为准；Linux 上带内核的真 R 一例从未跑；回滚时 Undo 抛异常一路无测试 |
| R15.31 | 过审 | 极限测试：四应用各 100 份同开同编辑 | `tests/perf/load.py`、`tests/perf/extreme.py`；`tasks/reviews/R15.31-r8.md` PASS | 打开期间键入未测（R15.42）；macOS、Linux 未测；样本都由 Python 写，未用 Office 生成的；三个源都改时横幅只点名 include.docx；ElyShow 同时编辑有一键 108 毫秒（R15.36）；更新链接后占住源交 R15.55，首键等恢复点交 R15.56；r8 未重跑 100 份，像素值 0.38、0.49、0.50 未重现 |
| R15.32 | 过审 | review.sh 换回 herdr TUI，每轮新开标签页 | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.32-r4.md` PASS | 只用替身测；Windows 无 jq，替身全套未跑；标签页其后由 R15.52 改为窗格 |
| R15.33 | 过审 | 配置目录 149 至 162 字符时 worker 启动即中止 | `profiles_whose_kernel_folders_end_at_248_characters_start`、`a_worker_that_dies_before_its_check_says_its_last_line`；`tasks/reviews/R15.33-r2.md` PASS | — |
| R15.34 | 过审 | ElyShow 开 50、100 份时前台键 10 秒无反馈 | `only_the_shown_deck_paints_its_pictures`、`a_hidden_view_drops_its_queued_screen`；`tasks/reviews/R15.34-r1.md` PASS | 同时经命令行编辑时 P95 仍 1258、1438 毫秒，交 R15.36；打开时每份主线程忙约 160 毫秒；前台自己的保存与恢复点挡键，交 R15.35 |
| R15.35 | 过审 | 恢复点等打字停顿再写，不挡键 | `typing_defers_the_point_to_the_next_pause`、`a_key_never_waits_behind_a_point`；`tasks/reviews/R15.35-r2.md` PASS | 主动保存期间的键仍等一次保存，交 R15.37；本行记真键实测待 R15.31；停顿后恰在写点时按下的键仍等快照（约 300 毫秒） |
| R15.36 | 过审 | 后台请求轮流取锁，前台键至多等一个 | `input_waits_for_one_background_request_not_all`、`a_stuck_edit_holds_later_requests_one_second_in_all`；`tasks/reviews/R15.36-r3.md` PASS | 修后只复测了 50 份，100 份未在本行复测；余下三个慢键是恢复点导出，归 R15.56；r3 未重跑负载 |
| R15.37 | 过审 | 主动保存等打字停顿再开始 | `a_key_after_a_save_goes_first_and_into_the_file`、`typing_on_starts_the_save_at_the_cap`；`tasks/reviews/R15.37-r4.md` PASS | 500 毫秒停顿未以真人打字实测；macOS、Linux 未测；停顿后恰在保存中按下的键仍等这次保存（t.pptx 约 300 毫秒） |
| R15.38 | 过审 | Ely 自写的错误文字按界面语言显示 | `a_merge_whose_source_closed_says_so_in_chinese_and_spanish`、`errors_ely_words_read_in_each_language`；`tasks/reviews/R15.38-r6.md` PASS | 未改的几类仍是英文（程序错误、系统与 Git 原话、命令行答复、ElyMail 代理握手、启动时搬文件夹），理由记于 decisions |
| R15.39 | 过审 | ElyTable 编辑中保存不结束编辑、不丢输入 | `a_save_keeps_the_cell_edit_and_writes_the_cell_as_entered`、`tests/scripts/windows-table-save-edit.py`；`tasks/reviews/R15.39-r5.md` PASS | — |
| R15.40 | 过审 | ElyDoc、ElyShow 换标签后键交给前台页面 | `a_find_box_that_leaves_with_its_tab_hands_the_keys_on`、`every_switch_hands_the_keys_to_the_front_deck`；`tasks/reviews/R15.40-r2.md` PASS | — |
| R15.41 | 过审 | ElyMark 自动保存在主线程写盘 | `a_waiting_disk_leaves_the_text_free`；`apps/ely-mark/src/outside/tests/saving.rs`；`tasks/reviews/R15.41-r7.md` PASS | 16 个写者的一次 413 毫秒停顿原因未查；另存为的目标解析（`identity`、`holder`）仍在界面线程；r7 未跑 Windows、macOS 的名字比较分支 |
| R15.42 | 过审 | 一次打开许多文档时主线程停数秒 | `waiting_for_the_turn_parks_no_thread`、`each_worker_has_its_own_turn`；`tasks/reviews/R15.42-r5.md` PASS | 打开期间键到图块的 P95 交 R15.57；r5 未重跑 50、100 份的停顿统计 |
| R15.43 | 未完 | ElyTable 外部引用只显示缓存值不随源 | `an_empty_csv_area_keeps_its_cells`、`a_formula_whose_sheet_is_gone_keeps_its_value`；`tasks/reviews/R15.43-r6.md` FAIL | r6 后修的 INDIRECT 未经审查；缺口交 R20.11（`[!]`），再交 R20.33（`[!]`），今余：存 ODS 再开，直接写的外部三维区域显示 `#REF!`，更新后仍是，原因未查；补丁 0141 的 Windows、macOS 内核待 R20.26；`extreme.py` 在 macOS、Linux 未实测 |
| R15.44 | 过审 | INCLUDETEXT 只显缓存文字、存回旧文字 | `includetext_is_a_linked_section_and_saves_as_one`、`a_linked_section_edged_by_tables_saves_as_includetext`；`tasks/reviews/R15.44-r6.md` PASS | 未验证 Word 打开 Ely 存出的文件；macOS、Linux 未实测；单元格里贴着嵌套表格的链接节只能存为文字（有横幅，改另存） |
| R15.45 | 过审 | OLE 链接存成静态图片、链接丢失 | `a_word_link_kept_as_a_picture_saves_only_as_a_copy`、`a_link_of_another_kind_is_named_even_in_a_locked_file`；`tasks/reviews/R15.45-r3.md` PASS | 只在 Windows 实测；未验证 Word 怎么写含 `#`、`%`、空格或非 ASCII 的目标；LINK 域的区域不按区域取，读整个文件 |
| R15.46 | 过审 | 超大演示键 P95 964 毫秒（快照被拒仍占内核） | `a_file_opened_over_its_space_takes_no_snapshot`、`a_document_over_its_space_is_not_snapshotted_again`；`tasks/reviews/R15.46-r2.md` PASS | macOS、Linux 未实测 |
| R15.47 | 过审 | ElyTable 改缩放时视图离开 A1 | `a_sheet_zoom_keeps_the_top_left_cell_and_the_pointed_one`、`tests/scripts/windows-table-zoom.py`；`tasks/reviews/R15.47-r3.md` PASS | — |
| R15.48 | 过审 | `edit to tile` 计时遇恢复点漏记成假延迟 | `a_recovery_point_between_edit_and_tile_still_pairs_them`、`an_edit_is_timed_by_the_first_whole_fresh_screen`；`tasks/reviews/R15.48-r4.md` PASS | 别的演示稿载入期间，前台演示稿的屏一直不完整、键入的字看不到，须内核按文档过滤失效（说要另立条目，未见）；适应窗口的幻灯片情形未单独测 |
| R15.49 | 过审 | 负载下中文分组测试与 never settled 偶败 | 无新增回归；所修即 `the_page_and_style_groups_speak_chinese`（连跑 20 次过）；`tasks/reviews/R15.49-r5.md` PASS | — |
| R15.50 | 过审 | ElyMark 外部改动检查晚到、大小写同名无主 | `a_missing_target_is_held_across_case`、`a_variant_listed_as_its_own_file_is_no_answer`；`tasks/reviews/R15.50-r5.md` PASS | sshfs 这类挂载未实测；未在真 casefold 的 ext4 上跑；r5 读不到 rclone 的证据，只读码判断 |
| R15.51 | 过审 | 保留我的在文件不在时直接覆盖新文件 | `keep_mine_over_a_gone_file_never_replaces_a_new_one`、`a_file_written_over_the_claim_is_never_replaced`；`tasks/reviews/R15.51-r3.md` PASS | — |
| R15.52 | 过审 | review.sh 改由 Claude Code 在右侧分屏审查 | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.52-r2.md` PASS | — |
| R15.53 | 过审 | 审查者后台运行时 review.sh 误报无结论 | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.53-r2.md` PASS | — |
| R15.54 | 过审 | macOS 内核模态对话框卡死 worker | `a_kernel_dialog_is_closed_and_reported`；`tasks/reviews/R15.54-r1.md` PASS | — |
| R15.55 | 过审 | 更新链接后内核约 1 分钟占着源工作簿 | `a_source_a_book_read_is_free_to_replace`、`a_source_a_document_read_is_free_to_replace`；`tasks/reviews/R15.55-r1.md` PASS | macOS、Linux 未实测；该行未销“三台共享引擎带 0089 重建”（R15.62 记 Mac 内核已含 0089）；Mac 上间歇 `Err:522` 由 R15.62 修 |
| R15.56 | 过审 | 停顿后第一键等恰好开始的恢复点 | `every_stop_of_an_impress_export_keeps_the_deck`、`a_point_owed_a_minute_no_longer_gives_way`；`tasks/reviews/R15.56-r3.md` PASS | 修后 ElyShow 真窗口实测未做（丢键是否随多出的幻灯片而去）；ElyTable 真窗口未测；macOS、Linux 未编译实测；共享引擎带新 0090 重建未记；快照末段与图块的余下等待交 R15.66 |
| R15.58 | 过审 | 测试靠外部 `unzip`，Windows 必败 | `excel_caches_recompute_unless_the_function_is_unknown`（改为进程内读包，`crates/ely-lok/src/client/tests/zip.rs`）；`tasks/reviews/R15.58-r1.md` PASS | — |
| R15.59 | 过审 | 审查窗格挤在一列收不到提示 | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.59-r3.md` PASS | — |
| R15.60 | 过审 | 许多 LOK worker 同起时偶尔起不来 | `a_failed_start_names_the_version_file_and_what_it_gives`、`a_file_held_without_delete_sharing_names_its_holder`；`tasks/reviews/R15.60-r2.md` PASS | 根因未明，原报三种失败未重现，只做到诊断就位；`test_worker` panic 的错误文字仍被输出过滤丢掉（2026-11-05 又见一次） |
| R15.64 | 过审 | 内核补丁的 SAL 日志被编掉，到不了 Ely | `a_patch_log_reaches_the_app_log_and_upstream_lines_do_not`、`a_source_read_and_a_failed_read_are_both_logged`（R18.10）；`tasks/reviews/R15.64-r2.md` PASS | 只在 Linux 实测；Mac、Windows 内核须带 0108 重建，再在发布的门里跑 `logs.rs` |
| R15.57 | 过审 | 批量打开时前台键到图块仍 1.9 至 2.5 秒 | `keys_typed_while_a_picked_tab_loads_reach_it`、`with_every_tab_closed_the_newest_waiting_file_comes_forward`；`tasks/reviews/R15.57-r2.md` PASS（行内所记 `an_inputs_tile_goes_before_queued_background_work` 已删去） | 真窗口负载（`windows-tab-keys.py`、`load.py`）未跑；50 至 100 份时 P95 是否达标未验证 |
| R15.65 | 过审 | review.sh 偶发 `agent_prompt_stalled` | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.65-r2.md` PASS | — |
| R15.66 | 过审 | 恢复点让路后余下的等待 | `a_shown_tab_s_tile_stops_another_tab_s_snapshot`；由 R18.8 完成：`tasks/reviews/R18.8-r3.md` PASS | R18.8 最后一次三份切换：ElyShow 27 键里 1 键过 100 毫秒，进入文本编辑的回车在内核里 584 毫秒，原因未查；早先一次 a.pptx 少存键，原因未查（其后各次无丢键）；R18.8 的 r3 只读审查，没跑测试与基准 |
| R15.67 | 过审 | 组件库与 ElyOffice 升到 `gpui-pre` =0.3.11 | 无专属测试；`tasks/reviews/R15.67-r1.md` PASS | Linux、macOS 只跑 clippy，不含 `ely-pdf-worker`，未跑全工作区测试；钉今已升到 =0.3.12 |
| R15.68 | 过审 | `PromptDialog` 输入框无名 | 组件库 `the_prompt_names_its_field_by_its_label`；`tasks/reviews/R15.68-r1.md` PASS | 真窗口 UI Automation 名字未验证（Windows 测试机无 ElyPDF 构建） |
| R15.70 | 过审 | Windows LOK worker 开着 DDE 服务、答应任何名 | `a_worker_answers_no_dde_service_name`；由 R18.9 完成：`tasks/reviews/R18.9-r1.md` PASS | 审查者未在 Windows 跑，采信同机对照；“另有 worker 时 DDE 缺源测试通过”未单独记 |
| R15.24 | 本条 | 本表 | — | — |
| R15.61 | 过审 | Mac 测试与发布脚本须在 bash 5 下跑 | `tests/scripts/mac-bash-rules.sh`、`tests/scripts/workflows.py`；`tasks/reviews/R15.61-r1.md` PASS | 审查未重建 bash，未经真发布核对 |
| R15.62 | 过审 | Mac 外部引用间歇读成 `Err:522` | `a_read_while_the_source_imports_gets_its_value`；`tasks/reviews/R15.62-r1.md` PASS | Windows、Linux 共享内核须带 0098 重建后再验 |
| R15.63 | 过审 | 非 Windows 上缺服务器的 DDE 链接不点名 | `a_dde_link_to_no_server_is_named_without_a_dialog`、`a_writer_dde_field_to_no_server_is_named_and_keeps_its_text`；`tasks/reviews/R15.63-r2.md` PASS | r2 未跑 macOS，也未重跑旧内核变异；非 Windows 上 DDE 从不读，一律点名未读（设计如此） |

## R20 与收尾时仍开着的条目

R20.47：R20.1 至 R20.106 逐条核对，另列暂缓的 C15（ElyAgent，用户 2026-11-19 定）与仍开着的 C20.2、C22.2、C23.1、R19.2、R19.3、R19.5、R19.6、R19.9，及 C24.1。2026-11-19 据 a7c92b66 写成，2026-11-20 据 5cf06502 补 R20.48 至 R20.64、重写已结束的各条，读每条的全文与其各轮审查；不少条目仍在进行，随其结束重写，末次在最后一条结束时。“过审”即 `[x]` 且末轮审查 PASS，“未完”为 `[!]`，“进行中”为 `[ ]`，“暂缓”为用户定的暂停；“延后”为仍开着、用户定不随 0.1.17 发布的条目，不算缺做。证据列该条为此所加、今日树里仍在的回归测试（带测试属性的函数，如 `#[test]`、`#[gpui::test]`、`#[tokio::test]`，或 `tests/perf` 的 `def test_`，至多两个）或测试文件与脚本，再列已入库的末轮审查；审查结论尚未入库的照写“尚未入库”。“未验证或余下”照该条最后的写法与末轮审查所记。本表不重跑测试。2026-11-21 据 b7577e41 再写一次：加“末次提交”一列（至 b7577e41 为止，提交说明冒号前点名该编号的最后一个，没有则是该条那一行的 `git blame`；写下本节的提交所点名的条目（这一次是 R20.104）写“本提交”：表写不进写它的那个提交的散列），R20.65 至 R20.106 补入；所引测试、路径与审查结论由脚本逐个核对。仍开着的条目在“余下”一列写明为何不随 0.1.17 关闭，取自各条自己的行。

| 编号 | 状态 | 事 | 证据 | 末次提交 | 未验证或余下 |
| --- | --- | --- | --- | --- | --- |
| R20.1 | 过审 | Windows 带符号内核，只供采样 | 建造任务无测试；`tasks/reviews/R20.1-r2.md` PASS | d6972592 | — |
| R20.2 | 过审 | Windows arm64：不做 | 无审查 | e25e7b2d | —（用户 2026-11-16 定 Linux、Windows 都不出 ARM） |
| R20.3 | 过审 | 文字与样式混合判定（补丁 0135，字号框） | `a_size_turned_mixed_shows_its_hint_not_the_old_size`、`half_raised_text_reads_mixed_and_all_raised_reads_on`；`tasks/reviews/R20.3-r3.md` PASS | 5ad75371 | macOS 共享内核 4ce869dd（R20.26，0001–0145）上门过（`format::mixed` 三条与 ElyDoc 的混合判定）；Windows lo-sym 258dcf4d（R20.26，0001–0145，测试机 7877）上 `format::mixed` 诸条过 |
| R20.4 | 过审 | 演示输出与检查的请求代次 | `a_check_asked_after_a_waiting_output_wins`；`tasks/reviews/R20.4-r1.md` PASS | 9aedcd76 | 审查未跑 Windows、macOS |
| R20.5 | 过审 | ElyShow 热启动，首次缺字体读名字表 | `the_start_check_reads_every_font_s_other_names`（只 Windows、macOS 编）；`tasks/reviews/R20.5-r1.md` PASS | 17ca9d04 | —（R20.26 以装好的 0.1.17 在测试机上热测：PowerPoint 1,313 对 ElyShow 779，五次中位；先前 723 对 1,231 是开发构建、带符号内核量的） |
| R20.6 | 未完 | ElyShow 首开 19 MB 演示稿，提前载入 | `the_first_load_takes_the_early_one_and_another_closes_it`、`a_launch_tab_closed_before_its_turn_closes_the_early_load`；`tasks/reviews/R20.6-r2.md` PASS | aa6bc969 | R20.26 已量完装完与重启后首开并入库两份记录（ElyShow 装完中位 2,065、重启后 2,104），那两份记录里没有 PowerPoint 的数，“快过 PowerPoint”未证；Windows 多出部分未归因 |
| R20.7 | 未完 | ElyDoc 热启动 big.docx 对 Word | 试过的改法未留，无测试；`tasks/reviews/R20.7-r3.md` PASS | 059f3984 | R20.26 以装好的 0.1.17 热测 Word 1,219 对 ElyDoc 1,754（五次中位），仍慢约 0.5 秒；Windows 多出部分未归因 |
| R20.8 | 过审 | 宏不执行，放开宏的正向对照 | `a_loaded_document_s_macro_never_runs`；`tasks/reviews/R20.8-r1.md` PASS | 767b5403 | macOS 共享内核 4ce869dd（R20.26，0001–0145）上 `a_loaded_document_s_macro_never_runs` 过；Windows lo-sym 258dcf4d（R20.26，0001–0145，测试机 7877）上过 |
| R20.9 | 过审 | 粘贴 HTML 的图片，html5ever 重写 | `inline_images_paste_only_when_they_decode`、`raw_text_holding_markup_is_refused`；`tasks/reviews/R20.9-r3.md` PASS | b39d6be4 | — |
| R20.10 | 过审 | Word EQ 域缓存结果不进文档（补丁 0136） | `an_eq_fields_cache_of_any_kind_is_not_drawn`；`tasks/reviews/R20.10-r3.md` PASS | e7069319 | macOS 共享内核 4ce869dd（R20.26，0001–0145）上过（`word` 的 EQ 四条）；Windows lo-sym 258dcf4d（R20.26，0001–0145，测试机 7877）上 `word` 的 EQ 四条过 |
| R20.11 | 未完 | 外部链接 INDIRECT 缺表点名（补丁 0131、0137） | `a_formula_whose_sheet_is_gone_keeps_its_value`、`an_indirect_pointed_elsewhere_reads_its_old_sheet_no_more`；`tasks/reviews/R20.11-r3.md` FAIL | 835fa522 | 两项交 R20.33 |
| R20.12 | 未完 | `animation.add` 偶被拒，拒因记日志（补丁 0138） | `a_refused_animation_logs_why`；`tasks/reviews/R20.12-r1.md` PASS | 5e966ec4 | 根因未明：Linux 18 次、Windows lo-sym 258dcf4d 上 ElyShow 整组十遍都未复现，0138 那一行未出现 |
| R20.13 | 过审 | Windows 空闲门改为记录周期的两倍 | `tests/perf/test_idle.py`；`tasks/reviews/R20.13-r1.md` PASS | 51868b94 | 周期与 WIL 计时器的对应未分开；审查时 7877 连不上，未重读 Windows 记录 |
| R20.14 | 未完 | 微软雅黑粗体：Impress 同设中文字体（补丁 0139） | `a_slide_font_with_chinese_glyphs_sets_both_scripts`、`a_latin_only_slide_font_keeps_the_chinese_font`；`tasks/reviews/R20.14-r1.md` PASS | 9461b8ee | 中文界面语言包与微软输入法下的复现矩阵、截图未做；“只有施发细”未解释；光标处一路未在中文键盘机上验；三端内核已由 R20.26 重建到 0001–0145（含 0139），0139 未在 Windows 上验 |
| R20.15 | 过审 | ElyTable 单元格字体与字号 | `the_panel_sets_a_cells_font_and_size`；`tasks/reviews/R20.15-r1.md` PASS | bc641a69 | — |
| R20.16 | 未完 | ElyPDF 两向滚动条，拖动跳页 | `a_dragged_bar_moves_the_pages_and_a_resize_keeps_them`、`a_dragged_bar_moves_the_row_of_pages`；`tasks/reviews/R20.16-r5.md` FAIL | bf1a6935 | 拖动的帧时在 Xvfb 上量过（拖动开头的慢帧是 gpui 对未聚焦窗口限帧），120 Hz 须真显示器，未测；macOS 未跑 |
| R20.17 | 过审 | ElyMail 窄侧边栏保住未读数 | 组件库 `a_narrow_column_shortens_the_title_and_keeps_the_count`；`tasks/reviews/R20.17-r2.md` PASS | 0ec86921 | — |
| R20.18 | 未完 | ElyTree Shell 注释照 Bash 的词法 | 组件库 `comments_start_where_bash_starts_them`；`tasks/reviews/R20.18-r3.md` FAIL | 43126770 | 三项交 R20.34 |
| R20.19 | 未完 | Markdown 行内与块级 HTML 用 html5ever 建树 | 组件库 `pre_keeps_its_white_space`、`a_slash_closes_only_what_html_closes`；`tasks/reviews/R20.19-r3.md` FAIL | d9dc61ba | 三项交 R20.35 |
| R20.20 | 过审 | 块命令核对保住块的种类与提示块 | 组件库 `a_change_that_makes_a_block_another_kind_is_refused`、`a_heading_in_an_alert_s_first_paragraph_keeps_the_alert`；`tasks/reviews/R20.20-r2.md` PASS | 68f08b08 | — |
| R20.21 | 过审 | ElyMark 公式：vendor 修过的 pulldown-cmark | 组件库 `a_link_around_a_footnote_reference_stays_a_link`、`a_note_in_a_markdown_link_frees_the_math_after_it`；`tasks/reviews/R20.21-r3.md` PASS | 3545ab8f | — |
| R20.22 | 过审 | ElyMark 设置里的文档字号（带重置） | 行内未点名测试；`tasks/reviews/R20.22-r2.md` PASS | d8afbf71 | — |
| R20.23 | 过审 | ElyMark 搜索结果里的目录可展开 | `a_folder_the_filter_finds_opens_to_its_files`；`tasks/reviews/R20.23-r3.md` PASS | c43d2a05 | — |
| R20.24 | 未完 | 组件库三个重名的包在 crates.io 换名 | 无审查 | c5e2f379 | 本机无 crates.io 令牌；换名不可撤回，等用户给令牌后单做 |
| R20.25 | 过审 | #89 S3 与 WebDAV：不做，关闭议题 | 用户定不做，无测试；`tasks/reviews/R20.25-r1.md` PASS | f10865e7 | — |
| R20.26 | 进行中 | 三台机器重建内核，量三处慢 | `tests/perf/test_firsts.py`；无审查 | 31319fe1 | 进行中：三端内核已重建（Linux 04023855、Mac 1c8c756c、Windows lo-src d128762f）；Windows 0.1.17 装完、重启、热测已写入 `SuperFast.md`，装完、重启两份记录入库，比 0.1.16 无一慢过三成；热测第 0 次是冷开（`firsts.py` 按映像名关应用，连引擎一起杀），已改按 pid 关、在重量；R20.12 的拒因在 Windows 整组十遍未复现 |
| R20.27 | 过审 | #14 换版式后的灰色箭头：复现矩阵与定性 | 复现矩阵，无回归测试；`tasks/reviews/R20.27-r2.md` PASS | b2cacd18 | 定为主题自带的项目符号，不是缺陷；报告者的文件或截图未到；议题回复拟文未发 |
| R20.28 | 未完 | R18.30 性能基线复审与 Linux 重跑 | `tests/perf/test_bench.py` 的 `test_a_failed_redo_keeps_what_it_finished_and_leaves_the_rest_void`、`test_a_failed_redo_on_another_machine_keeps_nothing`；`tasks/reviews/R20.28-r3.md` FAIL | a558a518 | 失败的 `--redo` 不存换页记录、点名不存在的项报得晚，两项另立 R20.85（已勾，7374b3f3）、R20.86（已勾，7374b3f3）；Linux 0.1.17 不再量（用户 2026-11-20 定），入库为局部记录，门与比较都不认；PERF-08 句柄检查已改为重读至多 5 秒，未在真跑里验过 |
| R20.29 | 未完 | `waits.py` 的判定复审，发布测试连跑十次 | `tests/scripts/waits.py`（自带样例）；`tasks/reviews/R20.29-r3.md` FAIL；其后 98906e9a 未审 | 98906e9a | 发布测试本机连跑十次等机器空闲 |
| R20.30 | 进行中 | 首个统一发布 0.1.17 | `tests/scripts/release-gate.sh`、`docs/perf/2026-11-21-13th-Gen-Intel-R-Core-TM-i5-13500H-Windows.md`；无审查 | b7577e41 | 进行中：未打标签、未建发布；Windows 0.1.17 的装后首开、重启后首开与基准记录皆已入库（基准 34 项皆算数，无一换页）；基准比 0.1.16 慢过三成五行，缘由未写，`baseline.py --check 0.1.17 windows` 因此不过（PERF-06 ElyTree 历史与查询的 P95 超出 1000 毫秒的目标），查明之前不写；Linux、macOS 沿用 0.1.16 的基准记录（用户 2026-11-20 定） |
| R20.31 | 未完 | #16 WinGet 交统一包 `ZacharyZhang.ElyOffice` | 无测试；`tasks/reviews/R20.31-r1.md` PASS | 8ae6ac68 | 七个旧 PR 已关；等 R20.30 发布后交统一清单、在 #16 答复，收录后 README 写安装与升级命令 |
| R20.32 | 过审 | ElyShow 拖动对象的智能参考线与吸附 | `a_drag_whose_read_went_stale_drops_nothing`、`crates/ely-lok/src/client/tests/carry.rs`；`tasks/reviews/R20.32-r2.md` PASS | 9e447262 | macOS 共享内核 4ce869dd（R20.26，0001–0145）上过（ElyShow `tests::carry` 两条、内核 `carry` 一条）；Windows lo-sym 258dcf4d（R20.26，0001–0145，测试机 7877）上 ElyShow `tests::carry` 两条、内核 `carry` 过 |
| R20.33 | 未完 | 外部三维区域逐表读与链接核对（补丁 0141） | `a_range_follows_its_named_ends_when_sheets_move`、`crates/ely-lok/src/client/tests/spanned.rs`；`tasks/reviews/R20.33-r3.md` FAIL | 9ff252a3 | 存为 ODS 再开，直接写的外部三维区域成 `#REF!`，更新后仍是，原因未查，缺回归；三端内核已由 R20.26 重建，本条在 Windows 上未跑 |
| R20.34 | 未完 | Shell 注释：here-doc 结束词与 `$'…'` 的 Bash 规则 | 组件库 `crates/ely-core/src/forms/shell/tests.rs`、`crates/ely-core/src/forms/shell/tests/braces.rs`（语料）；`tasks/reviews/R20.34-r3.md` FAIL；其后 1868d1cc 未审 | 1868d1cc | 余三项：`${x:-'…'}` 单引号组里的反斜杠照字面；双引号里 `${x:-…}` 的反斜杠照 `SX_STRIPDQ` 成对；`defaulted` 找下标的 `]` 要越过引号与展开；修法已进 pin 8d7723ec（1868d1cc） |
| R20.35 | 未完 | ElyMark 与写信对内嵌 HTML 的分行与短代码 | ElyMail `an_ancestor_s_end_closes_pre_as_the_composer_reads_it`、组件库 `what_a_browser_hides_draws_nothing`；`tasks/reviews/R20.35-r3.md` FAIL；其后 1868d1cc 未审 | 1868d1cc | 余下的一项（行尾只剩 `hidden` 的单元格仍补制表符）移交 R20.56（已勾，dfd69c97），于组件库 3a7f960b 修 |
| R20.36 | 过审 | ElyShow 拖动时对象轮廓跟着指针走 | `apps/ely-show/src/tests/carry.rs`；`tasks/reviews/R20.36-r1.md` PASS | c7a9fb68 | —（修法在 R20.32（已勾，26f0fb34）；macOS 4ce869dd、Windows lo-sym 258dcf4d 上真拖动两条都过） |
| R20.37 | 过审 | ElyShow 插入形状后点别处多出形状（补丁 0142） | `crates/ely-lok/src/client/tests/inserted.rs`；`tasks/reviews/R20.37-r1.md` PASS | b0301163 | 三端内核已由 R20.26 重建到 0001–0145；本条的测试在 Windows、macOS 上未跑 |
| R20.38 | 过审 | 缺失的中文字体回退到装饰字体（补丁 0143） | `crates/ely-lok/src/client/tests/fallback.rs`、`a_missing_font_logs_what_drew_it_and_why`；`tasks/reviews/R20.38-r2.md` PASS | 1e3ed14c | macOS 共享内核 4ce869dd（R20.26，0001–0145）上两条过：缺失的黑体、宋体取到同类，日志的原因为 zh-CN sans list、zh-CN text list；Windows lo-sym 258dcf4d（R20.26，0001–0145，测试机 7877）上两条过 |
| R20.39 | 未完 | ElyShow 放映时方向键不翻页（gpui 补丁 0014） | `tests/scripts/show-keys.py`；`tasks/reviews/R20.39-r3.md` PASS | ee5b8a58 | 用户那次丢键的条件未复现（Omarchy 原配置下不丢）；Windows、macOS 只有一块屏，演讲者视图 SKIP |
| R20.40 | 过审 | worker 启动即退出时报出它自己的原因 | `crates/ely-lok/src/client/tests/died.rs`；`tasks/reviews/R20.40-r2.md` PASS | 4efc9efd | — |
| R20.41 | 过审 | 主线程卡顿分忙、堵、饿，基准只计证实的饿之外的 | `a_thread_kept_off_its_cpu_is_starved_not_a_stall`、`a_thread_blocked_on_a_lock_stalls`；`tasks/reviews/R20.41-r2.md` PASS | e89832ae | Mac、Windows 没有每线程的等待时间，CPU 不足一律计入，分不出饿；cgroup 节流与 SIGSTOP 记作堵；虚拟机里宿主机拿走的 CPU 可能记作忙（未核实，未测）；真窗口只在 Linux Xvfb 验过 |
| R20.42 | 过审 | ElyMark 的 git 测试不受 TMPDIR 位置影响 | `spaces_stay_and_broken_repositories_say_why`；`tasks/reviews/R20.42-r1.md` PASS | c9172322 | — |
| R20.43 | 过审 | 合入外部 PR #90：ElyShow 插入超链接 | `apps/ely-show/src/tests/link.rs`；`tasks/reviews/R20.43-r1.md` PASS | a94dec69 | Windows 未跑（内核已由 R20.26 重建，此后未补跑）；查出的旧缺陷另立 R20.49（已勾，8c8ba466）、R20.51 |
| R20.44 | 过审 | 合入外部 PR #91：放映中跟随超链接与点击动作 | `a_show_click_follows_slides_and_the_web_never_files`、`a_next_page_shape_passes_pending_clicks_and_a_program_stays_put`；`tasks/reviews/R20.44-r2.md` PASS | 48298fbc | Windows 未跑；查出的旧缺陷另立 R20.50（已勾，0e8e6add）、R20.52（已勾，787bafdb）、R20.53（已勾，50df14ed） |
| R20.45 | 过审 | 复核并关闭 #87：ElyMark 搜索结果展开目录 | `a_folder_the_filter_finds_opens_to_its_files`、组件库 `a_folder_found_by_name_keeps_its_files_folded`；`tasks/reviews/R20.45-r2.md` PASS | 48298fbc | — |
| R20.46 | 过审 | 复核并关闭 #88：ElyMail 窄侧边栏的未读数 | 组件库 `a_narrow_column_shortens_the_title_and_keeps_the_count`；`tasks/reviews/R20.46-r1.md` PASS | 6f8f102e | 未在 macOS 验（Mac 不许驱动界面） |
| R20.47 | 未完 | 收尾的验收核对表（本节） | `tasks/reviews/R20.47-r3.md` FAIL | bc81a7f8 | 所余另立 R20.104 |
| R20.48 | 过审 | ElyDoc 关闭不等 worker 锁的测试改按因果判 | `closing_a_clean_tab_never_waits_for_the_worker`、`docs_build_while_the_engine_holds_its_lock`；`tasks/reviews/R20.48-r1.md` PASS | 758500b0 | 只在 Linux 满载下连跑；Windows 上的旧失败本机未复现 |
| R20.49 | 过审 | ElyShow 编辑文本时 Ctrl+Z 撤销键入（照 R19.3 的路） | `undo_and_redo_work_inside_a_slide_text_edit`、`the_undo_keys_step_through_typing_inside_a_text_box`；`tasks/reviews/R20.49-r1.md` PASS | 8c8ba466 | Windows 未跑（Linux 04023855、Mac 1c8c756c 过） |
| R20.50 | 过审 | 恶意 .pptx 的脚本链接使存 ODF 时 worker 崩溃（补丁 0146） | `crates/ely-lok/src/client/tests/scripted.rs`（夹具 `tests/fixtures/show/script-link.pptx`）；`tasks/reviews/R20.50-r1.md` PASS | 945e24b3 | 门已核补丁序列：R20.60（已勾，275c9e2a）使 `scripts/ci.sh` 在发布测试前跑 `apply-lo-patches.sh --check`；只核源码，不证内核在末一个补丁之后重链过，另立 R20.89；各机内核已重建带上 |
| R20.51 | 进行中 | 宏链接绕过 Ely 的拦截：查明是内核的哪道检查 | `a_macro_link_the_kernel_opens_itself_never_runs`（`crates/ely-lok/src/client/tests/opendoc.rs`）；`tasks/reviews/R20.51-r1.md` PASS | 85076ef1 | 0.1.17：根治在补丁 0148，已在 Linux 重链验过、未入库；新内核补丁须三端重建共享引擎，定在 0.1.17 发布后入库；本条待 0148 入库再勾，余下两处提示另立 R20.77、R20.78；其回归测试在 macOS 上的偶发败另立 R20.105 |
| R20.52 | 过审 | 放映中旋转、斜切与线类形状的链接照内核的存法定位 | `turned_sheared_and_line_links_cover_what_the_kernel_draws`、`a_click_two_pixels_from_a_line_follows_it`；`tasks/reviews/R20.52-r1.md` PASS | 787bafdb | Windows 未跑（Linux 04023855、Mac 1c8c756c 过） |
| R20.53 | 过审 | 每个应用交给系统打开的链接都过同一道门 | `a_raw_pptx_address_goes_out_encoded`、`every_address_sent_makes_an_nsurl`；`tasks/reviews/R20.53-r2.md` PASS | 50df14ed | Windows 未跑（随 R20.67 的时段） |
| R20.54 | 延后 | ElyDoc 长段落键入时 worker 内存只涨（内核两个文字缓存以整段为键） | 无审查 | 6a478998 | 0.1.17：用户 2026-11-19 定不带它，列为已知问题；两版补丁草稿都把 20 万字一段的键入到帧拖慢数十倍（23 → 3,624 毫秒），根因未明；须进补丁序列、三端内核重建、回归入库 |
| R20.55 | 过审 | 带内核的测试写了用户的数据文件夹；ElyDoc 字数读拿着标签 | `without_a_test_root_the_data_folder_fails`、`a_word_count_in_flight_follows_its_tab`；`tasks/reviews/R20.55-r2.md` PASS | 0668c8af | ElyShow、ElyTable 的同一问题另立 R20.61（已勾，bff46e6a） |
| R20.56 | 过审 | HTML 块里行尾的隐藏单元格不欠制表符 | 组件库 `hidden_cells_owe_no_tab`；`tasks/reviews/R20.56-r1.md` PASS | dfd69c97 | — |
| R20.57 | 过审 | `panics.py` 给自己的 clippy 设 `CARGO_CHECK_SHARED_DEPS=0` | `tests/scripts/panics-rules.sh`；`tasks/reviews/R20.57-r2.md` PASS | 716807dc | — |
| R20.58 | 未完 | 判增长按私有内存，换出的也算 | `tests/perf/test_bench.py`；`tasks/reviews/R20.58-r3.md` FAIL | b50eef1a | 余两项另立：PDH 的数据状态为 R20.81（已勾，7e38f5b4）、macOS 的 footprint 出处为 R20.82（已勾，d455e9e6）；R20.30 的 Windows 0.1.17 基准以此读法跑完，34 项皆算数、`void` 为空 |
| R20.59 | 进行中 | ElyDoc 长段落键入慢 | 无审查 | 45d0ddf9 | 0.1.17：未做（拆每键的时间、找根因、各端实测）；原定在 R20.54 的补丁上量，R20.54 不随 0.1.17 |
| R20.60 | 过审 | 发布机用共享引擎时不核补丁序列 | `tests/scripts/lo-series.sh`、`tests/scripts/ci-share.sh`；`tasks/reviews/R20.60-r1.md` PASS | 275c9e2a | —；限度（核的是源码，不证 instdir 在最后一个补丁后重链过）另立 R20.89 |
| R20.61 | 过审 | ElyShow、ElyTable 的自动读不再拿着标签 | `a_read_and_picture_in_flight_follow_their_deck`、`reads_in_flight_follow_their_workbook`；`tasks/reviews/R20.61-r1.md` PASS | bff46e6a | —（Mac 1c8c756c 与 Windows lo-src d128762f 上移动测与 ely-show、ely-table 整套过；ely-doc 整套那次败的一条先于本条，另立 R20.66（已勾，4359ce88）） |
| R20.62 | 进行中 | Windows 装后首开 ElyDoc 超 3 秒 | 无审查 | f32bd0dc | 0.1.17：未做；被 Defender 查的文件未查明，须在新装的 Windows 测试机上量 |
| R20.63 | 过审 | Windows 登录后冷开 ElyDoc 慢：软件渲染下动画按减少动效 | `the_system_animations_on_default_and_a_software_renderer_reduce_motion`；`tasks/reviews/R20.63-r3.md` PASS | c564bbd8 | 余下的冷读差归 R20.74（已勾，e87c05b9），登录项起得晚归 R20.71 |
| R20.64 | 过审 | 存盘失败的提示只说哪份文档、为什么、怎么办 | `a_drive_root_is_worded_without_its_path`、`a_protected_copy_the_system_refuses_is_worded`；`tasks/reviews/R20.64-r3.md` PASS | ec98c26e | 打印、OCR 等非存盘步骤的提示另立 R20.70（已勾，802dc224） |
| R20.65 | 过审 | `media-targets.sh` 只查他平台、由 pr.yml 跑，依赖由 ely-platform 生成 | `tests/scripts/media-targets.sh`；`tasks/reviews/R20.65-r1.md` PASS | 7ae559e8 | 真 crate 不能交叉检查（ring 要目标的 C 头文件），只查临时 crate |
| R20.66 | 过审 | ElyDoc 关闭提前载入的测试按事件等 | `a_launch_tab_closed_before_its_turn_closes_the_early_load`、`a_closed_load_lets_go_and_keeps_the_engine`；`tasks/reviews/R20.66-r1.md` PASS | 4359ce88 | Mac、Windows 未跑 |
| R20.67 | 进行中 | `tests/office/compare.ps1` 在 Windows 上从未整份跑完 | 无审查 | d84a58cc | 0.1.17：未做；须 Windows 测试机与 Microsoft 365；R15.11g 的复核也等它 |
| R20.68 | 过审 | ElyMark 的 vcs 测试不以为临时文件夹在 Git 仓库外 | `spaces_stay_and_broken_repositories_say_why`；`tasks/reviews/R20.68-r1.md` PASS | 88c0abfc | Windows 未跑（随 R20.67 的时段） |
| R20.69 | 过审 | ElyPDF 写入被拒的测试等的是新的提示 | `a_write_the_system_refuses_reports_it_without_a_crash`；随 R20.64 第 3 轮一并勾（`tasks/reviews/R20.64-r3.md` PASS）；无审查 | 6fdb300c | — |
| R20.70 | 过审 | ElyPDF 的通用失败提示不照显示系统原话 | `every_language_says_why_without_paths`、`each_step_names_itself_when_it_fails`；`tasks/reviews/R20.70-r1.md` PASS | 802dc224 | Windows 未跑 |
| R20.71 | 进行中 | Windows 登录项 ely-login 由 Run 键起，开机后起得早晚不一 | 无审查 | ef15464d | 0.1.17：不随 0.1.17；拟改为登录时的计划任务（decisions 2026-11-20，未定），量定之前不改代码、常设决定与 standing.md |
| R20.72 | 过审 | 软件渲染下的重复动画占满 CPU | 无审查 | 86e2f787 | —（量后关，未改代码：R20.63（已勾，6aee0f1d）已让软件渲染按减少动效） |
| R20.73 | 过审 | `vendor/` 下各 crate 自己的测试无处在跑 | `tests/scripts/vendor-tests.sh`；`tasks/reviews/R20.73-r2.md` PASS | cebed0d4 | PR 不加 macOS、Windows 的托管任务（主代理定），vendor 下的测试在这两端坏了要到发布的门才拦下 |
| R20.74 | 过审 | 清缓存后冷开 ElyDoc 仍比 Word 慢约 0.4 秒 | `a_worker_logs_each_start_stage_with_its_counters`；`tasks/reviews/R20.74-r1.md` PASS | e87c05b9 | — |
| R20.75 | 进行中 | main 的 CI 红了 20 小时无人察觉 | 无审查 | 6372bae6 | 0.1.17：未做；谁收、怎么收 main 上 pr.yml 的失败未定，未以一次真失败验证 |
| R20.76 | 过审 | pr.yml 只跑 tests/perf 的 test_idle | `.github/workflows/pr.yml` 的 “Perf unit tests, every test_*.py found”；`tasks/reviews/R20.76-r1.md` PASS | d455e9e6 | — |
| R20.77 | 进行中 | 不带宏的演示稿里宏链接报出内核的英文问话 | 无审查 | eec57122 | 0.1.17：根治在补丁 0148，0.1.17 发布后随之入库 |
| R20.78 | 进行中 | 编辑文本框时不按 Ctrl 点另一框的链接即打开 | 无审查 | 58fe2961 | 0.1.17：随补丁 0148 在 0.1.17 发布后入库（行内写明） |
| R20.79 | 过审 | 换成 `ely-office` 包后常驻引擎仍是旧版（用户单元遮住包的单元） | `starts_at_once_each_succeed_and_remove_each_path_once`、`a_path_another_start_removed_counts_as_gone`（`crates/ely-platform/src/install/units/tests.rs`）；`tasks/reviews/R20.79-r2.md` PASS | 4341670e | — |
| R20.80 | 进行中 | ely-login.exe 不可重现构建，Windows 安装包的内核编号随构建变 | 无审查 | 6e9e118d | 0.1.17：未做；同一提交建两次的编号不同，无法从源核对发布的编号 |
| R20.81 | 过审 | Windows 的换页读法不看 PDH 计数器的数据状态 | `tests/perf/test_counters.py` 的 `test_pdh_refuses_invalid_data_the_call_called_a_success`；`tasks/reviews/R20.81-r1.md` PASS | 7e38f5b4 | — |
| R20.82 | 过审 | macOS 的私有内存读法“换出的也算”无出处 | `tests/perf/test_bench.py` 的 `test_macos_reads_the_footprint_ledger_not_resident`；`tasks/reviews/R20.82-r1.md` PASS | d455e9e6 | 未做真的换出回归测试（那台 Mac 是有人在用的桌面，不许施加内存压力），以 xnu 源码为据 |
| R20.83 | 进行中 | 由 AppImage 换成 `ely-office` 包后菜单项仍开旧的 AppImage | 无审查 | 97a2dbc1 | 0.1.17：未做；照 R20.79（已勾，94000900）的办法，AppImage 用自己的桌面文件 ID |
| R20.84 | 过审 | Windows 上基准的子进程输出按 ANSI 代码页解码 | `tests/perf/test_child.py` 的 `test_a_byte_cp1252_reads_but_utf8_refuses_fails_naming_the_command`；`tasks/reviews/R20.84-r2.md` PASS | 5e658c04 | Windows 上的坏字节与换行另立 R20.98（已勾，1e093b42）、R20.102（已勾，854082ac）；测试机控制台本即 UTF-8，`drive.ps1` 那一行之必要未在此机显出 |
| R20.85 | 过审 | 失败的 `--redo` 存下完成项却不存其换页记录 | `tests/perf/test_bench.py` 的 `test_a_failed_redo_keeps_what_it_finished_and_leaves_the_rest_void`；`tasks/reviews/R20.85-r1.md` PASS | 7374b3f3 | — |
| R20.86 | 过审 | `--redo` 点名不存在的项要等其余各项跑完才报错，已完成的又丢 | `tests/perf/test_bench.py` 的 `test_a_redo_naming_an_unknown_case_measures_nothing`；`tasks/reviews/R20.86-r1.md` PASS | 7374b3f3 | — |
| R20.87 | 过审 | check.sh 在非检出顶层的目录上照样通过 | `tests/scripts/check-rules.sh`；`tasks/reviews/R20.87-r2.md` PASS | b5a29d7e | — |
| R20.88 | 过审 | 帧日志把按键的帧记到之前一次什么都没画的点击上 | `a_click_that_drew_nothing_leaves_the_key_its_frame`、`a_gap_drops_every_waiting_press`；`tasks/reviews/R20.88-r2.md` PASS | 20a4946b | — |
| R20.89 | 进行中 | 发布的门不核内核是否在最后一个补丁之后重链 | `tests/scripts/lo-series.sh`；无审查 | ba83800a | 0.1.17：落地即改四棵内核的编号（含 Windows 安装包里的 b8c76f72），定在 0.1.17 之后 |
| R20.90 | 进行中 | 发布的门收下同版本、却量的是别的构建的记录 | 无审查 | 0d3c74ef | 0.1.17：依赖 R20.80 与 R20.89，现有记录皆无 commit、kernel 两项，0.1.17 之后做 |
| R20.91 | 过审 | 发布记录的间隔一行不核其数 | `tests/scripts/release-gate.sh`；`tasks/reviews/R20.91-r1.md` PASS | 3810cace | — |
| R20.92 | 未完 | 读应用日志时坏字节被换掉而不报错 | `tests/perf/test_child.py` 的 `test_a_character_split_across_two_reads_reads_back_exactly`、`tests/perf/test_idle.py`；`tasks/reviews/R20.92-r3.md` FAIL | 35ca233c | Windows 的 Restart Manager 列出打开者而非持锁者，另立 R20.103；不阻 0.1.17（发布的空闲门期间无应用，3b 每个应用恰一个持有者） |
| R20.93 | 过审 | `linux-apprun.sh` 留下 gvfsd-fuse，CI 也从不跑它 | `tests/scripts/linux-apprun.sh`；`tasks/reviews/R20.93-r1.md` PASS | 98938338 | — |
| R20.94 | 进行中 | 由 AppImage 换成包后图标可能仍是旧的 | 无审查 | 7ab117bd | 0.1.17：未做，定在 0.1.17 之后落地 |
| R20.95 | 过审 | Windows 上基准结束应用时连自己一并杀掉 | `tests/perf/test_lineage.py` 的 `test_a_refusal_from_a_process_mid_exit_passes_a_live_one_raises`；`tasks/reviews/R20.95-r3.md` PASS | c671251b | — |
| R20.96 | 过审 | ElyTree 在 Windows 上停 Git 时按过时的父 pid 找进程树 | `a_helper_that_outlives_git_is_ended_too`、`git_gone_from_the_listing_still_judges_its_helpers`；`tasks/reviews/R20.96-r2.md` PASS | 2a9ef662 | — |
| R20.97 | 未完 | bench 作废了项、却删了自己的换页记录 | `tests/perf/test_kept.py` 的 `test_engines_that_outlive_the_run_keep_samples_and_results`；`tasks/reviews/R20.97-r3.md` FAIL | d51ca1d2 | 余下三处失败路径的边缘另立 R20.100（已勾，72182873） |
| R20.98 | 过审 | Windows 上 `child.run` 遇坏字节不失败 | `tests/perf/test_child.py` 的 `test_run_hands_subprocess_bytes_and_decodes_them_itself`、`test_a_byte_cp1252_reads_but_utf8_refuses_fails_naming_the_command`；`tasks/reviews/R20.98-r1.md` PASS | 1e093b42 | 换行的回归另立 R20.102（已勾，854082ac） |
| R20.99 | 进行中 | 常驻引擎 lok-profile\tmp 下的目录写入时间变了，而我们的东西没碰它们 | 无审查 | 6f7f4a5f | 0.1.17：起因未查明；测试机上 5 分钟的空闲对照里空闲引擎没写 profile，未见违“空闲不做事”的证据 |
| R20.100 | 过审 | bench 失败路径余下的三处会丢证据或顶替原错 | `tests/perf/test_kept.py`；`tasks/reviews/R20.100-r2.md` PASS | 72182873 | — |
| R20.101 | 过审 | Windows 的换页判定把后台写页文件当成换页 | `tests/perf/test_counters.py`；`tasks/reviews/R20.101-r1.md` PASS | 2f4fca18 | —（R20.30 的 Windows 0.1.17 基准照此规则跑完：每项低内存样本 0，34 项皆算数） |
| R20.102 | 过审 | R20.98 使 Windows 上子进程输出的换行成了 CRLF | `tests/perf/test_child.py` 的 `test_crlf_and_lone_cr_read_back_as_text_true_reads_them`、`test_drive_reads_when_a_crlf_helper_sent`；`tasks/reviews/R20.102-r1.md` PASS | 973c0151 | — |
| R20.103 | 进行中 | Windows 的空闲门以 Restart Manager 认持锁者，它列出的是打开者 | 无审查 | 35ca233c | 0.1.17：不阻（发布的空闲门期间无应用）；稳妥的修法要 worker 自己记 pid，在 crates/ 下，须等冻结解除 |
| R20.104 | 进行中 | 验收核对表的余下一列仍引 R20.26 已清掉的阻碍 | `tasks/reviews/R20.104-r2.md` FAIL | 本提交 | — |
| R20.105 | 进行中 | R20.51 的回归测试在 macOS 上败 | `a_macro_link_the_kernel_opens_itself_never_runs`（`crates/ely-lok/src/client/tests/opendoc.rs`）；无审查 | 825948c8 | 改为每项各载一份新的演示稿、编辑前等内核报出编辑；Mac 上原写法是偶发的败，新写法只以连跑 3 次过为证，变异不证它挡得住那一败；Mac 上图像映射问话之后的点击另立 R20.106 |
| R20.106 | 进行中 | macOS：图像映射的宏问话之后，视图里接下来的点击不进编辑 | 无审查 | 825948c8 | 0.1.17：不挡；修在内核（查模态循环的退出与该次按下的鼠标抬起），冻结之后做，新的内核补丁随三端共享引擎重建 |
| C15.1 | 暂缓 | 嵌入 Kigi CLI 作后端，不用 ACP | 无审查 | 2a72332f | 暂缓（用户 2026-11-19），待用户重开 |
| C15.2 | 暂缓 | 工具管理、配置管理、配置保护 | 无审查 | 2a72332f | 暂缓（用户 2026-11-19），待用户重开 |
| C15.3 | 暂缓 | 会话、输入输出、并行运行 | 无审查 | 2a72332f | 暂缓（用户 2026-11-19），待用户重开 |
| C15.4 | 暂缓 | 工具与授权、文件与产物 | 无审查 | 2a72332f | 暂缓（用户 2026-11-19），待用户重开 |
| C15.5 | 暂缓 | 会话恢复、执行控制、历史、用量 | 无审查 | 2a72332f | 暂缓（用户 2026-11-19），待用户重开 |
| C15.6 | 暂缓 | 本地模型、托盘、生命周期 | 无审查 | 2a72332f | 暂缓（用户 2026-11-19），待用户重开 |
| C20.2 | 延后 | 稳定性：磁盘写满、长时编辑、内核崩溃、休眠 | `tests/perf/soak.py`、`tests/perf/pause.py`；无审查 | 36f6df8e | Mac 不能自己醒（`pmset schedule wake` 须 root），未让它休眠；Windows 虚拟机不休眠；C20.2 仍开：R20.54（长段落内存）与真的系统挂起 |
| C22.2 | 过审 | `docs/qa.md`：QA-01 至 QA-28 的证据与缺口 | `docs/qa.md`；`tasks/reviews/C22.2-r2.md` PASS | 10aca431 | 只整理，未重跑；全有证据 0 项，部分 26 项，无证据 2 项（QA-23、QA-24，ElyAgent 暂缓） |
| C23.1 | 过审 | PRD §23 九项工程风险的证据与状态 | `docs/decisions.md` 的 C23.1 一节；`tasks/reviews/C23.1-r2.md` PASS | 21edbbaa | 只整理，未重跑；达到 0 项，部分 8 项，无 1 项（Agent 后端差异）；§23 末段缺升级后须跑哪些回归的规定 |
| C24.1 | 过审 | 交付物：README、构建说明、已知限制、测试报告 | `README.md`、`docs/compatibility.md`；`tasks/reviews/C24.1-r2.md` PASS | d7508015 | Mac 的 build-git 未在独立克隆里跑；Windows、Mac 未起界面；Windows、Mac 的内核已由 R20.26 重建（ElyMark 的那条限制已随 1868d1cc 改写、随 e287f3ce 去掉行尾隐藏单元格一项） |
| R19.2 | 未完 | #83 之二：ElyShow 微软雅黑粗体个别字发细 | 无审查 | 2052193e | 未复现，等报告者的版本、显示语言与文件；补丁 0139 修了一个根因，三端内核已由 R20.26 重建（含 0139），单字发细未在中文界面上复核 |
| R19.3 | 进行中 | 72 条 `[~]` 逐条再审 | `tasks/reviews/R15.11-r4.md`、`tasks/reviews/R15.11c-r4.md` PASS；无审查 | 500f7b74 | 余 R15.11g 一条：第 4 轮 FAIL（`tasks/reviews/R15.11g-r4.md`，补丁 0074 让 `m:lit` 的内容仍当 StarMath 命令读），待改补丁再审；Office 的整份对照未做（R20.67） |
| R19.5 | 过审 | 按键到图块晚一帧：图块到前压住重画与报告 | `a_kernel_change_repaints_and_reports_once_its_shown_tiles_land`、`an_input_landing_before_its_batch_repaints_and_reports_once_its_tiles_land`；`tasks/reviews/R19.5-r2.md` PASS | 905c0d9a | 延迟的绝对值出自多带 65 部词典的内核，不能与基准记录相比，只作有无对照；每跑末尾不足 2 秒的帧未核焦点 |
| R19.6 | 未完 | Windows 0.1.16 基准：首次启动慢与 ElyMail 卡顿 | `tasks/reviews/R19.6-r2.md` PASS | db49e3b9 | ElyPDF 10.9 秒与 11-12 的六次卡顿未复现，原因未明；首次启动的 Defender 扫描待签名（R-02）；分忙与饿交 R20.41（已勾，e89832ae） |
| R19.9 | 进行中 | ElyShow 整组跑时 `animation.add` 偶被拒 | 无审查 | fefebefb | 根因未定：R20.12 加了拒因的日志（补丁 0138），Windows lo-sym 258dcf4d 上整组十遍未复现 |

## R21 外部 PR、新议题与 #107 命令行

R21.38：R21.0 至 R21.43 逐条核对（用户 2026-11-24 定的外部 PR、新议题与 #107 命令行），2026-11-26 据 5c496888 写成。“过审”即 `[x]` 且末轮审查 PASS，“未完”为 `[!]`（未复现，等报告者），“受阻”为等外部条件，“进行中”为 `[ ]`。证据列该条的提交里新加或改过、今日树里仍在的测试（带测试属性的函数或 `tests/perf` 的 `def test_`，至多两个），没有时列该条点名的测试文件或脚本，组件库的写明；再列已入库的末轮审查。“末次提交”为提交说明点名该编号的最后一个，PR 为其合并提交。所引测试、路径与审查结论由脚本逐个核对（`.work/R21/r2138-evidence.py`，本机，未入库）。本表不重跑测试。

| 编号 | 状态 | 事 | 证据 | 末次提交 | 未验证或余下 |
| --- | --- | --- | --- | --- | --- |
| R21.0 | 过审 | deny 两条新公告与 Windows clippy | `scripts/deny.sh`；`tasks/reviews/R21.0-r1.md` PASS | f220e7dd | — |
| R21.1 | 过审 | PR #97 清除格式、字号增减、更改大小写、格式刷、段落底纹 | `a_paragraph_takes_shading`、`shading_colors_the_paragraph_not_its_letters`；`tasks/reviews/R21.1-r3.md` PASS | b97b13f2 | 句首大写在句点后跟小写时不断句，另立 R21.7a（已过审） |
| R21.2 | 过审 | PR #98 插入符号、文本框与基本形状 | `case_rows_and_a_symbol_change_the_saved_text`、`a_symbol_without_a_document_says_so`；`tasks/reviews/R21.2-r2.md` PASS | a10ac70a | — |
| R21.3 | 过审 | PR #99 加上与去掉水印 | `a_watermark_goes_into_the_page_and_comes_out`、`a_watermark_is_added_and_removed`；`tasks/reviews/R21.3-r2.md` PASS | df91d9f7 | — |
| R21.4 | 过审 | PR #100 Linux 缺的 Word 字体以最近的随附字体画 | `missing_word_fonts_draw_as_the_nearest_free_font`；`tasks/reviews/R21.4-r2.md` PASS | 61b84fc6 | —（三端重建的共享引擎上补跑过） |
| R21.5 | 过审 | PR #101 内核随附 Roboto，字体由脚本照清单取 | `scripts/fetch-fonts.sh`、`tests/scripts/fetch-fonts-rules.sh`；`tasks/reviews/R21.5-r3.md` PASS | 6f2da8de | —（三端重建的共享引擎上补跑过） |
| R21.6 | 过审 | PR #102 内核随附五个西文家族 | `the_free_fonts_are_not_missing_but_an_unknown_font_is`；`tasks/reviews/R21.6-r1.md` PASS | ec690ea6 | —（三端重建的共享引擎上补跑过） |
| R21.7 | 过审 | PR #104 内核随附 Noto 中文字体与中文别名 | `apps/ely-doc/src/tests/fonts/stand_ins.rs`；`tasks/reviews/R21.7-r2.md` PASS | 31f81e9e | —（安装包增量实测 xz 后 31.9 MB） |
| R21.7a | 过审 | 句首大写照 Word 断句（补丁 0147） | `every_full_stop_starts_a_sentence`、`a_selection_inside_sentences_changes_only_itself`；`tasks/reviews/R21.7a-r1.md` PASS | 6c8e2e5a | — |
| R21.8 | 过审 | PR #105 导出为 Markdown 交 ElyMark 打开 | `a_document_exports_as_markdown`、`an_open_document_is_not_overwritten_by_the_export`；`tasks/reviews/R21.8-r2.md` PASS | b5059ca9 | — |
| R21.9 | 过审 | PR #106 字号框箭头按一磅增减（组件库 `Combobox::steps`） | `the_size_arrows_send_a_point_up_and_down`、`the_arrows_step_one_point_within_the_limits`；`tasks/reviews/R21.9-r2.md` PASS | f9496b3d | — |
| R21.10 | 过审 | PR #108 ElyTable 边框样式、粗细与颜色 | `another_workbook_is_not_restyled`、`edges_follow_the_marks`；`tasks/reviews/R21.10-r2.md` PASS | f22b489b | 两个下拉在 AX 里无名，R21.37f 补上（已过审） |
| R21.11 | 过审 | PR #109 ElyTable 复制区域的静止虚线框 | `the_range_holds_every_selected_rect`、`copied_cells_keep_a_still_outline_until_an_edit_or_escape`；`tasks/reviews/R21.11-r1.md` PASS | a9ed1ce7 | — |
| R21.12 | 过审 | #94 AUR：`ely-office-bin` 的 PKGBUILD 由 `packaging/aur.sh` 生成 | `tests/scripts/aur.sh`；`tasks/reviews/R21.12-r1.md` PASS | 0db6771d | 推上 AUR 见 R21.12b；#94 未关 |
| R21.12b | 受阻 | #94 推上 AUR | 无测试；无审查 | - | 受阻：AUR 新账号注册暂停（用户 2026-11-25 实测），等注册重开、用户登记 Omarchy 的公钥 |
| R21.12a | 过审 | 竖向滚动条不再压在内容上（组件库 `ScrollArea` 留槽） | 组件库 `crates/ely-core/src/layout/scroll/tests/gutter.rs`；`tasks/reviews/R21.12a-r1.md` PASS | f7d53afc | — |
| R21.13 | 过审 | #110 Aptos、Arial Nova 的替换字体（补丁 0149） | `missing_office_fonts_draw_as_a_sans_of_like_width`；`tasks/reviews/R21.13-r4.md` PASS | 4b9e399b | — |
| R21.14 | 过审 | #93 ElyMark 表格长格里的查找命中可见（组件库） | 组件库 `crates/ely-core/src/forms/tests/shown.rs`、组件库 `crates/ely-documents/src/document/tests/found.rs`；`tasks/reviews/R21.14-r1.md` PASS | 2cf7422f | — |
| R21.15 | 过审 | #86 ElyTable 单元格字体与字号：核对发布版后关闭 | 核对发布版，无测试；`tasks/reviews/R21.15-r1.md` PASS | 5bfbd570 | —（无代码改动） |
| R21.16 | 未完 | #83 ElyShow 微软雅黑粗体个别字发细 | 探针未入库；`tasks/reviews/R21.16-r1.md` PASS | bb2d9f4e | 未复现，未修：“施”取了微软雅黑 Light；未试系统区域设置改 zh-CN 与真微软拼音经 GPUI 输入；已向报告者要 0.1.17 下的结果、文件与区域设置，#83 未关 |
| R21.17 | 过审 | #84 ElyPDF 缩略图列表的滚动条；左面板让出拉宽把手 | `the_page_list_has_a_bar_that_rolls_it`、`the_left_panel_ends_where_its_grip_begins`；`tasks/reviews/R21.17-r1.md` PASS | 6576acce | — |
| R21.18 | 过审 | #95 ElyMail 未读筛选下点开的邮件留在列表里 | `a_held_message_stays_though_it_no_longer_fits_the_filter`、`a_message_opened_under_unread_stays_until_the_filter_changes`；`tasks/reviews/R21.18-r2.md` PASS | b87d949b | — |
| R21.19 | 过审 | #96 同一地址不同收信服务器是两个账户（库 V19） | `one_address_on_two_servers_is_two_accounts`、`an_older_store_keeps_its_vault_names_and_ids`；`tasks/reviews/R21.19-r1.md` PASS | abd73e2e | — |
| R21.19a | 未完 | ElyMail 侧栏与列表的字下缘被截 | 探针未入库；`tasks/reviews/R21.19ab-r2.md` PASS | 4374281c | 未复现，未修：Mac 2x、Windows 1x 五种字号、Omarchy 1x/1.25x/1.6x 实画皆不截；已向用户要系统、缩放、界面字体与整窗截图 |
| R21.19b | 未完 | ElyMail 未读数不在徽章正中 | 探针未入库；`tasks/reviews/R21.19ab-r2.md` PASS | 4374281c | 未复现，未修：同 R21.19a 的环境下上下差 0–1 像素（Windows 字号 14 以上偏上），左右差来自字形留白；随 R21.19a 一并要环境与截图 |
| R21.20 | 过审 | #92 ElyMark 查找命中醒目、居中、边上标出（组件库） | 组件库 `crates/ely-documents/src/document/tests/found.rs`；`tasks/reviews/R21.20-r3.md` PASS | e5364269 | — |
| R21.21 | 过审 | #85 ElyMark 整体字号：核对发布版后关闭 | 核对发布版，无测试；`tasks/reviews/R21.21-r1.md` PASS | 35a443d0 | —（无代码改动） |
| R21.22 | 过审 | 无 `USERNAME` 等变量时引擎照起（系统取登录名） | `the_account_names_itself_without_its_environment`；`tasks/reviews/R21.22-r1.md` PASS | 30763e1b | Omarchy 构建容器里 uid 1000 无 passwd 条目，改前改后都取不到名字（容器的事，主机上过） |
| R21.23 | 过审 | `ely-*-start` 在应用起不来时非零退出 | `an_app_that_fails_before_its_window_fails_the_launcher`、`an_app_that_fails_after_the_splash_hid_still_fails_the_launcher`；`tasks/reviews/R21.23-r2.md` PASS | 7f8fbcaf | — |
| R21.24 | 过审 | 一个窗口的应用收到 `--new-window` 照开文件 | `a_one_window_app_keeps_the_files_and_drops_the_flag`；`tasks/reviews/R21.24-r1.md` PASS | 59947ac8 | — |
| R21.25 | 过审 | 命令行共通约定与 `open`、`new`、`export`、`docs`、`save` | `open_loads_once_and_export_writes_a_copy`、`verbs_stand_before_the_noun_and_files_become_absolute`；`tasks/reviews/R21.25-r2.md` PASS | ca89e9fd | — |
| R21.26 | 过审 | ElyTable `range get|set`（补丁 0150） | `a_range_reads_and_writes_by_address_in_one_undo_step`、`a_block_set_from_outside_reads_back_and_undoes_in_one_step`；`tasks/reviews/R21.26-r1.md` PASS | 3c3d4dd2 | — |
| R21.27 | 过审 | ElyTable `sheet`、`rows`、`columns`、合并（补丁 0151） | `sheets_rows_columns_and_merges_change_by_address_one_undo_step_each`、`structure_set_from_outside_shows_in_the_tabs_and_undoes`；`tasks/reviews/R21.27-r2.md` PASS | a017af11 | — |
| R21.27a | 过审 | `table-cli.sh` 的保存重叠检查改查日志次序 | `tests/scripts/table-cli.sh`；`tasks/reviews/R21.27a-r1.md` PASS | c6b87f11 | — |
| R21.28 | 过审 | ElyTable `format` 属性表（补丁 0152） | `the_kernel_gets_checked_lines_and_the_file_made_absolute`、`borders_expand_and_a_named_edge_overrides`；`tasks/reviews/R21.28-r2.md` PASS | 2727a753 | — |
| R21.28a | 过审 | structure 测试的冻结等待等行、列两个状态 | `crates/ely-lok/src/client/tests/by_structure.rs`；`tasks/reviews/R21.28a-r1.md` PASS | 85be0b56 | — |
| R21.29 | 过审 | ElyDoc `text replace` 带换行、`text append`、`text get --json`（补丁 0153） | `every_mapped_command_reaches_the_kernel_and_reports_state`、`without_a_break_the_plain_replace_stays`；`tasks/reviews/R21.29-r1.md` PASS | 6bdb837c | — |
| R21.30 | 过审 | ElyDoc `format` 按原文设格式与段落样式（补丁 0154） | `pairs_become_the_kernels_seven_fields`、`a_bad_find_or_pair_is_refused`；`tasks/reviews/R21.30-r1.md` PASS | 8a1d1db3 | — |
| R21.31 | 过审 | ElyDoc `table insert`（补丁 0155） | `data_becomes_the_kernels_rows_and_cells`、`bad_data_or_flags_are_refused`；`tasks/reviews/R21.31-r2.md` PASS | 4a42302b | — |
| R21.32 | 过审 | ElyDoc `image insert` | `the_picture_goes_as_a_url_with_its_width_in_twips`、`a_missing_picture_bad_width_or_flags_are_refused`；`tasks/reviews/R21.32-r1.md` PASS | 248a1fd4 | — |
| R21.33 | 过审 | ElyShow `slide list|add|duplicate|delete|move` | `the_names_follow_the_kernel_layouts`、`requests_carry_numbers_and_the_layout_id`；`tasks/reviews/R21.33-r1.md` PASS | 4baf2c06 | — |
| R21.34 | 过审 | ElyShow `text set`、带换行的 `text replace`、`notes get|set` | `requests_check_the_box_and_number`、`boxes_must_be_there_and_of_one_look`；`tasks/reviews/R21.34-r1.md` PASS | 2119937d | — |
| R21.35 | 过审 | ElyShow `image insert` | `the_picture_goes_as_a_url_with_its_slide_and_width`、`a_missing_picture_bad_slide_width_or_flags_are_refused`；`tasks/reviews/R21.35-r1.md` PASS | 60b41924 | — |
| R21.36 | 过审 | ElyMark `text set`、`text append` | `set_and_append_are_one_step_each_and_keep_the_line_endings`；`tasks/reviews/R21.36-r1.md` PASS | 70511930 | — |
| R21.37 | 过审 | 命令行收口：help 四节、常设决定、官网页；#107 关闭 | `tests/scripts/doc-cli.sh`、`tests/scripts/table-cli.sh`；`tasks/reviews/R21.37-r2.md` PASS | cf2e2c9b | —（ElyPDF 不设命令行是有意不做） |
| R21.37b | 过审 | #112 选区内字号不一时字号框为空 | `a_line_of_two_sizes_reads_mixed_either_way`、`a_line_of_two_sizes_reads_mixed_from_either_end`；`tasks/reviews/R21.37b-r1.md` PASS | 030dffff | 报告未写版本，按 v0.1.16 及以前的行为推断；macOS 未跑（见表下） |
| R21.37c | 过审 | #111 长 Word 文件排出首页即可用（补丁 0160，Writer 不再整篇 Repaginate） | `a_long_document_loads_before_its_layout_ends`；`tasks/reviews/R21.37c-r1.md` PASS | 041c754f | 报告者的原文件不能外发、版本未写，以同量级样本在 7877 实测；macOS 未跑（见表下） |
| R21.37d | 过审 | #113 双页视图封面单排、成对翻页；三端量滚动帧时 | `side_by_side_turns_a_pair_and_the_cover_stands_alone`、`a_book_shows_the_cover_alone_then_pairs`、`tests/perf/pdfscroll.py`；`tasks/reviews/R21.37d-r1.md` PASS | 3891e8b9 | 双页一半做完；卡顿一半未在报告者的环境复现、未修（见表下）：ElyPDF 帧绘制 2–4 ms、无丢帧，7877（WARP）受 DwmFlush 约 45 Hz 限、Omarchy（llvmpipe）CPU 光栅，软件渲染下达不到 120 FPS；真 120 Hz 显示器未测 |
| R21.37e | 过审 | #114 单页视图滚过页边翻页，方向键各视图可用 | `a_single_page_turns_when_the_wheel_goes_past_it`、`arrows_scroll_and_turn_in_every_view`；`tasks/reviews/R21.37e-r1.md` PASS | c930f690 | 真机触控板未试（Mac 上系统级滚轮探测过） |
| R21.37a | 过审 | `gpui-pre` 升到 0.3.14，组件库与 GPUI Kit 0.11.0 并存 | `scripts/vendor-gpui.sh`（`check`）、`tests/scripts/macos-names.sh`；`tasks/reviews/R21.37a-r1.md` PASS | 34a711ee | 未以 VoiceOver 亲听（AX 树经 `macos-names.sh` 核过） |
| R21.37f | 过审 | ElyTable 边框样式与粗细两个下拉有读屏名 | `tests/scripts/macos-names.sh`；`tasks/reviews/R21.37f-r1.md` PASS | c9a53564 | Windows、Linux 的无障碍脚本不核控件名，未跑 |
| R21.37g | 过审 | Omarchy 上四条单测失败（main 上同败）：两条中文字体测试放宽到 Linux 上同类的系统字体，ElyMark 替身移到子进程 | `a_file_open_in_elymark_is_not_overwritten`、`a_missing_chinese_font_falls_to_a_text_face_of_its_kind`；`tasks/reviews/R21.37g-r1.md` PASS | 57422954 | `stand_ins` 今日三遍不复现，R21.37c 时的失败未归因 |
| R21.37h | 过审 | `*-cli.sh` 收尾与重启前等 LOK worker 退出 | `tests/scripts/lok-workers.sh`；`tasks/reviews/R21.37h-r1.md` PASS | 60f9c085 | — |
| R21.37i | 过审 | main 上 `pr.yml` 自 R21.37d 起全红：三处本机证据补标，`vendor-gpui.sh` 的临时仓不随本机 Git 改行尾 | `scripts/vendor-gpui.sh`、`tests/scripts/records.py`；`tasks/reviews/R21.37i-r1.md` PASS | 27d5c803 | — |
| R21.38 | 过审 | 本表与顾问终审 | 无测试；`tasks/reviews/R21.38-r1.md` PASS | 6f8cce58 | — |
| R21.39 | 过审 | PR #115 Office 存盘时 PNG、JPEG 只存不压（补丁 0161） | `office_saves_store_png_and_jpeg_without_deflating`；`tasks/reviews/R21.39-r1.md` PASS | 2cddcc98 | macOS 未跑（见表下）；在 Microsoft Office 里打开存出的文件未试 |
| R21.40 | 过审 | PR #116 Linux 引擎缓存大图的缩放副本（补丁 0162–0169，上游回移） | `an_edited_large_picture_paints_as_a_fresh_load_does`；`tasks/reviews/R21.40-r1.md` PASS | 5c17f0be | 重绘耗时未在此重测（PR 在 6c8e2e5a 上量）；macOS 未跑（见表下） |
| R21.41 | 过审 | ElyShow 三条视频测试在 Omarchy 上失败：机器缺 gst-plugins-good、gst-libav；测试改为见到提示即报原因 | `the_next_slide_hands_the_last_picture_on`；`tasks/reviews/R21.41-r1.md` PASS | 0405098d | — |
| R21.42 | 进行中 | 验收表补 R21.39–R21.43 五行 | 无测试；审查随本条 | 本提交 | — |
| R21.43 | 过审 | PR #117：ElyPDF 审阅副本的独立 Python 示例（PyMuPDF） | `examples/pdf-review/test_review.py`；`tasks/reviews/R21.43-r1.md` PASS | 5c496888 | CI 不跑 `examples/` 的测试；只在 Omarchy 与 7877 的独立 venv 上跑过；macOS 未试 |

macOS：补丁 0150 至 0169（R21.26–R21.37c、R21.39、R21.40 的内核改动）只在 Omarchy 与 7877 的内核上建过、测过；Mac 的共享引擎停在 142 个补丁（R21.13 时重建），发布前须先以全部补丁重建再跑带内核的测试。

`[!]` 与未复现（皆未修，等报告者或用户的环境与材料）：

- R21.16 #83 ElyShow 微软雅黑粗体个别字发细：发细的字取了微软雅黑 Light 字面；7877 上四种输入路径、中文语言包与 zh-CN 区域格式下皆不复现；未试系统区域设置（要重启测试机）与真微软拼音经 GPUI 输入。#83 已留言要材料，未关。
- R21.19a ElyMail 侧栏与列表的字下缘被截：三端、多种缩放与字号实画皆不截。
- R21.19b ElyMail 未读数不在徽章正中：同上环境下不偏下；只 Segoe UI 下“999+”偏下 1 像素，与报告不符。
- R21.37d #113 的卡顿一半（条目本身过审，双页一半做完）：报告者的平台未知，未在其环境复现，ElyPDF 不改代码；软件渲染（WARP、llvmpipe）下达不到 120 FPS 的常设要求。#113 已关，回复请报告者在新版仍卡时重开并附系统、显卡、版本与样本。

靠推断的：R21.37b（#112 报告未写版本）、R21.37c（#111 原文件不能外发、版本未写），皆以同类样本复现并修。

仍开着：R21.12b（#94 推上 AUR，受阻于 AUR 暂停注册）。

顾问 (c) 的几次问询（R21.16、R21.19a/b、R21.20、#113）未得答复，各条照自己的判断定，各条目里写明。

GitHub 核对（2026-11-26，`gh`）：PR #97、#98、#99、#100、#101、#102、#104、#105、#106、#108、#109、#115、#116、#117 皆已合并，各有一条致谢并写明合并提交；议题 #84、#85、#86、#92、#93、#95、#96、#103、#107、#110、#111、#112、#113、#114 皆已关闭，末条为致谢回复；#83、#94 开着，末条为现状与所需材料。无开着的 PR。

`pr.yml`：main 自 R21.37d（3891e8b9）至 R21.37h（60f9c085）的推送皆败，交付前查出，由 R21.37i 修好。

顾问终审（2026-11-25，(d)）：除三条 `[!]` 与 R21.12b 外目标达成；已勾的条目无须重开或改标 `[!]`。所提四处缺口已照办：上面的 macOS 一段；Omarchy 上四条失败与 `*-cli.sh` 收尾的竞争另立 R21.37g、R21.37h 并做完；R21.37d 的卡顿一半列入未复现；R21.37b、R21.37c 写明靠推断。顾问未重跑测试、未逐个核对测试名（本表由脚本核对）。
