# ElyTable 的 R 宏（实验性）

在本机用 R 处理所选区域的一份副本，结果一步写回、一步撤销。只用 base R，不装包。ElyTable 不安装 R。

R 以当前用户的权限运行，**不是沙箱**：宏能读写用户能碰的任何文件。只运行可信的代码。宏从不随文档运行，也不存进 XLSX、ODS；只在用户点“运行”时执行（文档内的宏仍照 LOK-11 不执行）。

## 启用

- 装 R（https://cran.r-project.org；Linux 装发行版的 `r-base` 或 `R` 包）。
- ElyTable 依次找 `Rscript`：环境变量 `ELY_RSCRIPT`，`PATH`，再找 R 自己的安装位置（Windows 读注册表 `HKCU`、`HKLM` 的 `SOFTWARE\R-core\R` 的 `InstallPath`；macOS 查 `/Library/Frameworks/R.framework/Resources/bin`、`/opt/homebrew/bin`、`/usr/local/bin`，因从访达启动的应用拿不到 shell 的 `PATH`）。都找不到即说明如何安装，不运行。
- R（GPL）不随包，三端的安装包都不含它，只作用户自装的外部程序调用，不影响 Ely 的许可。Rscript 继承 ElyTable 的环境；打包脚本不设 `LD_LIBRARY_PATH` 之类的变量。
- 内核须带补丁 `0081-lok-calc-r-macro-batches.patch`。没有它的内核读不出选区，运行即报错。

## 使用

选一个矩形区域，菜单“宏 → 运行 R 宏”。编辑器第一次打开时填入示例（`apps/ely-table/src/macros/example.R`：去掉文字首尾空格、表头加粗、列宽自适应），之后保留上次的代码。“打开 .R…”“保存 .R…”只读写 `.R` 文件，代码不进工作簿。运行时按钮变为“停止”；停止、关掉对话框或超过 30 秒，R 连同它起的子进程一并结束，表格不改。

R 运行期间对工作簿的任何改动（键入、点按、粘贴、命令）都使这一批作废，提示重新运行。

## API

```r
selection <- ely$selection()
data <- ely$read_range(selection)
ely$write_range(selection, data)
ely$format_header(selection, bold = TRUE)
ely$autofit_columns(selection)
```

- `selection()` 只代表点“运行”时选中的矩形，不收任意地址。
- `read_range` 给一个 `data.frame`，每列是 list，每格保留自己的类型：文字为 UTF-8 字符串，数字为 double，逻辑值为 logical，空格为 `NULL`。
- 公式格是 `ely_formula` 对象，`formula` 为公式文字，`value` 为算得的值；只读，改了即报错。公式文字是 Calc API 的写法（英文函数名、参数以分号分隔），不是界面上的 Excel A1 写法。
- 改一格：`data[[1]][2] <- list("00123")`；清空：`data[[1]][2] <- list(NULL)`。写空字符串 `""` 也是清空：Calc 没有空文字格（同 Excel 的 `Range.Value = ""`）。维数与 list 列须不变。
- 每格至多写一次，表头与列宽各至多一条。

## 运行方式与限制

ElyTable 先取文档的版本号与选区的类型化副本（内核补丁 0081），放开 worker，再在自己的线程里以参数数组起 `Rscript --vanilla api.R 输入 宏 输出`，工作目录是一个私有临时目录；不经 shell，代码与数据都以文件交给 R。Windows 上 R 不弹控制台窗口，放进一个 Job 对象（结束即整树结束）；Unix 上 R 自成进程组，结束时整组 `SIGKILL`。

R 正常结束后，ElyTable 先校验整批（格式、类型、位置、公式不动、大小），再交 worker；worker 核对 R 读选区时的版本号，任何写入者（窗口里的键入、`ely-table cell set`、命令）此后改过文档即作废（`table.r.stale`）；窗口里排队尚未到达内核的输入先落地，批随之作废；核对版本号与写入在同一把 worker 锁下、内核里接连完成，其间没有写入者插得进来；批交出时还在等这把锁的命令行请求排在批之后，同两次先后的编辑。不改文档也不改选区的命令（换界面配色）不使版本号前进。内核重读选区与快照比对、再校验一遍，然后在一个绑定当前视图的撤销组里写值、表头加粗（西文、亚洲与复杂文种三种字重）与列宽。每格写入前照 Calc 键入的规则（`ScDocument::IsBlockEditable`）判断能否改动，受保护工作表上解除锁定的格照常可写；中途被拒即撤回这一组并清空重做，撤不回（含 Undo 抛出异常）时如实报告（`table.r.rollback`）。worker 中途退出、回复丢失或内核未给原因时，结果无从得知，ElyTable 说改动可能已部分写入（`table.r.unsure`），不说“表格未改动”。R 失败或校验不过时文档不动。内核拒绝时带一个代号（读选区时在结果里，写入时经 `.uno:ElyMacroRefusal` 取一次），ElyTable 按代号说明原因：超过一万格、不是一块区域、超过 4 MiB、改动连同选区超过 4 MiB、撤销已关、选区已变、格被锁定、受保护的工作表上改格式、含合并格、撤不回。

限制：10000 格，30 秒，输入、输出与源码各 4 MiB，交给内核的整批（选区副本加改动）4 MiB，单格文字 32767 字节。多重选区、含合并格的区域与撤销已关的文档被拒；改格式（加粗、列宽）只看工作表是否受保护，与 Calc 一样，数组公式的格可改格式。内核读选区时先按回复的确切字节（转义后的 JSON、外框与分隔）逐格计数，超过 4 MiB 即拒，不建出整份副本；恰好 4 MiB 照读。Ely 里没有只读的文档：磁盘上只读的文件、存为“以只读打开”的表格都照常可编辑（键入与 R 一样），`.uno:EditDoc` 会使 LOK 失去文档，Ely 也从不把视图设为只读；文档若只读，Calc 的规则即拒绝每格写入。列宽自适应作用于所选整列。写逻辑值时设逻辑格式；对逻辑格写数字时改回常规数字格式。内核在一批写到中途崩溃时不保证回滚。

日志只记 Rscript 的退出状态与用时、读入的格数、写回的字节数与拒绝的原因，不记代码、单元格的值与 R 的输出。

## 测试

`apps/ely-table/src/macros/` 下：`protocol.rs`（传输、类型、整批校验），`runner.rs`（缺 R、退出码、坏输出、超时、停止；用 `tests/fixtures/table/rscript-*` 替身，只在 Unix），`contain.rs`（结束时 R 起的子进程一并结束，三端），`files.rs`（只认 `.R`、大小上限），`integration.rs`（真 R 与内核）。`crates/ely-lok/src/client/tests/r_macros.rs` 测内核一侧（校验、冲突、公式不动、一步撤销、撤销关闭时拒绝、数字精度）。用真 R 的测试在找不到 Rscript 时打 SKIP。
