# 测试样本

测试共用的样本。加了、改了样本，同步改本文件。

- media/clip.mp4：1 秒 H.264/AAC
- pdf/aes128.pdf：AES-128 加密，打开密码 user、所有者密码 owner
- pdf/signer.p12 与 signer.der：自签名测试证书，密码 ely
- pdf/chain-root.der 与 chain-leaf.der：测试用的根与其签发的证书
- show/sections.pptx：t.pptx 加两个章节（p14:sectionLst），sections-utf16.pptx 同而 presentation.xml 为 UTF-16LE，sections-locked.pptx 为其加密版，打开密码 ely，sections-locked-tail.pptx 同而目录移到文件末尾、`EncryptedPackage` 在最后一格
- tree/launcher-fails、launcher-held、launcher-pwd、launcher-session：ElyTree 终端启动器测试用的可执行脚本（Git 里 100755）
- git/git-2.39、git-2.40：只报版本的假 Git（版本下限测试），git-fails：报出版本后以 7 失败的假 Git
- pdf/stale-worker、crossed-worker：答错盖章与错号的假 PDF worker（皆 Git 里 100755：测试里现写现跑的程序会撞上别的测试 fork 出的子进程而报 ETXTBSY）
- table/kept.xlsx：照 Excel 写法的 XLSX，含 Calc 不认的函数（`CUBEVALUE`、`_xlfn.TRANSLATE`，一组共享公式）与其缓存值，kept-more.xlsx 含布尔、空串、缺值、坏数、数组（含各类成员）与易变函数，kept-more.ods 含属性顺序不同的空串、空串与坏数的矩阵成员，kept-strings.xlsx 含内联与共享字符串的数组成员、坏索引与用未知函数的定义名称，spill.xlsx 含 Excel 的动态数组（`cm` 指向 `xl/metadata.xml` 的 XLDAPR，其一此刻只占一格）、普通数组与块非动态的数组，spill-broken.xlsx 同而 metadata.xml 读不出，spill-empty.xlsx 同而 metadata.xml 为零字节，booleans.xlsx 含公式里裸的 `TRUE`、`FALSE`，date1904.xlsx 为 1904 日期系统（Excel for Mac 的 `date1904`）的序列号、`TEXT` 与 `DATE`，皆由同目录 make-kept.py 生成
- r-macro-demo.fods：R 宏的样本（首尾带空格的文字、前导零的文字、数字、逻辑值、空格与公式），rscript-fails、rscript-held、rscript-invalid：以 7 失败、一直不退、写出越界指令的假 Rscript（Git 里 100755，理由同 git-fails）
- pivot.xlsx：照 Excel 写法的紧凑透视表，无标题属性（Excel 显示 Row Labels、Column Labels、Grand Total），pivot-named.xlsx 同而三个标题属性为中文，pivot-blank.xlsx 行、列标题为空串，pivot-columns.xlsx 只有列字段、标题为中文，pivot-values.xlsx 同而两个值字段按行排（行区只有数据布局字段），pivot-values-named.xlsx 同而 `dataCaption` 为中文（数值），pivot-empty.xlsx 只有列字段而无值字段、旁边 H4 与下方 E5、G5 有用户的值，pivot-name-formula.xlsx、pivot-name-quote.xlsx、pivot-name-number.xlsx 同 pivot-columns.xlsx 而值字段名为 `=1+1`、`'Revenue`、`123`（照 Excel 存的文件），皆由同目录 make-pivot.py 生成
- links.xlsx：两张表的超链接，一个 `location` 自带 `#`（第三方导出工具的写法）、一个不带、一个外部网址，由同目录 make-links.py 生成
- office/backslash.docx、backslash.xlsx：腾讯文档式的反斜杠目录条目，broken.docx：名字含问号的坏包，malformed.docx：正文 XML 截断一半（内核在导入中问是否修复），equation.docx：公式里数学连字符（Word 显示为减号）与一段普通文字的连字符，toc.docx：内容控件里有标题的目录、域结束独占末段，toc-untitled.docx 同而无标题，toc-bare.docx 同而无内容控件（裸目录域），toc-native.fodt：Writer 自建的无标题目录，其后是标题与正文，toc-tail.docx 与 toc-tail-untitled.docx：有、无标题的目录，域结束后同段还有正文，toc-tail-entry.docx 与 toc-tail-entry-untitled.docx 同而域结束在末条目录里、其后同段是正文，toc-shape.docx 与 toc-shape-untitled.docx 同而其后同段只有浮动文本框，toc-inline.docx：目录条目在行内内容控件里，redline.docx：书签、指向它的 PAGEREF 与一处插入一处删除（打开后不得算已修改），includetext.docx：INCLUDETEXT 独占一段（带 `\* MERGEFORMAT`）、同段另有文字、跨两段（带 `\!`）各一处（其源 include.docx 由测试现写），ole-link.docx：Word 写法的链接工作表（LINK 域里 `Type="Link"` 的对象，外部目标 source.xlsx，预览图），ole-link-other.docx 同而为未知种类（`AcroExch.Document.DC`，Ely 只能存成图片），ole-link-other-locked.docx 为其由 Word 加密的版本（打开密码 ely），测试另以 Word 的绝对目标（`file:///C:\...`）现写 ole-link-word.docx，皆由同目录 make-packages.py 生成
- office/alias.ttf：DejaVu Sans 截到 ASCII、改名的字体，家族名英文为 Ely Alias Sans、简体中文为“别名黑体”（DejaVu 的许可要求改过的字体换名），由同目录 make-alias.py 以 DejaVuSans.ttf 生成；Linux 的字体测试把它放进 worker 配置目录的 `user/fonts`
- office/word.docx：Gitea #9 的 Word EQ 域（分数、根号、`\SU`、`\CO2`、读不通的域、`\s\do` 带与不带降低量）与随所在运行降低的图片，手写
- show/video.pptx：1 秒的 clip.mp4、其淡入与点击即播放暂停的触发序列（PowerPoint 的写法），由同目录 make-media.py 生成
- epub/sample.epub：两章、目录、中文与一张缺失的图片（MuPDF 排版时警告），由同目录 make-sample.py 生成
- mail/sample.eml：带编码主题、分组抄送、纯文本加 HTML、cid 图片与附件的邮件，LF 行尾，测试换成 CRLF
