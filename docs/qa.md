# QA-01 至 QA-28 的证据

对应 PRD §22.2。每项写今日库内能证明它的东西：测试（函数名与所在文件）、脚本、记录（任务号或 `docs/decisions.md` 的日期条）。标注：**全**，通过条件都有证据；**部分**，缺什么写明；**无**，库内没有证据。“自动”指单测或脚本，“实机”指在真窗口、真安装或真系统上跑过的记录。平台照记录写，记录没写的不写。2026-11-19 整理，未重跑。

## QA-01 安装与离线启动 — 部分

- 有的：`tests/scripts/apps-start.sh` 以调试构建起八个应用、看日志里的就绪行，即八个应用都打得开；不是干净机器、不是安装、不断网。
- 缺：干净测试机断网安装；八个应用打开的记录；无登录、CDN、外部文档服务器依赖的核对。C22.2 曾记“六个发行应用空闲时无网络套接字（`lsof -i`）”，记录未入库，已不在。

## QA-02 独立安装 — 部分

- macOS：`tests/scripts/release-macos.sh` 核对只在选了 Doc、Table 或 Show 时才选上隐藏的内核组件、各应用不自带内核；`tests/scripts/bundle-macos.sh` 核对 ElyDoc 包里无 `Frameworks`。依据 `packaging/macos/release.sh` 按是否直接依赖 `ely-lok` 定。
- Windows：`tests/scripts/windows-setup.sh` 以替身应用核对两个应用共用一份内核、第三个没有，最后一个文档应用卸掉时内核随之删除（`packaging/windows/installer.nsi`）。记录 SF.7。
- Doc 单独导出 PDF：只有仓库内的应用测试 `pdfs_export_a_range_and_locked_files_ask`（`apps/ely-doc/src/navigate/tests.rs`），不是只装 Doc 的安装。
- 缺：真机上只装 Mark、Tree 或 Agent 的安装；这些应用运行时不起 LOK 的核对（ElyMark 经 `ely-office` 依赖 `ely-lok`，未查它运行时是否起内核）。

## QA-03 统一界面 — 部分

- 内核对话框即关并报错：`a_kernel_dialog_is_closed_and_reported`（`crates/ely-lok/src/client/tests/commands.rs`）、`a_kernel_dialog_reaches_apps_as_closed`（`crates/ely-lok/src/view/tests/commands.rs`）、`writer_dialog_keys_leave_every_document_working`（`crates/ely-lok/src/client/tests/many.rs`）。
- “打开方式”里无 LibreOffice：`tests/scripts/lo-unregistered.sh`。
- 记录：C05.3、C05.4（macOS 实机），C07.4（Linux、Windows 未核）。
- 缺：按 §5 逐项核对菜单、属性、错误、快捷键、主题的整体检查，与三端的记录。

## QA-04 输入与焦点 — 部分

- 自动（模拟输入法回调）：`an_ime_composition_commits_once`、`a_cancelled_composition_leaves_nothing`（`crates/ely-lok/src/client/tests/input.rs`），`typing_and_composing_reach_the_document`、`the_platform_ime_callbacks_compose_in_utf16`（`crates/ely-lok/src/view/tests/input.rs`），`formula_caret_paint_and_ime_follow_the_same_scrolled_cell`（`crates/ely-lok/src/view/tests/text.rs`）。
- 实机：`tests/scripts/linux-xim.sh`（Linux 上 fcitx5 经 XIM，R14.48d）；`tests/scripts/lokview.sh`（macOS 真窗口）；C05.5 记 macOS 输入法候选确认。
- 缺：`docs/accessibility.md` 写明系统拼音输入法只手试过；三端“选文本、开工具栏、提交组合输入、撤销”整套的记录。

## QA-05 多屏 — 部分

- 自动：`tiles_paint_in_logical_pixels_at_2x`（`crates/ely-lok/src/view/tests.rs`）；`unchanged_formula_cursor_does_not_follow_zoom_or_geometry_changes`（`crates/ely-lok/src/view/tests/text.rs`，倍率 1、2、1 来回）；`a_show_covers_the_display_on_every_platform`、`a_start_whose_screen_is_gone_says_so_and_drops_its_copy`（`apps/ely-show/src/slideshow/start.rs`）。
- 缺：C05.5 记 100%、150%、200% 与跨屏缩放未核；C10.5 记两块屏与演讲者视图未测（测试机只有一块屏）。

## QA-06 保存失败 — 部分

- 写满磁盘：`a_full_disk_refuses_the_save_and_keeps_the_original`（`crates/ely-files/src/save/full.rs`，4 MB 磁盘映像，只在 macOS 跑）。只读目录：`a_failed_write_keeps_the_original`（`crates/ely-files/src/save/tests.rs`，Unix）。未保存状态留着：`a_failure_before_writing_shows_until_a_save_lands`（`crates/ely-files/src/session/tests/failed.rs`）、`a_failed_autosave_stays_dirty`（`crates/ely-files/src/policy/tests.rs`）、`a_kernel_write_failure_stays_visible`（`apps/ely-doc/src/tests/more.rs`）。
- 记录：C06.2（macOS），C20.2。
- 缺：目标盘断开；保存失败后恢复内容的核对。

## QA-07 外部冲突 — 部分

- 自动：`the_save_flow_checks_external_changes_first`、`overwrite_resolves_a_conflict_with_the_local_version`（`crates/ely-files/src/save/tests.rs`）；`an_outside_change_asks_and_keep_mine_overwrites`（`apps/ely-doc/src/tests.rs`）；`a_save_over_an_outside_change_asks_then_overwrites`（`apps/ely-mark/src/outside/tests/clash.rs`）；`an_outside_write_during_the_save_wins_and_asks`（`apps/ely-mark/src/outside/tests/saving.rs`）；`an_external_change_stops_autosave_for_good`（`crates/ely-files/src/policy/tests.rs`）。
- 记录：C06.2（macOS：冲突保留两版，重载、保留本地、比较三条出路）。
- 缺：真的外部程序覆盖的实机记录；Table、Show、PDF 的冲突测试。

## QA-08 并发保存 — 部分

- 自动：`an_old_ticket_never_marks_newer_edits_saved`（`crates/ely-files/src/session/tests/mod.rs`）、`an_edit_during_the_export_stays_unsaved`（`crates/ely-files/src/save/tests/write.rs`）、`a_save_leaves_newer_edits_their_due`（`crates/ely-files/src/policy/tests.rs`）；应用层 `apps/ely-doc/src/tests/overlap.rs`（关闭时的保存期间键入不丢）。
- 实机：`docs/perf/2026-11-04-AMD-Ryzen-9-9950X3D-16-Core-Processor-Windows-extreme.md`（Windows：键入 40 键期间 Doc 存 43 次、Table 42 次、Show 42 次、Mark 157 次，键都存下）；查的是不丢键，不是“仍显示未保存”。
- 记录：C06.1（macOS）、C06.4。
- 缺：真窗口里新修订仍显示未保存的核对；Linux 实机。

## QA-09 崩溃恢复 — 部分

- 实机（只 macOS）：`tests/scripts/recovery.sh` 杀应用与杀 LOK worker 后找回文字、不丢标签；`tests/scripts/pdf-recovery.sh` 杀 PDF worker、封存的恢复点。C06.5 第 4 轮 PASS（2026-11-04）。
- 自动：`a_recovery_copy_lands_beside_never_over`（`crates/ely-files/src/recovery/tests.rs`，不覆盖原件）；`a_failed_index_write_keeps_the_last_one`（`crates/ely-office/src/recover/tests.rs`）；`apps/ely-doc/src/tests/points.rs` 核对恢复提示写明光标与撤销历史不随恢复。
- Windows：`docs/perf/2026-11-04-AMD-Ryzen-9-9950X3D-16-Core-Processor-Windows-extreme.md` 杀引擎后文档以“未命名”副本重开。
- 记录：C06.3。
- 缺：`tests/scripts/recovery.sh` 从空白文档起，真窗口里“磁盘上的原件不动”未核；Windows、Linux 上杀应用的核对。

## QA-10 Doc — 部分

- 内核往返：`a_contract_keeps_numbering_comments_and_changes`、`a_long_report_keeps_headings_contents_and_a_split_table`（`crates/ely-lok/src/client/tests/roundtrip.rs`）。
- 独立读者：`a_docx_keeps_text_comments_and_tracking_across_sessions`（`crates/ely-lok/src/client/tests/independent.rs`，以 `tests/scripts/ooxml.py` 读，macOS 另以 `textutil`）；`docs/compatibility.md` 记三端过。只覆盖合同一例，不含长报告、目录、跨页表格。
- Word 往返：R15.11（`tests/office/compare.ps1`、`tests/office/make-word.ps1`、`tests/office/read-back.ps1`，Windows，Office 365），状态 `[~]`：编辑经内核示例而非 Ely 应用，目录页码变文字、分节符段落丢（R15.11c），第 3 轮修后未以 Office 重跑。
- 缺：长报告、目录、跨页表格、修订在独立应用里的往返；经 Ely 应用编辑的往返。

## QA-11 Table — 部分

- 函数与引用：`an_xlsx_keeps_formulas_and_results_across_sessions`（`crates/ely-lok/src/client/tests/independent.rs`，`tests/scripts/ooxml.py` 读；`docs/compatibility.md` 记三端过）。
- 日期：`a_1904_workbook_keeps_its_dates_through_saves`（`crates/ely-lok/src/client/tests/dates.rs`）。透视表：`crates/ely-lok/src/client/tests/pivot.rs`（`tests/scripts/ooxml_pivot.py` 读）。图表：`crates/ely-lok/src/client/tests/analysis.rs`。CSV：`the_preview_reads_what_the_kernel_imports`（`apps/ely-table/src/tests/output.rs`），比的是预览与内核所导入，不是外部预期文件。
- 未知公式：`unknown_functions_keep_the_files_results_until_an_input_changes`（`crates/ely-lok/src/client/tests/kept.rs`）：照决定 D-02（R14.37）显示文件里存的结果并列入横幅，输入变后即 `#NAME?`；C09.5 原写“即显示 `#NAME?`”已过时。
- 记录：C09.5（macOS）；Excel 往返属 R15.11（`[~]`，Windows；动态数组 R15.11a、透视表标签 R15.11d 有差异）。
- 缺：CSV 对外部预期结果的核对；Excel 往返的差异。

## QA-12 Show — 部分

- 自动：`a_pptx_keeps_notes_motion_and_media_editable`（`crates/ely-lok/src/client/tests/playlist.rs`）；母版 `crates/ely-lok/src/client/tests/show.rs`；图表 `a_scatter_keeps_editable_data_through_pptx`（`crates/ely-lok/src/client/tests/charts.rs`）；本地媒体 `embedded_media_goes_in_mid_slide_and_survives_a_save`（`crates/ely-lok/src/client/tests/media.rs`）；独立读者核对幻灯片、隐藏页与备注（`crates/ely-lok/src/client/tests/independent.rs`，`docs/compatibility.md` 记三端过）。
- 缺：多屏放映不露备注无证据，C10.5 记两块屏与演讲者视图未测。

## QA-13 PDF 编辑 — 部分

- 独立读者：`text_a_field_and_page_order_read_back_elsewhere`、`text_in_a_subset_font_reads_back_elsewhere`（`crates/ely-pdf-worker/tests/independent.rs`；macOS 以 PDFKit，别处以 lopdf；`docs/compatibility.md` 记三端过）。
- 失败不毁原对象：`bad_edits_are_refused_and_change_nothing`（`crates/ely-pdf-worker/tests/edit.rs`）、`a_failed_save_leaves_the_history_whole`（`crates/ely-pdf-worker/tests/sign.rs`）、`a_damaged_file_opens_as_a_repaired_copy_and_stays_as_it_was`（`crates/ely-pdf-worker/tests/output.rs`），以及 `apps/ely-pdf/src/tests/output.rs`。
- 记录：C11.8（macOS，第 4 轮 Windows 复核，`tasks/reviews/C11.8-r4.md` PASS）。
- 缺：改图后在独立读者里的核对（`docs/decisions.md` 2026-10-07 条与 C11.8 都写明未做）。

## QA-14 PDF 删除 — 部分

- 自动：`removed_text_is_gone_from_the_file_and_the_rest_stays`（`crates/ely-pdf-worker/tests/redact.rs`：经 worker 重开字已不在，解码后的文件字节里无此词，只一个 `startxref` 即无旧修订）；`a_picture_under_a_region_loses_those_pixels` 与同文件里结构树、链接、表单残留的各项。
- 记录：C11.6（macOS；Windows 第 4 轮过，R19.3）。
- 缺：文字核对经 worker 自己的 MuPDF 读，只有字节扫描独立于它；Linux 记录（`docs/decisions.md` 仍写 Windows、Linux 未核）。

## QA-15 PDF OCR 与签名 — 部分

- OCR：`a_scan_reads_and_its_text_layer_finds_without_drawing`、`chinese_reads_in_both_scripts`（`crates/ely-pdf-worker/tests/ocr.rs`），`ocr_reads_the_pages_then_lays_their_text_in_one_step`（`apps/ely-pdf/src/tests/structure.rs`）；记录 C11.5（macOS）。
- 完整性与信任：`a_signed_file_lists_its_checks_and_saves_as_a_copy`（`apps/ely-pdf/src/tests/sign.rs`，未受信任的证书写明），`a_changed_byte_shows_as_changed`、`a_second_signature_leaves_the_first_intact_and_marked_later`（`crates/ely-pdf-worker/tests/sign.rs`）；记录 C11.7（macOS，第 4 轮 Windows 复跑）。
- 吊销：界面恒有一行“未核验吊销状态”（`apps/ely-pdf/src/sign.rs`），`docs/decisions.md` 2026-11-04 C11.7 条定不做在线 OCSP、CRL。
- 系统信任库三端都有测试：`crates/ely-platform/src/trust.rs`（Windows 经 CryptoAPI，Linux 经系统 CA 库，R12.35），R14.21 记 macOS 八项、Windows 八项、Linux（WSL）九项皆过（`docs/decisions.md` 2026-10-27 R14.21 诸条）；C11.7 所写 Windows、Linux 未核已被它取代。
- 缺：没有测试断言“未核验吊销状态”那一行，没有断网下的运行。

## QA-16 Mail 离线 — 部分

- 自动：`offline_mail_reads_searches_keeps_drafts_and_queues`（`apps/ely-mail/src/tests/offline.rs`）：两个服务端口关着，已缓存的正文读得到，未缓存的附件不显示大小、取失败后仍无、打不开，搜索、草稿、入队（入发件箱、带错误）都照常。记录 C12.23（`tasks/reviews/C12.23-r2.md` PASS）。
- 缺：断网指端口不通，不是整机断网；平台未记。

## QA-17 Mail 发送 — 部分

- 发前失败：`a_refused_recipient_fails_and_is_never_sent_again`（`apps/ely-mail/src/tests/whole.rs`，RCPT 答 550，只连一次、不重试、状态失败）。接受后断开：`an_accepted_message_with_no_quit_answer_waits_only_for_its_copy`（同文件；服务端在 250 后挂住，不是断开）。发送中断：`a_send_cut_off_is_unknown_after_a_restart_and_never_resent`（`crates/ely-mail-engine/tests/outbox.rs`）。已发送副本失败：`a_queued_message_is_sent_once_then_filed`（同文件，只在存储层）。
- 记录：C12.14、C12.23。
- 缺：整个窗口里 IMAP 存“已发送”副本失败的测试；接受后真断开连接的测试。

## QA-18 Mail 同步 — 部分

- 自动：`a_new_uidvalidity_rebuilds_and_a_refused_login_keeps_local_mail`（`apps/ely-mail/src/tests/whole.rs`：新 UIDVALIDITY 重建邮箱、令牌失效写明、本地邮件与草稿不动）；`deletions_flags_and_dropped_folders_follow_the_server`、`a_new_uidvalidity_rebuilds_the_mailbox`、`an_examine_without_exists_does_not_delete_mail`（`crates/ely-mail-engine/tests/sync.rs`）。
- 记录：C12.6、C12.23。
- 缺：文件夹改名、移动（只测了删掉的文件夹）；服务端删除与文件夹变化时草稿仍在的核对。

## QA-19 Git 结果 — 部分

- 自动，界面操作后以 git 命令核对：`a_merge_conflict_is_settled_saved_resolved_and_continued`（`apps/ely-tree/src/tests/flows.rs`：未合并文件、工作区文件、合并提交两个父提交）；`apps/ely-tree/src/tests/recovery.rs`（索引 `diff --cached`；中断的变基放弃后 `rev-parse HEAD` 与 `status --porcelain`）。
- 记录：C13.6（macOS，随包 Git 2.56.0）。
- 缺：每次界面操作后对索引、提交、引用、工作区一并核对；实机记录。

## QA-20 Git 异常 — 部分

- 自动：冲突（`apps/ely-tree/src/tests/flows.rs`、`apps/ely-tree/src/tests/branching.rs`）；hook 失败（`apps/ely-tree/src/tests/flows.rs`、`apps/ely-tree/src/tests/history.rs`，只 Unix，只查提示）；凭据失败 `a_refused_credential_says_so`（`apps/ely-tree/src/tests/recovery.rs`，只查提示）；锁竞争与中断的变基（同文件）；取消（`apps/ely-tree/src/tests/place_flows.rs`：取消克隆不留文件夹）。
- 记录：C13.6（macOS：锁、凭据、变基）。
- 缺：hook、凭据、取消拉取之后的仓库状态核对；Windows 上的 hook 测试；实机记录。

## QA-21 Mark 原文 — 部分

- 自动：`unedited_text_writes_the_same_bytes`（`apps/ely-mark/src/text.rs`：CRLF、BOM、单独 CR、空文件、无末尾换行，编码层）；`switching_modes_writes_no_byte`、`an_edit_keeps_the_rest_of_the_file_byte_for_byte`（`apps/ely-mark/src/view/tests.rs`，源码模式编辑）。
- 记录：C14.4（`tasks/reviews/C14.4-r2.md`）。
- 缺：可视模式编辑的测试；带 front matter 与未知语法的样本；真保存后比对磁盘字节。

## QA-22 Mark 外部变更 — 部分

- 自动：原子替换 `a_replace_that_keeps_the_time_still_reloads`、`a_stale_read_never_lands`（`apps/ely-mark/src/outside/tests.rs`）；删除 `a_deleted_file_keeps_its_text_and_says_so`（同文件）、`a_deleted_file_shows_in_the_status_bar`（`apps/ely-mark/src/outside/tests/clash.rs`）；改名 `a_renamed_file_is_followed`（`apps/ely-mark/src/outside/tests.rs`）；并行编辑不覆盖未保存的 `an_outside_write_during_the_save_wins_and_asks`、`an_edit_after_the_copy_is_taken_stays_unsaved`（`apps/ely-mark/src/outside/tests/saving.rs`）；导出不写到打开的文件 `export_never_writes_over_the_open_file`（`apps/ely-mark/src/export/tests.rs`），导出原子写 `an_export_is_independent_and_never_overwrites`（`crates/ely-files/src/save/tests.rs`）。
- 记录：C14.11；C14.16 在 Windows、Linux（WSL）上冒烟，macOS 未核。
- 缺：文件边被改边导出时不截断的核对。

## QA-23 Agent 能力 — 无

- ElyAgent 只是一个空白窗口（`apps/ely-agent/src/main.rs`），属第二批；C15.1 至 C15.6 未做。

## QA-24 Agent 生命周期 — 无

- 同 QA-23：无会话与生命周期的实现与测试（C15.6 未做）。

## QA-25 隐私与断网 — 部分

- 有的：`tests/scripts/privacy.sh`（macOS：日志与诊断包里无文档正文）；更新只手动检查（C01.2b）；`plain_http_to_other_hosts_and_credentials_in_the_url_are_refused_before_any_request`（`crates/ely-platform/tests/update.rs`）。C22.2 曾记“空闲时无网络套接字”，记录已不在。
- 缺：抓测试机的流量，确认未授权时无文档、正文、日志、CDN、自动更新请求。

## QA-26 升级与卸载 — 部分

- 升级失败留旧版：`a_failed_windows_setup_leaves_a_note_and_still_reopens`、`an_appimage_that_cannot_be_replaced_leaves_a_note`（`crates/ely-platform/tests/install.rs`）；`tests/scripts/windows-installer.sh`（写不进即还原旧版、文件被占即不动）；记录 R6.11。
- 剩余应用：`tests/scripts/windows-setup.sh` 以替身应用核对卸掉一个应用后共享内核仍在；`tests/scripts/linux-packages.sh` 装卸 Linux 包。
- 卸载不删用户目录：只有规定（`docs/decisions.md`、`docs/standing.md`：卸载只动数据目录且先问）与 C01.2a 的说法，C01.2a 同时写明 Windows 安装与卸载未实跑。
- 缺：卸载后用户目录仍在的核对；卸载后其余真应用照常启动；macOS 卸载。

## QA-27 许可证 — 部分

- 构建即查：`packaging/licenses.sh`（缺许可或声明文件即停，`packaging/macos/bundle.sh`、`packaging/stage.sh` 调）。
- macOS 发布核对：`tests/scripts/release-macos.sh`（源码包完整、上游源码为打过补丁的 MuPDF，每个 `.app` 带 `LICENSE`、`rust-crates.txt` 与 MuPDF、字体的许可）。
- 其余：`tests/scripts/source-rules.sh`（外部库源码包按散列）、`tests/scripts/lo-tarballs.sh`、`tests/scripts/winget.sh`、`tests/scripts/nix.sh`（清单的许可字段）；记录 C21.1，清单 `docs/licenses.md`。
- 缺：Windows、Linux 安装包的同等核对；随包资源（图标、`packaging/windows/new` 的模板、字体）逐项有来源与许可的审计。

## QA-28 无障碍与性能 — 部分

- 性能：`docs/perf/2026-11-12-AMD-Ryzen-9-9950X3D-16-Core-Processor-Linux.md`、`docs/perf/2026-11-12-Mac16,10.md`、`docs/perf/2026-11-12-13th-Gen-Intel-R-Core-TM-i5-13500H-Windows.md`，方法见 `docs/perf/README.md`（`tests/perf/bench.py`、`tests/perf/cycles.py`、`tests/perf/idle.py`）；记录 C20.1，R18.30（`[!]`）。内存为应用与其子进程之和（`tests/perf/run.py`），登录后起的引擎另由 `tests/perf/idle.py` 量。
- 无障碍：`docs/accessibility.md`（Windows、Linux 2026-10-27，macOS 2026-10-30，缺口列明）；实机脚本 `tests/scripts/windows-a11y.sh`、`tests/scripts/linux-a11y.sh`、`tests/scripts/windows-names.sh`、`tests/scripts/linux-names.sh`、`tests/scripts/macos-names.sh`；记录 C20.3。
- 缺：报告只记每个应用的总内存，不分子进程，也无各子进程的 CPU；指标与键盘、辅助技术检查合成一份 QA-28 报告。
