# 缺陷清单

2026-10-09 对首版六个应用（ElyDoc、ElyTable、ElyShow、ElyPDF、ElyMark、ElyTree）与其共用 crate 的全量静态审查。逐行读了 `apps/` 六个应用、`crates/` 全部源码（含 C 垫片与 ElyMark 网页脚本）；Calc 与 Impress 的内核行为对照上游 LibreOffice `libreoffice-26-2` 分支源码核实（`sc/source/ui/view/*.cxx`、`sd/source/ui/view/*.cxx`、`desktop/source/lib/init.cxx`）。审查容器里没有引擎与组件库，没有运行与构建；凡标“已确认”都是沿调用链读到底的结论，“可能”是路径成立但依赖未能运行核实的条件。

编号前缀：T ElyTable，S ElyShow，P ElyPDF，D ElyDoc，M ElyMark，G ElyTree，K LOK 桥（`crates/ely-lok`），X 共用外壳（`ely-office`、`ely-files`、`ely-ui`、`ely-platform`、`ely-ipc`）。严重度：致命＝崩溃、丢数据或功能不可用；高＝常见路径结果错或功能坏；中＝边缘路径坏或界面误导；低＝小问题。


## 统计

| 节 | 致命 | 高 | 中 | 低 | 合计 |
| --- | --- | --- | --- | --- | --- |
| T ElyTable 与 LOK 画布 | 1 | 1 | 7 | 6 | 15 |
| S ElyShow | 2 | 0 | 2 | 6 | 10 |
| P ElyPDF | 1 | 3 | 4 | 6 | 14 |
| D ElyDoc | 0 | 2 | 2 | 4 | 8 |
| M ElyMark | 0 | 1 | 5 | 8 | 14 |
| G ElyTree | 0 | 0 | 3 | 11 | 14 |
| K LOK 桥 | 0 | 1 | 1 | 2 | 4 |
| X 共用外壳与平台 | 1 | 2 | 4 | 7 | 14 |
| 合计 | 5 | 10 | 28 | 50 | 93 |

致命：T-01 内核的图块与命中按像素网格，Ely 的光标、选区、表头按文档 twips，越远越错位；S-01 切换幻灯片后主画布不重画；S-03 ElyShow 的缩放一两帧后弹回“适合幻灯片”，缩放功能不可用；P-01 文件系统的 `PermissionDenied` 被拼成不存在的文案键，应用崩溃；X-01 设置窗口在前时 Cmd+Q 直接退出，跳过未保存询问并抹掉恢复点。

高：T-02 单元格光标露出边缘即把视图滚到中央，点击与拖选时视图跳动；D-01 导出或打印对话框开着时关闭其标签即崩溃；D-02 邮件合并以当前标签为模板，预览后生成的每份都是同一条记录；P-02 每次编辑落地清掉全部图块与缩略图，页面空白、刚画的墨迹消失；P-03 “编辑文字和图片”先命中图片对象再命中文字；一点抖动就移动了背景图；P-04 `TEXT_BLOCKS` 对每个文字块重跑一遍整页内容过滤，编辑工具长时间无反应；M-01 键入期间外部以相同内容重写文件，丢掉上次停顿之后键入的全部文字；K-01 非 macOS 的 worker 不运行 VCL 主循环，内核的回调与异步输入永远不送达；X-02 共用 `settings.json` 读不出时 Doc、Table、Show 根本起不来；X-03 第二个实例启动即清掉运行中实例的工作目录与恢复点。

## 用户报告的三个问题

### ElyTable：点一个单元格，光标跳到别处；拖不出多格选区

根因是 T-01：Ely 把 Calc 的“打印 twips”（文档坐标）与“图块 twips”（按内核像素网格对齐的坐标）当成同一套。内核画图块与命中测试都按它自己每行每列取整后的像素网格（`ScViewData::ToPixel` 截断），而单元格光标、选区、表头几何都以文档 twips 报回；越往右下，两套坐标相差越多，到第 300 行左右相差整整一行。点击落在内核的像素网格上（选中的是鼠标下画出的那格），但光标框与表头按文档 twips 画，出现在相邻的格上，看起来就是“跳到别处”。T-02 让部分可见的格一被点中就把视图滚半屏，拖选到边缘时内容在指针下移动，选区随之错位。T-03、T-04 是同一组问题的两个侧面。

### ElyShow：换幻灯片后主视图不变，要拉一下窗口才刷新

根因是 S-01：Impress 在 `SwitchPage` 期间压掉一切图块作废通知（`sd::Window::LogicInvalidate` 里 `IsInSwitchPage()` 直接返回），只发 `LOK_CALLBACK_SET_PART`；`LokView` 收到 `SET_PART` 只加计数，不作废图块，`Input::Part` 落地时也不作废。旧幻灯片的图块一直显示，直到窗口尺寸变化让适合缩放变了、全部图块退役重画。

### ElyPDF：编辑不工作、画不出

鼠标、命中与坐标换算本身是对的（`pages.rs` 按工具分派到 `press`/`drag_to`/`release` 或 `sketch_*`，`page_at`/`from_view` 与垫片 `placed()` 互逆）。失败来自状态与时序：P-02 每次编辑落地都清掉全部图块与缩略图，页面先变成空白纸再一块一块重画，刚画的墨迹要等所在图块重画后才出现，重的页面上像“什么也没画出来”；P-05 上一笔还在落地时结束的下一笔被拒并提示“文档已改动”，连续手绘会丢笔画；P-03 “编辑文字和图片”先命中图片再命中文字，整页背景图或扫描件上永远点不到文字，点击稍一抖动就成了移动背景图；P-04 该工具的层读取在每个文字块上重跑一遍整页内容过滤，密集页面要等很久，等待期间点击被静默忽略；P-06 字体默认“原字体”，嵌入 Type 1/Type 3 字体的文件一应用就报原始引擎错误。另有 P-01：写到无权限目录等任何 `PermissionDenied` 都被拼成不存在的文案键，直接崩溃。

## T ElyTable 与 LOK 画布（Calc）

### T-01 内核的图块与命中按像素网格，Ely 的光标、选区、表头按文档 twips，越远越错位
- 严重度：致命 ｜ 置信度：已确认（上游源码核实）
- 位置：`crates/ely-lok/src/view/scroll.rs:112`（`doc_point` 线性换算）、`crates/ely-lok/src/view/render.rs:275-330`（图块按 `TILE_TWIPS` 线性排布）、`apps/ely-table/src/grid.rs:96-121`（表头按 `place()` 线性换算）、`crates/ely-lok/src/view.rs:382-387`（单元格光标按打印 twips 直接用）
- 现象：滚到几百行之后点一格，名称框显示的地址是鼠标下画出的那格，但光标框画在相邻行列上，表头标号也对不上；拖选时选区高亮与格线错开；行高非默认（如 0.2 cm）的表十几行就错一行。
- 原因：上游 `ScGridWindow::PaintTile`（`sc/source/ui/view/gridwin4.cxx:1459-1600`）把图块起点换成像素（`fTilePosXPixel = nTilePosX * nOutputWidth / nTileWidth`），再用 `ScPositionHelper`/`ScBoundsProvider` 在**累加取整的像素**网格里找首行首列，以该列的像素起点为原点画；`ScModelObj::postMouseEvent`（`docuno.cxx:852`）把 twips 乘 `GetPPTX()` 成像素后 `GetPosFromPixel`（`viewdata.cxx:2858`）逐行 `ToPixel` 累加命中。默认行高 256 twips 取整成 17 像素（真值 17.07），每行差 0.067 像素，约 255 行差一行；默认列宽 1280 twips 取整成 85 像素，约 256 列差一列。而 `sc_print_twips_msgs` 下 CELL_CURSOR、TEXT_SELECTION、`SheetGeometryData` 都是文档 twips，上游专门有 `ScViewData::GetPrintTwipsPosFromTileTwips`（`viewdata.cxx:2637`）做两套坐标的换算，Collabora Online 的客户端也以 `SheetGeometry` 在两者之间换算；Ely 没有这一步。
- 复现：新建工作簿，键入 1 回车三百次（或打开任何几百行的 xlsx），滚到第 300 行，点 A300：光标框落在 A301 附近，左侧表头标的是 299；拖选 A300:C305，高亮与格线错开。
- 修法：以 `.uno:SheetGeometryData` 为准在客户端建像素网格（按内核同样的 `ToPixel` 截断累加），把鼠标与可见区从文档 twips 换成图块 twips 再发给内核，把 CELL_CURSOR、选区、引用标记从文档 twips 换回图块 twips 再画；并调用 `setClientZoom` 让内核的 PPTX 与 Ely 的图块比例一致（见 T-04）。

### T-02 单元格光标露出边缘即把视图滚到中央，点击与拖选时视图跳动
- 严重度：高 ｜ 置信度：已确认
- 位置：`crates/ely-lok/src/view/scroll.rs:45-64`（`show`），调用处 `crates/ely-lok/src/view.rs:372-387`
- 现象：点一个只露出一半的格（右缘或底缘），整张表横向跳半屏、纵向跳三分之二屏；拖选拖到边缘时内容在指针下滑走，选区落错位置。
- 原因：`show()` 对任何不完全可见的矩形都重置滚动：`scroll.0 = x - fx - vw / 2`、`scroll.1 = y - fy - vh / 3`，不是最小滚动量；而 `apply_batch` 对每次光标或单元格光标变化都调它。
- 复现：把窗口缩到只露出 A 到 H 列，点半露的 H 列；或从 A1 向右下拖选到视口边缘。
- 修法：`show()` 只滚到刚好露出目标（`scroll = x + w - vw` 或 `x`），与一般表格软件一致；拖选期间不因选区变化滚动。

### T-03 视口比工作表宽时网格先居中再左移一次
- 严重度：中 ｜ 置信度：已确认
- 位置：`crates/ely-lok/src/view/render.rs:243-253`（`spare` 居中）、`crates/ely-lok/src/view/scroll.rs:99-108`（`extent`）
- 现象：宽屏上打开工作簿，表格先出现在视口中央、两侧是空底色，内核按 `setClientVisibleArea` 扩大平铺区并报 `DOCUMENT_SIZE_CHANGED` 后整张表再跳到左边；窗口每次拉宽都重现一次。
- 原因：`prepare` 把窄文档居中（`slack(viewport.0, extent.0)`），这对 Writer 页面合理，对 Calc 的网格不合理：Calc 的文档尺寸只是已用区加余量（上游 `GetTiledRenderingArea`），随可见区增长。
- 修法：Calc 文档不居中（`spare = (0, 0)`），或以 `reach` 先把 `extent` 撑到至少一个视口宽。

### T-04 从不调用 `setClientZoom`，内核命中与几何按默认缩放算
- 严重度：中 ｜ 置信度：已确认
- 位置：`crates/ely-lok/src/ffi/document.rs`（无对应调用）、`crates/ely-lok/src/view/zoom.rs:44-62`（缩放只改本地 `geom`）
- 现象：Ely 缩放到 125% 或 200% 后，内核的 PPTX 仍是 100% 的 1/15，`getDocumentSize`、`GetPosFromPixel` 的像素取整都按 100% 网格算；与 T-01 叠加，缩放越大错位越明显（图块按 Ely 的比例画，命中按内核的比例算，两套像素网格的取整不同）。
- 原因：上游 `ScModelObj::setClientZoom`（`docuno.cxx:1100`）是客户端告知像素/twips 比例的唯一入口，Collabora Online 在载入与每次缩放时都调；Ely 的 `Document` 没有包装它。
- 修法：载入与每次 `zoom_at` 后发 `setClientZoom(tile_pixels, tile_pixels, TILE_TWIPS, TILE_TWIPS)`。

### T-05 文档尺寸按内核像素取整，表尾被裁
- 严重度：低 ｜ 置信度：已确认（上游源码核实）
- 位置：`crates/ely-lok/src/view.rs:360-368`（`doc_size` 直接取 `getDocumentSize`）
- 现象：`getDocumentSize`（上游 `docuno.cxx:740-780`）返回“像素累加后再除 PPTX”的 twips（85 像素×15＝1275 twips 一列，而不是 1280），平铺区越大短得越多；滚动到最右最下时最后一列、最后几行露不全。
- 修法：以几何（`SheetGeometryData`）算 Calc 的文档范围，不用 `getDocumentSize`。

### T-06 滚轮向下被内核平铺区卡住
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-lok/src/view/scroll.rs:20-28`（`clamp_scroll` 以 `extent` 为界）、`render.rs:264-268`（可见区在绘制时才发）
- 现象：快速向下滚动空白区时每次只能滚到当前文档底，内核收到可见区后再扩 25 行，下一次滚动才能继续；连续滚动一顿一顿。
- 修法：Calc 下 `extent` 至少放到 `scroll + 2 个视口`，或滚动时提前发可见区。

### T-07 表头点选、行高列宽拖动是“按编号的命令”，而每次点击与拖选都推进修订，紧接着的表头操作被当作过期拒绝
- 严重度：中 ｜ 置信度：已确认（时序窗口为一次后台读取）
- 位置：`crates/ely-lok/src/commands/sheet.rs:154-164`（`select.column`/`select.row` 的 `Col`/`Row` 在 `NUMBERED` 内，`commands.rs:63-72`）、`crates/ely-office/src/lok.rs:187-202`（`run_in` 对编号命令取 `read_at` 发 EXPECT）、`crates/ely-lok/src/worker/revision.rs:51-66`（鼠标按下、松开、按键拖动都 `bump`）、`apps/ely-table/src/grid.rs:204-211`、`:306-327`、`apps/ely-table/src/book.rs:199-246`（`reread` 之后 `read_at` 才更新）
- 现象：点一格（或拖选、键入）后马上点列头或行头、或拖完列边：命令带着上一次读取的修订发 EXPECT，内核修订已被鼠标事件推进，回 `stale:`，弹“文档在此间已改动，请重试”，整列没有选中。`read_at` 还没来得及由 `reread` 更新时（刚打开工作簿、worker 忙于画图块时）必现。
- 原因：列号、行号不会被光标移动改变，EXPECT 的保护在这里是多余的；而所有鼠标按下与松开（含拖选的每一步移动）都计入修订。
- 修法：`select.column`、`select.row`、`columns.width`、`rows.height` 不作编号命令（或 `run_in` 对它们不带 `at`），只保留对工作表、对象、参考线等真会被改动重排的编号做 EXPECT。

### T-08 自动筛选的列菜单对光标所在的表而不是按钮所在的表筛选、排序
- 严重度：中 ｜ 置信度：可能（内核侧按补丁追到；取决于单元格光标在哪）
- 位置：`apps/ely-table/src/data.rs:412-437`（`column_asked` 发 `data.filter`/`data.sort`）、`:379-409`（`ask_column`）；内核 `patches/libreoffice/0017-lok-calc-filter-validation-columns.patch` 的 `lcl_Query`（`rView.GetDBData()`）与 `SID_SORT` 路径（`GetDBData()`）
- 现象：单元格光标在筛选区之外（刚在别处键入）时点列头的筛选箭头、勾选值或“升序”：要么弹内核错误（`Check::Undoable` 失败），要么筛选、排序了光标所在的另一块数据。菜单本身能开，因为 `.uno:AutoFilterValues?col=&row=` 用的是按钮格。
- 原因：内核自己的菜单对按钮格 `GetDBAtCursor(nCol, nRow)` 操作，补丁后的 `LaunchAutoFilterMenu` 不移动光标；Ely 的答复却经 `GetDBData()`（当前光标或选区）解析。
- 修法：`column_asked` 先 `cell.goto` 到 `address(column, row)`（`Dialog::Column` 记下 `row`），或让补丁按按钮格取数据区。

### T-09 导入文本文件时整文件的 UTF-8 校验在界面线程跑
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-table/src/text.rs:241-245`（`in_window` 里 `Pending::new`）、`:58-63`、`:108-116`（`guess_encoding` 对全文 `from_utf8`）
- 现象：打开几百 MB 的 CSV，导入对话框出现前整窗冻结几百毫秒到数秒（树莓派更久）。
- 原因：读字节在后台，`Pending::new`（编码与分隔符猜测）却在主线程闭包里。
- 修法：猜测放进后台任务，`in_window` 只接现成的 `Pending`。

### T-10 CSV 导入对话框开着时打开任何别的对话框，待导入的文件静默消失
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-table/src/text.rs:254-262`（`next_import` 把 `Pending` 移进 `Dialog::Import`）；无条件 `self.dialog = Some(..)` 的：`apps/ely-table/src/output.rs:218`、`rules.rs:146`、`:278`、`:341`、`analysis.rs:123`、`:149`、`page.rs:146`、`protect.rs:61`、`formulas.rs:119`、`:209`、`data.rs:231`、`:317`、`:397`
- 现象：访达双击 .csv 出现导入对话框，按 Cmd+E 再取消：`Pending`（已检查字节的唯一副本）被丢，文件再也不打开，无提示。
- 修法：替换 `Dialog::Import` 时把它放回 `imports` 队头，或有对话框开着时拒绝再开。

### T-11 对话框比其标签活得久：导出、定义名称、重命名不可见却“开着”，挡住导入；名称表的“转到”激活已关闭的标签
- 严重度：低 ｜ 置信度：可能
- 位置：`apps/ely-table/src/dialog.rs:644-648`（导出 `sheet_in(*tab)?`）、`:563-571`、`:542-545`；`apps/ely-table/src/text.rs:255-257`（`next_import` 要 `dialog.is_none()`）；`apps/ely-table/src/formulas.rs:255-258`（`core.active = tab` 不核对）
- 现象：Cmd+E 后 Cmd+W 关掉该标签：对话框不画了但 `self.dialog` 仍为 `Some`，无法关闭，之后打开 .csv 没反应，直到再开关一次别的对话框；名称表的“转到”把 `core.active` 指到已关标签，正文显示“没有打开的文档”。ElyDoc 的 D-01 是同一类问题（那里直接崩溃）。
- 修法：`Book` 实现 `Host::tab_closed` 清掉指向该标签的 `dialog` 与 `choices`；“转到”先核对 `tab(tab).is_some()`。

### T-12 打印区域是整列或整行时页面设置无法应用
- 严重度：中 ｜ 置信度：可能（内核以 `$A:$D` 格式化整列区域）
- 位置：`apps/ely-table/src/page.rs:75-83`（`cell_ok` 要求字母加数字）、`:86-97`（`areas`）、`:169-176`（`page_asked` 以 `table.page.bad_reference` 拒绝）
- 现象：Excel 文件的打印区域 `=Sheet1!$A:$D` 预填进对话框，只改方向或缩放再按应用也被拒“Write ranges like A1:D20…”；不重写区域就用不了页面设置。
- 修法：`areas()` 接受 `COL:COL` 与 `ROW:ROW`（内核 `ParseAny` 本就接受），或内核给的原值免校验。

### T-13 打印失败报成“导出失败”（非 macOS 必现）
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-table/src/output.rs:147-150`（`print` 把 `print_later` 的失败交给 `output_failed`，文案 `table.export.failed`）、`crates/ely-platform/src/print.rs:87-89`
- 修法：单独的 `table.print.failed` 文案；平台未实现时禁用或说明。

### T-14 导入扫描把整个文件的每个字段都建成字符串
- 严重度：低 ｜ 置信度：已确认（影响随文件大小）
- 位置：`apps/ely-table/src/text.rs:78-106`（`scan`）、`:158-173`（`split` 返回全部行的 `Vec<Vec<String>>`）
- 现象：只需要 8 行预览与每列的类型标志，却为 200 MB 的 CSV 分配约 1–2 GB 堆，树莓派上可能耗尽内存。
- 修法：`split` 改为迭代器，只留前 `SHOWN` 行与列标志。

### T-15 非 UTF-8 文本的默认编码跟系统区域而不是界面语言
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-table/src/text.rs:236`（`i18n::resolve(&locale::system())`）
- 现象：英文系统上把应用语言设成简体中文，打开 GB18030 的 CSV 默认仍是 Windows-1252（反之亦然），与 agents.md 的“中文界面 GB18030、英文界面 Windows-1252”不符。
- 修法：用 `ely_ui::i18n` 解析后的当前语言。

## S ElyShow（Impress）

### S-01 切换幻灯片后主画布不重画
- 严重度：致命 ｜ 置信度：已确认（上游源码核实）
- 位置：`crates/ely-lok/src/events.rs:274`（`SET_PART` 只加 `cell.sheets`）、`crates/ely-lok/src/view/input.rs:231-235`（`show_part` 只排队 `Input::Part`）、`crates/ely-lok/src/view/lane.rs:95-124`（`Input` 落地不作废图块）、`apps/ely-show/src/slides.rs:244-247`、`apps/ely-show/src/deck.rs:308-314`（只在母版视图切换时 `invalidate`）
- 现象：在左侧缩略图条点另一张幻灯片（或经批注栏、演示检查跳转），缩略图高亮变了，主画布仍是上一张；拖一下窗口边才变。
- 原因：上游 `DrawViewShell::SwitchPage`（`sd/source/ui/view/drviews1.cxx:1003-1012`）只发 `LOK_CALLBACK_SET_PART`；切换期间 `sd::Window::LogicInvalidate`（`sd/source/ui/view/sdwindow.cxx:1016-1020`）因 `IsInSwitchPage()` 直接返回，不发 INVALIDATE_TILES。Collabora Online 的图块请求自带 `part`，切页即换缓存；Ely 的 `paint_tile` 不带 part，图块缓存按坐标键保留，所以旧页一直显示。窗口尺寸变化时 `fit_zoom` 改了缩放、`retire_tiles` 才重画。Calc 切表经 `SetTabNo` 走 `PaintGrid` 会发作废，所以 ElyTable 没有这个症状。
- 复现：新建演示，“幻灯片›新建”，点第一张缩略图。
- 修法：`LokView` 在 `Input::Part` 落地（或收到 `SET_PART`）时 `retire_tiles()` 并清 `failed`，或把 part 编进图块键。

### S-02 `deck.rs` 超过 500 行
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-show/src/deck.rs`（506 行）
- 原因：违反规则 5（文件不超过 500 行）；工作区内只有这一个文件超限。
- 修法：把 `reread` 的读取闭包或 `Extra` 的图片管理拆到独立模块。

### S-03 ElyShow 的缩放一两帧后弹回“适合幻灯片”，缩放功能不可用
- 严重度：致命 ｜ 置信度：已确认
- 位置：`apps/ely-show/src/deck.rs:139-150`（`Host::observed` 每次都 `view.fit_page(cx)`）、`crates/ely-office/src/open.rs:253`（每个 `Reported` 都调 `observed`）、`crates/ely-lok/src/view/lane.rs:121` 与 `view.rs:413`（任何输入落地、任何内核事件批次都发 `Reported`）、`crates/ely-lok/src/view.rs:262-268`（`fit_page` 置 `fit = true`）、`crates/ely-lok/src/view/zoom.rs:51`（`zoom_at` 置 `fit = false`）、`render.rs:256`、`:286-289`
- 现象：Cmd+=、Cmd+-、Cmd+滚轮、捏合、状态栏缩放控件、“视图”菜单：幻灯片放大一帧，随即跳回适合页面，状态栏百分比也跳回。agents.md 写的“手动缩放即停止适合”不成立。
- 原因：缩放后重绘改变了可见区，排队 `Input::Visible`，落地即 `Reported` → `observed` → 重新 `fit_page`；没有可见区变化时下一批内核事件也一样。
- 复现：打开任一演示，Cmd+=。
- 修法：只在标签首次观察时适合一次（`Extra` 里记标志），`observed` 不再调 `fit_page`；Cmd+0 已由 `Zooming::zoom(Reset)` 处理。

### S-04 选屏对话框被别的对话框顶掉后，`show_starting` 卡住、放映副本泄漏
- 严重度：中 ｜ 置信度：可能（多屏才会到选屏对话框）
- 位置：`apps/ely-show/src/slideshow/start.rs:180-183`（置 `show_starting`、开对话框）、`:289-310`（只有取消与开始两条路清理）、无条件覆盖 `self.dialog` 的：`apps/ely-show/src/new.rs:298`、`apps/ely-show/src/table.rs:24-28`、`apps/ely-show/src/remarks.rs:80`、`apps/ely-show/src/chart.rs:453-454`；守卫 `start.rs:123-125` 只记日志
- 现象：两块屏，F5 出选屏对话框时按 Cmd+N（或插入›表格、批注、图表）顶掉它；此后 F5、Shift+F5、放映菜单都没反应也无提示，`show-N.odp` 副本留在内核与磁盘里直到下次启动。同样的覆盖也会丢掉打了一半的批注或图表数据。
- 修法：任何 `ask_*` 替换对话框时（或 `watch_dialogs` 发现种类离开 `Dialog::Show`）跑与取消相同的清理；或有对话框开着时拒绝再开。

### S-05 一条批注的时间恰为午夜或只有日期，整个批注栏不显示
- 严重度：中 ｜ 置信度：可能（上游 `utl::toISO8601` 时分秒全零时不写时间部分；jiff 的 `civil::DateTime` 要求有时间）
- 位置：`apps/ely-show/src/remarks.rs:28-36`（逐条 `parse::<jiff::civil::DateTime>()?`）、`:168-176`（一条失败整栏换成错误文字）；内核 `sd/source/ui/unoidl/unomodel.cxx` 以 `utl::toISO8601` 写 `dateTime`
- 现象：含 00:00:00 批注（LibreOffice “去除个人信息”保存的文件、只写 `dc:date` 的工具）的演示，右栏只显示“a bad comment time 2025-03-01: …”，画布上的标记还在但栏内无法编辑删除。ElyDoc 的 D-04 是同一类问题。
- 修法：无时间部分时按 `civil::Date` 解析，单条失败只影响该条。

### S-06 导出或打印的“从–到”填 0 时导出全部幻灯片
- 严重度：低 ｜ 置信度：可能
- 位置：`apps/ely-show/src/output/dialog.rs:82`（`value.round() as u32`，无下限）、`apps/ely-show/src/output.rs:53-59`（`pages()` 只排序）、`crates/ely-lok/src/worker.rs:403-430`（`from == 0` 表示全部）、`apps/ely-show/src/output.rs:117-125`（`targets` 的 `number as usize - 1`，0 时溢出：发行构建回绕、调试构建 panic）
- 修法：`set` 里夹到 `1..=last`，`pages()` 再校验。

### S-07 母版视图下演示检查的行按幻灯片序号跳转
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-show/src/check.rs:184-194`（`show_part(part)` 用幻灯片序号）、`:112`（`check_show` 无母版视图守卫，`slides.rs:93-99` 有）
- 现象：显示母版时点“第 3 张”，内核对母版列表 `setPart(2)`，显示的是第 3 个母版或空白，对话框关闭。
- 修法：先退出母版视图，或母版视图下禁用行。

### S-08 刚编辑过幻灯片就进备注栏键入，备注可能丢
- 严重度：低 ｜ 置信度：可能
- 位置：`apps/ely-show/src/notes.rs`（`notes.at` 在聚焦期间冻结在上次读完的修订；`run_on(tab, &at, "notes.set", …)` 是编号命令，EXPECT 拒绝过期修订；强制重读后内核旧备注覆盖输入框，而 `shown` 已置为键入文字）
- 现象：编辑幻灯片后在后台重读完成前点进备注框键入、失焦：提示“文档在此间已改动，请重试”，键入的备注被旧备注换掉；再失焦不再发送（`text == shown`）。
- 修法：聚焦时取视图的 `read_at()` 更新 `notes.at`；发送失败时让 `shown` 保持内核文字以便下次失焦重发。

### S-09 改尺寸失败后 `size_asked` 一直留着
- 严重度：低 ｜ 置信度：可能
- 位置：`apps/ely-show/src/design.rs:62-64`（命令前置 `size_asked`）、`apps/ely-show/src/deck.rs:343-347`（只在读回 0.5 mm 内才清）；用它的 `deck.rs:71-74`、`apps/ely-show/src/insert.rs:144`、`apps/ely-show/src/media.rs:116`、`apps/ely-show/src/guides.rs:200`、`:318`
- 现象：`slide.size` 失败（worker 错误、无撤销步）后宽高字段显示内核没接受的尺寸，新参考线、媒体按它定位裁剪。`transition_asked` 有落地判定，尺寸没有。
- 修法：命令报错时清掉，或像切换那样按序号落定。

### S-10 开始放映的两处拒绝无提示（规则 2）
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-show/src/slideshow/start.rs:194-198`（标签或屏幕已不在 → 只记日志并 `discard`；选屏对话框开着时关掉标签再按开始可达）、`:123-125`（正在启动 → 只记日志）
- 修法：两处都弹提示。

## P ElyPDF（MuPDF）

### P-01 文件系统的 `PermissionDenied` 被拼成不存在的文案键，应用崩溃
- 严重度：致命 ｜ 置信度：已确认（文案层缺键即失败）
- 位置：`apps/ely-pdf/src/view.rs:123-126`（`restricted` 拼 `pdf.may.{needed}`）、`:228`（`fail` 对一切 `PermissionDenied` 都走 `restricted(&error.to_string())`）、`apps/ely-pdf/src/protect.rs:315`、`apps/ely-pdf/src/ask.rs:507-509`
- 现象：导出、提取、拆分、签名、保护、附件另存写到不可写文件夹，或任何 OS 权限错误，不显示错误而是崩溃。
- 原因：`restricted` 假定错误文字是 worker 的 `restricted:<名>`（`client.rs:297-302`），但 `ely_files::export`→`save_atomic` 保留 OS 的错误类别，`holder()`→`same_file`→`canonicalize` 同样，`annotate()` 里的 `user::name()` 读设置也可能是 `PermissionDenied`；得到的键 `pdf.may.Permission denied (os error 13)` 不在任何表里。
- 复现：文件›导出… 到只读文件夹。
- 修法：只在文字是 `may::NAMES` 之一时走 `restricted`，否则走 `pdf.failed`/`office.error.save`；或给 worker 的拒绝单独的错误类型。

### P-02 每次编辑落地清掉全部图块与缩略图，页面空白、刚画的墨迹消失
- 严重度：高 ｜ 置信度：已确认
- 位置：`apps/ely-pdf/src/edits.rs:102-111`（`refresh` 清 `tiles`、`thumbs`）、`apps/ely-pdf/src/pages.rs:272-279`（每帧只请求一块）
- 现象：任何编辑（一笔手绘、勾一个复选框、填一个字段）后可见页面变成空白纸，再以每帧一块 512 像素的速度填回；期间刚加的批注看不见；缩略图也全部重画。图多的页面上每次编辑要几秒，感觉“画了没反应”。
- 原因：LOK 画布作废时旧图继续显示到新图到达（agents.md 已定），PDF 视图没有照做：`retire_tiles()` 直接清空 `self.tiles`，`page_view` 画 `bg(paper)` 加空的 `tiles`。
- 修法：旧图块保留到替换到达时才退役（`paint()` 的 `tiles.insert` 处已这样做），缩略图同。

### P-03 “编辑文字和图片”先命中图片对象再命中文字；一点抖动就移动了背景图
- 严重度：高 ｜ 置信度：已确认
- 位置：`apps/ely-pdf/src/content.rs:172-187`（命中顺序）、`:206`（1 点阈值）
- 现象：整页背景图、扫描件、包裹整页的表单 XObject 上，点任何文字选中的都是图片（面板显示替换/删除）；按下与松开相差 1 个页面点（适合宽度下约 1 像素）即成“移动对象”编辑，把背景图挪走。图下的文字永远编辑不到。
- 原因：`layer.drawn.iter().rposition(|d| inside(..))` 先于文字块查找，不看目标大小；`release()` 的阈值是页面点不是窗口像素。
- 修法：对象整页或包住文字块时优先文字块；移动阈值按窗口像素（如 3 像素）。

### P-04 `TEXT_BLOCKS` 对每个文字块重跑一遍整页内容过滤，编辑工具长时间无反应
- 严重度：高 ｜ 置信度：可能（代价推断，未计时）
- 位置：`crates/ely-pdf-engine/shim/ely_text.c:227-245`、`:156-179`、`:133-145`（每块一次 `pdf_filter_page_contents`）；`apps/ely-pdf/src/content.rs:129-156`（逐页读）、`:169-171`（层未到即静默返回）
- 现象：密集页面上开编辑工具很久不出框，点击被忽略，图块与缩略图也停住（worker 单线程被占满）。
- 修法：每页一遍干跑记下全部字形原点与字符，再逐块比对；或只对点中的块判定可编辑。

### P-05 连续快画或快点被拒：“文档在此间已改动，未执行”
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-pdf/src/annotate.rs:189-203`（`settled_edit`）、`apps/ely-pdf/src/content.rs:209-214`、`apps/ely-pdf/src/writes.rs:176-190`、`apps/ely-pdf/src/edits.rs:56-93`
- 现象：上一笔还在落地（`annotate`＋重读页数、页框、书签）时结束的下一笔、下一个矩形、下一个复选框点击被丢弃，并把原因说成“文档改动”；多笔手绘丢笔。
- 原因：`settled()` 是队列为空且不在编辑中；按坐标的批注不会被在途编辑作废（按编号的才会，worker 已以 `stale:` 拒绝），却被一律拒绝而不是排队。
- 修法：按坐标的批注进编辑队列；拒绝只留给按编号的编辑，并改正文案。

### P-06 默认字体“原字体”在 Type 1/Type 3 字体的文件上失败，只给引擎原话
- 严重度：中 ｜ 置信度：可能（`pdf_font_writing_supported` 只认 TrueType/CFF/OpenType）
- 位置：`apps/ely-pdf/src/editing.rs:77`、`crates/ely-pdf-engine/shim/ely_write.c:61-67`、`apps/ely-pdf/src/writes.rs:115`、`:138-142`
- 现象：LaTeX 或旧 PDF 上“应用”报“previewing text failed: the document's font cannot write new text”，不提示换字体（缺字形的情形倒有 `pdf.text.missing` 提示）。
- 修法：像 `Wrote::Missing` 一样在面板里提示换字体，或读块时预判可写性、默认 Helvetica。

### P-07 含连字的文字块永远不可编辑
- 严重度：中 ｜ 置信度：可能
- 位置：`crates/ely-pdf-engine/shim/ely_text.c:91-109`（`block_targets` 按 stext 字符计数）、`:112-130`（`take_glyph` 按字形计数）、`:167`
- 现象：含 fi、fl、ffi 连字的段落不出编辑框、点击无效；真放过去 `ely_remove_block` 也会报“N of M glyphs were found”。
- 修法：按同一原点的字符分组计字形，或每字形按 `ucslen` 多计命中。

### P-08 页面读取失败后每帧重试、每帧弹提示
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-pdf/src/annotate.rs:76-110`（`ask_notes`）、`:113-134`、`apps/ely-pdf/src/content.rs:129-156`（`ask_layers`）、`apps/ely-pdf/src/attach.rs:184-205`、`apps/ely-pdf/src/sign.rs:49-75`
- 现象：某页的批注、层、附件、名称或签名读一次失败后，`fail` 弹提示、`notify` 重绘、重绘再问、再失败，没完没了；worker 死掉后每个可见页每帧一条错误提示，直到按“重新打开”。
- 原因：失败不记页（图块有 `failed`/`thumbs_failed`，这些没有）。
- 修法：各类读取加按页的失败集合（`refresh()` 清），引擎已结束时不再问。

### P-09 识别文字的写入被拒后仍弹“成功”
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-pdf/src/ocr.rs:510-513`（`toast(.., Success)` 在 `on_same` 之后无条件执行）
- 修法：成功提示移进 `on_same` 的闭包。

### P-10 预览回复被丢弃时“应用”静默无事
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-pdf/src/writes.rs:72-86`、`apps/ely-pdf/src/view.rs:241-247`（`read` 丢弃编辑前的回复而不调 `then`）
- 修法：改用带版本核对的 `off`，不符即提示 `pdf.object.changed`。

### P-11 撤销或重做在 MuPDF 侧失败后 worker 的步数不再同步
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-pdf-engine/src/bin/ely-pdf-worker/main.rs:62-86`、`:161-165`
- 修法：失败时 `doc.sync(&self.mupdf, None)`。

### P-12 `unhook` 中途抛出时泄漏保留的对象
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-pdf-engine/shim/ely_insert.c:17-49`、`:352-356`（`n` 只在返回时发布，`fz_always` 以 0 释放）
- 修法：以 `int *n` 传入并逐个递增。

### P-13 优化导出的“之前”大小取磁盘文件而非当前文档
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-pdf/src/output.rs:217-224`
- 修法：量 worker 的快照，或标签有改动时不写“之前”。

### P-14 文字块含控制字符时 JSON 回复不可解析
- 严重度：低 ｜ 置信度：可能
- 位置：`crates/ely-pdf-engine/shim/ely_text.c:240`（`%q` 不是 JSON 转义；别处用的是 `ely_json_string`）
- 现象：ToUnicode 坏掉的文件文字里有 U+0001–U+001F，`text_blocks` 解析失败，编辑工具永远读不到该页，并落入 P-08 的循环。
- 修法：`text`、`font` 用 `ely_json_string` 写。

## D ElyDoc（Writer）

### D-01 导出或打印对话框开着时关闭其标签即崩溃
- 严重度：高 ｜ 置信度：已确认
- 位置：`apps/ely-doc/src/actions.rs:84-86`（`self.tab(*tab).expect("a modal keeps its tab")`），对话框由 `apps/ely-doc/src/output.rs:38-48` 打开；关闭路径 `crates/ely-office/src/close.rs:38-42`、`:142-154`、`:191-221`
- 现象：开着“导出…”或“打印…”对话框按 Cmd+W、点窗口关闭钮或 Cmd+Q，标签被移除后下一帧渲染 `expect` 失败，进程崩溃；Cmd+Q 时先于其余脏标签的保存询问崩溃，这些标签只剩 10 秒一次的恢复点。
- 原因：`Dialog::Output` 记着标签号，`dialog_element` 每帧无条件解包；没有任何地方在标签关闭时清 `self.dialog`（`Docs` 未实现 `Host::tab_closed`），`release_tab` 还把 `output()` 排队的 `when_settled` 续动作丢掉。其他对话框（`Ask`、`Link`、`Reference`、外壳的 `Unsaved`）都是防御性写法。
- 复现：打开任一文档 → 文件›导出… → Cmd+W。
- 修法：`dialog_element` 在 `tab()` 为 `None` 时清掉对话框并返回 `None`；`Docs` 实现 `tab_closed` 丢掉指向该标签的 `Dialog::Output`。

### D-02 邮件合并以当前标签为模板，预览后生成的每份都是同一条记录
- 严重度：高 ｜ 置信度：已确认
- 位置：`apps/ely-doc/src/merge.rs:88-94`（`source()` 返回 `core.active`）、`:152-164`（预览）、`:178-195`（生成）；`apps/ely-doc/src/merge/run.rs:351-371`（预览经 `load_into` 开新标签）、`crates/ely-office/src/open.rs:231`（新标签成为当前）
- 现象：选 CSV、插字段、点“预览”（填好的副本开成新标签并成为当前标签，合并面板仍在右侧）再点“生成”：模板成了预览副本，字段早已替换成第 N 条记录，N 个输出文件内容相同，文件名还带预览标签名；再点预览是预览的预览。无任何提示。
- 原因：`Merge` 不记来源标签，每次点击时取 `core.active`；单测 `a_merge_keeps_its_source_when_the_tab_changes` 直接传 `source`，没覆盖界面路径。
- 复现：任一含 `«Name»` 的文档 → 邮件合并 → 三行 CSV → 预览 → 生成。
- 修法：选 CSV 或插字段时记下来源标签号（或在 `Extra` 标记预览标签），面板显示来源名，预览与生成都用它，当前标签是预览副本时拒绝。

### D-03 界面线程同步持 worker 锁读内核（样式、字体、书签、公式、分节名）
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-doc/src/insert.rs:318-324`（`insert_section`）、`apps/ely-doc/src/refs.rs:81-86`（`read`，书签、交叉引用、公式对话框用）、`apps/ely-doc/src/props.rs:116-122`（`read_fonts`）、`:133-137`（`read_styles`，每次切标签与 `Reported` 都跑）
- 现象：保存大文档、邮件合并几百条记录（每条载入、替换、另存、关闭都持锁）期间，切标签或打开“插入›书签…”整窗冻结到锁空出来为止。
- 原因：违反 2026-10-05 决策“界面线程不碰 worker”，这四处直接 `worker.lock()` 而不走 `read_in`/`read_off`。
- 修法：改为后台读取，回来时核对仍是同一标签与视图。

### D-04 一条批注的时间读不出，全部批注与修订都不显示
- 严重度：中 ｜ 置信度：可能（内核对无日期批注写 `0-00-00`，`utl::toISO8601` 的空日期即如此）
- 位置：`apps/ely-doc/src/review.rs:164-167`、`:373-393`（`threads` 遇第一个解析失败即整体 `Err`）、`:179-181`
- 现象：打开含无日期批注的 DOCX（`w:comment` 无 `w:date`），批注与修订两栏都空，并弹引擎错误提示；每次审阅事件重读再失败再弹。
- 原因：单条解析失败被提升为整表失败，`and_then` 连已解析的 `changes` 一起丢掉。
- 修法：逐条解析时间，失败的那条用占位时间；`changes` 不因批注失败而丢。

### D-05 邮件合并某条记录失败时内核文档泄漏
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-doc/src/merge/run.rs:464-470`
- 原因：`fill`/`save_as` 以 `?` 提前返回，`close_doc` 不执行，`Doc` 无 `Drop`；每次重试多漏一份，worker 的 `N documents open` 计数漂移。
- 修法：先取结果再一律 `close_doc`，最后传出首个错误。

### D-06 导航栏大纲点击按当前列表取下标，后台重读后可能越界
- 严重度：低 ｜ 置信度：可能
- 位置：`apps/ely-doc/src/navigate.rs:277-283`（`navigator.outline[at]`）
- 修法：用 `get(at)`，或在渲染时把目标矩形放进行的闭包。

### D-07 导航栏隐藏时仍随每次光标移动重读大纲与书签
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-doc/src/chrome.rs:236-245`（`left_view` 不看 `navigating` 就 `navigator.show(tab)`）、`apps/ely-doc/src/navigate.rs:79`、`:91-123`
- 现象：关掉导航栏后每次按键仍排两次内核读取，与键入争 worker 锁（规则 9：空闲不做事）。
- 修法：隐藏时 `show(None)`，显示时重读一次。

### D-08 Linux、Windows 上“打印…”可用，导出 PDF 后才报“此平台未构建”
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-doc/src/output.rs:28-30`、`:80-88`；`crates/ely-platform/src/print.rs:87-89`
- 现象：菜单与页码范围对话框都正常，最后一步弹出带临时路径的错误提示。各文档应用同（见 X-13）。
- 修法：平台未实现时禁用菜单项或在对话框里写明；错误文字去掉路径。

## M ElyMark（CEF）

### M-01 键入期间外部以相同内容重写文件，丢掉上次停顿之后键入的全部文字
- 严重度：高 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/outside.rs:223-224`（`(true, _) | (false, false) => take_disk`）、`:243-268`（`take_disk`）、`apps/ely-mark/src/view.rs:184-196`（`refresh` 把 `changed`、`reported` 归零）、`apps/ely-mark/web/src/main.js:52-59`（全文停手 300 毫秒才送）、`:179-187`（`refresh` 清掉待送）
- 现象：连续键入时外部进程以相同字节重写文件（`touch`、同内容的 `git checkout`/`stash pop`、云同步重盖、格式化器输出相同），编辑器以旧文字重建，上次停顿以来键入的字全部消失，标签还显示已保存；无提示。
- 原因：页面只在停手 300 毫秒后送全文，键入中 Rust 侧 `doc.current` 是停顿前的文字；`disk.text == current` 的分支不看 `dirty`（它含 `edit_pending`），直接 `take_disk`。与 agents.md “有未保存内容（含页面还没送回的编辑）即不动磁盘与编辑器”相悖。
- 复现：打开 `a.md` 连续键入几秒，期间在终端 `touch a.md`。
- 修法：`disk.text == current` 而视图未落定时不 `take_disk`：等 `when_settled` 后重判，或当作 `Outside::Changed`；落定且相同时只换指纹不动编辑器。

### M-02 `refresh` 归零版本计数却不换代，在途的一键让标签一直“未落定”
- 严重度：中 ｜ 置信度：可能
- 位置：`apps/ely-mark/src/view.rs:193`、`:264-272`（`Loaded` 路径有换代）、`apps/ely-mark/web/src/main.js:179-187`、`crates/ely-web-content/src/engine.rs:127-143`；受影响 `crates/ely-office/src/close.rs:96-99`、`save.rs:325-326`、`apps/ely-mark/src/compare.rs:381-382`
- 现象：干净标签被外部改动刷新后，Cmd+S、Cmd+W、Cmd+Q、比较与自动保存都静静等 `when_settled`，直到再键入一个字才一起执行。
- 原因：刷新前排队的一键在渲染进程里先产生 `changed = N+1`，再执行 `refresh` 丢掉待送全文；Rust 已归零又收到 `changed`，没有 `text` 跟上，`settled()` 为假；`refresh` 不像 `Loaded` 那样换代，旧回报没被丢。
- 修法：`refresh` 自己换一代（`self.load += 1` 传给 `ely.refresh(text, load)`）。

### M-03 干净标签接过磁盘文字后关闭仍问“是否保存”
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/outside.rs:243-268`（`take_disk` 不更新 `tab.mark`）、`apps/ely-mark/src/view.rs:192`、`apps/ely-mark/src/engine.rs:133-136`、`crates/ely-office/src/close.rs:107-128`
- 现象：文件在别处被改、ElyMark 静默重载后，Cmd+W 对没动过的标签弹未保存询问；比较对话框选“用磁盘上的”后同。
- 原因：`request_close` 比较 `Engine::history`（磁盘文字散列）与 `tab.mark`（载入或上次保存时的散列），不等即问。
- 修法：`take_disk`（与合并采用后）同步 `tab.mark = mark(&disk.text)`。

### M-04 第二次打印或导出 PDF 悄悄取消第一次，没有文件也没有错误
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/export.rs:93-97`、`:276`（打印与导出共用 `print-{id}.html`）、`:315-334`（`self.printing = Some(..)` 覆盖）
- 原因：`Desk.printing` 只有一个槽，赋新值即丢掉上一个的隐藏 `WebView` 与订阅，第一份的 `Event::Printed` 无人接。
- 修法：按作业编号保存多份，或一份未完时拒绝第二份并提示。

### M-05 点页面里的链接什么也不发生
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/view.rs:274`（`Event::Leave(url) => log::info!(..)`）、`crates/ely-web-content/src/fence.rs:319-324`
- 现象：预览与可视模式里点 `http(s)`/`mailto` 链接：不跳转、不开浏览器、不提示。PRD 允许用户主动打开外部链接，agents.md 说“交给应用”，应用没处理。
- 修法：`Leave` 时 `http`、`https`、`mailto` 交系统打开，其他协议提示拒绝（同 ElyPDF 的链接策略）。

### M-06 `on_app_quit` 无条件删恢复点与崩溃锁，AppKit 直接终止时丢未保存内容
- 严重度：中 ｜ 置信度：可能（依赖 GPUI 对非 Quit 动作的终止也跑 `on_app_quit`）
- 位置：`apps/ely-mark/src/desk.rs:244-248`、`apps/ely-mark/src/restore.rs:145-155`（`Keeping::close`）
- 现象：注销、关机或 Dock 的“退出”经 `terminate:` 结束进程，`proceed(After::Quit)` 的未保存流程不跑，退出观察者却删掉 `session.lock` 与 `recovery/`；下次启动读到“干净结束”，什么也不回来。外壳的 `Recovering::end` 有同样假设（见 X-01）。
- 修法：退出观察者只在无标签（或全部干净）时清理，否则留下锁与恢复点。

### M-07 崩溃恢复的未命名副本按“第一个显示的未命名标签”配对，内容可能落错标签
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/restore.rs:195-201`、`apps/ely-mark/src/desk.rs:226-228`
- 修法：`Waiting` 条目记下为它新建的标签号，`restore_shown` 按号配对。

### M-08 页面重载后悬着的渲染回调永不完成，导出既不成功也不报错
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/view.rs:264-272`（`Loaded` 不清 `renders`）、`:210-220`、`apps/ely-mark/src/export.rs:165-185`
- 修法：`Loaded`、`Failed` 时把 `renders` 全部以错误结束。

### M-09 导出的保存面板出错被当作取消
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/export.rs:129-131`（`let Ok(Ok(Some(path))) = chosen.await else { return log::info!(..) }`）
- 修法：照 `Desk::open_picked` 分三种情形，错误走 `panel_failed`。

### M-10 关闭查找框只清当前标签的高亮
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/finding.rs:116-124`（`close_find` 只对 `active_view` 发 `unfind`）、`:60-69`、`apps/ely-mark/web/src/main.js:132`、`:270-273`
- 修法：对每个标签的视图发 `unfind`。

### M-11 页面把搜索词写进应用日志
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-mark/web/src/main.js:246`（`console.warn` 带 `place.query`）、`crates/ely-web-content/src/handlers.rs:272-275`（控制台警告进 `log::warn!`）
- 原因：违反规则 4（日志不含正文）。
- 修法：只记行号与是否标题。

### M-12 浏览器创建完成前 `WebView` 已释放，浏览器泄漏
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-web-content/src/view.rs:582-587`（`Drop` 时还没有浏览器）、`crates/ely-web-content/src/handlers.rs:349-354`（`on_after_created` 无条件存入）、`crates/ely-web-content/src/engine.rs:165-170`
- 现象：开了标签在约 100 毫秒内关掉，渲染进程留到退出。
- 修法：`on_after_created` 发现 `surface.closed` 即 `close_browser(true)`。

### M-13 编辑器构建失败时页面空白且不上报
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-mark/web/src/main.js:117-139`（`show` 无 `try/catch`）、`apps/ely-mark/web/src/visual.js:106-109`
- 现象：Milkdown `create()` 或 `newTable` 抛出时没有 `shown` 也没有 `Failed`，`#document` 空着，`MarkView.shown` 永为假（跳转、外观等一直等）。
- 修法：`build` 包 `try/catch`，可视失败退回源码并 `tell({kind:'failed'})`。

### M-14 `render_pdf` 在界面线程内嵌图片并写文件
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-mark/src/export.rs:276-282`（渲染回调里 `inline`、`read_to_string`、`fs::write`）；对照 `:172-176` 的 `write_output` 已在后台
- 修法：暂存步骤移到 `background_spawn`。

## G ElyTree（Git）

### G-01 打开“文件历史”或“逐行追溯”后，中间区不再响应任何别的选择
- 严重度：中 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/view.rs:185-187`（`body` 先于 `match self.side` 返回 `look_pane`）；入口 `apps/ely-tree/src/hist_pane.rs:43-66`、`apps/ely-tree/src/pane.rs:483`
- 现象：按下差异标题栏的“文件历史”或“逐行追溯”后，点改动列表里另一个文件、历史里另一个提交、分支或储藏，中间区都不变；只有历史面板的小叉能退出。
- 原因：`show()`（`pick.rs:302`）、`pick_commit`、`pick_place`、`set_side` 都不清 `self.look`，只有 `close_look` 与 blame 的 `show_commit` 清。
- 修法：任何新选择都清 `self.look`，或只在 `side == Changes` 且所选文件不变时画它。

### G-02 进行中操作读取失败被吞掉；`git am` 冲突无提示条无错误
- 严重度：中 ｜ 置信度：已确认（`git am` 细节为可能）
- 位置：`apps/ely-tree/src/ops.rs:81-98`（`doing`）、`apps/ely-tree/src/moves.rs:74`、`apps/ely-tree/src/conflict_pane.rs:62`、`:106-109`
- 现象：`doing()` 出错时 `opened.doing = Some(Err(..))`，没有任何显示或日志；冲突文件列出来了，却没有“继续/跳过/中止”提示，冲突栏两侧退回泛称。
- 原因：`rebase-apply` 一律当变基读 `onto`，`git am` 的 `rebase-apply/` 没有 `onto`，整个读取 `NotFound`；`describe()` 失败（如 `MERGE_HEAD` 指向已被清理的对象）同样整体失败。违反规则 2。
- 复现：终端里 `git am` 停在冲突，在 ElyTree 打开该仓库。
- 修法：`Some(Err)` 以提示条显示 Git 原话并记日志；无 `onto` 的 `rebase-apply` 视为 `git am` 进行中。

### G-03 新建的空仓库在“仓库”页显示两段 Git 致命错误（引用日志、LFS）
- 严重度：中 ｜ 置信度：可能
- 位置：`apps/ely-tree/src/recover.rs:50-69`（`reflog … HEAD --`）、`apps/ely-tree/src/structure.rs:137-138`（`git lfs ls-files`）、`apps/ely-tree/src/places.rs:40-50`、`apps/ely-tree/src/side_places.rs:261-264`、`:279-281`
- 现象：“新建仓库…”后仓库页出现 “fatal: bad revision 'HEAD'” 与 “Git can't resolve ref: HEAD” 两块红字；仓库没有问题，空列表才是正确状态。
- 原因：HEAD 未诞生时显式传 `HEAD` 的 `reflog show` 以 128 退出，`run_or_one` 不接受；`git lfs ls-files` 同样先解析 HEAD。
- 修法：`status.head.oid` 为空时两项直接给空列表。

### G-04 停止取回/拉取/推送失败的提示写成“克隆没有停下”
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/network.rs:126`（`stop_remote` 复用 `tree.error.stop`）、`crates/ely-ui/src/i18n/tree.rs:53`
- 修法：加一条不限动词的文案。

### G-05 合并结果保存失败报成“Git 失败”，`tree.error.save` 是死文案
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/work.rs:424`（`(_, Err(error)) => failed("tree.error.write", …)`）、`crates/ely-ui/src/i18n/tree_ops.rs:127`、`:255`（定义却无人用，规则 6）
- 修法：`written()` 加 `(Write::Save, Err)` 分支用 `tree.error.save`。

### G-06 首次提交后紧接着取消暂存，按过期的“已有提交”判定跑成 `git rm --cached`
- 严重度：低 ｜ 置信度：可能（竞争窗口为一次状态重读）
- 位置：`apps/ely-tree/src/work.rs:352`（`born` 取自缓存的 `opened.status`）、`:426-427`（`read` 异步、`next_write` 同步）、`:85-93`、`:103-112`
- 现象：新仓库里暂存、提交，列表刷新前按“全部取消暂存”：第二条写操作仍按 `oid: None` 跑 `rm --cached -f -r -q -- .`，把刚提交的全部文件标为删除（可恢复，但吓人）。
- 修法：写操作执行前在后台 `rev-parse --verify -q HEAD` 判定，不用缓存状态。

### G-07 任一引用没有创建日期，整个分支页读不出
- 严重度：低 ｜ 置信度：可能
- 位置：`apps/ely-tree/src/branches.rs:33`（`%(creatordate:unix)`）、`:106`（空串解析失败整体 `Err`）、`apps/ely-tree/src/side_refs.rs:227-235`
- 复现：`git tag blobtag HEAD:README.md`，打开分支页。
- 修法：日期可缺（显示“—”），或跳过非提交对象的引用。

### G-08 清理预览后文件被外部删除，对话框只给裸 OS 错误
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/work.rs:270`（`symlink_metadata(&at)?`）、`:415-422`
- 现象：标题“Git did not do it”，正文 “No such file or directory (os error 2)”，没有路径，Git 并未运行。
- 修法：映射成带路径的“自预览后已变”文案。

### G-09 后台读取回来的对话框覆盖用户同时打开的对话框
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/network.rs:186`（推送选远程）、`:237`（强制推送）、`apps/ely-tree/src/recovering.rs:116`（移动分支）
- 现象：按“推送”（无上游）后立刻按“储藏改动…”并开始键入，`git remote` 回来时储藏对话框连同输入被换掉。
- 修法：只在 `tree.ask` 仍为空（或仍是本次请求）时赋值。

### G-10 选中的分支、标签、位置是窗口级的，跨仓库标签串台
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/tree.rs:107-108`（`picked_ref`、`picked_place` 在 `Tree` 上）、`apps/ely-tree/src/refs_pane.rs:25-33`、`apps/ely-tree/src/side_refs.rs:238-245`
- 现象：仓库 A 选中 `main`，切到仓库 B，B 的分支页显示它自己的 `main` 为已选并带动作按钮。
- 修法：移入 `Opened`，或 `activate()` 时清掉。

### G-11 已选边的冲突块再选另一边，提示“结果已手改”
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/conflict.rs:219-224`
- 修法：给“该块已选定”单独文案，或支持改选。

### G-12 符号链接父目录下，当前打开的工作树被当成别的工作树而可“移除”
- 严重度：低 ｜ 置信度：可能
- 位置：`apps/ely-tree/src/places_pane.rs:253-255`、`:265-271`、`apps/ely-tree/src/structure.rs:245-247`
- 原因：`here` 用 `git worktree list` 记录的原始路径比对已规范化的 `opened.repo.root`（macOS `/tmp` → `/private/tmp`）。
- 修法：比对前 `canonicalize`。

### G-13 非 macOS 平台的死入口
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/running.rs:167-173`（停止远程命令非 Unix 报未构建）、`apps/ely-tree/src/tools.rs:109-115`（“在终端中打开”非 macOS 报未构建）、`crates/ely-ui/src/i18n/tree_places.rs:147`、`:281`（“在访达中显示”文案在 Linux、Windows 照样显示）
- 修法：按 `cfg` 禁用或隐藏，`reveal` 换平台中性文案。

### G-14 `repo.rs` 的孤立文档注释
- 严重度：低 ｜ 置信度：已确认
- 位置：`apps/ely-tree/src/repo.rs:10`（“How many repositories the recent list keeps.” 的常量早已移到 `ely_files::recent`，注释挂到了 `struct Repo` 上）
- 修法：删掉。

## K LOK 桥（`crates/ely-lok` 的 worker、客户端、事件）

T-01、T-04、T-07 与 S-01 的根源都在这一层，这里只列此外的问题。

### K-01 非 macOS 的 worker 不运行 VCL 主循环，内核的回调与异步输入永远不送达
- 严重度：高 ｜ 置信度：可能（代码已读；Linux、Windows 未运行）
- 位置：`crates/ely-lok/src/worker.rs:149-164`（非 macOS 的 `serve` 只是阻塞的 `read_frame` 循环）、`patches/libreoffice/0001-lok-macos-tile-painting.patch`（`CallbackFlushHandler::scheduleFlush` 非 MACOSX 分支仍 `Application::PostUserEvent`）
- 现象：Linux、Windows 上 worker 能答同步请求（载入、画图块、保存），但作废、光标、状态、对话框关闭等回调一条也推不出来，键盘与鼠标（内核经 `SfxLokHelper::post*EventAsync` 以用户事件派发）也到不了文档——编辑器是一张静止的图。树莓派是明确目标。
- 原因：用户事件只在 `Application::Yield`/`lo_runLoop` 里派发；只有 macOS 路径以 GCD 加 `run_loop` 跑循环。
- 修法：各平台都以 unipoll 的 `runLoop` 加轮询回调驱动请求队列，照 macOS 的做法。

### K-02 图块的代际在画完之后才取，画到一半的作废被盖掉
- 严重度：中 ｜ 置信度：可能
- 位置：`crates/ely-lok/src/worker/input.rs:76-81`（`painted`：先 `paint`，再 `events::generation(id)`）、`crates/ely-lok/src/view.rs:466`（`tile.generation >= self.generation` 即接受）
- 现象：`paintTile` 期间内核发出作废（Writer 画时懒排版、Impress 空闲任务），返回的像素是作废前的，却带着新代际；视图接受并不再重画，旧内容留到下一次无关的作废。
- 修法：`paint()` 之前先读代际并盖到图块上。

### K-03 `rect_of` 宽容解析把读不出的载荷悄悄吞掉
- 严重度：低 ｜ 置信度：可能
- 位置：`crates/ely-lok/src/events/parse.rs:25-34`，用于 `events.rs:212`、`:217-218`、`:221`、`:248`
- 现象：不是 “x, y, w, h…” 的载荷（如 Impress/Writer 表格多选时 GRAPHIC_SELECTION 的 JSON）要么被静默丢掉（选区框与手柄消失、不报 fault），要么凑出四个数字就被误读。与规则 2 相悖。
- 修法：严格解析开头的数字字段，读不出像 `found_of` 一样进 `faults`。

### K-04 `Sheets.current` 用 `as u64` 转有符号的 part
- 严重度：低 ｜ 置信度：可能
- 位置：`crates/ely-lok/src/worker.rs:377`（`doc.part() as u64`、`count as u64`）、`crates/ely-lok/src/client/doc.rs:40`
- 现象：`getPart()` 返回 -1（没有视图外壳，关闭或重排期间）时 `current` 成了 2^64-1；今天的应用用 `.get` 不会 panic，但列表与标签页会错。
- 修法：`try_from`，为负即报错。

## X 共用外壳与平台（ely-office、ely-files、ely-ui、ely-platform、ely-ipc）

### X-01 设置窗口在前时 Cmd+Q 直接退出，跳过未保存询问并抹掉恢复点
- 严重度：致命 ｜ 置信度：已确认
- 位置：`crates/ely-ui/src/run.rs:77`（应用级 `on_action(Quit → cx.quit())`）、`crates/ely-ui/src/commands.rs:170`（`secondary-q` 不带键位上下文）、`:150`（菜单项）、`crates/ely-ui/src/settings/window.rs`（设置窗口根上没有 `Quit` 处理）、`crates/ely-office/src/recover.rs:59-71`、`crates/ely-files/src/crash.rs:29-46`
- 现象：文档应用开着设置窗口（Cmd+,）时按 Cmd+Q 或选应用菜单“退出”，进程立即结束：未保存文档没有询问；`on_app_quit` 跑 `Keeping::end()` 把这次运行标为干净结束，下次启动 `Crash::check` 读到干净、`prune` 删光恢复点，内容也找不回。
- 原因：动作先沿聚焦窗口的树派发，没人处理才落到应用级监听；文档宿主只在自己窗口根上截 `Quit`（走 `Closing::proceed(After::Quit)`），设置窗口的 `Shell` 不截，于是直接 `cx.quit()`。
- 复现：ElyDoc 新建文档键入几字，Cmd+, 打开设置，Cmd+Q。
- 修法：全局 `Quit` 改为让每个文档窗口走自己的退出流程（遍历 `cx.windows()` 派发），或设置窗口把 `Quit` 转发给主窗口宿主；`Keeping::end` 只在全部标签干净时删恢复点。

### X-02 共用 `settings.json` 读不出时 Doc、Table、Show 根本起不来
- 严重度：高 ｜ 置信度：已确认
- 位置：`crates/ely-office/src/lok.rs:135`（启动任务里 `set_author(&user::name()?)`）、`:82`（`restart` 同）、`crates/ely-platform/src/user.rs:7`（`Shared::load()?`）
- 现象：`<数据目录>/ElyOffice/settings.json` 不能解析、`version` 大于 1 或某字段类型不对时，三个 LOK 应用都以“the document engine did not start: …settings.json…”的致命对话框退出（码 1）；引擎崩溃后的“重新打开文档”同样失败。agents.md 规定设置读不出“即用默认值、记下原因”，`settings::install` 也确实这样做，唯独这条路径把它变成致命错误。
- 复现：往 settings.json 写 `{"version": 2}`，启动 ElyDoc。
- 修法：`start`/`restart` 里设置读不出时退回 `user::account()` 并记日志（设置窗口已有 Banner 说明）；或让 `user::name()` 用 `ely_ui::settings::current` 已读好的值。

### X-03 第二个实例启动即清掉运行中实例的工作目录与恢复点
- 严重度：高 ｜ 置信度：机制已确认；频率为可能（Linux 桌面每个文件起一个进程；macOS `open -n` 或开发构建）
- 位置：`crates/ely-office/src/lib.rs:219-226`（`fresh_work` 无条件 `remove_dir_all`），调用 `crates/ely-office/src/lok.rs:128`、`apps/ely-pdf/src/viewer.rs:102`、`apps/ely-mark/src/desk.rs:162`；`crates/ely-office/src/recover.rs:59-71` 与 `crates/ely-files/src/crash.rs:40-46`（锁里 pid 活着＝干净结束）
- 现象：实例 B 启动时 (a) 删掉 `<data>/work`——A 的每个 `tab.work`——此后 A 的每次保存都在消失的目录里写 `save.{format}` 失败（`files.rs:48-50`），状态变“保存失败”、自动保存停止；(b) `Crash::check` 见 A 的 pid 活着判为干净，`prune` 删掉 A 的全部恢复点与 `index.json`；(c) `Crash::open` 把 `session.lock` 改成 B 的 pid，A 正常退出时删掉 B 的锁，B 之后崩溃被当作干净结束。
- 复现：Linux 从文件管理器连开两个 .docx；macOS `open -n -a ElyDoc`；然后在第一个实例 Cmd+S。
- 修法：数据目录加单实例锁（flock），已有实例在时把文件交给它；或工作目录按进程分（`work/<pid>`）、恢复点按锁 pid 存活判定不清理。

### X-04 属性栏字号菜单的 `expect("sizes are numbers")` 是脆弱点，但当前不可达
- 严重度：低 ｜ 置信度：已确认（上游源码核实）
- 位置：`crates/ely-office/src/panel.rs:220-228`、`:283-285`
- 说明：上游 `FontHeightPayload`（`sfx2/source/control/unoctitm.cxx:960-966`）只发 `=<数字>`（禁用时为 0），不发 `disabled`；混合选区经补丁 0003 的 JSON 走 `MIXED` 被过滤。所以这里不会 panic。保留为脆弱点：任何将来改动内核状态格式都会让整个属性栏崩溃，`points` 应改为可失败。

### X-05 恢复点写不成时每 10 秒弹一次提示
- 严重度：中 ｜ 置信度：已确认
- 位置：`crates/ely-office/src/recover.rs:227-231`、`crates/ely-files/src/recovery.rs:105-116`
- 现象：快照超过恢复点容量上限（默认 64 MB，可设到 32 MB；带视频的 PPTX、大 XLSX），或磁盘满、数据目录只读，只要标签还脏，每个计时周期都弹“office.recovery.failed”。
- 原因：失败时 `slot.written` 不变、也没有像 `keeping.behind` 那样记住上次失败，下个周期 `due` 又含该标签。
- 修法：按槽记上次失败文字，变了或成功才再提示；超容量时写 `written = Some(edited)` 并解释一次。

### X-06 `session.lock` 空或损坏即永久关闭恢复功能
- 严重度：中 ｜ 置信度：已确认
- 位置：`crates/ely-files/src/crash.rs:21-25`（非原子写）、`:29-50`（其他内容返回 `InvalidData`）、`crates/ely-office/src/recover.rs:59`、`:134-141`
- 现象：断电截断 `session.lock` 后，每次启动都显示“office.recovery.off: bad session lock”、不写恢复点；没有任何代码重写该文件，用户得手删。
- 修法：解析不出当作 `Marker::Crashed`（给出恢复点并重写锁），锁文件原子写。

### X-07 加密文档的恢复点每次写入都解密全部已有点，且持 worker 锁
- 严重度：中 ｜ 置信度：可能
- 位置：`crates/ely-files/src/recovery.rs:120-121`、`:164-190`、`:193-199`、`:225-237`；调用 `crates/ely-office/src/recover.rs:201-214`（在 `read_in` 内）
- 现象：带密码的 PDF 每 10 秒写点时读并 AES-GCM 解密每个 `.enc`（至多 20 份×文档大小）只为列出；`RecoveryKey::load()`（钥匙串加 1 秒锁循环）也每次都跑；PDF 视图排队的编辑与读取随之停顿（树莓派上更久）。
- 修法：按文件名列点，恢复时才解密；密钥每次运行读一次。

### X-08 另存为在等待关闭或退出时被拒绝，关闭链悬空，ElyMark 的会话清单冻结
- 严重度：中 ｜ 置信度：已确认
- 位置：`crates/ely-office/src/save.rs:219-246`（`identity` 出错、`office.error.format`、`office.error.taken`、`office.error.convert` 四处提前返回不调 `close_cancelled`），对照 `:191-194`、`:419-424` 有调；`apps/ely-mark/src/desk.rs:85-88`、`apps/ely-mark/src/restore.rs:282-285`、`:341-343`、`:362-366`
- 现象：退出或关窗途中对未命名标签答“保存”、选了不可保存的扩展名或别的标签正打开的文件：弹一条提示，退出静静停住；ElyMark 的 `keeping.leaving` 留在 `true`，`keep()` 不再写 `session.json`，到下次尝试退出前打开、关闭、改名的标签都不进清单，崩溃后恢复缺它们。
- 复现：ElyMark 新建未命名文档，Cmd+Q → 保存 → 选 `notes.txt`。
- 修法：`write_as` 的每个提前返回在 `then.is_some()` 时调 `close_cancelled`。

### X-09 保存期间落地的编辑可能让自动保存停摆
- 严重度：低 ｜ 置信度：可能（与事件次序有关）
- 位置：`crates/ely-office/src/open.rs:241-251`（只在 `ModifiedStatus` 假→真时 `session.edit()`）、`crates/ely-office/src/save.rs:68-70`、`:131-132`、`crates/ely-files/src/session.rs`（票据按写入时的修订）
- 现象：`save_owned` 清掉内核 modified、随后导出期间键入的 `=true` 若在导出续动作调 `ely_files::save` 之前处理，`reported` 看到 `(true, true)` 不调 `edit()`，`finish_save` 置 `Saved`；`tab.dirty()` 为真而 `state != Dirty`，自动保存不再触发直到手动保存。
- 修法：导出开始时取票据，或每次 `Engine::edited` 递增都 `session.edit()`。

### X-10 别的标签的文件所在卷消失时，本标签的另存为失败并被标为保存失败
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-office/src/save.rs:32-41`（`holder` 对每个标签 `same_file(open, target)?`）、`:231-234`、`:429-450`（`report` → `save_failed()`）
- 修法：`holder` 里解析不出的别的标签路径视为“不是同一文件”并记日志；写入前的拒绝不调 `save_failed()`。

### X-11 worker 的 stderr 中继遇到第一行非 UTF-8 即永久停止
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-ipc/src/relay.rs:31-34`（`lines()` 出错即 `return`）
- 现象：LibreOffice 或 MuPDF 打出一行非 UTF-8（Linux 上 Latin-1 文件名）后，之后的结构化日志、崩溃栈与 `Relay::last_error` 全丢。
- 修法：`read_until(b'\n')` 加 `from_utf8_lossy`，出错继续。

### X-12 恢复路径在界面线程做文件 I/O
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-office/src/recover.rs:89-98`（`flush_index` 的暂存写加 `sync_all`，`keep_tab`/`forget_tab` 调）、`:287`、`:316`（`restore` 同步调钥匙串与拷贝）
- 现象：每次开关标签都在界面线程 fsync `index.json`；崩溃后首启在界面线程查钥匙串（最多等 1 秒锁）并逐个拷贝恢复点。慢介质（树莓派、SD 卡）上可见卡顿（规则 9）。
- 修法：移到后台执行器，`in_window` 续。

### X-13 非 macOS 构建里仍提供只会报“未构建”的入口
- 严重度：低 ｜ 置信度：已确认
- 位置：`crates/ely-platform/src/print.rs:87-89`（打印，各文档应用“文件›打印…”与 Cmd+P 照常）、`crates/ely-platform/src/trust.rs:23`（PDF 签名对话框）、`crates/ely-platform/src/media.rs:52`（ElyShow 插入媒体）、`crates/ely-files/src/rename.rs:63`（改名）
- 修法：按 `cfg` 或能力标志禁用菜单项，或补齐平台实现。

### X-14 `agents.md` 引用的 `docs/engineering-risks.md` 不存在
- 严重度：低 ｜ 置信度：已确认
- 位置：`agents.md`（三处引用“证据见 `docs/engineering-risks.md` R-01”等）；`docs/` 下无此文件
- 修法：补回文件或改引用。
