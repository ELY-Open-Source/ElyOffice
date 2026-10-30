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

`tests/perf/bench.py` 在 macOS、Windows 上直接跑，Linux 经 `tests/perf/linux.sh`（容器里的 Xvfb、私有 AT-SPI 总线）；各平台的进程与输入在 `drive.py`（macOS 经 AX 与 CGEvent，Windows 经 `drive.ps1` 的 UI Automation，Linux 经 `atspi.py` 与 XTest），Windows 上每次按键与点击前核对前台是本应用的窗口，不是即取回，取不回即失败、从不把键送进别的程序；应用窗口的输入法不是美式键盘即先切换并读回核对（拼音会把字母拿去组字），驱动的辅助进程不开控制台窗口。PERF-02 用另一份带 `ely-ui/frames`（gpui 的事件日志）的发行构建，其余各项用用户拿到的发行构建。应用日志的时间戳到毫秒。

PERF-01、03、04、05、07 从启动进程计时，到日志里出现标记为止（脚本每 10 毫秒读一次日志）：PERF-01 到首帧（`ELY_SMOKE=1` 打印 `ready`），首次运行（新的用户目录）与日常启动分记，每次等上一次的进程（含仍在启动的内核）退出后再测；Doc、Table、Show 另记空白文档就绪（`opened Untitled`），它要等内核启动；PERF-03、04 到第一块图块真正画到窗口上（`lok: first tile drawn`，在 `paint_image` 成功之后记；其间内核画图块失败或窗口画图块失败即整次作废），PERF-03 之前先核对样本经内核排版恰为 100 页（`lok: 100 pages`）；PERF-05 到第一页的第一块画到窗口上（`pdf: first tile drawn, page 0`）；PERF-07 到可视模式可以键入（`mark: shown as "visual"`）。内存为应用及其全部子进程（内核 worker）的 RSS 之和，任一进程读不到即整次作废。PERF-07 另有内存预算 400 MB（H-02，`BUDGETS`）。

PERF-02（`feedback.py`）七个应用各按 30 键，每键须各有一条反馈，缺即失败：Doc、Table、Show 为按键到显示该编辑的图块（`lok: edit to tile`；Show 先以 Tab、Enter 进入第一个文本框），PDF（1000 页样本上按 PageDown）、Mark、Tree、Mail 为按键到其后第一帧画出（`ui: key_down to frame`，取自 gpui 事件日志：按键派发之始到呈现之末）；并以 `ELY_STALLS=1` 记主线程连续忙于 100 毫秒以上的次数。

PERF-06（`search.py`）从驱动送出输入的时刻（驱动打印的墙上时间）计到应用日志里结果那一行的时间戳，驱动进程的启动与返回不计：ElyTree 在 10 万提交的仓库上按“历史”到首屏（`history shows`），按回车到查询结果（`searched true`）；ElyMail 在引擎写入的 10 万封已索引邮件上，于搜索框键入常见词 `report`，从最后一键到结果列表（`shows N messages, searched true`，含停手 200 毫秒的等待）。

PERF-08（`cycles.py`）七个应用各开关 150 次：Doc、Table、Show 以快捷键新建再关闭；PDF、Mark 由第二个进程把文件交给正在运行的那份再关闭；Tree 同样交一个仓库；Mail 新建一封再丢弃。先开关一次预热（第一个窗口或文档让系统一次性载入线程池、ETW 登记、ALPC 端口等，Windows 上 ElyMail 首个写信窗口即多出约 25 个，此后 40 轮持平），之后才取基线。每次关闭后等活的文档数回到基线（标签数 `tabs open`、内核 `lok-worker: N documents open`、`pdf-worker: … N documents open`、仓库数 `repositories open`、窗口数 `a window closed, N open`），等不到即失败。句柄：macOS、Linux 按身份比对，比基线多出的只许是系统的只读资源且此后每次都相同；Windows 只能计数，最后四分之一轮的最小值不得高于前一个四分之一轮的最大值（到末尾仍在增长即失败；开头的预热可以增长），也不得比基线多出 8 个以上（线程池的增减）。内存只看是否收敛（同上，且最后两个四分之一轮中位数之差每轮超过一页 4 KB 才算仍在增长），不设留存上限：glibc 每个线程的 malloc 区域留下已释放的内存备用，留多少随核数而定，不是泄漏（Linux 上 ElyPDF 约 40 轮内多出 55 MB 后持平，对比 /proc/pid/smaps 见增量全在约 23 个线程区域各 2 MB；ElyDoc 约 80 轮内由 387 MB 升到 396 MB 后持平，ElyMark 约 100 轮后持平，故用 150 轮）。记录写出留存的量。记录写基线与每次的内存、句柄数与活的文档数。

记录开头的 JSON 写机型、CPU、核数、内存、GPU、系统、系统字体文件数、各应用发行二进制的散列与样本散列。冷启动未清系统文件缓存（需要管理员权限），“日常启动”即文件缓存已热时的启动。

`tests/perf/folder-search.rs` 单独测 ElyMark 文件夹搜索的内存部分（不含读盘与界面），新旧实现成对计时并逐条核对结果；方法与结果见 `2026-10-23-folder-search.md`。
