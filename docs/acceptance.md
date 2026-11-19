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
| R-06 | P1 | 未复审的项更多 | R18.32：说明公布 `[~]` 的数目（门 r1 过审）；涉及保存、恢复、更新、发信的十一项各补审一轮，全部过审 |
| R-07 | P1 | 证书信任未过审 | R18.19 三轮用尽（`[~]`）：r3 查出的 Unicode 3.2 规范化差异已修；自签名不受信的测试三端都有 |
| R-08 | P1 | 上游补丁 | R18.11 过审（r2）：`patches/UPSTREAM.md` 列出 53 处纯缺陷修复与复现；一个都没交上游（要所有者的账号与贡献协议），验收的“一半有链接”未达 |
| R-09 | P2 | gpui 修改压成一个提交 | R18.23 过审（r2）：`patches/gpui` 十三个补丁，`vendor-gpui.sh` 由原版重建并逐字节核对 |
| R-10 | P1 | 性能基线过期 | R18.30 三轮用尽（`[!]`）：门的一半过审（r2）；基准进发布的门，缺项与变慢三成都须写原因；两处退步查明并修好一处；ElyTree 的“内存上涨”是 malloc 分区，不是泄漏（`MALLOC_ARENA_MAX=2`）；本机 Linux 全量记录已写（`7937f129`），全部达标；三行按键到帧较 WSL2 下的上一份慢过三成，原因未查明（环境与代码同时变了）；r3 查出 PERF-08 的两处漏报，末轮后已修、未复审 |
| R-11 | P2 | 更新只依赖 github.com | R18.25 过审（r1）：屏蔽 GitHub 后经官网转发检查与下载都成 |
| R-12 | P1 | 读屏只读到当前段落 | R18.7 过审（r3）：ElyDoc 交出正文全部段落，Orca 从插入点连读到文末；表格与演示仍只到当前格、当前段 |
| N-01 | P0 | Windows 内核开着 DDE 服务 | R18.9 过审（r1）：补丁 0107，Windows 实测带补丁过、撤补丁败 |
| N-02 | P1 | 发布节奏 | R18.33 过审（r2）：间隔不足七天须在更新日志写原因，首行缺日期即败 |
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

R15.24：R15 的九十条逐条核对，2026-11-19 据 6675f984 写成。“过审”即 `tasks/TASKS.md` 里为 `[x]` 且末轮审查 PASS（R15.66、R15.70 由 R18.8、R18.9 做完，列其审查）；“部分”为 `[~]`，“未完”为 `[!]`。证据只列今日树里在的：该条所记、代码里确有其名的测试（至多两个）、`tests/scripts` 的脚本与末轮审查；本表不重跑测试。“未验证或余下”照该条最后的写法。

| 编号 | 状态 | 事 | 证据 | 未验证或余下 |
| --- | --- | --- | --- | --- |
| R15.0 | 过审 | 审查脚本改用 herdr 桌面应用的 `hc`（其后由 R15.32 换回 TUI） | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.0-r3.md` PASS | — |
| R15.1 | 过审 | #42 安装包体积 | `packaging/linux/package.sh` 遇带符号的 ELF 即失败；`tasks/reviews/R15.1-r1.md` PASS | — |
| R15.2 | 过审 | #44 缩放与位置 | `a_long_two_page_row_keeps_its_right_hand_page`；`a_wide_page_further_down_takes_no_zoom_from_the_page_read`；`tasks/reviews/R15.2-r5.md` PASS | — |
| R15.3 | 过审 | #44 左侧栏收起 | `apps/ely-pdf/src/keys.rs` 的单测；`tasks/reviews/R15.3-r1.md` PASS | — |
| R15.4 | 过审 | #44 拖动排页 | `a_drag_held_at_the_list_foot_rolls_to_the_last_page`；`a_drop_mid_roll_lands_on_the_line`；`tasks/reviews/R15.4-r3.md` PASS | — |
| R15.5 | 过审 | #44 常规编辑 | `a_move_leaves_the_structure_tree_as_it_was`；`a_moved_block_keeps_fonts_spacing_colour_and_hidden_text`；`tasks/reviews/R15.5-r8.md` PASS | — |
| R15.6 | 过审 | #46 ElyPDF 打开 EPUB | `a_failed_open_keeps_no_memory`；`tasks/reviews/R15.6-r2.md` PASS | — |
| R15.7 | 过审 | #48 Gmail 发信“starting TLS 时没有回应” | `a_quiet_server_is_named_in_the_readers_language`；`mail_sends_own_timeout_keeps_the_step`；`tasks/reviews/R15.7-r3.md` PASS | #48 的根因未查明（议题只有截图、无日志） |
| R15.8 | 过审 | #48 Outlook.com 收信“authenticated but not connected”、发信“SmtpClientAuthentication is disabled” | `outlooks_held_account_says_what_to_do`；`tasks/reviews/R15.8-r2.md` PASS | #48 的账号属哪种情形未查明 |
| R15.9 | 过审 | #47 ElyMark 正文两侧的空白处滚轮也滚动正文 | `a_wheel_beside_the_text_scrolls_it`；`source_margins_scroll`；`tasks/reviews/R15.9-r6.md` PASS | — |
| R15.10 | 过审 | #47 ElyMark 设置 | `a_bar_that_stays_never_wakes_to_fade`；`a_hold_cut_short_by_staying_never_wakes`；`tasks/reviews/R15.10-r2.md` PASS | — |
| R15.10a | 过审 | #49 腾讯文档导出的 docx、xlsx 永远停在“正在打开” | `a_broken_package_fails_instead_of_asking`；`a_broken_package_says_it_is_damaged_and_the_engine_stays`；`tasks/reviews/R15.10a-r4.md` PASS | 无补丁 0064 时卡住系照源码推断，未实测 |
| R15.11 | 部分 | 以 Office 建的复杂样本经 Ely 编辑保存、Office 读回 | `spelling_suggests_and_ignores`；`tasks/reviews/R15.11-r3.md` FAIL | 末轮 r3 两项的修复未经 Office 实跑（测试机无 Office，装 Office 的机器离线） |
| R15.11a | 过审 | XLSX 动态数组存回仍是动态数组 | `dynamic_arrays_save_as_dynamic_and_plain_ones_as_plain`；`pasted_and_undone_dynamic_arrays_stay_dynamic`；`tasks/reviews/R15.11a-r3.md` PASS | — |
| R15.11b | 过审 | DOCX 公式的减号存回仍是 U+2212 | `equation_minus_signs_save_as_math`；`tasks/reviews/R15.11b-r2.md` PASS | — |
| R15.11c | 部分 | DOCX 分节符所在的空段落与目录后的段落照原样存回 | `titled_toc_field_ends_in_its_own_paragraph`；`tasks/reviews/R15.11c-r3.md` FAIL | 末轮 FAIL 后修，未复审；核对须 Word |
| R15.12 | 过审 | Windows 启动时的空白窗口 | `docs_build_while_the_engine_holds_its_lock`；`tasks/reviews/R15.12-r2.md` PASS | — |
| R15.13a | 过审 | ElyShow 打开带视频的演示文稿，内核崩溃 | `painting_a_video_slide_starts_no_player`；`tasks/reviews/R15.13a-r1.md` PASS | — |
| R15.14a | 过审 | 缩放后文字被裁切 | `two_tiles_match_one_tile_over_both`；`tasks/reviews/R15.14a-r1.md` PASS | — |
| R15.13 | 过审 | 打开时“正在打开”久留：分段计时、缩短到首屏 | `a_panel_that_comes_with_content_takes_its_width_at_once`；`tasks/reviews/R15.13-r2.md` PASS | — |
| R15.13b | 过审 | 未改动的 Office 文件一打开即算已修改 | `a_docx_with_page_references_opens_unmodified`；`tasks/reviews/R15.13b-r1.md` PASS | — |
| R15.13c | 过审 | ElyDoc 同开几份编辑后“文档引擎已停止 (exit code: 0)” | `three_documents_edit_save_and_close`；`tasks/reviews/R15.13c-r1.md` PASS | — |
| R15.13d | 过审 | Windows 上引擎启动不再与内核的 Python 互锁；重启的引擎沿用原来的程序与配置目录 | `crates/ely-office/tests/restart.rs`；引用的 `a_slow_load_offers_the_stop_and_the_stop_ends_the_engine` 已随 SF.13 去掉横幅删去（740b0d0a）；`tasks/reviews/R15.13d-r2.md` PASS | — |
| R15.13e | 过审 | 编辑中按出 Writer 的旧式对话框不再让引擎退出 | `writer_dialog_keys_leave_every_document_working`；`tasks/reviews/R15.13e-r2.md` PASS | — |
| R15.14 | 过审 | 翻页时图块逐块出现 | `a_held_screen_keeps_its_border_reach`；`a_move_retries_a_failed_tile_and_holds_for_every_reader`；`tasks/reviews/R15.14-r8.md` PASS | 两项只在真按键下可见，未测 |
| R15.14b | 过审 | ElyPDF 翻页时页面逐块出现 | `apps/ely-pdf/src/tests/turn.rs`；`tasks/reviews/R15.14b-r3.md` PASS | — |
| R15.11d | 过审 | XLSX 透视表存回保留 Excel 的标签 | `tests/scripts/ooxml_pivot.py`；`tasks/reviews/R15.11d-r3.md` PASS | — |
| R15.11e | 过审 | PPTX 视频的进入动画存回照旧 | `tests/scripts/ooxml_motion.py`；`tasks/reviews/R15.11e-r2.md` PASS | — |
| R15.11f | 过审 | XLSX 公式里的逻辑常量存回仍是 `FALSE`、`TRUE`，不成函数 `FALSE()` | `tests/scripts/ooxml_formula.py`；`tasks/reviews/R15.11f-r1.md` PASS | — |
| R15.11g | 部分 | DOCX 公式的双线体 | `tests/scripts/ooxml_word.py`；`tasks/reviews/R15.11g-r3.md` FAIL | 核对须 Word，未再审 |
| R15.11h | 过审 | DOCX 目录域结束与正文同段时 | `a_text_box_after_a_toc_field_end_stays_outside_the_field`；`an_inline_content_control_keeps_the_toc_end_alone`；`tasks/reviews/R15.11h-r3.md` PASS | — |
| R15.11i | 过审 | XLSX 只有列字段的透视表存回照 Excel 的版式 | `a_pivot_with_only_columns_keeps_excel_s_layout`；`tasks/reviews/R15.11i-r2.md` PASS | — |
| R15.11j | 过审 | XLSX 透视表的数据布局表头存回照文件的 `dataCaption` | `values_on_rows_keep_the_file_s_data_caption`；`tasks/reviews/R15.11j-r1.md` PASS | — |
| R15.11k | 过审 | DOCX 目录的域结束导出在索引节后一段的开头，不写进末条目录 | `a_writer_toc_keeps_the_text_after_it`；`an_untitled_toc_ends_in_the_paragraph_after_it`；`tasks/reviews/R15.11k-r2.md` PASS | — |
| R15.11l | 过审 | XLSX 无值字段的透视表存回照 Excel 只到列项那一行 | `a_pivot_with_no_values_leaves_its_neighbour`；`tasks/reviews/R15.11l-r1.md` PASS | — |
| R15.25 | 过审 | #54 ElyDoc 新建文档的默认纸张 | `a_new_document_starts_on_the_chosen_paper`；`new_paper_follows_the_region_unless_chosen`；引用的 `only_doc_offers_the_new_paper` 随 R18.17（ElyPDF 也可选纸张）改去；`tasks/reviews/R15.25-r3.md` PASS | — |
| R15.26 | 过审 | 合并 PR #55 | `tests/scripts/windows-installer.sh`；`tasks/reviews/R15.26-r1.md` PASS | — |
| R15.27 | 过审 | #56 ElyTable 粘贴制表符分隔的中文文字 | `a_skipped_trailing_column_keeps_its_cell`；`pasted_lines_of_tabbed_text_fill_cells_without_a_dialog`；`tasks/reviews/R15.27-r3.md` PASS | — |
| R15.28 | 过审 | #57 ElyTable 单击超链接单元格 | `a_closed_message_box_reports_what_it_said`；`a_kernel_dialog_reaches_apps_as_closed`；`tasks/reviews/R15.28-r3.md` PASS | — |
| R15.29 | 过审 | #58 ElyDoc 打开邮件合并后右侧栏回不到文字、段落、样式 | `the_side_tabs_lead_back_from_mail_merge`；`tasks/reviews/R15.29-r1.md` PASS | — |
| R15.30 | 过审 | #59 界面字体到达对话框与 tooltip | `a_ghost_wears_the_theme_font`；`unstyled_text_takes_the_theme_overlays_too`；`tasks/reviews/R15.30-r1.md` PASS | — |
| R15.15 | 过审 | 合并 PR #51 | `builtin.rs` 的三语样式无重名测试；`tasks/reviews/R15.15-r2.md` PASS | — |
| R15.16 | 过审 | 西班牙语覆盖全部应用 | `a_failed_edit_lets_a_waiting_save_go`；`tasks/reviews/R15.16-r2.md` PASS | — |
| R15.17 | 过审 | 设置里的界面语言改为下拉列表 | `each_language_is_named_in_its_own_words`；`every_listed_language_has_its_catalog`；`tasks/reviews/R15.17-r1.md` PASS | — |
| R15.18 | 过审 | 日语覆盖全部应用 | `japanese_builtin_names_come_from_libreoffice`；`ods_empty_strings_keep_and_bad_numbers_do_not`；`tasks/reviews/R15.18-r1.md` PASS | — |
| R15.19 | 过审 | 韩语 | `every_listed_locale_can_be_picked`；`excel_caches_recompute_unless_the_function_is_unknown`；`tasks/reviews/R15.19-r3.md` PASS | 未经韩语母语者审读 |
| R15.20 | 过审 | 越南语 | `csv_imports_read_vietnamese_windows_1258`；`profiles_whose_kernel_folders_end_at_248_characters_start`；`tasks/reviews/R15.20-r2.md` PASS | — |
| R15.21 | 过审 | 法语 | `a_french_excel_csv_opens_in_its_code_page`；`french_translates_every_module_once_with_its_arguments`；`tasks/reviews/R15.21-r2.md` PASS | — |
| R15.22 | 过审 | 德语 | `a_german_excel_csv_opens_in_its_code_page`；`tasks/reviews/R15.22-r1.md` PASS | — |
| R15.23 | 过审 | 合并 PR #53 | `a_batch_over_the_limit_is_not_blamed_on_r`；`a_cell_set_waiting_for_the_worker_lands_after_the_batch`；`tasks/reviews/R15.23-r6.md` PASS | 真 R 的四例在 Linux 未执行（本机无 R；Mac、Windows 已跑） |
| R15.31 | 过审 | 极限测试 | `tests/perf/extreme.py`、`tests/perf/load.py`；`tasks/reviews/R15.31-r8.md` PASS | 打开期间键入未验证 |
| R15.32 | 过审 | review.sh 换回 herdr TUI | `tests/scripts/review-rules.sh`；`tests/scripts/review-rules.sh`；`tasks/reviews/R15.32-r4.md` PASS | — |
| R15.33 | 过审 | Windows 上配置目录路径 149 至约 162 字符时 worker 起不来 | `crates/ely-lok/src/client/tests/profile.rs`；`tasks/reviews/R15.33-r2.md` PASS | — |
| R15.34 | 过审 | ElyShow 同开 50、100 份演示文稿时前台的键 10 秒无反馈 | `a_hidden_view_drops_its_queued_screen`；`only_the_shown_deck_paints_its_pictures`；`tasks/reviews/R15.34-r1.md` PASS | 同时经命令行编辑全部文档时键 P95 50 份 1258、100 份 1438 毫秒，未达 PERF-02 |
| R15.35 | 过审 | 保存前台文档时，同一文档的键要等保存在 worker 里做完 | `a_key_never_waits_behind_a_point`；`an_autosave_keeps_the_round_going`；`tasks/reviews/R15.35-r2.md` PASS | 用户主动保存时，保存期间的键仍等那次保存（约 200 毫秒），转 R15.37 |
| R15.36 | 过审 | 别的进程经命令行同时编辑全部打开的文档时，前台的键仍慢 | `a_delete_stays_with_its_workbook`；`a_hidden_book_set_from_outside_reads_nothing_until_shown`；`tasks/reviews/R15.36-r3.md` PASS | 实测 ElyShow 编辑中 P95 179 毫秒，30 键里 3 键过 100 毫秒 |
| R15.37 | 过审 | 保存不再占住内核线程 | `a_failed_print_says_printing_failed`；`a_key_after_a_save_goes_first_and_into_the_file`；`tasks/reviews/R15.37-r4.md` PASS | macOS、Linux 与真人打字未实测 |
| R15.38 | 过审 | Ely 自己以 `io::Error::other` 写的错误文字 | `a_misspelled_word_offers_suggestions`；`a_rewrite_keeps_empty_marked_content_and_drops_its_own`；`tasks/reviews/R15.38-r6.md` PASS | — |
| R15.39 | 过审 | ElyTable 正在编辑单元格时保存 | `a_cancelled_composition_leaves_the_cell_and_other_edits`；`a_closing_save_enters_merged_hidden_and_filtered_cells_and_types_nothing_into_locked_ones`；`tests/scripts/windows-table-save-edit.py`；`tasks/reviews/R15.39-r5.md` PASS | — |
| R15.40 | 过审 | ElyDoc、ElyShow 换标签后键无处可去，要点一下页面才行 | `tests/scripts/windows-tab-keys.py`；`tasks/reviews/R15.40-r2.md` PASS | — |
| R15.41 | 过审 | ElyMark 自动保存在主线程写盘 | `a_waiting_disk_leaves_the_text_free`；`tasks/reviews/R15.41-r7.md` PASS | 保存之外的三个外部改动竞态未另立条目 |
| R15.42 | 过审 | 一次打开许多文档时主线程停数秒 | `each_worker_has_its_own_turn`；`queued_keys_share_one_hold_of_the_worker`；`tasks/reviews/R15.42-r5.md` PASS | — |
| R15.43 | 未完 | ElyTable 引用别的工作簿的公式只显示文件里的缓存值，源文件改了也不更新 | `a_dde_link_to_no_server_is_named_without_a_dialog`；`a_formula_whose_sheet_is_gone_keeps_its_value`；`tasks/reviews/R15.43-r6.md` FAIL | 缺口交 R20.11、R20.33；今余：存为 ODS 再开的外部三维区域显示 `#REF!`（R20.33） |
| R15.44 | 过审 | ElyDoc 的 INCLUDETEXT 域只显示缓存的文字，不随源文件，保存时写回旧文字 | `a_linked_section_edged_by_tables_saves_as_includetext`；`a_linked_section_saved_as_doc_keeps_its_paragraphs`；`tasks/reviews/R15.44-r6.md` PASS | Word 打开 Ely 存出的文件，macOS、Linux 未实测 |
| R15.45 | 过审 | OLE 链接存成静态图片，链接丢失，预览也不随源更新 | `a_word_link_kept_as_a_picture_saves_only_as_a_copy`；`tasks/reviews/R15.45-r3.md` PASS | 只在 Windows 实测；Word 对含 `#`、`%`、空格或非 ASCII 的目标怎么写未验证 |
| R15.46 | 过审 | ElyShow 超大演示 | `a_document_over_its_space_is_not_snapshotted_again`；`a_file_opened_over_its_space_takes_no_snapshot`；`tasks/reviews/R15.46-r2.md` PASS | macOS、Linux 未实测 |
| R15.47 | 过审 | ElyTable 改缩放时视图离开原位 | `tests/scripts/windows-table-zoom.py`；`tasks/reviews/R15.47-r3.md` PASS | — |
| R15.48 | 过审 | 应用日志的 `lok: edit to tile` 计时在恢复点同时落下时漏记已画出的键，读成约 9 秒的假延迟 | `a_deck_still_opening_does_not_stop_the_keys_being_timed`；`a_hidden_view_never_draws_its_edits`；`tasks/reviews/R15.48-r4.md` PASS | — |
| R15.49 | 过审 | ElyDoc 测试 the_page_and_style_groups_speak_chinese 在负载下偶发失败 | `the_page_and_style_groups_speak_chinese`（`apps/ely-doc/src/props/chinese.rs`）；`tasks/reviews/R15.49-r5.md` PASS | — |
| R15.50 | 过审 | ElyMark 外部改动的后台检查晚到 | `a_missing_target_is_held_across_case`；`a_variant_listed_as_its_own_file_is_no_answer`；`tasks/reviews/R15.50-r5.md` PASS | sshfs 这类挂载未实测 |
| R15.51 | 过审 | 保留我的在文件不在时直接覆盖 | `a_failed_keep_mine_leaves_no_empty_file`；`a_failed_keep_mine_never_removes_a_file_written_over_the_claim`；`tasks/reviews/R15.51-r3.md` PASS | — |
| R15.52 | 过审 | review.sh 改由另一个 Claude Code 审查 | `tests/scripts/review-rules.sh`；`tests/scripts/review-rules.sh`；`tasks/reviews/R15.52-r2.md` PASS | — |
| R15.53 | 过审 | review.sh 在审查者等自己的后台运行时报“没有结论” | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.53-r2.md` PASS | — |
| R15.54 | 过审 | macOS 上内核弹出的模态对话框卡死 worker | `a_kernel_dialog_is_closed_and_reported`；`a_kernel_dialog_reaches_apps_as_closed`；`tasks/reviews/R15.54-r1.md` PASS | — |
| R15.55 | 过审 | ElyTable 点“更新链接”读了源工作簿后，内核约 1 分钟内占着源文件，其间在 ElyTable 里存源 | `crates/ely-lok/src/client/tests/freed.rs`；`tests/perf/extreme.py`；`tasks/reviews/R15.55-r1.md` PASS | macOS、Linux 未实测；ElyDoc 的链接是否同样占着源未测 |
| R15.56 | 过审 | 停顿 2 秒后的第一键等恰好开始的恢复点 | `a_noted_wait_stops_a_snapshot_before_it_starts`；`a_point_owed_a_minute_no_longer_gives_way`；`tests/scripts/lo-patches.sh`；`tasks/reviews/R15.56-r3.md` PASS | 修后 ElyShow、ElyTable 真窗口与 macOS、Linux 未实测 |
| R15.58 | 过审 | `excel_caches_recompute_unless_the_function_is_unknown` 在 Windows 缺 `unzip` | `excel_caches_recompute_unless_the_function_is_unknown`；`tasks/reviews/R15.58-r1.md` PASS | — |
| R15.59 | 过审 | review.sh 把审查窗格都叠在调用者右侧的同一列 | `tests/scripts/review-rules.sh`；`tests/scripts/review-rules.sh`；`tasks/reviews/R15.59-r3.md` PASS | — |
| R15.60 | 过审 | 测试用的 LOK worker 在许多 worker 同时起时偶尔起不来或报错 | `a_dde_link_to_no_server_is_named_without_a_dialog`；`keys_never_wait_behind_the_event_watch`；`tasks/reviews/R15.60-r2.md` PASS | 根因未明，诊断就位 |
| R15.64 | 过审 | 内核补丁里的日志到不了 Ely | `crates/ely-lok/src/client/tests/logs.rs`；`crates/ely-ipc/src/relay.rs` 的单测；`tasks/reviews/R15.64-r2.md` PASS | — |
| R15.57 | 过审 | 一次打开 50 至 100 份文档期间，前台文档的键到图块仍要 1.9 至 2.5 秒 | `a_dde_link_to_no_server_is_named_without_a_dialog`；`excel_caches_recompute_unless_the_function_is_unknown`；`tasks/reviews/R15.57-r2.md` PASS | 真窗口负载下的 P95 未验证 |
| R15.65 | 过审 | R15.59 之后 review.sh 起审查仍偶发 `agent_prompt_stalled` | `tests/scripts/review-rules.sh`；`tasks/reviews/R15.65-r2.md` PASS | — |
| R15.66 | 过审 | 恢复点让路后余下的等待 | 由 R18.8 完成：`tasks/reviews/R18.8-r3.md` PASS | Windows 真窗口重跑三份切换、丢键是否仍在，未验证 |
| R15.67 | 过审 | 组件库与 ElyOffice 升到 `gpui-pre` =0.3.11 | 今钉 `gpui-pre` =0.3.12（`agents.md`）；`tasks/reviews/R15.67-r1.md` PASS | — |
| R15.68 | 过审 | 组件库 `PromptDialog` 只画标签、不给 `TextInput` 名字 | 组件库 3aab6d40 的单测；`tasks/reviews/R15.68-r1.md` PASS | 真窗口的 UI Automation 名字未验证（Windows 测试机无 ElyPDF 构建） |
| R15.70 | 过审 | Windows 上每个 LOK worker 都开着内核的 DDE 服务，答应任何服务名 | `a_dde_link_to_no_server_is_named_without_a_dialog`；由 R18.9 完成：`tasks/reviews/R18.9-r1.md` PASS | — |
| R15.24 | 本条 | 本表 | — | — |
| R15.61 | 过审 | Mac 的测试与发布脚本在 bash 5 下运行 | `tests/scripts/mac-bash-rules.sh`；`tasks/reviews/R15.61-r1.md` PASS | — |
| R15.62 | 过审 | Mac 上跨工作簿的外部引用间歇读成 `Err:522` | `a_read_while_the_source_imports_gets_its_value`；`a_source_a_book_read_is_free_to_replace`；`tasks/reviews/R15.62-r1.md` PASS | — |
| R15.63 | 过审 | 非 Windows 上指向不存在服务器的 DDE 链接，“更新链接”不点名 | `a_dde_link_to_no_server_is_named_without_a_dialog`；`a_writer_dde_field_to_no_server_is_named_and_keeps_its_text`；`tasks/reviews/R15.63-r2.md` PASS | — |
