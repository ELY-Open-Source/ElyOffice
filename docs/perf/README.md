# 性能记录

`tests/perf/bench.py` 在发行构建上测 PERF-01 至 PERF-08，每项默认 30 次，记 P50 与 P95，写成本目录下的 `<日期>-<机型>.md`。

样本由 `tests/perf/samples.py` 按固定种子生成，放在 `engines/bench`（不入库），记录里写每个样本的 SHA-256（仓库写 HEAD）：

| 样本 | 内容 | 用于 |
| --- | --- | --- |
| doc.docx | 100 页，20 张图片，约 9 MB | PERF-03 |
| sheet.xlsx | 10 万个数值，1 万个 `SUM` 公式 | PERF-04（载入即重算，重算计在打开之内） |
| pages.pdf | 1000 页，每页一张图，约 200 MB | PERF-05 |
| repo | 10 万个提交 | PERF-06 |
| notes.md | 1 MB | PERF-07 |

计时从启动进程起，到日志里出现标记为止（脚本每 10 毫秒读一次日志）：PERF-01 到首帧（`ELY_SMOKE=1` 打印 `ready`），首次运行（新的用户目录）与日常启动分记，每次等上一次的进程（含仍在启动的内核）退出后再测；Doc、Table、Show 另记空白文档就绪（`opened Untitled`），它要等内核启动；PERF-03、04 到第一块图块真正画到窗口上（`lok: first tile drawn`，在 `paint_image` 成功之后记；其间内核画图块失败或窗口画图块失败即整次作废），PERF-03 之前先核对样本经内核排版恰为 100 页（`lok: 100 pages`）；PERF-05 到第一页的第一块画到窗口上（`pdf: first tile drawn, page 0`；画失败即整次作废）；PERF-06 的历史首屏从按下“历史”到显示（`history shows`），查询从按下回车到结果（`searched true`），性能要求的指标是后者；PERF-07 到可视模式可以键入（`mark: shown as "visual"`：文档的块读完，按下即可编辑）。PERF-02 为 ElyDoc 每次按键到显示该编辑的图块（`lok: edit to tile`，每键须各有一条，缺即失败），并以 `ELY_STALLS=1` 记下主线程连续忙于 100 毫秒以上的次数（`main thread was busy`）。内存为应用及其全部子进程（内核 worker）的 RSS 之和。PERF-07 另有内存预算 400 MB（H-02，`bench.py` 的 `BUDGETS`），记录里逐条标出是否在预算内。PERF-08 为 ElyDoc 新建再关闭文档 40 次：循环前记下基线（总内存、全部句柄的类型与名称、内核中的文档数 `lok-worker: N documents open`），每次关闭后等内核回到基线文档数（等不到或没有这条日志即失败），再按身份比对句柄：比基线多出的只许是 `/System/Library/` 下的只读系统资源（第一次打开模板对话框时一次性载入两个系统字体与 AppKit 的两个本地化表），且此后每次都相同；多出任何别的句柄即失败。记录写基线与每次的内存、句柄数、文档数。

记录开头的 JSON 写机型、CPU、核数、内存、GPU、系统、系统字体文件数、各应用发行二进制的散列与样本散列。冷启动未清系统文件缓存（需要管理员权限），“日常启动”即文件缓存已热时的启动。

`tests/perf/folder-search.rs` 单独测 ElyMark 文件夹搜索的内存部分（不含读盘与界面），新旧实现成对计时并逐条核对结果；方法与结果见 `2026-10-23-folder-search.md`。
