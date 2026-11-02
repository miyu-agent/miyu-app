## 桌面端的外壳

状态：草案（施工 DT-2，2026-10-30）。项目主人 2026-10-29 定的七条见 `docs/construction/README.md` 第一节；核心仓库的设计 `21-网页.md` X9 记的是为什么这样做，这一页写外壳怎么做。

### 它是什么

一个 Tauri 2 程序 `miyu-desktop`：一个窗口、一个托盘图标。窗口里开的是网页软件 `miyu-web` 在本机给的页面，和浏览器里打开的是同一份；页面照网页仓库浏览器的那一份宿主（`src/host/served.js`）跑，连核心、媒体地址、软件后台页都由网页软件给。外壳不连核心、不读本机令牌、不依赖核心和网页的 crate，只调装在旁边的主程序 `miyu`：

| 什么时候 | 调什么 | 拿到什么 |
|---|---|---|
| 开窗口 | `miyu web --print --format json --ticket` | 一行 `{"url": …, "first": …}`；网址是 `http://127.0.0.1:<口子>/#ticket=<票>`，票 60 秒、一次性（核心 LT-1，网页 W-977、W-978，主程序 CL-5） |
| 托盘「退出」 | `miyu core stop` | 退出码 0：核心停了，或者本来就没在跑；1：没停成，标准错误上一句「miyu: …」（核心 DT-4 下） |

`miyu web` 自己会把核心、网页软件拉起来（核心没在跑的起核心，网页软件没在跑的起 `miyu-web serve`），所以外壳不管谁先起。

### 在哪

| 文件 | 管什么 |
|---|---|
| `Cargo.toml`、`build.rs`、`tauri.conf.json` | 一个 crate；Tauri 的配置：不打页面进应用（没有 `frontendDist`，窗口开时现给网址），没有 `capabilities/`、`capabilities` 是空的（远程来源的页面没有任何权限），不开 `withGlobalTauri` |
| `src/main.rs` | 起来：单实例、窗口状态、对话框三个插件，菜单的事件，日志和数据目录，事件循环 |
| `src/log.rs` | 外壳自己的日志 |
| `src/menu.rs` | macOS 的应用菜单 |
| `src/program.rs` | 找主程序 `miyu`，调它、读一行 JSON、读退出码和标准错误 |
| `src/window.rs` | 开窗口、要网址、出错的对话框、导航和下载交给谁、藏起来、叫回来 |
| `src/download.rs` | 下载存成哪个文件（纯逻辑，单测） |
| `src/navigate.rs` | 导航、新窗口、下载交给谁（纯逻辑，单测） |
| `src/tray.rs` | 托盘图标和菜单 |
| `src/quit.rs` | 彻底退出：藏窗口、叫核心停、退 |
| `src/notify.rs`、`src/notify/`（一个网页视图一份） | 网页视图的通知权限 |
| `src/text.rs`、`resources/text/{zh,en,ja}.toml` | 给人看的字 |
| `icons/` | 应用图标、托盘图标；`icons/source/` 是原图 `logo.png`、两份矢量稿和出图的脚本 `build.sh`（施工 DT-5：换图重跑） |
| `.github/workflows/ci.yml` | 三平台：格式、clippy、测试、行数、正式版的程序（试验用）；Linux 上再跑 `cargo xtask check`（许可证） |
| `xtask/`、`.cargo/config.toml`、`THIRD-PARTY-LICENSES.txt` | 门禁的「许可证」一项（施工 DT-7，照核心的 `licenses.rs`、`notices.rs`）：三个平台上编进外壳的第三方依赖，许可证都能和 AGPL-3.0-or-later 合在一起发；外壳的第三方许可证清单是最新的。`cargo xtask notices` 重新生成清单，升了、加了依赖的那一步跟着跑，和代码进同一个提交 |
| `pins.toml`、`packaging/`、`.github/workflows/release.yml` | 打安装包：钉的核心和网页版本，搭内容的脚本，三个平台的打包配置，Windows 安装包的自定义步骤，AUR 的 PKGBUILD 模板（施工 DT-6，「装在哪」） |

### 怎么走

1. **起来**：单实例（`tauri-plugin-single-instance`）。已经有一个在跑的，第二次开只叫那一个把窗口拿出来、放到最前，自己退出（退出码 0）。没有托盘的桌面（GNOME 默认不显示托盘）靠这一条叫回窗口：再点一次应用图标。
2. **找主程序**：先看环境变量 `MIYU_PROGRAM`（开发时指到别处，和网页软件找主程序同一个名字）；再看外壳自己所在的目录里有没有 `miyu`（Windows 是 `miyu.exe`）；都没有的照 `PATH` 找。安装包里三样放在同一个目录（「装在哪」）。找不到的弹一个系统对话框「找不到 Miyu 的主程序」，只有「退出」。
3. **开窗口**：先建窗口（标题 `Miyu`，大小、位置照上次的，`tauri-plugin-window-state`；第一次照主屏的七成五、不小于 1100×760、不超过屏幕的九成，居中（项目主人 2026-10-30：1200×800 在大屏幕上左栏显得太宽）；量不出主屏的 1200×800），网页视图先开等待页（`waiting.rs`：网页包带的 `loading.html`，欢迎页那个能动的字符画剪影，网页 W-1032；把 `{{starting}}` 换成这种语言的「正在启动 Miyu…」；读不到的放一行字；都作为 `data:` 页面：窗口先开 `about:blank`，建好了再导航过去，Tauri 不开 `webview-data-url` 功能不让拿 `data:` 建窗口，三个平台一起动就 panic，2026-10-31 Mac 上抓到）；调 `miyu web --print --format json --ticket`（不开控制台窗口：Windows 上 `CREATE_NO_WINDOW`；最多等 60 秒（刚装完的 Windows 上 Defender 先扫新程序，30 秒不够；项目主人 2026-10-31 定）），拿到网址就导航过去。每次新开窗口现要一次，不存网址、不存票：票一次性，页面用票连上以后自己在这个网页视图里存设备令牌，刷新照它进。
   - 退出码非 0、超时、读不出那一行的：弹对话框，头一句「打不开 Miyu 的页面。」，下面是 `miyu` 标准错误的最后几行（照它的语言）或者超时、读不懂的一句，两个按钮「重试」「退出」。「退出」只退外壳，不叫核心停。
4. **网页视图的数据**（`localStorage` 里的设备令牌、缓存）放在外壳自己的数据目录（Tauri 的 `data_directory`，在应用标识 `io.github.miyu-agent.desktop` 的目录下），不放进 Miyu 的数据根；卸载外壳不动数据根。
5. **页面能碰到的外壳**：只有一条命令 `notify`（系统通知的替身用，第 8 条；项目主人 2026-10-31 定）。`capabilities/page-notify.json` 只给主窗口里 `http://127.0.0.1:*` 的页面、只在 Windows、macOS 上、只有 `allow-notify` 这一项，`tauri.conf.json` 只打开这一份能力；不开 `withGlobalTauri`，页面自己的代码里没有 `window.__TAURI__`。不装 opener 插件（它默认接管 `target=_blank` 的点击，DT-1 碰到过），开系统的浏览器直接用 `open` 这个 crate。页面要的别的东西都照浏览器的写法写，外壳在网页视图外面接住（下面 6 到 9）；要页面多给什么，找网页加在 `served.js` 里，不在外壳里开口子。往这个口子里加东西先改这一条、找项目主人；测试 `src/notify/tests.rs` 守着。
6. **导航**（`on_navigation`）：和页面同一个来源（同协议、同主机、同口子）的放行；`about:blank`、`blob:`、`data:` 照网页视图自己的规矩；别的一律不在窗口里开，交给系统的浏览器（`http`、`https`、`mailto`），别的协议不开。**新窗口**（`target=_blank`、`window.open`，`on_new_window`）同一条：同来源的也交系统浏览器（桌面端一个窗口一份状态，网页的「页面自己守的」第 4 条）。
7. **下载**（`on_download`）：照网页视图给的建议文件名存进系统的「下载」目录，重名的加「 (1)」「 (2)」；没有名字的叫 `download`。存完不弹东西（和浏览器一样，下载的东西在「下载」目录里）。
8. **通知**：页面自己照网页的规矩决定什么时候弹（网页 `web.md`「系统通知」，`src/host/notify.js`），外壳让它弹得出来、点了把窗口拿回来（施工 DT-8）：
   - Linux（WebKitGTK）：网页视图自己的通知。外壳接 webkit2gtk 的 `permission-request`，通知的放行，别的（定位、摄像头、麦克风……）一律拒；接 `show-notification`，点了拿回窗口。弹不弹得出来看桌面有没有通知服务。
   - Windows（WebView2 默认拒绝网页通知的权限）、macOS（WKWebView 没有 `Notification`）：外壳往页面里放一个 `Notification` 的替身（`resources/notification.js`，`initialization_script`）：权限一律 `granted`，`new Notification(标题, {body, silent})` 调命令 `notify`，外壳照应用标识弹系统的通知（Windows 的 toast，MSI 的快捷方式带同一个 AppUserModelID；macOS 的通知中心，第一次系统问要不要让 Miyu 发通知）；`silent` 的不出声（网页 W-1046：看着别的会话时后台会话的那一种，2026-11-02 项目主人定）：Windows 的 toast 写成静音，macOS 的本来就不带声音；点了拿回窗口，再叫页面里那一条的 `onclick`。页面的代码不用改。
9. **剪贴板**：照网页视图自己的（`navigator.clipboard`，DT-1 在 Linux 上试过能写）。macOS 上外壳要有带「编辑」那一栏的应用菜单，不然 `⌘C`、`⌘V` 在网页视图里不起作用（Tauri 默认的菜单有）。
10. **关窗口**：拦下关闭，窗口藏起来，网页视图留着：页面照旧连着核心、照旧收推送、照旧自己弹通知（DT-1：藏着时 `document.hidden` 为真，动画不跑）。macOS 上点程序坞的图标（`Reopen`）把窗口拿回来。
11. **托盘**：Linux、Windows 用带浅色底板的小图标（深浅任务栏都看得见）；macOS 用单色模板图，系统照菜单栏着色。菜单两项：「显示 Miyu」「退出」。Windows、macOS 左键点图标同「显示 Miyu」；Linux 只有菜单（托盘库在 Linux 上收不到图标的点击）。拿出窗口：显示、取消最小化、放到最前。
12. **彻底退出**（托盘「退出」；macOS 应用菜单的「退出 Miyu」`⌘Q` 同这一条。Tauri 默认菜单里的「退出」直接退外壳、不经这一条，所以 macOS 上外壳自己建应用菜单：「退出 Miyu」、系统的「编辑」一栏、「窗口」一栏的最小化和「关闭窗口」`⌘W`（同关窗口）；Linux、Windows 不设菜单栏）：先藏窗口、拿掉托盘图标，调 `miyu core stop`（它最多等 60 秒），再退出外壳。核心没停成（退出码 1）照样退，标准错误那句记进外壳的日志；网页软件不管，核心停了以后它照自己的规矩空闲 10 分钟退。浏览器里还开着的标签页重连时会把核心再拉起来，当成可以接受（DT-4 下）。
    - 关机、注销时系统叫外壳退：只退外壳，不叫核心停（系统会停它）。
13. **语言**：外壳自己的字（托盘菜单、对话框）照系统的语言，中文、日文、英文三份，别的用英文。不问核心的 `ui.language`：外壳不连核心，菜单就两三个词。
14. **排查用的调试口子**：环境变量 `MIYU_DESKTOP_DEBUG_PORT` 写了 1024 以上的口子时，WebView2 开只听本机的调试口子（`--remote-debugging-port`，连 wry 默认的 `--disable-features=…` 一起传），从 SSH 隧道连上去量页面；平时不写不开。只有 WebView2 认，别的网页视图不理（施工 DT-6，Windows 上查动画）。
15. **日志**：外壳自己的一行一条写进它的数据目录里的 `desktop.log`，只记起来、要网址的耗时和结果、调 `miyu` 失败、退出；不记网址（里面有票）。

### 装在哪

外壳和核心的三个程序放在一起：主程序 `miyu`、沙盒助手 `miyu-sandbox`、抽字的 `miyu-extract`（三个平台都带；语义模型 `miyu-embed` 不带，从软件源装；核心的主会话 2026-10-30 定）。主程序找资源目录照核心的规矩（`MIYU_RESOURCES`，程序旁边的 `resources/`，上一级的 `share/miyu/`，上一级的 `Resources/miyu/`）。资源目录是核心仓库的整份 `resources/`，加网页包的文件夹 `packages/miyu-webui/`（出厂层，`heads.toml` 定的发版位置）：清单补上 `version`、不写 `targets`，`web.json`、`matugen/`，`pages/` 是网页 `web-demo/` 里给浏览器的七样，程序 `miyu-web` 放在包文件夹的 `bin/` 里（`miyu web` 先找包文件夹的 `bin/`；网页的会话 2026-10-30 定）。

| 平台 | 程序 | 资源目录 |
|---|---|---|
| Linux（deb、rpm、AUR） | `/usr/bin/` 下 `miyu-desktop`、`miyu`、`miyu-sandbox`、`miyu-extract` | `/usr/share/miyu/` |
| macOS（`.app`，dmg） | `Miyu.app/Contents/MacOS/` 下四个 | `Miyu.app/Contents/Resources/miyu/`（核心 DT-4 上） |
| Windows（MSI） | 安装目录下四个 `.exe` | 安装目录下的 `resources\`，多一个 `pwsh\`：官方 PowerShell 7 的 zip 版（施工 DT-9），核心优先用它，模型写的 `&&` 才认 |

- Linux 的包写依赖 WebKitGTK 4.1、ayatana 的托盘库、`bubblewrap`，不自带。AUR 的包 `miyu-desktop-bin` 拆发版里的 deb，带了核心，和单独装的 `miyu` 冲突。
- macOS 的资源目录照 `bundle.macOS.files` 放进 `Contents/Resources/miyu/`（不能用 `resources`：编的时候它也拷进 `target/release/miyu/`，和外壳旁边的程序 `miyu` 撞名）。没签名，只做本机的临时签名（`signingIdentity` 是 `-`，Apple 芯片上不签就不让跑）；第一次打开照样要在「隐私与安全性」里放行。沙盒用系统的 Seatbelt，什么都不用装。
- Windows 出 64 位的 MSI（WiX，x64；项目主人 2026-10-30 定安装程序也要 64 位，NSIS 只有 32 位），中、英、日各一个，照 perMachine 装进 `Program Files\Miyu\`，安装目录进 PATH。装、卸、修之前先 `miyu core stop`，再用 64 位的 PowerShell 照路径结束安装目录下还在跑的程序（`packaging/windows/miyu.wxs`）。沙盒不用配：MXC 跑命令不要沙盒用户、不要管理员（核心的主会话 2026-10-30）。`upgradeCode` 钉死在 `packaging/windows.json`，换了就升级不了。
- 许可证（施工 DT-7）：资源目录下的 `licenses/` 放三份，各是那个仓库根上的 `LICENSE` 和第三方许可证清单 `THIRD-PARTY-LICENSES.txt`：`miyu-desktop/`（外壳，AGPL-3.0-or-later）、`miyu/`（核心，AGPL-3.0-or-later，另带 `LICENSES/`：核心里几处用 Apache-2.0）、`miyu-web/`（网页，清单照网页包里的那份）；`README.txt` 说哪个目录是谁的。Windows 的 `pwsh\` 里原样留着 PowerShell 自己的 `LICENSE.txt`、`ThirdPartyNotices.txt`。缺一份打包就停（`packaging/stage.py`）。外壳清单里 MPL-2.0 的包写上它在 crates.io 上的源码地址（MPL 第 3.2 条）。
- 核心、网页软件用哪一版钉在 `pins.toml`（完整的提交号）；发版的流水线 `.github/workflows/release.yml` 照它编、`packaging/stage.py` 搭内容、Tauri 打包（`packaging/<平台>.json`），用包里的 `miyu --version` 试一下起得来，钉的提交号写进发版说明。

### 给人看的字

| 键 | 中文 |
|---|---|
| `tray.show` | 显示 Miyu |
| `tray.quit` | 退出 |
| `error.no_program` | 找不到 Miyu 的主程序。重新装一次 Miyu 试试。 |
| `error.open` | 打不开 Miyu 的页面。 |
| `error.timeout` | 等了 60 秒，Miyu 还没起来。 |
| `starting` | 正在启动 Miyu… |
| `error.bad_reply` | Miyu 给的网址读不懂。 |
| `menu.quit` | 退出 Miyu（macOS 的应用菜单） |
| `menu.edit` | 编辑 |
| `menu.window` | 窗口 |
| `menu.close` | 关闭窗口 |
| `button.retry` | 重试 |
| `button.quit` | 退出 |

### 守着它的

| 测试 | 守什么 |
|---|---|
| `src/navigate/tests.rs` | 同来源放行（口子不同、`localhost` 换 `127.0.0.1` 的都不算同来源）；外链交系统浏览器；`javascript:`、`file:` 这些不开；新窗口一律交出去 |
| `src/program/tests.rs` | 找主程序的顺序；读那一行 JSON：多一个字段照读，少 `url` 的、不是 `http://127.0.0.1` 的算出错；退出码非 0 带回标准错误的最后几行；调的程序超时杀掉 |
| `src/download/tests.rs` | 重名加编号；没有名字的；名字里带路径分隔符、`..` 的只取最后一段 |
| `src/text/tests.rs` | 三份字的键一样多，没有空的；代码用的键都在；系统语言挑哪一份 |
| CI 三平台 | 编译、clippy `-D warnings`、测试；每个 `.rs` 不超过 500 行 |
| 手测清单（`docs/construction/` 里每步的验收） | 窗口、托盘、通知、下载、关窗口、退出：网页视图里的行为单测测不到，每个平台出试验包手测一遍 |
