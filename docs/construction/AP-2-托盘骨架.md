## AP-2：托盘骨架

状态：已完成（2026-11-03，Linux 本机单测、门禁、真核心的两个手动测试过；三平台 CI 见「结果」）。托盘的界面三个平台都没人手测过，见「要人手测的」。

### 目的

把原来的 Tauri 窗口外壳改成只有托盘的 `miyu-app`（图纸 `docs/blueprint/app.md`，项目主人 2026-11-03 定）：改名，去掉窗口、网页视图和只给它们用的几样，Tauri 换成 `tray-icon` + `tao`，搭好托盘菜单的骨架和状态行。

### 图纸改哪几节

`app.md` 照实际做法改了这几处（都标了「AP-2」）：第 1 条（单实例按数据根、锁放在哪）、第 3 条（Linux 收不到图标的点击）、第 4 条（状态行的「正在启动…」怎么算、圆点怎么画；默认项不能加粗；「设置…」退回不带参数；局域网的「在手机上打开…」改成用浏览器打开网页设置里的二维码那一组；macOS 的开机时启动用 LaunchAgent；关于用系统的消息框；「启动」怎么拉起核心）、第 6 条（日志放哪）、「要核心给的」第一条（盯整个 `run/`；连法同 `connect_running` 但自己拿着读的一半）和 TR-1、TR-2 的形状、「原来外壳的去留」最后一条（用了哪些库）。

### 做了什么

1. 改名：crate、程序 `miyu-app`；托盘提示、安装包产品名、消息框标题写「Miyu」。CI、打包流水线的附件名，`packaging/` 三个平台的主页，Linux 包名（`productName` `miyu-app`）、桌面文件模板（`packaging/linux/miyu-app.desktop`），AUR 包 `miyu-app-bin`（`replaces`、`conflicts` 旧的 `miyu-desktop-bin`），`tauri.conf.json` 的 `identifier` 换成 `io.github.miyu-agent.miyu-app`、加 `mainBinaryName: miyu-app`，README、AGENTS 照改，仓库地址写 `miyu-agent/miyu-app`。
2. 删了：`src/window.rs`（窗口、窗口大小、等待页的那一套、出错的重试对话框）、`src/navigate.rs`、`src/download.rs`、`src/notify.rs` 和 `src/notify/`、macOS 的应用菜单 `src/menu.rs`、`build.rs`（`tauri-build`）；依赖 `tauri`、`tauri-build`、三个 Tauri 插件、`webkit2gtk`、`glib`、`url`、`serde`。`resources/notification.js` 在 main 上本来就没有（只在 `step/DT-8-通知` 上）。
3. 留下：`program.rs`（找主程序，调它；要网址读 JSON 的那一套去掉，换成 `miyu web`、`--settings`、`core stop`、`--version`）、`quit.rs`（只留叫核心停，交回给人看的一句）、`log.rs`（轮转、限流照旧；文件改成数据根的 `var/log/miyu-app.log`）、`folder.rs`（只给自己看的目录，加托盘自己的目录在哪）、`text.rs` 和三语的字（键全换成托盘菜单的）、`icons/`、打包流水线、DT-10 的审计修复（`PATH` 只认绝对目录、目录 0700、日志轮转限流、流水线拆权限和钉提交号、PKGBUILD 只填两行）。DT-10 的 D5-1（框里的跳转）、D5-5（调试口子）跟着网页视图一起没了。
4. 新加：
   - `src/main.rs`：参数（`--autostart`）、找数据根（`DataRoot::locate`）、单实例、事件循环。macOS 在运行时设成不占程序坞（`ActivationPolicy::Accessory`；`LSUIElement` 留给 AP-5）。
   - `src/app.rs`：界面线程收事件、改状态、派活。
   - `src/tray.rs`：托盘图标和菜单，照样板的组、顺序、文案；软件包入口、局域网两组留了位置和注释（TR-2 的形状写在注释里）。
   - `src/status.rs`：状态怎么算、菜单哪几行亮着（纯的，单测）。
   - `src/link.rs`：连核心看状态、盯 `run/`（纯的部分单测，另有两个要真核心的手动测试）。握手之后给 TR-1 的 `notices.claim` 留了位置。
   - `src/actions.rs`：每一项实际调什么（见下表），出错的弹系统的消息框。
   - `src/autostart.rs`：开机时启动，三个平台写什么、写在哪（纯的，单测）。
   - `src/instance.rs`：单实例（锁、「请打开」的文件、一秒里只算一次）。
   - `src/about.rs`：关于里的三样版本和钉的提交号。
   - `src/icon.rs`：PNG 解成 RGBA、状态行的小圆点现画。
   - `icons/source/lucide/` 四个 Lucide 原稿（`square`、`play`、`rotate-cw`、`power`，ISC，许可证一起放进去），`icons/source/build.sh menu` 出 `icons/menu/*.png`（灰 `#808080`、32px）。
5. 依赖（crates.io 上 2026-11-03 的稳定版，`Cargo.lock` 里）：`tao` =0.37.2（关默认功能）、`tray-icon` =0.26.1（默认 AppIndicator 后端）、`rfd` =0.17.2（只开 `gtk3`）、`notify` 8.2.1（CC0-1.0）、`png` 0.18.1、`open` 5.4.5、`serde_json` 1.0.154、`sys-locale` 0.3.2、`tokio` 1.54.1、`toml` 1.1.8、`winreg` 0.56.0（只 Windows）；`miyu-client` 照 git 钉核心 main，`pins.toml` 的核心一起换成同一个：先钉 `7783ce2c082b4993bfb39cc83bb5ca5bc1aa22d8`，核心 TR-1、TR-2 合进 main 以后换成 `d58f7b0990d92784c04142e114b886c8b1511ce5`（在它后面，2026-11-03 经协调的会话转来；托盘这一步还不用 TR-1、TR-2，换了以后 AP-3、AP-4 不用再换钉）。CI、打包流水线在 Linux 上装 `libgtk-3-dev libayatana-appindicator3-dev`，不再装 WebKitGTK、librsvg、libxdo、libssl；deb、rpm、AUR 的依赖照改（`libgtk-3-0`/`gtk3` 加 AppIndicator）。

### 菜单每一项实际调什么

| 菜单 | 调什么 |
|---|---|
| 状态行 | 不能点。托盘自己的连接：`miyu-client` 的 `connect::connect`（同 `Core::connect_running`，不拉起）加 `hello`（`head.kind` 报 `miyu-app`），读到头就是断了 |
| 停止 | `miyu core stop`（最多等 70 秒）；做着时写「正在停止…」 |
| 启动 | `DataRoot::prepare` 建骨架，`miyu_client::connect::connect_or_start` 拉起 `miyu core`，等它说好了就放掉那条连接（不开浏览器、不起网页服务）；做着时写「正在启动…」 |
| 重新启动 | 先「停止」，停成了再「启动」 |
| 打开网页界面 | `miyu web`（最多等 90 秒） |
| 设置… | `miyu web --settings`；退出码 2（用法不对，旧网页不认）的记一行，再 `miyu web` |
| 开机时启动 | Windows：`HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 值 `Miyu` = `"<miyu-app.exe>" --autostart`；macOS：`~/Library/LaunchAgents/io.github.miyu-agent.miyu-app.plist`；Linux：`$XDG_CONFIG_HOME/autostart/miyu-app.desktop`（`Exec=<miyu-app> --autostart`） |
| 打开日志所在的文件夹 | `open::that_detached(<数据根>/var/log)`；还没有的弹「还没有日志」 |
| 关于 Miyu | `rfd` 的消息框：托盘版本、`miyu --version`、资源目录里 `packages/miyu-webui/package.toml` 的 `version`、编进程序的 `pins.toml` |
| 退出 Miyu | 拿掉托盘图标，`miyu core stop`，没停成照退 |
| 点图标 | Windows 左键 `miyu web`；macOS 出菜单；Linux 左键、右键都出菜单（AppIndicator 不给点击） |
| 第二次打开 | 写托盘目录里的「请打开」文件，第一个托盘收到了 `miyu web`；带 `--autostart` 的什么都不做 |

### 要拍板的

- Linux 左键打不开网页界面：图纸第 3 条要左键「打开网页界面」，`tray-icon` 默认的 AppIndicator 后端不给点击。换 `ksni` 后端（`tray-icon` 0.26 的特性，直接走 StatusNotifierItem）左键报得上来，也不用 libayatana-appindicator，代价是多一套 zbus；没有 SNI 宿主的老托盘（只认 XEmbed 的）显示不出来。这一步照派活时说的用 AppIndicator，换不换等项目主人定。
- 「启动」没用 `miyu web --print`：`miyu core --help` 写的就是「核心进程：由头拉起」，照头的办法拉它（同终端、`miyu web` 拉起核心的那一个函数）只起核心；`miyu web --print` 连网页服务一起起，网页包不在时还报「未知命令」。

### 不做什么

- 局域网那一组、软件包挂的入口（AP-3，等核心 TR-2 合进 main）；系统通知（AP-4，等核心 TR-1）；「没有响应」「出错了」的判断（AP-3）。都留了位置。
- 许可证门禁和清单（DT-7）、随包的 PowerShell 7（DT-9）：AP-5。
- 打包换掉 Tauri 的打包工具、macOS 的 `LSUIElement`、Windows 程序本身的图标（原来 `tauri-build` 编进 exe，现在没有了；安装包、开始菜单的图标照旧由打包工具给）：AP-5。

### 怎么验

1. 每个提交前 `cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test --locked`（66 个）过；每个 `.rs` 不超过 500 行（最长的 `actions.rs` 约 280 行）。
2. 单测钉住的纯逻辑：状态怎么算（连接、起停、拉起成了等连接、「重新启动」的先后）、每种状态下起停两行的字、图标、亮不亮；开机时启动三个平台写的内容和位置（含路径带空格、`$`、`%`、`&` 的转义）；单实例的锁（同一个数据根第二个拿不到、别的数据根不碍事、第一个退了再拿得到）、「请打开」每次写的不一样、一秒里只算一次；盯哪个目录、哪些文件变动算数（读、打开不算，数据根外面的不算）；握手那一句和回应怎么读；关于里的版本、钉的提交号怎么拼；`miyu web --settings` 退出码 2 才退回；出错时给人看的那一句；编进程序的 PNG 都解得出、菜单图标 32px；托盘自己的目录三个平台在哪。
3. 真核心的两个手动测试（`#[ignore]`，在 `src/link/tests.rs`，只认指到临时目录的 `MIYU_HOME`）：
   `MIYU_HOME=$(mktemp -d) MIYU_RESOURCES=<配套的资源目录> MIYU_TEST_PROGRAM=<miyu> cargo test -- --ignored a_real_core`
   用 `~/.cache/miyu-web-demo/core-target-debug/debug/miyu` 和 `core-d03197d1d/resources` 连跑三遍都过：
   - 拉起（`start_core`）→ 托盘的连接连上 → `miyu core stop` → 连接断、再连不上；
   - 只靠盯 `run/`（没人叫醒）：空的数据根先报连不上，别人拉起核心以后报连上，停了报断。
   主人的数据根没碰。
4. 三平台 CI：见「结果」。

### 要人手测的（三个平台，出包以后）

- 托盘图标出不出来、提示是不是「Miyu」；macOS 的模板图跟着菜单栏变色；Windows 深色、浅色任务栏上看得见。
- 菜单的组、顺序、文案和样板一样；状态行的圆点颜色；起停两行的图标（灰色线条；macOS 上是模板图，看深色菜单上清不清楚）；不能点的灰着。
- Windows 左键打开网页、右键出菜单；macOS 点了出菜单、⌘, ⌘Q 在菜单里显示；Linux（KDE、GNOME 加 AppIndicator 扩展、waybar 这类）点了出菜单。
- 停止、启动、重新启动时状态行走「正在停止…」「已停止」「正在启动…」「运行中」；别的头（终端、浏览器标签页）把核心拉起来以后托盘跟着写「运行中」。
- 「设置…」在新网页（W-1085）上直接到设置；旧网页上退回打开首页。
- 开机时启动：打勾以后注销再登录，托盘起来、不开浏览器；取消以后不再起；Windows 的「启动应用」、macOS 的「登录项」里看得到。
- 第二次从应用菜单打开：不出第二个图标，浏览器再开一次；macOS 点「应用程序」里的 Miyu 同样。
- 关于的消息框：三样版本、提交号；Windows 上 Ctrl+C 能复制。
- 退出：图标马上没了，核心停了，托盘进程退了。
- Windows 上调 `miyu` 不闪黑窗口（`CREATE_NO_WINDOW` 照旧）、正式版不开控制台。
- 装好的包：程序叫 `miyu-app`、应用菜单里叫「Miyu」、从应用菜单打开起得来（打包流水线只证明打得出来）。

### 风险

- 打包工具还是 Tauri 的：`cargo tauri build` 编的时候总带 `--features tauri/custom-protocol`，没有 tauri 依赖编不过（第一次推上去三台都红在这里）；流水线改成先 `cargo build --release`，再 `cargo tauri bundle` 只打包。`mainBinaryName` 让装好的程序叫 `miyu-app`。`identifier` 不能以 `.app` 结尾（和 macOS 的程序包撞），定成 `io.github.miyu-agent.miyu-app`，LaunchAgent 的标签同它。
- `rfd` 在 Linux 上自己起一个 GTK 线程：`tao` 的主循环一直占着默认的主上下文，那个线程拿不到、只是等着，消息框实际在界面线程上画；所以消息框只能在后台线程里弹（界面线程自己弹会等死自己），代码里都是这样调的。
- 托盘的连接一直连着核心：核心要是以后按「没有头连着」做空闲退出，托盘会让它一直不退，到时候再看。
- 断了马上再连一次：核心关的时候套接字还在听的那一下，可能报一次「运行中」再报「已停止」，菜单会闪一下。

### 结果

- 提交：`dd66d96` 改名、`58af077` 换钉、`69cbf3b` 图纸和施工单、`3605bc2` 托盘、`662cc21` Windows 上 clippy 报没用到、`1e05afa` 打包改成先编再 `tauri bundle`，以及补这一节的那一个。`dd66d96` 误带进了删窗口那几个文件和旧的 `Cargo.lock`（暂存区里先删了，那一次 `git add` 因为 `build.rs` 已删而失败，提交照样带走了暂存的删除）；已经推了，照规矩不改，`3605bc2` 的正文写明了。`69cbf3b` 那一个提交编不过（删了旧代码、新代码还没进），它的 CI 取消了。
- CI（`ci.yml`）：运行 41604148798（`3605bc2`）Linux、macOS 绿，Windows 红在 clippy（`no_home` 没用到）；运行 41605555955（`662cc21`）三台都绿；运行 41607363169（`1e05afa`）三台都绿。
- 打包（`release.yml`）：运行 41604148711（`3605bc2`）Linux、macOS 红在 `cargo tauri build`（`tauri/custom-protocol` 特性），Windows 跑到一半取消；运行 41607363127（`1e05afa`）三台都绿，出了 deb、rpm、PKGBUILD、dmg、MSI。
- 拆开 Linux 的 deb 看：`usr/bin/miyu-app`、`miyu`、`miyu-sandbox`、`miyu-extract`，`usr/share/applications/miyu-app.desktop`，图标 `miyu-app.png`，网页包在 `usr/share/miyu/packages/miyu-webui/`。依赖里还有 `libwebkit2gtk-4.1-0`：Tauri 的打包工具在 Linux 上不管配置写什么，总给 deb 加 `libwebkit2gtk-4.1-0`、`libgtk-3-0`，给 rpm 加 `libwebkit2gtk-4.1.so.0`（`tauri-cli` 2.12.4 `interface/rust.rs`），关不掉。AUR 的 PKGBUILD 自己写依赖，不受影响。要去掉得换打包工具或者打完改 control，留给 AP-5。
- Windows 安装包不再带 WebView2 的引导（`windows.json` 加 `webviewInstallMode: skip`，`e286f30`）：CI 运行 41610616635 三台都绿。
- 换钉核心 `d58f7b099`（最后一个提交）：见下一次推上去的 CI、打包。
