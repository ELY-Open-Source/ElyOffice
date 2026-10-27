# 缺陷清单进度

`docs/BugList.md` 的 93 条，按清单次序逐条处理：先核对当前代码，已由先前改动修好的写明出处，其余修好、审查、提交、推送后勾上。

## 0.1.3（用户 2026-10-10 定：修完全部缺陷与 K-01，Windows、Linux 与 macOS 同等可用，零错误零警告；Omarchy 上主题随 Omarchy；三端确认后经 Gitea 运行器发布 0.1.3）

次序：先 E-01（审查要能编译），再按下列清单次序逐条审查未审的修复与 K-01，最后三端与发布。

- [x] E-01 Windows 与 Linux（WSL）的构建环境：工具链、MuPDF、CEF、ElyMark 页面、tessdata、Git，`cargo check --workspace --all-targets` 两端通过（LibreOffice 随 L-01、W-01） — R1 CEF 未设 API 版本、macOS 编译回归、日志测试不触发错误、Windows 硬链接不认作同一文件、WSL 无 Git；R2 注释超长；R3 PASS

## 0.1.3 先发布（用户 2026-10-10 改定：非 P0 的缺陷先放下，三端先打包发布；Omarchy 主题跟随必须有）

P0 次序：W-01、K-01、L-01、O-01、W-02、L-02、W-03、L-03、W-04、L-04、CI-01、REL-3。N-01 至 N-06 在 0.1.3 写为已知限制（打印、媒体、证书信任、Windows 与 Linux 的自更新安装、停止 Git、同级应用），发布后再做；其余未勾缺陷（G-03 起、K-02 起、X-04 起）同。

## 0.1.2 先做（用户 2026-10-09 定：先修致命与高，加自动更新，发布 0.1.2；其余随后按清单次序）

次序：S-03、P-01、X-01、D-01、D-02、P-02、P-03、P-04、M-01、X-02、X-03、K-01（Linux、Windows，本机不能运行）、UPD-1、REL-1、REL-2。K-01 不进 0.1.2：只影响 Linux 与 Windows 的 worker，本机不能运行验证，0.1.2 只发 macOS；留在下面的清单里，未勾。T-05 已改一半（审查查出缩放后尺寸不重测），暂放，补丁存在草稿区。

- [x] UPD-1 自动更新：应用内检查新版本、下载、校验，经用户同意后替换并重启；发布时生成更新清单，不再要点 GitHub 链接 — 启动检查默认关、菜单“检查更新…”、退出后再装、失败记录先于重开、经 Gitea API 找清单；R1–R4 授权、提交点、保存询问、版本绑定、横幅位置、取消退出、记录先后；R5 两处（启动检查失败记日志、以替身 open 断言先后）已修并以变异验证，五轮上限后由 UPD-2 复核通过

- [x] T-01 内核的图块与命中按像素网格，Ely 的光标、选区、表头按文档 twips，越远越错位 — R2.16 已修（图块 twips 与打印 twips 互换）
- [x] T-02 单元格光标露出边缘即把视图滚到中央，点击与拖选时视图跳动 — R1 拖选中仍滚、超宽格跳回起点；R2 PASS
- [x] T-03 视口比工作表宽时网格先居中再左移一次 — 载入回复带文档种类，表格不居中；R1 测试漏几何到达前的首帧；R2 PASS
- [x] T-04 从不调用 `setClientZoom`，内核命中与几何按默认缩放算 — R2.16 已修（worker 每次报可见区前 setClientZoom）
- [x] T-05 文档尺寸按内核像素取整，表尾被裁 — 内核尺寸按其测量时的比例换到当前比例（加载即设比例、缩放即改比例并重测、迟到的测量也换算）；R1–R4 缩放后未重测、表尾锚点被截、连续缩放迟到、首次测量无比例与文件超行；R5 PASS
- [x] T-06 滚轮向下被内核平铺区卡住 — 表格的可滚范围先行两屏、止于表尾；R1 PASS
- [x] T-07 表头点选、行高列宽拖动是“按编号的命令”，而每次点击与拖选都推进修订，紧接着的表头操作被当作过期拒绝 — 行列号不再算作按编号的命令；R1 PASS
- [x] T-08 自动筛选的列菜单对光标所在的表而不是按钮所在的表筛选、排序 — 先把光标移到按钮所在格再排序或筛选；R1 PASS
- [x] T-09 导入文本文件时整文件的 UTF-8 校验在界面线程跑 — 编码与分隔符的猜测随读文件在后台；R1 PASS
- [x] T-10 CSV 导入对话框开着时打开任何别的对话框，待导入的文件静默消失 — 被挤开的导入回到队头，已读完的保留列类型；R2 PASS
- [x] T-11 对话框比其标签活得久：导出、定义名称、重命名不可见却“开着”，挡住导入；名称表的“转到”激活已关闭的标签 — 关标签时一并关掉其对话框与选项列表；R1 PASS
- [x] T-12 打印区域是整列或整行时页面设置无法应用 — 打印区域可为整列、整行（每端至多一个 $）；R2 PASS
- [x] T-13 打印失败报成“导出失败”（非 macOS 必现） — print_later 自己以“打印失败”报错，五个应用同；R1 PASS
- [x] T-14 导入扫描把整个文件的每个字段都建成字符串 — split 逐行给出，扫描只留预览行与列标记；R1 PASS
- [x] T-15 非 UTF-8 文本的默认编码跟系统区域而不是界面语言 — 默认编码按应用所显示的语言；R1 PASS
- [x] S-01 切换幻灯片后主画布不重画 — 已由 be31260（R2.4）修好并有单测，此处补勾
- [x] S-02 `deck.rs` 超过 500 行 — 随 S-03 把 Extra 移到 extra.rs，deck.rs 463 行
- [x] S-03 ElyShow 的缩放一两帧后弹回“适合幻灯片”，缩放功能不可用 — 只在标签首次观察时适合一次；R1 PASS
- [x] S-04 选屏对话框被别的对话框顶掉后，`show_starting` 卡住、放映副本泄漏 — 有对话框开着时不再顶掉它，提示先完成；选屏对话框被拒即丢副本；R2 PASS
- [x] S-05 一条批注的时间恰为午夜或只有日期，整个批注栏不显示 — 读不出时间的批注只占一行汇总，其余照常显示（jiff 本可读只有日期的时间）；R2 PASS
- [x] S-06 导出或打印的“从–到”填 0 时导出全部幻灯片 — 键入的页码夹到 1..=末页；R1 PASS
- [x] S-07 母版视图下演示检查的行按幻灯片序号跳转 — 母版视图下点行提示先关母版，不跳；R1 PASS
- [x] S-08 刚编辑过幻灯片就进备注栏键入，备注可能丢 — 发出的备注待读回确认，键入中读回只移基线不动草稿；R2 PASS
- [x] S-09 改尺寸失败后 `size_asked` 一直留着 — 所要尺寸带输入序号，发送前先记、未入队即清，命令落地后的读回即清；R2 PASS
- [x] S-10 开始放映的两处拒绝无提示（规则 2） — 三处拒绝都提示（另含读回时已换标签）；R2 PASS
- [x] P-01 文件系统的 `PermissionDenied` 被拼成不存在的文案键，应用崩溃 — worker 的拒绝改为自有错误类型 Refused；R1 测试不跨平台、不看提示；组件库 f834f55 加 Toaster::shown；R2 PASS
- [x] P-02 每次编辑落地清掉全部图块与缩略图，页面空白、刚画的墨迹消失 — 图块随 R2.6；缩略图在编辑后留到重画；R1、R2 测试未证实际绘制；R3 PASS
- [x] P-03 “编辑文字和图片”先命中图片对象再命中文字；一点抖动就移动了背景图 — 包住文字块的对象让位于文字，移动须过 3 个窗口像素；R1 漏掉同点的小图；R2 PASS
- [x] P-04 `TEXT_BLOCKS` 对每个文字块重跑一遍整页内容过滤，编辑工具长时间无反应 — 整页一遍干跑判定全部文字块（1000 块 1.5 s→0.1 s）；R1 转向文字后错位、共用原点只计一块；R2 PASS
- [x] P-05 连续快画或快点被拒：“文档在此间已改动，未执行” — 按坐标的编辑排队、跑在出队时的状态上，只在页面变动时拒绝；R2 PASS
- [x] P-06 默认字体“原字体”在 Type 1/Type 3 字体的文件上失败，只给引擎原话 — 原字体写不了新字（非 CID 编码、页面内直接字体、不在本页）时面板提示换字体；R2 PASS
- [x] P-07 含连字的文字块永远不可编辑 — 字形按 stext 的写法展开（多字字形、连字码点），整页、单块与删除同一判法；R3 PASS
- [x] P-08 页面读取失败后每帧重试、每帧弹提示 — 失败的读取按页或种类记下，编辑后重读；签名读取以保存计数判过期
- [x] P-09 识别文字的写入被拒后仍弹“成功” — on_same 回报是否执行；被拒时只有警告
- [x] P-10 预览回复被丢弃时“应用”静默无事 — 预览回复照常交回，版本已变即提示，不显示过期预览
- [x] P-11 撤销或重做在 MuPDF 侧失败后 worker 的步数不再同步 — 撤销失败后重读历史标记
- [x] P-12 `unhook` 中途抛出时泄漏保留的对象 — 保留的链接随保留随计数
- [x] P-13 优化导出的“之前”大小取磁盘文件而非当前文档 — 有未保存编辑时不与磁盘文件比大小
- [x] P-14 文字块含控制字符时 JSON 回复不可解析 — 共用 JSON 写入：控制字符转义，非法字节与孤立代理为 U+FFFD
- [x] D-01 导出或打印对话框开着时关闭其标签即崩溃 — Docs 实现 tab_closed，关标签即撤下其导出对话框；R1 PASS
- [x] D-02 邮件合并以当前标签为模板，预览后生成的每份都是同一条记录 — 选数据时记下模板标签，预览副本不作模板；R1 测试绕过选 CSV 入口；R2 PASS
- [x] D-03 界面线程同步持 worker 锁读内核（样式、字体、书签、公式、分节名） — 后台读取，回来核对标签、视图与对话框请求号
- [x] D-04 一条批注的时间读不出，全部批注与修订都不显示 — 逐条解析，读不出的批注写明作者与原因（AX 可读、可滚动），修订照留
- [x] D-05 邮件合并某条记录失败时内核文档泄漏 — 失败的记录照样关闭内核文档，再传出首个错误
- [x] D-06 导航栏大纲点击按当前列表取下标，后台重读后可能越界 — 点击按绘制时的列表跳转；补点击路径的回归测试
- [x] D-07 导航栏隐藏时仍随每次光标移动重读大纲与书签 — 隐藏时不跟随标签、不读；再显示时读一次
- [x] D-08 Linux、Windows 上“打印…”可用，导出 PDF 后才报“此平台未构建” — 未构建平台禁用打印、不绑快捷键、错误不带路径；命令面板先查禁用
- [x] M-01 键入期间外部以相同内容重写文件，丢掉上次停顿之后键入的全部文字 — 先等页面送出未送的键；同文只接过文件与字节、标签为已保存；磁盘回到上次保存的文字时保留编辑并恢复自动保存；R1–R4 字节、自动保存、在途写入、已保存基线；R5 PASS
- [x] M-02 `refresh` 归零版本计数却不换代，在途的一键让标签一直“未落定” — 刷新即换代、归零并结束欠下的渲染（tally.rs）；R1 渲染回调悬空、无在途消息的测试；R2 测试未证旧代归零；R3 PASS
- [x] M-03 干净标签接过磁盘文字后关闭仍问“是否保存” — 接过磁盘文字的各路（同文、干净重载、合并）都经 adopt_disk，标签与视图的保存基线同设；R1 合并采用未同步视图基线；R2 注释超长；R3 PASS
- [x] M-04 第二次打印或导出 PDF 悄悄取消第一次，没有文件也没有错误 — 每份打印与导出各有编号与文件，作业按编号保留、互不取消；每份作业完成时记下编号与结果；R1 窗口测试只数日志；R2 PASS（窗口测试只在 macOS，本机未运行）
- [x] M-05 点页面里的链接什么也不发生 — 页面拦下链接单击，地址取链接标记原文，预览单击、可视 Cmd/Ctrl 单击交应用（审查 4 轮）
- [x] M-06 `on_app_quit` 无条件删恢复点与崩溃锁，AppKit 直接终止时丢未保存内容 — 未保存时退出只放下锁、不清点，窗口释放时不再删（审查 1 轮）
- [x] M-07 崩溃恢复的未命名副本按“第一个显示的未命名标签”配对，内容可能落错标签 — 未命名副本按新建时记下的标签号配对，经 Keeping::take（审查 2 轮）
- [x] M-08 页面重载后悬着的渲染回调永不完成，导出既不成功也不报错 — 页面失败后挂起与新到的渲染都以错误结束，载入成功才恢复（审查 2 轮）
- [x] M-09 导出的保存面板出错被当作取消 — 面板出错走 panel_failed，取消只记日志（审查 1 轮）
- [x] M-10 关闭查找框只清当前标签的高亮 — 关闭查找框对每个标签的页面清除高亮（审查 1 轮）
- [x] M-11 页面把搜索词写进应用日志 — 控制台的文字与来源都可由文档伪造，只记行号；页面自己的诊断经 elyQuery；R1 前缀可伪造、R2 data 来源、R3 sourceURL、R4 编码路径与目录、R5 缺回归测试：已补并经变异核对（五轮上限，未再审）
- [x] M-12 浏览器创建完成前 `WebView` 已释放，浏览器泄漏 — 浏览器在泵里同步建好即登记，视图要么在建前已走、要么有浏览器可关；webprobe 加建前丢弃与 0–27 毫秒丢弃，35 页都在保持前释放（审查 4 轮）
- [x] M-13 编辑器构建失败时页面空白且不上报 — 可视建不成退回源码并告知，两者都建不成即报失败、留下文字、模式请求重建、不留容器；新测 mark-broken.mjs（审查 2 轮）
- [x] M-14 `render_pdf` 在界面线程内嵌图片并写文件 — 打印页的内嵌、读样式与写入经 paper::stage 在后台执行，GPUI 单测证明执行器运行前未写（审查 2 轮）
- [x] M-15 打印与导出 PDF 的暂存页执行文档里的脚本 — 打印与导出 PDF 的暂存页以 WebView::open_inert 打开，脚本关；webprobe 以同页开脚本为对照（审查 2 轮）
- [x] G-01 打开“文件历史”或“逐行追溯”后，中间区不再响应任何别的选择 — 每个新选择（文件、提交、侧栏、分支、位置）都清掉文件历史与逐行追溯；测试窗口先装菜单（审查 2 轮）
- [x] G-02 进行中操作读取失败被吞掉；`git am` 冲突无提示条无错误 — 读不出进行中的操作即以 Git 原话的红色提示与日志说明；无 onto 的 rebase-apply 视为 git am，跳过、中止有文案（审查 2 轮）
- [ ] G-03 新建的空仓库在“仓库”页显示两段 Git 致命错误（引用日志、LFS）
- [ ] G-04 停止取回/拉取/推送失败的提示写成“克隆没有停下”
- [ ] G-05 合并结果保存失败报成“Git 失败”，`tree.error.save` 是死文案
- [ ] G-06 首次提交后紧接着取消暂存，按过期的“已有提交”判定跑成 `git rm --cached`
- [ ] G-07 任一引用没有创建日期，整个分支页读不出
- [ ] G-08 清理预览后文件被外部删除，对话框只给裸 OS 错误
- [ ] G-09 后台读取回来的对话框覆盖用户同时打开的对话框
- [ ] G-10 选中的分支、标签、位置是窗口级的，跨仓库标签串台
- [ ] G-11 已选边的冲突块再选另一边，提示“结果已手改”
- [ ] G-12 符号链接父目录下，当前打开的工作树被当成别的工作树而可“移除”
- [ ] G-13 非 macOS 平台的死入口
- [ ] G-14 `repo.rs` 的孤立文档注释
- [x] K-01 非 macOS 的 worker 不运行 VCL 主循环，内核的回调与异步输入永远不送达 — c96d39a 已做（Windows `worker/messages.rs`、Linux `worker/unipoll.rs`）；R14.13 照实关闭
- [x] K-02 图块的代际在画完之后才取，画到一半的作废被盖掉 — d59159c 已做（先读代际再画）；R14.13 核对
- [x] K-03 `rect_of` 宽容解析把读不出的载荷悄悄吞掉 — d59159c 已做（`Unreadable` 记为故障）；R14.13 补测试 `an_unreadable_rectangle_is_reported_never_swallowed`
- [x] K-04 `Sheets.current` 用 `as u64` 转有符号的 part — d59159c 已做（`u64::try_from`，-1 报错）；R14.13 核对
- [x] X-01 设置窗口在前时 Cmd+Q 直接退出，跳过未保存询问并抹掉恢复点 — 退出一律经主窗口宿主的流程（route_quit），延后到动作结束、先把宿主窗口提到前面；R1 主窗口 Cmd+Q 不问；R2 询问被设置窗口挡住；R3 PASS
- [x] X-02 共用 `settings.json` 读不出时 Doc、Table、Show 根本起不来 — 作者名取启动时读入的设置；R1 重启失败丢作者、脚本退出码、client.rs 超行；R2 作者更新的先后与停机期间的更新；R3 PASS
- [x] X-03 第二个实例启动即清掉运行中实例的工作目录与恢复点 — 每个应用一个实例锁，第二份被拒并报出持锁进程（Unix fcntl F_GETLK）；R1–R3 进程号竞争、Windows 断言；R4 PASS
- [x] X-04 属性栏字号菜单的 `expect("sizes are numbers")` 是脆弱点，但当前不可达 — d59159c 已做（`ely_office::panel::ordered`：读不成点数的排在最后，不 panic）；R14.16 核对
- [x] X-05 恢复点写不成时每 10 秒弹一次提示 — d59159c 已做（每个标签每种原因只提示一次，下一拍照常重试）；R14.9 核对
- [x] X-06 `session.lock` 空或损坏即永久关闭恢复功能 — d59159c 修空锁与乱码；R14.9 补非 UTF-8 的锁（`Crash::check` 读字节，照崩溃处理并重写）
- [x] X-07 加密文档的恢复点每次写入都解密全部已有点，且持 worker 锁 — d59159c 修解密（修剪只读文件名）；R14.11 把加密与写盘移出 worker 锁（`write_point`），测试 `an_unsaved_tab_gets_a_point`
- [x] X-08 另存为在等待关闭或退出时被拒绝，关闭链悬空，ElyMark 的会话清单冻结 — d59159c 已做（`refuse_save` 停下等着的关闭）；R14.10 补标签已关的一路与测试 `a_refused_save_as_stops_the_close_it_held`
- [x] X-09 保存期间落地的编辑可能让自动保存停摆 — d59159c 已做（`watch` 按编辑计数前进再记编辑）；R14.10 核对，`an_edit_after_a_save_raced_it_counts_again`
- [x] X-10 别的标签的文件所在卷消失时，本标签的另存为失败并被标为保存失败 — d59159c 已做（`holder` 跳过比不了的标签，目标错误照报）；R14.10 核对，`a_tab_on_a_vanished_folder_takes_no_other_file`
- [x] X-11 worker 的 stderr 中继遇到第一行非 UTF-8 即永久停止 — d59159c 已做（按字节读、宽松转文字）；R14.11 核对，`a_line_in_another_encoding_stops_nothing`
- [x] X-12 恢复路径在界面线程做文件 I/O — d59159c 修文档应用；R14.9 补 ElyMark（每 10 秒的扫点与写 session.json 改后台，退出当场写，`Turns` 防旧写盖新写）
- [ ] X-13 非 macOS 构建里仍提供只会报“未构建”的入口
- [ ] X-14 `agents.md` 引用的 `docs/engineering-risks.md` 不存在

## 0.1.2 发布

- [x] UPD-2 复核 UPD-1 第五轮的两处修复：启动检查（含 Gitea 查询）失败写日志；失败记录先于重开，以替身 open 断言 — R1 PASS
- [x] REL-1 dmg 里有指向“应用程序”的快捷方式，可直接拖入安装（用户 2026-10-09 报） — release.sh 在 dmg 里放“应用程序”替身，随 UPD-1 审查
- [x] REL-2 修完上列各条后打包、发布 0.1.2，官网指向新发布页 — v0.1.2 发在 git.zacharyzhang.com（六个 dmg、两个源码包、六份更新清单；release-macos.sh 通过；经 API 取清单与 dmg 散列核对一致），官网显示 0.1.2

## 0.1.3 三端与发布

- [ ] L-01 Linux 的 LibreOffice：钉住的源码加全部补丁在 Linux 构建，LOK 探测通过
- [ ] L-02 Linux 全工作区零警告构建，`cargo test --workspace` 通过（引擎齐全，无 SKIP）
- [ ] N-01 打印：Windows、Linux 交系统打印（与 macOS 同一份 PDF、纸张与取消规则），不再禁用
- [ ] N-02 ElyShow 媒体：Windows 以 Media Foundation、Linux 以 GStreamer 实现 `media::Player`（载入、可播、失败原因、帧、循环与播完） — Windows 已做（R14.17，`media/mf.rs`）；Linux 待 R14.18
- [x] N-03 自动更新的安装与重开：Windows 安装程序、Linux AppImage，照 macOS 的退出后替换、失败记录与重开 — 4a74360 已做（`install::after_quit`：Windows 等进程退出后静默运行安装程序并重开，AppImage 换文件后重开，失败写入 `updates/failed.txt`；`tests/install.rs` 核对）；R14.4 删去非 macOS 的 `install`、`reopen` 桩
- [ ] N-04 签名验证的证书信任：Windows 系统信任库、Linux 系统 CA，禁网，不核吊销
- [ ] N-05 ElyTree 与系统：停止 Git（Windows 结束进程树）、打开终端、在文件管理器中显示，Windows、Linux
- [ ] N-06 应用之间：Windows、Linux 的同级应用查找与启动、系统默认程序打开、单实例与诊断的崩溃报告位置
- [ ] L-03 Linux 真窗口：六个应用开样本、编辑、保存、打印禁用说明、设置、更新检查
- [ ] O-01 Omarchy：外观为“系统”时随 Omarchy 主题，切换主题即刻跟随（Omarchy 虚拟机里实测）
- [ ] L-04 Linux 打包：六个 AppImage，带引擎、许可与桌面项；更新清单含 linux-x86_64
- [ ] W-01 Windows 的 LibreOffice：同一源码与补丁在 Windows 构建，LOK 探测通过
- [ ] W-02 Windows 全工作区零警告构建，`cargo test --workspace` 通过（引擎齐全，无 SKIP）
- [ ] W-03 Windows 真窗口：六个应用开样本、编辑、保存、设置、更新检查
- [ ] W-04 Windows 打包：六个安装程序，带引擎、许可、文件类型；更新清单含 windows-x86_64
- [ ] CI-01 Gitea 工作流：打标签即在 macOS、Windows、Linux 运行器上构建、测试、打包并上传到发布
- [ ] REL-3 版本号 0.1.3，打标签触发运行器，三端产物与清单上传，官网指向新发布

## 0.1.3 之后（三端实测查出，非 P0）

- [ ] P3-01 拼写建议：Windows 与 Linux 上右键不出拼写菜单（`tools::spelling_suggests_and_ignores`）
- [ ] P3-02 Windows：Calc 深色主题的图块为全黑（`paint::the_page_follows_a_dark_theme_and_back`）
- [ ] P3-03 Windows：放映图层大于一帧时分片缺行（`motion::a_layer_larger_than_one_frame_arrives_in_pieces`）
- [ ] P3-04 Windows：放映的页脚字段文字不在占位处（`playlist::field_text_lands_where_its_placeholder_is`）
- [ ] P3-05 Windows：辅助功能的段落选区不桥接（`commands::a11y_follows_the_focused_paragraph`）
- [ ] P3-06 Windows：删不掉内核仍开着文件的标签工作目录（错误 32），写入失败无从模拟（`tests::more::a_kernel_write_failure_stays_visible`）
- [ ] P3-07 Windows：测试以 `kill -9` 杀 worker，Git Bash 的 kill 不认 Windows 进程号，worker 不死（`view::tests::commands::a_dead_worker_reaches_apps_while_idle`）
- [ ] P3-08 Windows（AMD）：多个 worker 同时启动时内核的 OpenCL 探测卡在驱动的 `clCreateCommandQueue` 里数分钟（`openclwrapper::switchOpenCLDevice`）；测试以 `SAL_DISABLE_OPENCL=1` 绕开；是否在 `ely.xcd` 关掉 `UseOpenCL` 待用户定
- [x] P3-09 Windows：内核给的选区文字与文本导出以 CRLF 换行，三个 ElyTable 单测按 LF 比对（`tests::data::sort_filter_duplicates_and_columns_run_from_their_dialogs`、`a_column_button_sorts_its_own_block_not_the_cursors`、`tests::output::exports_write_the_shown_sheet_as_text`）；导出该用哪种换行待定。2026-10-20：导出照内核随系统的换行（同 LibreOffice），测试按平台比对；选区文字的测试按行比对
- [ ] P3-10 Windows：`crates/ely-office/tests/restart.rs` 设 `HOME` 想用临时目录，但 Windows 上 `data_dir` 取已知文件夹（不看 `HOME`），测试在真实的 ElyDoc 配置里运行；中途被杀即留下挡路的 `lok-verify.fodt` 文件夹，真实 ElyDoc 的内核核对随之失败（2026-10-14 审查中出现，已手动删去）
- [ ] P3-11 ElyShow：正编辑表格单元格时在幻灯片空白处右键，内核（Impress）只结束编辑、不发 `LOK_CALLBACK_CONTEXT_MENU`，再右键一次才有幻灯片菜单；Windows 实测，他端未核；修需内核补丁
- [ ] P3-12 Windows：`output::tests::targets_skip_hidden_slides_and_pad_numbers` 以 `/` 拼的路径比对，Windows 的 `join` 用 `\`，单测恒失败（代码无误）
- [ ] P3-13 Windows：ElyPDF 三个单测恒失败（2026-10-14 组件 6b033f3 下实测，与组件无关）：`reads::a_failed_read_is_not_asked_again` 用 `kill -9` 杀 worker（同 P3-07）；`sign::a_signed_file_lists_its_checks_and_saves_as_a_copy` 报 “self-signed”（证书信任在 Windows 未构建，未细查）；`organize::a_save_writes_the_edits_and_undo_after_it_is_unsaved_again` 报状态未落定（原因未查）
- [ ] P3-14 Windows：ElyTree 九个单测恒失败：五个比对 Git 写出的文字，本机 Git 的 `core.autocrlf` 写成 CRLF（测试仓库未设 `core.autocrlf false`）；其余写 Windows 不许的文件名（os error 123）。代码行为未见错，未修
- [ ] P3-15 Windows：全量并行单测中偶有一个 LOK worker 读到的 BuildId 为 `development`（`desktop/source/app/app.cxx` 读不到 bootstrap 的 buildid 时的默认值），握手照规则拒绝；同一时刻别的 worker 读到正确的 8221e31，内核与 worker 都未重建，单独重跑即过（2026-10-14 ElyDoc `review::tests::one_unreadable_time_spares_the_other_notes`，三次重跑皆过）。疑为并发时读 `version.ini` 偶发失败，原因未查
- [ ] P3-16 发布：`ci.sh` 在三端各自调用 `packaging/source.sh` 并上传同名的源码包与上游源码包，发布里同名文件出现三份（Gitea 允许重名）；上游源码包打的是各机的 `engines/mupdf-src`，含构建留下的文件，三份大小不同（v0.1.5：Linux 108 MB，macOS 161 MB）。v0.1.5 手动删去后到的两份，留 Linux 的一份。应只出一份，且上游包只含源码
- [ ] P3-17 ElyTable（Windows，0.1.5 安装版，2026-10-15 录视频时见）：A1 选中后按两次 Ctrl+= 缩放到 150%，网格停在半滚动处：第 5 行被表头截去一半，第 1–4 行看不见，Ctrl+Home 也回不到 A1。应保持首行与首列在视图内（同 R2.7 的按键缩放）
- [ ] P3-18 Windows：`tests::menu::right_clicks_open_excel_rows_on_cells_and_headers` 第二次右键 A2（先 `cell.goto D9`）等不到菜单；不带 R9.3、R9.4 的源码同样失败，原因未查
- [ ] P3-19 Windows：ElyTable 全量单测并行跑（默认线程数或 6）时，`output` 的测试与 worker 一起停在零 CPU、不再前进；逐个跑（`--test-threads=1`）不停，原因未查
- [ ] P3-21 Windows：Calc 读鼠标的行位置随系统 DPI，不随视图报的比例（`view::tests::large::a_click_far_down…`、`a_drag_far_down…` 设 2×）。本机系统缩放 150%：设 1.5 即过，2.0 时 B4998 落到 B4953，1.0 时落到 B4820；HEAD 与 0.1.9 的内核同样，不是 R10 引起。单显示器时两者相同，不出错；窗口在缩放不同的另一块显示器上时，越往下点越偏。原因在内核按设备 DPI 截行高，未查
- [x] P3-22 Windows：ElyShow、ElyTable、ElyDoc 的应用单测每个都留下一个 LOK worker（gpui 的 `TestAppContext` 从不释放应用，worker 随之不关），ElyShow 全量串行跑到约 50 个 worker（20 GB）后，新 worker 启动即堆损坏退出（0xc0000374），其后 11 个测试失败。已修（2026-10-21）：测试的 worker 经 `ely_office::lok::test_worker` 建，测试线程结束即关（`ely_ui::at_thread_end`）；修后同时只有一个 worker，ElyShow 只剩 P3-12

- [x] P3-20 发布（v0.1.9 实测）：macOS 任务在 `macos-signing` 运行器上读 `ELY_SHARE`（`~/Documents/GitHub/ElyOfficeSuite`）时，系统为“文稿”文件夹弹出访问许可，无人点按即一直等：`source.sh` 的 tar 卡在 `open()` 一小时不动。本次取消该任务，经 ssh 在同一 CI 克隆跑 `ci.sh macos v0.1.9`（ssh 有完全磁盘访问）。待用户在 Mac 上给运行器（`~/actions-runner-signing`）完全磁盘访问，或允许该提示；之后的标签才能由流程自建 macOS 包。已修（2026-10-20，用户定不授权）：Mac 的共享引擎（`engines`、`reference`）移到 `~/ely-share`，“文稿”里的开发克隆与 CI 克隆都以链接指向它，`release.yml` 的 macOS `ELY_SHARE` 随之改，运行器不再读“文稿”
