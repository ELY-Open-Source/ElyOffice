# 功能编号与测试对照

对应 PRD §22。由 `scripts/test-map.py` 从 PRD、`tasks/TASKS.md` 与 `docs/decisions.md` 生成：每个编号列出引用它的任务与决策，其中写明、且在代码里确有其名的测试（单测函数名或 `tests/scripts` 的脚本），以及该领域测试所在之处。Agent 属第二批，不列。

| 编号 | 要求 | 任务 | 记下的测试 | 测试所在 |
| --- | --- | --- | --- | --- |
| UX-01 | 文件菜单中的新建、打开、保存、另存为、导出和打印使用统一命名 | C05.3 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-02 | 相同操作使用相同快捷键语义；macOS 使用 Command，其他平台使用 Ctrl | C05.3 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-03 | Tab 顺序可预测，焦点可见；Esc 关闭当前浮层，不连带退出文档 | C05.4 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-04 | 禁用操作说明原因；混合选择显示“混合”，不冒用首个对象属性 | C05.4 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-05 | 长任务显示范围、状态、错误和可用的取消操作 | C05.4 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-06 | 失败提示保留用户输入，并提供重试或另存路径 | C05.4 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-07 | 危险操作说明具体对象和影响，不只显示“确定吗” | C05.4 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-08 | 支持简体中文与英文；错误正文不用未解释的内部错误码替代 | C05.5 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-09 | 中文输入法组合输入、候选框、撤销与跨区域焦点一致 | C05.5 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-10 | 多屏缩放变化不造成点击偏移、选区漂移或浮层遮挡 | C05.5 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-11 | 正文、标签、错误和状态不只通过颜色表达 | C05.5 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| UX-12 | 使用系统辅助技术所需的名称、角色、状态和文本信息 | C05.5 | — | crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh |
| FILE-01 | 新建文件在首次保存前没有隐含目标路径；关闭前处理未保存内容 | C06.1 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-02 | 显示加载中、已保存、未保存、保存中、只读、外部已修改、保存失败等状态 | C06.1 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-03 | 保存任务记录对应修订号；旧修订保存完成不得把更新的编辑标为已保存 | C06.1 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-04 | 写入临时文件并检查写入结果，再替换目标；失败保留原文件与未保存内容 | C06.2 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-05 | 保存前检查外部修改；冲突提供重新加载、保留本地副本和比较，不静默覆盖 | C06.2 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-06 | 导出默认生成独立文件，不改变正在编辑文件的格式和路径 | C06.2 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-07 | 复制、粘贴、拖放和附件导入失败时不删除原始数据 | C06.2 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-08 | 保存、导出、打印都针对可识别的文档修订，不输出一半新一半旧的内容 | C06.2 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-09 | 路径支持空格、中文、非 ASCII 字符和长文件名；保留原有大小写 | C06.2 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| FILE-10 | 对损坏、加密、缺少字体和不支持的内容给出可执行提示 | C06.2 | — | crates/ely-files、crates/ely-office、recovery.sh、autosave.sh |
| LOK-01 | 使用随应用交付的固定 LOK 构建，隔离用户已安装的 LibreOffice 配置 | C07.1（!） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-02 | 每项界面操作有明确的内核命令、参数、结果和状态回调映射 | C07.4（~） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-03 | 处理内容区域失效通知，只更新必要图块；旧修订的画面不得覆盖新画面 | C07.2（~） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-04 | 统一转换文档坐标、视图缩放、滚动和平台 DPI；输入与选区使用同一坐标规则 | C07.2（~） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-05 | 字体、字号、样式、选择对象、修改状态和撤销可用性以内核为准 | C07.4（~） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-06 | 复制粘贴覆盖纯文本、富文本、图片和表格；不只实现键盘事件转发 | C07.3（~） | `lokview.sh` | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-07 | 中文输入法包含组合开始、更新、提交和取消；不拆成普通按键模拟输入 | C07.3（~） | `lokview.sh` | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-08 | 页面设置、图表、样式、批注等需要的对话框由 Ely 实现；上游原生对话框不得泄露 | C07.4（~） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-09 | 保存与导出只通过内核支持的过滤器；错误码不能被当成成功 | C07.1（!） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-10 | 关闭文档释放内核对象、图块和临时文件；不终止其他应用的会话 | C07.1（!） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-11 | 不执行宏和自动更新外部链接；打开后明确显示这些内容的处理状态 | C07.1（!） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| LOK-12 | 维护功能到接口的映射表和三端测试，不以“LOK 本身支持”作为交付证据 | C07.4（~） | — | crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh |
| DOC-01 | 新建与模板 | C08.1（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-02 | 文字编辑 | C08.2（~） | `doc-props.sh` | apps/ely-doc/src/tests、doc-props.sh |
| DOC-03 | 段落 | C08.2（~） | `doc-props.sh` | apps/ely-doc/src/tests、doc-props.sh |
| DOC-04 | 样式 | C08.2（~） | `doc-props.sh` | apps/ely-doc/src/tests、doc-props.sh |
| DOC-05 | 页面与分节 | C08.3（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-06 | 页眉页脚 | C08.3（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-07 | 表格 | C08.3（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-08 | 图片与对象 | C08.3（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-09 | 引用结构 | C08.4（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-10 | 公式 | C08.4（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-11 | 审阅 | C08.4（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-12 | 查找与检查 | C08.5（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-13 | 文档比较 | C08.5（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-14 | 邮件合并文档 | C08.5（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-15 | 打印与导出 | C08.6（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| DOC-16 | 长文档导航 | C08.6（~） | — | apps/ely-doc/src/tests、doc-props.sh |
| TABLE-01 | 工作簿 | C09.1（~）、决策 C09.1 | `a11y_follows_the_focused_paragraph` | apps/ely-table/src/tests、theme.sh |
| TABLE-02 | 区域编辑 | C09.1（~） | `a11y_follows_the_focused_paragraph` | apps/ely-table/src/tests、theme.sh |
| TABLE-03 | 公式输入 | C09.2（~）、决策 C09.2 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-04 | 公式计算 | C09.2（~）、决策 C09.2 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-05 | 计算控制 | C09.2（~）、决策 C09.2 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-06 | 名称与引用 | C09.2（~）、决策 C09.2 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-07 | 格式 | C09.3（~）、决策 C09.3 | `typing_during_a_closing_save_is_never_lost`、`a_format_code_lands_in_the_workbook_that_asked`、`two_tabs_saving_as_one_path_keep_one_owner`、`interleaved_previews_keep_their_sources`、`closing_a_clean_tab_never_waits_for_the_worker` | apps/ely-table/src/tests、theme.sh |
| TABLE-08 | 数据整理 | C09.3（~）、决策 C09.3 | `typing_during_a_closing_save_is_never_lost`、`a_format_code_lands_in_the_workbook_that_asked`、`two_tabs_saving_as_one_path_keep_one_owner`、`interleaved_previews_keep_their_sources`、`closing_a_clean_tab_never_waits_for_the_worker` | apps/ely-table/src/tests、theme.sh |
| TABLE-09 | 规则 | C09.3（~）、决策 C09.3 | `typing_during_a_closing_save_is_never_lost`、`a_format_code_lands_in_the_workbook_that_asked`、`two_tabs_saving_as_one_path_keep_one_owner`、`interleaved_previews_keep_their_sources`、`closing_a_clean_tab_never_waits_for_the_worker` | apps/ely-table/src/tests、theme.sh |
| TABLE-10 | 汇总分析 | C09.4（~）、决策 C09.4 | `undoing_a_new_kind_brings_the_whole_chart_back`、`enter_and_tab_move_the_cell_cursor`、`a_read_layout_applies_back_unchanged`、`a_dialog_asked_from_a_workbook_left_behind_is_dropped` | apps/ely-table/src/tests、theme.sh |
| TABLE-11 | 图表 | C09.4（~）、决策 C09.4 | `undoing_a_new_kind_brings_the_whole_chart_back`、`enter_and_tab_move_the_cell_cursor`、`a_read_layout_applies_back_unchanged`、`a_dialog_asked_from_a_workbook_left_behind_is_dropped` | apps/ely-table/src/tests、theme.sh |
| TABLE-12 | CSV/TSV | C09.5（~）、决策 C09.5 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-13 | 导出 | C09.5（~）、决策 C09.5 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-14 | 打印 | C09.5（~）、决策 C09.5 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-15 | 保护 | C09.5（~）、决策 C09.5 | — | apps/ely-table/src/tests、theme.sh |
| TABLE-16 | 大数据浏览 | C09.1（~）、C09.5（~）、决策 C09.5 | `a11y_follows_the_focused_paragraph` | apps/ely-table/src/tests、theme.sh |
| SHOW-01 | 幻灯片管理 | C10.1（~）、决策 C10.1 | — | apps/ely-show/src/tests |
| SHOW-02 | 模板与母版 | C10.1（~）、决策 C10.1 | — | apps/ely-show/src/tests |
| SHOW-03 | 文字 | C10.2（~）、决策 C10.2 | `lo-patches.sh` | apps/ely-show/src/tests |
| SHOW-04 | 图形 | C10.2（~）、决策 C10.2 | `lo-patches.sh` | apps/ely-show/src/tests |
| SHOW-05 | 图片 | C10.2（~）、决策 C10.2 | `lo-patches.sh` | apps/ely-show/src/tests |
| SHOW-06 | 对象操作 | C10.2（~）、决策 C10.2 | `lo-patches.sh` | apps/ely-show/src/tests |
| SHOW-07 | 表格与图表 | C10.3（~）、决策 C10.3 | `a_retitled_scatter_keeps_its_lines`、`a_scatter_with_words_for_x_keeps_its_draft_and_names_the_cell`、`a_comment_picked_on_another_slide_shows_its_slide_again` | apps/ely-show/src/tests |
| SHOW-08 | 备注与审阅 | C10.3（~）、决策 C10.3 | `a_retitled_scatter_keeps_its_lines`、`a_scatter_with_words_for_x_keeps_its_draft_and_names_the_cell`、`a_comment_picked_on_another_slide_shows_its_slide_again` | apps/ely-show/src/tests |
| SHOW-09 | 动画与切换 | C10.4（~）、决策 C10.4 | `media-targets.sh`、`typing_ends_a_preview_before_any_undo_step`、`two_fields_sent_before_a_read_both_land`、`an_undone_duration_is_not_sent_again`、`two_effects_take_the_same_value_before_a_read` | apps/ely-show/src/tests |
| SHOW-10 | 媒体 | C10.4（~）、决策 C10.4 | `media-targets.sh`、`typing_ends_a_preview_before_any_undo_step`、`two_fields_sent_before_a_read_both_land`、`an_undone_duration_is_not_sent_again`、`two_effects_take_the_same_value_before_a_read` | apps/ely-show/src/tests |
| SHOW-11 | 放映 | C10.5（~）、决策 C10.5 | — | apps/ely-show/src/tests |
| SHOW-12 | 多屏 | C10.5（~）、决策 C10.5 | — | apps/ely-show/src/tests |
| SHOW-13 | 输出 | C10.6（~）、决策 C10.6 | — | apps/ely-show/src/tests |
| SHOW-14 | 演示准备 | C10.6（~）、决策 C10.6 | — | apps/ely-show/src/tests |
| PDF-01 | 阅读 | C11.1（~）、决策 C11.1 | `pdf-idle.sh` | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-02 | 页面组织 | C11.2（~）、决策 C11.2 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-03 | 文字编辑 | C11.3（~）、决策 C11.3 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-04 | 图像与对象 | C11.3（~）、决策 C11.3 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-05 | 批注 | C11.4（~）、决策 C11.4 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-06 | 表单 | C11.4（~）、决策 C11.4 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-07 | 文档结构 | C11.5、决策 C11.5 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-08 | OCR | C11.5、决策 C11.5 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-09 | 内容删除 | C11.6（~）、决策 C11.6 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-10 | 文档清理 | C11.6（~）、决策 C11.6 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-11 | 密码 | C11.7（~）、决策 C11.7 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-12 | 签名 | C11.7（~）、决策 C11.7 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-13 | 输出 | C11.8（~）、决策 C11.8 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| PDF-14 | 文件恢复 | C11.8（~）、决策 C11.8 | — | apps/ely-pdf/src/tests、crates/ely-pdf-worker/tests、pdf-recovery.sh |
| MAIL-01 | 账号 | C12.1（~）、C12.5（~）、C12.7 | `a_failed_removal_puts_the_secret_back`、`two_saves_in_flight_add_an_address_once`、`a_uidl_listed_twice_comes_down_once` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-02 | 授权 | C12.4、C12.5（~） | `a_failed_removal_puts_the_secret_back`、`two_saves_in_flight_add_an_address_once` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-03 | 邮箱视图 | C12.9 | `remove_needs_an_open_account`、`picking_every_one_of_many_messages_is_quick`、`the_side_scrolls_past_many_folders_to_the_next_account`、`a_rebuilt_mailbox_drops_the_pick`、`an_upgrade_leaving_a_row_without_its_parent_fails` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-04 | 阅读 | C12.2（~）、C12.10（~）、C12.11、决策 C12.10、决策 C12.11 | `mail_never_draws_svg`、`hidden_list_items_and_hidden_parts_of_code_stay_out`、`a_message_with_only_headers_says_so`、`a_body_that_arrives_replaces_the_headers_shown`、`a_flagged_headers_only_v5_message_upgrades_and_stays_writable`、`an_important_hidden_style_in_any_case_stays_out`、`mixed_bodies_read_in_order_and_alternatives_once`、`every_body_reads_in_order_with_pictures_once`、`every_body_of_a_mixed_letter_shows`、`a_trust_change_reaches_a_reply_still_being_read`、`a_damaged_letter_fails_alone` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-05 | 写信 | C12.2（~）、C12.12（~）、决策 C12.12 | — | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-06 | 草稿 | C12.13（~）、决策 C12.13 | `a_new_sender_hands_back_the_old_accounts_copy`、`deleting_while_files_load_waits_for_them`、`a_file_unread_while_closing_keeps_the_window`、`spacing_alone_is_an_edit_and_spaces_alone_are_blank`、`deleting_a_draft_while_it_reopens_leaves_no_window`、`a_server_copy_failing_after_its_window_closed_says_so_here` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-07 | 发送 | C12.14（~）、决策 C12.14 | — | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-08 | 组织 | C12.15（~）、决策 C12.15 | `a_copy_kept_before_the_source_goes_is_not_made_twice`、`a_move_without_uids_finds_it_by_message_id_for_a_newer_change`、`a_newer_change_to_a_move_lost_to_view_is_told`、`server_flags_read_mid_change_keep_the_change`、`a_body_downloaded_mid_change_keeps_the_change`、`a_refused_flag_is_let_go_with_the_server_words`、`a_refused_flag_keeps_a_newer_one`、`a_refusal_outlives_a_broken_logout`、`what_is_known_is_kept_when_a_later_step_breaks`、`a_change_while_paused_counts_as_waiting`、`delete_goes_to_trash_and_from_trash_for_good`、`pop_mail_changes_here_only_and_stays_deleted`、`a_refused_flag_unread_back_returns_to_the_servers_last`、`a_refusal_cut_short_is_not_undone_yet`、`a_refused_erase_brings_the_message_back`、`an_erase_the_server_cannot_do_brings_it_back`、`an_erased_message_goes_once_the_server_expunges_it`、`a_destination_no_longer_listed_lets_the_move_go_and_keeps_the_flag` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-09 | 搜索 | C12.16、决策 C12.16 | `an_original_parked_by_a_crash_returns_before_reindexing`、`a_chosen_range_fits_a_narrow_window`、`plain_and_html_bodies_are_both_searched`、`upgrading_indexes_html_beside_plain_text`、`a_found_message_moves_within_its_own_account` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-10 | 附件 | C12.17（~）、决策 C12.17 | `a_file_named_with_a_path_opens_inside_its_folder`、`written_bytes_stay_in_their_folder`、`a_message_removed_meanwhile_stays_removed`、`a_folder_rebuilt_meanwhile_gets_no_old_message`、`a_file_name_is_never_a_command`、`a_file_that_runs_code_is_marked_and_never_opened`、`an_attached_message_root_waits_whole`、`a_whole_message_attached_waits_and_downloads` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-11 | 规则 | C12.18（~）、决策 C12.18 | `mail_a_rebuild_brings_back_is_no_news`、`an_archive_rule_needs_an_archive_and_never_blocks_the_others`、`rules_move_only_to_server_folders` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-12 | 联系人 | C12.19（~）、决策 C12.19 | `comments.py`、`only_the_four_escapes_read`、`one_message_counts_an_address_once` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-13 | 迁移 | C12.20（~）、决策 C12.20 | `rules_move_only_to_server_folders`、`a_read_failure_names_the_message_and_leaves_no_folder`、`raw_binary_mail_and_gone_folders_never_export` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-14 | 同步控制 | C12.6（~）、C12.8 | `a_v1_store_gains_sync_and_keeps_its_mail`、`an_attached_multipart_waits_whole`、`a_related_image_marked_attachment_comes_down`、`a_related_start_root_ignores_its_disposition`、`a_missing_tls_is_the_sync_failure`、`a_related_start_may_name_a_multipart`、`two_saves_in_flight_add_an_address_once` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| MAIL-15 | 通知 | C12.21（~）、决策 C12.21 | `mail_delivered_while_away_is_news_whatever_its_date`、`a_rebuild_without_message_ids_is_no_news`、`a_new_mail_notice_clears_the_filters`、`a_rebuild_tells_unnamed_mail_apart_by_its_body`、`a_rebuild_knows_unnamed_mail_however_it_was_kept`、`upgrading_gives_old_mail_the_identity_new_mail_gets`、`an_earlier_switch_saved_late_leaves_the_latest` | apps/ely-mail/src/tests、crates/ely-mail-engine/tests |
| TREE-01 | 仓库 | C13.1、决策 C13.1 | — | apps/ely-tree/src/tests |
| TREE-02 | 工作区 | C13.1、决策 C13.1 | — | apps/ely-tree/src/tests |
| TREE-03 | 差异 | C13.2（~）、决策 C13.2 | — | apps/ely-tree/src/tests |
| TREE-04 | 暂存 | C13.2（~）、决策 C13.2 | — | apps/ely-tree/src/tests |
| TREE-05 | 提交 | C13.3（~）、决策 C13.3 | — | apps/ely-tree/src/tests |
| TREE-06 | 历史 | C13.3（~）、决策 C13.3 | — | apps/ely-tree/src/tests |
| TREE-07 | 分支与标签 | C13.4（~）、决策 C13.4 | — | apps/ely-tree/src/tests |
| TREE-08 | 合并与变基 | C13.4（~）、决策 C13.4 | — | apps/ely-tree/src/tests |
| TREE-09 | 冲突 | C13.4（~）、决策 C13.4 | — | apps/ely-tree/src/tests |
| TREE-10 | 暂存工作 | C13.5、决策 C13.5 | — | apps/ely-tree/src/tests |
| TREE-11 | 远程 | C13.5、决策 C13.5 | — | apps/ely-tree/src/tests |
| TREE-12 | 仓库结构 | C13.5、决策 C13.5 | — | apps/ely-tree/src/tests |
| TREE-13 | 恢复与清理 | C13.6（~）、决策 C13.6 | — | apps/ely-tree/src/tests |
| TREE-14 | 外部工具 | C13.6（~）、决策 C13.6 | — | apps/ely-tree/src/tests |
| MARK-01 | 本地文件 | C14.4、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-02 | 编辑模式 | C14.6、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-03 | Markdown | C14.7、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-04 | 扩展语法 | C14.2、C14.7、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-05 | 导航 | C14.8、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-06 | 编辑工具 | C14.9、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-07 | 附件 | C14.10、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-08 | 外部修改 | C14.11、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-09 | 书写视图 | C14.12、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-10 | 搜索 | C14.13、C14.16、决策 C14.4 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-11 | 输出 | C14.14、C14.16、决策 C14.4 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-12 | 会话恢复 | C14.15、C14.16、决策 C14.4 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-13 | Git 配合 | C14.16、决策 C14.4 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-14 | 大文件 | C14.6、C14.16 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-15 | 格式命令 | 无 | — | apps/ely-mark 单测、mark-*.sh |
| MARK-16 | 右键菜单 | 无 | — | apps/ely-mark 单测、mark-*.sh |
| PERF-01 | 启动 | C20.1（~）、决策 C20.1、决策 C11.1 | `chord.swift`、`pdf-idle.sh` | tests/perf/bench.py、lag.sh |
| PERF-02 | 交互 | C20.1（~）、决策 C20.1 | `chord.swift` | tests/perf/bench.py、lag.sh |
| PERF-03 | 文档打开 | C20.1（~）、决策 C20.1 | `chord.swift` | tests/perf/bench.py、lag.sh |
| PERF-04 | 表格打开 | C20.1（~）、决策 C20.1 | `chord.swift` | tests/perf/bench.py、lag.sh |
| PERF-05 | PDF 浏览 | C20.1（~）、决策 C20.1 | `chord.swift` | tests/perf/bench.py、lag.sh |
| PERF-06 | 本地搜索 | C12.16、C12.23（ ）、C20.1（~）、决策 C20.1、决策 C12.16 | `an_original_parked_by_a_crash_returns_before_reindexing`、`a_chosen_range_fits_a_narrow_window`、`plain_and_html_bodies_are_both_searched`、`upgrading_indexes_html_beside_plain_text`、`a_found_message_moves_within_its_own_account`、`release-macos.sh`、`chord.swift` | tests/perf/bench.py、lag.sh |
| PERF-07 | Markdown | C14.5、C14.6、C20.1（~）、决策 C20.1、决策 C14.4 | `chord.swift` | tests/perf/bench.py、lag.sh |
| PERF-08 | 资源释放 | C20.1（~）、决策 C20.1 | `chord.swift` | tests/perf/bench.py、lag.sh |
| PERF-09 | 多会话 | C20.1（~） | `chord.swift` | tests/perf/bench.py、lag.sh |
| QA-01 | 安装与离线启动 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-02 | 独立安装 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-03 | 统一界面 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-04 | 输入与焦点 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-05 | 多屏 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-06 | 保存失败 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-07 | 外部冲突 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-08 | 并发保存 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-09 | 崩溃恢复 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-10 | Doc | C08.6（~）、C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-11 | Table | C09.5（~）、C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-12 | Show | C10.6（~）、C22.2（ ）、决策 C10.6 | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-13 | PDF 编辑 | C11.8（~）、C22.2（ ）、决策 C11.8 | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-14 | PDF 删除 | C11.6（~）、C22.2（ ）、决策 C11.6 | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-15 | PDF OCR 与签名 | C11.7（~）、C22.2（ ）、决策 C11.7 | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-16 | Mail 离线 | C12.23（ ）、C22.2（ ） | `release-macos.sh` | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-17 | Mail 发送 | C12.14（~）、C12.23（ ）、C22.2（ ）、决策 C12.14 | `release-macos.sh` | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-18 | Mail 同步 | C12.6（~）、C12.23（ ）、C22.2（ ）、决策 C12.13 | `a_v1_store_gains_sync_and_keeps_its_mail`、`an_attached_multipart_waits_whole`、`a_related_image_marked_attachment_comes_down`、`a_related_start_root_ignores_its_disposition`、`a_missing_tls_is_the_sync_failure`、`a_related_start_may_name_a_multipart`、`release-macos.sh` | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-19 | Git 结果 | C13.6（~）、C22.2（ ）、决策 C13.6 | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-20 | Git 异常 | C13.6（~）、C22.2（ ）、决策 C13.6 | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-21 | Mark 原文 | C14.4、C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-22 | Mark 外部变更 | C14.11、C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-23 | Agent 能力 | C15.6（ ）、C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-24 | Agent 生命周期 | C15.6（ ）、C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-25 | 隐私与断网 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-26 | 升级与卸载 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-27 | 许可证 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
| QA-28 | 无障碍与性能 | C22.2（ ） | — | 无库内证据：C22.2 的记录已移除，须重做 |
