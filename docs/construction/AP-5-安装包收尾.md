## AP-5：安装包收尾

状态：在做（2026-11-04）。本机单测、门禁、拆包脚本在上一次打包的 deb、rpm 上试过；三平台 CI、打包见「结果」。

### 目的

图纸 `docs/blueprint/app.md`「装在哪」那张表里还差的几样，都在安装包上：

- 原来外壳挪过来的两步（「原来外壳的去留」）：DT-7 许可证门禁和随包的第三方许可证清单、DT-9 Windows 安装包带 PowerShell 7。分支 `step/DT-8-通知` 上做过，照它搬，适配现在没有 Tauri 运行时的托盘。
- `miyu-app.exe` 本身带 Miyu 的图标（资源管理器、任务管理器里看到的）：原来 `tauri-build` 编进去，AP-2 去掉 Tauri 以后没了。
- macOS 的 `Miyu.app` 不占程序坞（`LSUIElement`）。
- deb、rpm 不再要 WebKitGTK：Tauri 的打包工具自己加的，关不掉（AP-2 施工单「结果」）。

这一步编号先前还用过一次：`bce8997` 安装包里的网页换成正式发版的 0.1.0（`85fcd43c`）。

### 图纸改哪几节

`app.md`：

- 「装在哪」：表下面多一段「实际做法（AP-5）」，写许可证放哪、PowerShell 钉哪一版、exe 的图标和名字怎么编进去、`LSUIElement` 怎么写进去、deb/rpm 的依赖怎么改，打包流水线拆开核对什么。
- 「原来外壳的去留」最后一条：打包工具不换，照旧 Tauri 的，写明为什么。

### 做了什么

1. **许可证门禁**（原 DT-7）：`xtask/` 照网页仓库的 `xtask/src/licenses.rs`、`notices.rs` 搬（那一份照 miyu-im 的，miyu-im 的照核心的；网页这一份最新、最像：它也照 git 钉核心的 crate）。根上 `Cargo.toml` 加 `[workspace]`（成员 `xtask`），`.cargo/config.toml` 设 `cargo xtask` 别名。xtask 自己抄一份同样的 lints，不改根上的 `[lints]`（免得和别的分支在同一处撞）。
   - `cargo xtask licenses`：四个平台（`x86_64-unknown-linux-gnu`、`aarch64-unknown-linux-gnu`、`aarch64-apple-darwin`、`x86_64-pc-windows-msvc`，核心设计 12 R11；托盘的包现在只出三种，多查一个不碍事）各跑一次 `cargo metadata --filter-platform`，依赖图里的第三方包（构建、开发依赖也算）许可证照 SPDX 表达式算，要能和 AGPL-3.0-or-later 合在一起发。名单同核心蓝图 `licenses.md` 第 3 条加 GPL-3.0-or-later、AGPL-3.0-or-later，四个仓库一样。git 钉的核心 crate（`miyu-client` 和它带进来的 13 个）是 AGPL-3.0-or-later，放行。
   - 2026-11-04 的依赖（四个平台合起来 324 个第三方包，加构建依赖以后 327 个，合进 AP-4 补以后 326 个）实际用到的：MIT、Apache-2.0（含 `WITH LLVM-exception`）、Unicode-3.0、Unlicense（`ksni` 只写了这一种，`memchr`、`jiff` 这些是 `Unlicense OR MIT`）、Zlib（`foldhash`、`zlib-rs`）、ISC（`inotify`）、BSD-3-Clause、BSD-2-Clause、0BSD（都在 `OR` 里）、CC0-1.0（`notify`）、AGPL-3.0-or-later（核心）。名单里另有 Unicode-DFS-2016、BSL-1.0、MPL-2.0、CDLA-Permissive-2.0、GPL-3.0-or-later 现在没用上，留着和别的仓库同一份。zbus 一套（`zbus`、`zvariant`、`zbus_names`）MIT，smol 一套（`async-io`、`async-executor`、`blocking`、`polling`）Apache-2.0 OR MIT，`windows` 一套 MIT OR Apache-2.0，`winreg` MIT，`notify-rust` MIT OR Apache-2.0。
2. **第三方许可证清单**（原 DT-7）：`cargo xtask notices` 生成根上的 `THIRD-PARTY-LICENSES.txt`，提交进仓库；`cargo xtask licenses` 重算一遍，和仓库里的不一样就不过。
   - 照网页的做法：四个平台上从托盘顺着正常依赖走得到的包（构建依赖、开发依赖不编进程序，不算；门禁的小程序也在工作区里，不从它走；照 git 钉的核心 crate 照核心的 `LICENSE`，不算第三方），读各包带的许可证文件，没带的用 `xtask/src/notices/` 的标准原文；原文一样的合一段。
   - 和网页不一样的：编进去的第三方资源只有菜单的四个 Lucide 图标（`icons/source/lucide/LICENSE`，ISC，带 Feather 的 MIT）；应用图标、托盘图标是项目主人画的，不算。原来外壳 DT-7 的两处留着：MPL 的包段前那一行写 crates.io 的源码地址（MPL 第 3.2 条；现在依赖里没有 MPL 的包，AP-3 换 ksni 以后 `option-ext` 没了）、标准原文多一份 MPL-2.0；Windows 上检出成 CRLF 的标准原文换行统一。
   - 2026-11-04 生成出来 171 段、约 520 KB；合进 AP-4 补以后 169 段（它自己的两份原文和别人都不一样，各一段）。
   - CI（`ci.yml`）：clippy、测试带 `--workspace`，行数也数 `xtask/src`、`build.rs`，Linux 上跑 `cargo xtask licenses`。
3. **安装包带三份许可证**（原 DT-7）：`packaging/stage.py` 的 `stage_licenses` 放进资源目录的 `licenses/`，缺一份就停：
   - `miyu-app/`：这个仓库的 `LICENSE`、`THIRD-PARTY-LICENSES.txt`；
   - `miyu/`：核心检出（`pins.toml` 钉的那一版，`61f1f3ef2` 有）的 `LICENSE`、`LICENSES/`（Apache-2.0 的那几处）、`THIRD-PARTY-LICENSES.txt`；
   - `miyu-web/`：网页检出的 `LICENSE`、`miyu-webui/THIRD-PARTY-LICENSES.txt`；
   - `README.txt`（英文）说哪份是什么，Windows 的多一句 `pwsh\` 里有 PowerShell 自己的两份。
   - 网页包的文件夹照网页 `tools/pack-web.py` 装好的样子补上包根的 `LICENSE`、`THIRD-PARTY-LICENSES.txt`（原来 stage.py 没带）。`loading.html` 是给原来外壳的窗口用的，托盘不用，不带。
   - 装好以后在：Linux `/usr/share/miyu/licenses/`，Windows `C:\Program Files\Miyu\resources\licenses\`，macOS `Miyu.app/Contents/Resources/miyu/licenses/`。
4. **随包 PowerShell 7**（原 DT-9）：`pins.toml` 加 `[pwsh]`，`stage.py` 的 `stage_pwsh` 在 Windows 上照它下官方的 zip（`https://github.com/PowerShell/PowerShell/releases/download/v<版本>/PowerShell-<版本>-win-<x64|arm64>.zip`），SHA256 对不上就停，解进资源目录的 `pwsh\`（核心 SW-5 先找 `<资源目录>\pwsh\pwsh.exe`，`61f1f3ef2` 的 `miyu-basesystem/src/shell/program.rs` 还是这样找），解开以后要有 `pwsh.exe`、`LICENSE.txt`、`ThirdPartyNotices.txt`。
   - 版本 7.6.9（7.6 是当前的 LTS，附件名 `powershell-lts-*`；7.6.9 是 2026-10-25 发的，2026-11-04 核对还是 7.6 最新的补丁，后面只有 7.7.0-rc.1 预发布）。校验值和 DT-9 钉的一样，2026-11-04 又核对了两处：发版说明里的 SHA256、GitHub 发版附件的 `digest`：
     - `PowerShell-7.6.9-win-x64.zip`（109,620,855 字节）：`ce7484804c0f0ad4dda10c958e71acdaf7eebfb22505c1b480f89e0e9f1da94d`
     - `PowerShell-7.6.9-win-arm64.zip`（102,310,068 字节）：`2157fb002178e606c2a5532c56647118d90d029742f04d4900a71e3b478c2363`
   - 只在 CI 里下（每次打 Windows 包下 110 MB）。`pins.txt`（发版说明）多一行 PowerShell 的版本。
   - MSI 装、卸、修之前停进程的名单加 `pwsh`（只停安装目录下的，`packaging/windows/miyu.wxs`）。
5. **`miyu-app.exe` 的图标和名字**：新的 `build.rs`，编给 Windows 时写一份资源脚本（图标 `icons/icon.ico`，编号 1；版本信息 `FileDescription`、`ProductName` 是「Miyu」，`CompanyName` `miyu-agent`，版本照 `Cargo.toml`），交 `embed-resource` 3.0.12（MIT，构建依赖，不编进程序；MSVC 上找 Windows SDK 的 `rc.exe`，交叉编的找 `windres`）编、只链进 `miyu-app` 这个程序。编给别的平台什么都不做。任务管理器的「名称」照 `FileDescription` 写，不写的话是 `miyu-app.exe`。
   - 挑它不挑 `winres`（不再维护）、`winresource`、`tauri-winres`：`tauri-winres` 底下就是它，出错时直接 panic；`embed-resource` 交回结果，build.rs 自己报。都是 MIT、构建时用、仓库代码里没有 `unsafe`（`forbid(unsafe_code)` 管 build.rs）。新进来的构建依赖：`embed-resource`、`vswhom`、`vswhom-sys`（MIT，只 MSVC 上找 Visual Studio），`cc`、`rustc_version`、`toml`、`memchr` 本来就在锁文件里。
   - 不写应用清单（manifest）：原来 `tauri-build` 带的那份（通用控件 6、DPI）托盘用不上：`tao` 在运行时设 DPI，消息框照系统的。
   - 图形子系统：`src/main.rs` 头上本来就有 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`（release 不开控制台），照图纸在了，不动。
6. **macOS 不占程序坞**：`packaging/macos/Info.plist` 只写 `LSUIElement = true`，`macos.json` 的 `bundle.macOS.infoPlist` 指过去，Tauri 的打包工具（tauri-cli 2.12.4 在 macOS 上 `merge_plist`，bundler 2.10.4 `create_info_plist` 最后并进去、同名的这份算）并进 `Miyu.app/Contents/Info.plist`。运行时照旧设 `ActivationPolicy::Accessory`（AP-2）。
7. **deb、rpm 去掉 WebKitGTK**：Tauri 的打包工具（tauri-cli 2.12.4 `interface/rust.rs`，`#[cfg(target_os = "linux")]` 那一段）不管配置，总给 deb 加 `libwebkit2gtk-4.1-0`、`libgtk-3-0`，给 rpm 加 `libwebkit2gtk-4.1.so.0()(64bit)`、`libgtk-3.so.0()(64bit)`。新的 `packaging/drop-webkit.py`（只用 Python 自带的库）打完以后改，只改依赖那一项，装进去的文件一个字节不动：
   - deb：ar 包里只换 `control.tar.gz` 里的 `control`：`Depends` 去掉 `libwebkit2gtk*`，重复的 `libgtk-3-0` 去掉一个，成了 `libgtk-3-0, bubblewrap`；别的成员（`debian-binary`、`data.tar.gz`）和成员头原样。没用 `dpkg-deb -R` 再打：那样整个 `data.tar` 要重新打一遍，属主、权限得自己保证。
   - rpm：主头里 `REQUIRENAME`、`REQUIREFLAGS`、`REQUIREVERSION` 三组去掉 `libwebkit2gtk*` 那一项，数据区照原来的顺序和对齐重排（照原样重排一遍要和原来的字节一样，不一样就不改）；签名头里照主头算的 SHA1、SHA256 和「主头加内容」的大小原地换成新的。签名头里有认不出的（GPG、MD5 这些，签了名的包）就停：第一版不签名。`libgtk-3.so.0()(64bit)` 和配置里写的 `gtk3` 都留着。
   - 没有 webkit 的照原样留着，说一句（以后 Tauri 不加了也不坏）。在 AUR 的 PKGBUILD 算校验值之前跑（PKGBUILD 照改完的 deb 算）。AUR 的 `depends` 本来就是自己写的，不受影响。
8. **打包流水线拆开核对**（`release.yml`，收安装包以后）：
   - Linux：`dpkg-deb -f` 的 `Depends`、`rpm -qpR` 的 `Requires` 里没有 webkit、有 GTK 3 和 bubblewrap；`rpm -K --nosignature` 摘要对得上；deb、rpm 的文件列表里有 `/usr/share/miyu/licenses/` 的三份和网页包根上的两份。
   - macOS：`Miyu.app`、dmg 挂上以后里面的 `Miyu.app`：`Info.plist` 过 `plutil -lint`，`LSUIElement` 是 true；`Contents/Resources/miyu/licenses/` 三份清单都在。
   - Windows：三种语言的 MSI 各用 `msiexec /a` 解出来（只解文件，不跑自定义步骤、不写注册表）：`resources\pwsh\pwsh.exe` 跑出来的版本是 `pins.toml` 的那一版，`pwsh\` 里有 PowerShell 自己的两份许可证，`licenses\` 三份齐、`README.txt` 提了 PowerShell；`miyu-app.exe` 的版本信息 `FileDescription`、`ProductName` 是「Miyu」（和图标编在同一份资源脚本里，编不进去 build.rs 就失败）。

### 要拍板的

- 打包工具不换（图纸「原来外壳的去留」最后一条留给 AP-5 的）：Tauri 的打包工具除了 deb/rpm 加 webkit 这一处，别的（MSI 三种语言和开始菜单的应用身份、dmg、`.app`、deb/rpm 的桌面文件和图标）都合用，换掉要重写三个平台的打包。webkit 这一处打完改掉。以后 Tauri 打包工具升级，先看 `drop-webkit.py` 还认不认它出的包（流水线拆开核对会红）。
- 许可证放在资源目录的 `licenses/`（同 DT-7）：三个平台的包都把资源目录整份带上，不用分平台写打包配置。核心 `packaging/README.md` 说安装包根目录放 `LICENSE` 和清单，这里三份放一个目录，看许可证的人一处看全。

### 不做什么

- 不改核心、网页的清单：各自生成、各自的门禁守着，这里照钉的提交拷。
- 不签名（第一版，原来外壳第一节第 6 条）：Windows 的 exe、MSI，macOS 的 `.app` 照旧不签（`signingIdentity: "-"` 是临时签）。
- 不给 deb 补 `/usr/share/doc/miyu-app/copyright`：许可证在 `/usr/share/miyu/licenses/`。
- MSI 装、卸之前不停 `miyu-app` 自己：原来就没停（托盘被占着时安装程序照 Windows 自己的「文件在用」处理），这一步不动。

### 风险

- `drop-webkit.py` 自己解析 rpm 头：只认 rpm v4 不签名、Tauri 的 rpm 库（`rpm` 0.30）写出来的样子；认不出的就停，不出坏包。改完的包靠流水线里 `rpm -K`、`rpm -qpR` 验，本机没有 rpm 工具，只在上一次打包（41813382751）的 rpm 上试过：照原样重排对得上、签名头的摘要照新头算对、`bsdtar` 读得出 1043 个文件。
- PowerShell 的 zip 每次打包从 GitHub 下，GitHub 抽风时打包失败，重跑就行。PowerShell 出新补丁不会自动跟，要人改 `pins.toml`。
- 许可证清单跟着依赖变：升级、加依赖忘了跑 `cargo xtask notices`，CI 的许可证一步红，照提示跑一遍提交上来。合进 main 的 AP-4 补（`7f717f3`）把 `notify-rust` 改成只在非 Windows 上用、Windows 上直接用 `windows` 0.62（依赖图里本来就有）：合进这个分支时重新生成了清单，`tauri-winrt-notification` 0.8.1 出去了（少两段），名单不用动。
- Windows 交叉编（本机 `--target x86_64-pc-windows-gnu`）要有 `x86_64-w64-mingw32-windres`，没有的 build.rs 报错。

### 怎么验

1. 每个提交前 `cargo fmt --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`（Linux 和 `--target x86_64-pc-windows-gnu`）、`cargo test --workspace --locked` 过；每个 `.rs` 不超过 500 行。
2. xtask 的测试（照网页的，加两条）：SPDX 表达式（含核心的 AGPL-3.0-or-later）、每个包只报一次；清单只算正常依赖走得到的、不从门禁的小程序走、核心自己的 crate 不算；许可证文件、文件夹、`license-file` 都读；没带文件的用标准原文（含 MPL-2.0）；认不出的、空的报出来；一样的原文一段；MPL 的写源码地址。
3. `cargo xtask licenses` 过；清单末尾加一个字以后报「不是最新的」，重新生成以后过。
4. `stage.py` 照钉的两个提交（`git archive` 出核心、网页，程序用假文件）搭了一遍 Linux 的：`licenses/` 三份齐、网页包根上有 `LICENSE` 和清单。`stage_pwsh` 换掉下载、喂自己造的 zip 试：校验值对的解开、不对的停、ARM 的挑 arm64 的地址。
5. 本机交叉编 Windows 的 `miyu-app.exe`（debug）：资源里有图标（3）、图标组（14）、版本信息（16），`FileDescription` 是 `Miyu`。
6. `drop-webkit.py` 在上一次打包（41813382751）的 deb、rpm 拷贝上跑：deb 的 `Depends` 成了 `libgtk-3-0, bubblewrap`、`dpkg-deb -I` 读得出、`data.tar.gz` 的校验值没变；rpm 的 `Requires` 剩 `bubblewrap, gtk3, libgtk-3.so.0()(64bit)` 和三个 `rpmlib(...)`；再跑一遍说「没有，不改」。
7. 三平台 CI、打包，打包流水线拆开核对的几步：见「结果」。

### 要人手测的（出包以后）

- Windows 虚拟机（装这一步的 MSI）：资源管理器里 `C:\Program Files\Miyu\miyu-app.exe` 是 Miyu 的图标；任务管理器「进程」里托盘那一行写「Miyu」、带图标；「属性」→「详细信息」的文件说明、产品名称是 Miyu。`resources\pwsh\pwsh.exe` 起得来，网页里让模型跑 `echo a && echo b` 两行都出来（沙盒里用的是带的 pwsh）。`resources\licenses\` 三份。卸载以后 `pwsh\` 一起没了。
- macOS（dmg 拖进「应用程序」）：打开 Miyu，程序坞里不出图标（连启动那一下也不闪），⌘Tab 里没有它，菜单栏有图标。
- Linux：`apt install ./miyu-app_0.1.0_amd64.deb` 在没装 WebKitGTK 的系统上不拉 `libwebkit2gtk-4.1-0`；`dnf install` 的 rpm 同样；装好起得来。

### 结果

（等 CI、打包跑完补。）
