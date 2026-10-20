# 许可与源码

这是发布检查清单，不替代对最终发行包的法律审查。

## 自有代码

Ely Office 的自有代码以 MIT 发布，全文在仓库根的 `LICENSE`，并随每个应用包放在 `Resources/Licenses/LICENSE`。随包的引擎不随之改许可：ElyPDF 链接的 MuPDF 是 AGPL-3.0，ElyPDF 的发行包整体须按 AGPL 履行（对应源码见上游源码包）；ElyDoc、ElyTable、ElyShow 的 LibreOffice 仍为 MPL-2.0 等；ElyTree 带的 Git 仍为 GPL-2.0。组件库 Ely-GPUI-Components 保留其 MIT/Apache-2.0。

## 每个发行版附带

| 文件 | 内容 | 生成 |
| --- | --- | --- |
| `ElyOffice-<版本>-source.tar.gz` | 自有源码、`Cargo.lock`、`rust-toolchain.toml`、上游补丁（`patches/libreoffice`、`patches/mupdf`）、构建与取上游的脚本（各上游钉在提交，脚本里写明地址与散列）、测试、文档 | `packaging/macos/release.sh` |
| `ElyOffice-<版本>-upstream-source.tar.gz` | 实际编译的 MuPDF 源码（钉住的提交加我们的补丁，AGPL）与随包 Git 的源码（v2.56.0，GPLv2） | 同上；`tests/scripts/release-macos.sh` 核对其中是打过补丁的 MuPDF |
| 各应用包 `Resources/Licenses/` | Ely 的 MIT 全文；`rust-crates.txt`（链进该应用各二进制的全部 crate 与其许可文件，无许可文件的取标准文本并写明作者）；所带引擎的许可与通知（见下） | `packaging/licenses.sh`，缺任何一份即打包失败 |

## 第三方，按应用

| 依赖 | 许可 | 随包 | 应用 |
| --- | --- | --- | --- |
| LibreOffice 26.2.6.3 | MPL-2.0 等 | LICENSE、NOTICE、CREDITS；修改以补丁形式在源码包里，基于钉住的提交 8221e31 | Doc、Table、Show |
| 英文拼写词典（dictionaries 子模块 9925407） | 见词典目录内许可 | 随词典目录 | Doc、Table、Show |
| MuPDF 1.28.5 | AGPL-3.0 | COPYING，编进去的每个第三方库与每组内嵌字体的许可（按实际构建） | PDF |
| OpenSSL（libcrypto，静态） | Apache-2.0 | LICENSE.txt | PDF |
| Tesseract 语言数据（tessdata_fast eng、chi_sim、chi_tra） | Apache-2.0 | 按提交与散列取得 | PDF |
| Great Vibes 字体 | OFL-1.1 | OFL | PDF |
| Git 2.56.0 | GPL-2.0（部分 LGPL-2.1） | COPYING、LGPL-2.1；源码在上游源码包 | Tree（Mark 使用同级 Tree 包内的 Git） |
| git-lfs 3.8.0 | MIT | LICENSE.md（按标签与散列取得） | Tree |
| KaTeX 字体（随 RaTeX 嵌入） | OFL-1.1 | OFL、FONT_NOTICE | Mark |
| 媒体播放 | 系统框架（AVFoundation），不随包 | — | Show |

## 未尽之处

- LibreOffice 的整份源码（数 GB）不随发行，只给钉住的上游提交与我们的补丁；内核构建时下载的外部库（`external/tarballs`）是否需随包提供源码，待法律审查。
- 第二批的 MailCore 2 与 ElyAgent 依赖随第二批补上。
- Windows 与 Linux 的打包脚本未在本机运行，其许可收集未验证。
