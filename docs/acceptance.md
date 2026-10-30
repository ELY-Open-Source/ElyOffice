# 验收核对表

R14.49：2026-10-27 的审计（`OPTIMIZATION.md`（本机，未入库），五十项）逐条的结论，三个新命令行与 ElyMark 的跨段选取。2026-10-30 据提交 e960b24 写成；状态以 `tasks/TASKS.md` 为准，“过审”是末轮审查 PASS，“已修，末轮后未复审”是三轮用尽、末轮的缺陷已修并有回归测试。

## 五十项

| 编号 | 级 | 问题 | 结论与证据（任务、首个提交、末轮审查） |
| --- | --- | --- | --- |
| A-01 | P1 | PRD 与发布范围矛盾 | R14.1 过审（738dc9e，审查 r1 PASS） |
| A-02 | P0 | `[x]` 语义被违反 | R14.46 已修，末轮后未复审（18f5029，审查 r3 FAIL） |
| A-03 | P1 | 未通过复审即关闭 | 不修：每条至多三轮是用户定的规则；R14.46 把末轮 FAIL 的 `[x]` 如实改标 |
| A-04 | P0 | BUGS.md 状态陈旧 | R14.46 已修，末轮后未复审（18f5029，审查 r3 FAIL） |
| A-05 | P2 | 同一问题双重登记 | R14.46 已修，末轮后未复审（18f5029，审查 r3 FAIL） |
| A-06 | P0 | 验收证据不入库 | R14.47 过审（18f5029，审查 r3 PASS） |
| A-07 | P1 | 结论性文档过期 | R14.8 过审（c17f7df，审查 r3 PASS）；R14.47 过审（18f5029，审查 r3 PASS） |
| A-08 | P2 | 过程文档体积 | 不修：体积不是缺陷，审查按编号找条目 |
| A-09 | P2 | 历史悬空引用 | R14.46 已修，末轮后未复审（18f5029，审查 r3 FAIL） |
| B-01 | P0 | 没有合并前 CI | R14.44 已修，末轮后未复审（12a2166，审查 r3 FAIL）；main 的分支保护未设：会挡住每条任务后的直接推送，待用户定 |
| B-02 | P0 | 发布不以测试为门 | R14.43 过审（cfcd05e，审查 r2 PASS） |
| B-03 | P0 | 测试静默跳过 | R14.40 过审（c3a0625，审查 r1 PASS） |
| B-04 | P1 | ElyTree 测试依赖随包 Git | R14.28 过审（e16a6de，审查 r2 PASS） |
| B-05 | P2 | 权限测试不兼容 root | R14.12 过审（a2f09a8，审查 r1 PASS） |
| B-06 | P1 | 构建依赖未写全 | R14.41 已修，末轮后未复审（d950854，审查 r3 FAIL） |
| B-07 | P1 | 发布 runner 单点 | R14.43 过审（cfcd05e，审查 r2 PASS）；Linux 任务改上 GitHub 托管机器未做：每次要重建或缓存十余 GB 的内核 |
| B-08 | P2 | 行尾没有约束 | R14.33 过审（fee9074，审查 r3 PASS） |
| B-09 | P2 | 项目规则无自动检查 | R14.34 已修，末轮后未复审（0e883f5，审查 r3 FAIL） |
| C-01 | P1 | ElyShow 媒体 | R14.17 过审（8ea2251，审查 r2 PASS）；R14.18 过审（e6ee7a2，审查 r2 PASS） |
| C-02 | P1 | ElyPDF 证书信任 | R14.21 已修，末轮后未复审（de49701，审查 r3 FAIL） |
| C-03 | P1 | 自动更新安装 | R14.4 过审（2102edb，审查 r1 PASS） |
| C-04 | P2 | ElyTree 系统集成 | R14.27 已修，末轮后未复审（1c34485，审查 r3 FAIL） |
| C-05 | P1 | 验证覆盖不对称 | R14.36 过审（af23267，审查 r2 PASS） |
| C-06 | P2 | 会报错的入口仍可见 | R14.27 已修，末轮后未复审（1c34485，审查 r3 FAIL） |
| D-01 | P0 | 未与 Microsoft Office 交叉核对 | R14.36 过审（af23267，审查 r2 PASS）；与 Microsoft Office 的交叉核对未做：没有 Office |
| D-02 | P0 | 保存时静默丢数据 | R14.20 过审（88a7055，审查 r3 PASS）；R14.37 已修，末轮后未复审（2bb2a5a，审查 r3 FAIL） |
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
| F-01 | P0 | 更新包的真实性 | R14.3 过审（66351d4，审查 r2 PASS）；R14.49 过审（22efdb3，审查 r2 PASS）；Windows 安装程序的 Authenticode 签名未做：没有代码签名证书 |
| F-02 | P1 | ElyMark 被 AGPL 传染 | 不修：ElyMark 自身代码仍是 MIT，MuPDF worker 是单独进程，许可字段照实列出随包组件，源码随发行 |
| F-03 | P1 | unsafe 缺说明 | 不修：无 SAFETY 注释不是缺陷，健全与否要逐处审 |
| G-01 | P1 | panic 点 | R14.38 已修，末轮后未复审（71988f7，审查 r3 FAIL） |
| G-02 | P1 | 审计出的缺陷积压 | R14.49 过审（22efdb3，审查 r2 PASS） |
| H-01 | P1 | 性能基线过期 | R14.39 过审（c4a0cb8，审查 r3 PASS） |
| H-02 | P1 | ElyMark 内存 | R14.31 过审（ee77c0b，审查 r2 PASS） |
| H-03 | P2 | 性能项覆盖不全 | R14.39 过审（c4a0cb8，审查 r3 PASS） |
| H-04 | P2 | 120 fps 未实测 | R14.39 过审（c4a0cb8，审查 r3 PASS）；120 Hz 未实测：本机显示器最高 75 Hz |
| I-01 | P1 | 文档内容对读屏不可达 | R14.7 过审（2fce780，审查 r2 PASS）；R14.49 过审（22efdb3，审查 r2 PASS）；VoiceOver 与 AT-SPI 的文本读取未实测（只经 UI Automation）；整篇文档仍只到当前段落 |
| I-02 | P1 | 无障碍复核范围 | R14.8 过审（c17f7df，审查 r3 PASS）；R14.49 过审（22efdb3，审查 r2 PASS）；macOS 未按新改动复核，ElyMail 未纳入 macOS 复核（`docs/accessibility.md`） |
| I-03 | P2 | 国际化缺口 | R14.6 过审（a52d706，审查 r2 PASS） |


## 三个命令行

2026-10-30 在 Windows 上据提交 dfa1b8a 重跑，皆过：开着应用与一个文件，另一进程列出、读、替换（含中文）、保存，保存答复时读磁盘核对；缺的词、未开的文件与短命令被拒。

| 命令 | 任务 | 测试 | 结果 |
| --- | --- | --- | --- |
| `ely-doc text docs\|get\|replace\|save` | R14.15 过审 | `tests/scripts/doc-cli.sh` | 过 |
| `ely-show text docs\|get\|replace\|save` | R14.19 过审 | `tests/scripts/show-cli.sh` | 过 |
| `ely-mark text docs\|get\|replace\|save` | R14.30 过审 | `tests/scripts/mark-cli.sh` | 过 |

## ElyMark 的跨段选取

R14.29（已修，末轮后未复审）：可视模式拖动与 Shift 加上下越出所开的块，所跨的块合成一个原文文本框，复制、删除、键入随之，一步撤销，行内格式逐块施加。修在组件库；2026-10-30 在 pin 9d922a3 上重跑组件库 `ely-documents` 的 `document::tests::span`、`document::tests::format` 与 `blocks::tests::span`，49 个皆过；现 pin 01fe68d 推送前组件库全部库测试皆过。

## GitHub issue

| issue | 任务 | 结论 | 测试 |
| --- | --- | --- | --- |
| #49 腾讯文档导出的 docx 永远停在“正在打开” | R14.50 过审（r3 PASS） | 未复现：二十余个变体在 0.1.12 与今日的内核上 1 至 4 秒打开，根因要报告者的文件。已修“卡住无路可走”：载入过 30 秒，空页或标签之上都出横幅与“停止文档引擎”（写明会关闭全部文档、丢失恢复点之后的改动），停止后载入失败、接上“重新打开文档” | `ely-lok` 的 `client::tests::stop`；ElyDoc 的 `tests::slow`（真窗口里按下两个按钮） |
| #50 ElyTable 自动筛选加搜索 | R14.51 过审（r3 PASS） | 值列表上方加搜索框（有名），按显示的名字、完整大小写折叠（unicase 2.10）；搜索只收窄所见，未见项的勾选照旧，“全部”只作用于所见，无匹配时说明 | `data::dialogs` 单测；`tests::data::the_column_search_narrows_the_list_and_keeps_hidden_ticks`（真窗口里键入、点单项与“全部”） |

## 未完成

| 任务 | 状态 | 缘由 |
| --- | --- | --- |
| R14.48 23 MB 的大表格 DOCX 打不开 | 受阻 | 八种样本（`tests/perf/bigtable.py`）在三端 0.1 至 3.6 秒打开，复现不出；要用户的文件或换掉文字的副本 |
| R14.48c Windows 偶尔不画第一帧 | 未复现 | 连续启动 600 次与此后各次基准都没再出现 |
| D-01 与 Microsoft Office 交叉核对 | 未做 | 没有 Office |
| B-01 main 的分支保护 | 待用户定 | 要求 PR 检查会挡住每条任务后的直接推送 |
| F-01 Windows 安装程序的 Authenticode 签名 | 未做 | 没有代码签名证书 |
| I-01 VoiceOver 与 AT-SPI 的文本读取 | 未实测 | 只经 Windows 的 UI Automation 读到；整篇文档仍只到当前段落（LOK 只报当前段落） |
| I-02 macOS 的无障碍复核 | 未做 | macOS 只在 2026-10-08 复核六个应用，未按此后的改动重做，ElyMail 未在其中 |
| H-04 120 Hz 实测 | 未做 | 本机显示器最高 75 Hz |
| B-07 Linux 发布任务上 GitHub 托管机器 | 未做 | 每次要重建或缓存十余 GB 的内核；Linux 任务已不等 Windows |
