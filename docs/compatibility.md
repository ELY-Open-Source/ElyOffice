# 兼容清单

随发行说明发布，每个版本一份。支持某格式不等于支持它的全部功能；下表只写已验证的内容、已知限制与保存行为。

版本：Ely Office 0.1.11（首批七个应用：ElyDoc、ElyTable、ElyShow、ElyPDF、ElyTree、ElyMark、ElyMail）。日期 2026-10-23。
内核：LibreOffice 26.2.6.3（8221e31，加 `patches/libreoffice` 0001–0056）；MuPDF 1.28.5（8ad45e9，加 `patches/mupdf` 0001–0005）；Git 2.56.0（a018953）与 git-lfs 3.8.0。
验证机：macOS 27.0 arm64。Windows 11 上跑单测与部分真窗口核对；Linux 的 deb、rpm、Arch 包在 Debian 11、Ubuntu 26.04、Fedora 44、Arch 容器里装上并核对每个 ELF 的库，glibc 不高于 2.31。下面的往返验证只在 macOS 做过。本机没有 Microsoft Office：与 Office 的交叉检查未做（待做）。

## 往返验证

每份样本在 Ely 中打开、编辑、保存、关闭、再打开、再编辑、再保存，最后由不经 LibreOffice 的读取器检查（`tests/scripts/ooxml.py`：自己解 OOXML，预期值由它自己算；DOCX 另交 macOS 的 `textutil` 读）。用例在 `crates/ely-lok/src/client/tests/independent.rs`。

| 用例 | 检查 | 结果 |
| --- | --- | --- |
| DOCX 合同 | 两次会话键入的文字（含中文）、批注、修订记录的插入、修订开关保留；`textutil` 读得出文字 | 通过 |
| XLSX 公式 | 公式文本与缓存结果：整数、日期序号、`#DIV/0!` 错误值、文本严格相等，浮点按用例声明的 1e-12 容差；第二次会话的公式 | 通过 |
| PPTX 演示 | 三张幻灯片、隐藏的那张仍隐藏、备注文字（含中文） | 通过 |

打印：macOS 经 PDFKit 的打印面板，Windows 经系统打印对话框（真窗口核对到 Microsoft Print to PDF），Linux 经桌面门户（只对替身门户核对，真桌面未核对）；纸张取文档的，按原大印。打印机没有该纸张、或页面放不下所选纸张时报错，不裁切。Linux 由 Ely 照对话框的选择排页（页码范围、奇偶页、倒序、份数与逐份、缩放、每张页数），只交出 PDF。

字体：缺失字体由内核报出，ElyDoc 页面上方列出缺失的字体（`doc.fonts.missing`），分页可能与有字体时不同；字体一致与缺失两种环境的对照未做。产品不带微软字体。

## ElyDoc

| | 格式 |
| --- | --- |
| 编辑与保存 | DOCX、ODT（保存保留原格式；另存为默认 DOCX） |
| 导入或受限 | DOC、RTF 按原格式保存；含宏的文件（docm、dotm、带 Basic 的 ODF/OLE）进风险模式，只能另存副本，宏不运行 |
| 输出 | PDF（页码范围、批注、修订标记）、打印 |

已验证：文字与段落格式、样式、页面与分节、页眉页脚、表格、图片锚点与环绕、目录、脚注尾注、书签、交叉引用、题注、公式（含 Word 的 EQ 域，转为公式对象）、批注与回复、修订、查找替换、拼写（英文）、比较、邮件合并。限制：交叉引用只指向书签；公式导出 DOCX 后在 Word 中是否可编辑未验证。

## ElyTable

| | 格式 |
| --- | --- |
| 编辑与保存 | XLSX、ODS（保存保留 xlsx、ods、xls、fods；另存为默认 XLSX） |
| 导入或受限 | XLS；CSV、TSV、TXT 经导入对话框（编码、分隔符、引号、逐列类型）成为新工作簿，原文件不动；xlsm、xltm 进风险模式 |
| 输出 | PDF（全部工作表，按页面设置）、CSV 与 TSV（当前工作表的显示值，UTF-8）、打印 |

公式语法为 Excel A1。XLSX 与非 LibreOffice 生成的 ODF 载入时总是重算。限制：不支持的函数显示 `#NAME?`，原公式保留，缓存值不保留；外部链接不刷新，显示保存时的值；1904 日期系统未专门验证。

## ElyShow

| | 格式 |
| --- | --- |
| 编辑与保存 | PPTX、ODP（另存为默认 PPTX） |
| 导入或受限 | PPT；章节（`p14:sectionLst`）内核不保留；媒体默认嵌入 |
| 输出 | PDF（幻灯片或讲义 1、2、3、4、6、9 张一页）、PNG（每张 1920 宽）、打印 |

限制：Ely 提供一组基础切换与动画，文件自带的其他效果照原样保留并显示内核名称；批注不做回复（Impress 无回复结构）；Linux 的媒体播放未构建（macOS 以 AVFoundation、Windows 以 Media Foundation 播放）。

## ElyPDF

| | 范围 |
| --- | --- |
| 编辑与保存 | 页面增删移转裁剪、文字与图片改动、批注、表单字段（文本、复选、单选、选择框）、书签、链接、属性、附件、OCR 文字层、内容删除与清理、密码与权限、数字签名 |
| 导入或受限 | XFA 表单不支持；已签名文件另存为未签名副本；修复过的文件只能另存；多选列表框按单选填写 |
| 输出 | PDF、页面图片（PNG 72/150/300 dpi）、文本、优化的 PDF、打印 |

保存经 MuPDF 写快照后紧缩写出；签名经增量更新写出并重开核对摘要。独立阅读器 PDFKit 读回页序、改动的文字与字段值（`crates/ely-pdf-engine/tests/independent.rs`）。限制：证书吊销状态不核验；Windows 与 Linux 的证书信任未构建。

## ElyTree

标准本地 Git 仓库，随包 Git 2.56.0。浅克隆、LFS（只显示状态，不下载对象）、子模块（显示状态与检出记录的提交）照 Git 的限制显示。Ely 写的一切都是 Git 原生提交与引用；不提供交互式变基与硬重置。

## ElyMark

| | 格式 |
| --- | --- |
| 编辑与保存 | MD、Markdown（UTF-8，带不带 BOM；保留换行风格；文本即原文，只写回所改的字） |
| 导入或受限 | UTF-16 与非 UTF-8 拒绝打开并指出字节位置；markdown-rs 不认的语法照原文显示；超过 2 MiB 只以源码打开 |
| 公式 | `$…$`、`$$…$$` 与 `\(…\)`、`\[…\]`，经 RaTeX 排版（行内与块） |
| 输出 | HTML（自包含或带资源文件夹）、PDF、纯文本、富文本复制、打印 |

## ElyMail

| | 格式 |
| --- | --- |
| 协议 | IMAP4rev1（有 UIDPLUS、MOVE、SPECIAL-USE、LIST-EXTENDED 即用）、POP3、SMTP（8BITMIME、SMTPUTF8、SIZE）；Microsoft 走 OAuth，Gmail 与 iCloud 走应用专用密码；随系统代理（HTTP CONNECT、SOCKS5，不支持 PAC） |
| 读 | MIME（嵌套至多 64 层）；纯文本与 HTML 按块的安全子集画出，CSS 不读，SVG 图片不显示；`.eml` 只读查看 |
| 导入或受限 | MBOX（mboxrd）导入为本机文件夹，只在本机；vCard 2.1、3.0、4.0（UTF-8）；只报 IMAP4rev2 的服务器读不通 |
| 输出 | 写信发纯文本，或纯文本加 HTML；MBOX（mboxrd，只写完整的邮件：缺附件、原始二进制、末尾无换行的不导出）、EML、vCard 3.0 |

## 第二批

ElyAgent 不在首批，验证随第二批补上。
