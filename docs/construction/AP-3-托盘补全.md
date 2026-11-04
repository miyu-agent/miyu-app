## AP-3：托盘补全

状态：已完成（2026-11-04，Linux 单测、真核心的手动测试、waybar 上经 D-Bus 测过左键和菜单每一项；三平台 CI、打包见「结果」）。Windows、macOS 的界面没人手测过，见「要人手测的」。

### 目的

补上 AP-2 留着位置的几样（图纸 `docs/blueprint/app.md` 第 3、4 条）：Linux 左键打开网页界面（换 `ksni` 后端，2026-11-04 项目主人定换）、软件包挂的入口（核心 TR-2）、局域网一组、状态行的「没有响应」「出错了」。

### 图纸改哪几节

`app.md` 照实际做法改了这几处（都标了「AP-3」）：

- 第 3 条：Linux 换 `ksni` 后端定了、写明日期，没有 StatusNotifierItem 宿主的托盘显示不出来。
- 第 4 条状态行：「没有响应」「出错了」怎么判断、什么时候放下。
- 第 4 条「软件包挂的入口」：怎么跟、哪些算开着、变了怎么换菜单；「要核心给的」TR-2 那一条。
- 第 4 条「局域网」：怎么认开没开、怎么读、怎么写、设没设密码怎么知道、「在手机上打开…」调什么。
- 「原来外壳的去留」最后一条：`tray-icon` 的后端。

### 做了什么

1. **Linux 换 `ksni` 后端**：`tray-icon` 关掉默认特性（`libappindicator`），开 `ksni`（它自己带 `muda-snapshot`：菜单照快照交给 ksni 的线程，不用 GTK 画）。托盘库在 Linux 上把宿主调的 `Activate` 报成左键抬起，托盘照原来 Windows 的那一段打开网页界面，代码不用分平台；`with_menu_on_left_click` 在这个后端上不看。
   - 依赖：少了 `libappindicator`、`libappindicator-sys`、`libloading`、`dirs`、`dirs-sys`、`option-ext`（MPL-2.0，原来要在许可证清单里写源码地址的那一个）、`redox_users`、`libredox`；多了 `ksni` 0.3.6（Unlicense）、`zbus` 5.19.0 和 `zvariant`、`zbus_names` 这些（MIT），`async-io`、`async-executor`、`blocking`、`polling` 这一套 smol 的小库（Apache-2.0 OR MIT），`uuid`、`tempfile`、`hex`、`serde_repr`、`enumflags2` 等（MIT OR Apache-2.0），`endi`、`zcheapstr`、`uds_windows`、`tracing-attributes`（MIT）。都能和 AGPL-3.0-or-later 合在一起发。仓库还没有许可证门禁和第三方许可证清单（原 DT-7，挪到 AP-5），这一步照 `cargo metadata` 一个个看的。
   - 打包：deb 去掉 `libayatana-appindicator3-1`，rpm 去掉 `libayatana-appindicator-gtk3`，AUR 去掉 `libayatana-appindicator`；CI、打包流水线不再装 `libayatana-appindicator3-dev`。GTK 3 照旧要。README 照改。

2. **状态行的「没有响应」「出错了」**：
   - 新的 `src/rpc.rs`：一行一条的一问一答，托盘那条连接和以后临时连的都用它。编号每条连接一个随机前缀（`RandomState`，同一个进程里每个也不一样）接递增的数字：核心照编号认重发、不分连接，用过的编号再发拿到上一次的回应、什么都不做。等回应时推来的先攒着；读一行用 `read_until` 接着读，等回应过了时限丢下正在读的也不丢字。
   - 「没有响应」：问出去的过了 `rpc::SLOW`（10 秒）没回，报一次「问不出话」，接着等；回了再报一次「答得上」。原来握手 30 秒没回就当连不上、写「已停止」，现在写「没有响应」，一直等到回或者断。不为这个定时去问（图纸「不定时去问」）。
   - 「出错了」：`status.rs` 记最近一次的错，断着的时候写「出错了」和原因。哪些算：「启动」拉不起核心（核心的 `StartError::Refused` 只取原话，不带「核心起不来：」）、`miyu web` 没成、握手被拒（`link` 报 `Refused`，带核心回应的 `message`）、起来时找不到数据根、找不到主程序。连上了、再点「启动」放下。
   - `link.rs` 换成照 `Rpc` 写，连上、问不出话、断了、被拒四种报给界面（`Report`）。

3. **软件包挂的入口**：
   - `link.rs` 握手以后订 `packages` 流：回应是整份列表，之后推 `package.changed`，`resync` 的重新订；订阅被拒的（老核心）记一行，这条连接照用。核心参考 `d58f7b099` 的 `connection.en.md`「subscribe」（中文版那一节缺了，英文版有）、`pushes.md`、`packages.md`。不轮询，菜单打开前也不问（托盘库不给「菜单要打开了」的口子）。
   - 新的 `src/extras.rs`：记着列表，算出该列的那几行（规矩见图纸第 4 条），纯的，单测。
   - `tray.rs` 的菜单改成照「核心来的几行」整份建：变了的时候 `set_menu` 换一份，「开机时启动」照原来的勾，状态行再画一遍。菜单项编号 `package:<编号>`，点了 `miyu web --package <编号>`（网页 F-6 下就认），退出码 2 的同「设置…」退回 `miyu web`。
   - `actions::open_web` 改成收一个 `Page`（首页、设置、软件包），以后「在手机上打开…」也走它。

4. **局域网**：
   - `link.rs` 握手以后再订 `config` 流：先 `config.get {keys: ["web.listen"]}`，之后 `config.changed` 的 `keys` 里有它的再读；`package.changed` 里装上、卸掉一个包的也再读（本机实测：`miyu pkg install` 装网页包，核心不推 `config.changed`，不补这一下的话装了网页包菜单里也不出这一组）。不认的（`unknown_config_key`）报「不知道」，不出这一组。
   - 新的 `src/lan.rs`：点了临时连核心（不拉起），握手看 `password_set`，再 `config.set`（打开 `{"value": "0.0.0.0"}`，关掉 `{"unset": true}`），一共最多等 30 秒。没设密码的、老核心不说的交回「要先设密码」，界面改成 `miyu web --settings access`。
   - 「运行中 · 局域网」：`status::look` 多收一个「局域网开着没有」；`Look` 多一格「局域网访问」能不能点（只在运行中）。
   - 「在手机上打开…」开着时才有，`miyu web --settings access`，退出码 2 的同上退回 `miyu web`。
   - 字（三份）：`status.running_lan`「运行中 · 局域网」、`menu.lan`「局域网访问」、`menu.phone`「在手机上打开…」、`error.lan`「没能改局域网访问。」。

### 风险

- 没有 StatusNotifierItem 宿主的托盘（只认 XEmbed 的老托盘、GNOME 没装 AppIndicator 扩展）显示不出图标：libayatana-appindicator 在这种地方会退回 GTK 的老图标，ksni 不会。`ksni` 照 `assume_sni_available(true)` 起，没有宿主不报错，宿主后来起来了照样挂上。
- 「没有响应」只在托盘自己问的时候看得出：连着、核心没推东西、托盘也没问的时候核心卡住了，状态行照旧写「运行中」，直到下一次问或者断了。要随时知道得定时去问，图纸不许。
- 「出错了」的原因有的只有中文：数据根认不出、建不成，连不上核心这几句是核心门面 `miyu-client` 写死的中文（`miyu-store`、`miyu-ipc` 的 `Display`），英文、日文系统上照样是中文；核心拉起时说的原因照核心的语言。要分语言得核心门面给原因码。
- 局域网那一行只在连着、读到过 `web.listen` 以后才出：核心没起过的时候菜单里没有这一组，起来以后才多出来。
- 托盘改 `web.listen` 不经网页软件的 `POST /listen`：网页设置页存完会请网页软件当场换、拿换完的地址，托盘只写配置、靠网页软件自己跟着配置的推送换（网页 `follow.rs`，2026-10-30 起有）。网页软件没在跑的时候只是写了配置，下次起来照它听。
- 菜单由宿主照 dbusmenu 画：字号、图标大小、灰的样子照宿主的（KDE、waybar、GNOME 扩展各不一样）；状态行带原因的那两行字在一个菜单项里，宿主认不认换行看它自己。

### 不做什么

- 系统通知（AP-4）：`link.rs` 握手以后照旧留着 `notices.claim` 的位置。
- 菜单打开前去问一遍（软件包、局域网、核心卡没卡住）：托盘库不给「菜单要打开了」的口子（ksni 有 `menu_about_to_show`，`tray-icon` 没转出来），也违背「不定时去问」。
- 许可证门禁、第三方许可证清单（AP-5）。

### 怎么验

1. 每个提交前 `cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test --locked`（做完是 83 个）过；每个 `.rs` 不超过 500 行（最长的 `actions.rs` 约 350 行）。
2. 单测钉住的：一问一答照编号对上、中间推来的攒着、编号每条连接不一样、拒了的带原因码和原话、过了 10 秒才回的先报慢再报好、半行先到不丢字、断了就断（`rpc`，暂停的时钟）；软件包挂哪几行、推来的变动换掉删掉、装卸和只是变了分得开（`extras`）；`web.listen` 的值哪些算开、`config.get` 的回应、推来的 `keys` 里有没有它（`extras`）；打开、关掉写的参数、握手回应里设没设密码（`lan`）；没有响应、出错了什么时候写、什么时候放下，「运行中 · 局域网」，「局域网访问」只在运行中能点（`status`）；每一处网页的参数（`actions`）；三份字一样的键（`text`）。
3. 真核心的手动测试（`#[ignore]`，`src/link/tests.rs`，只认指到临时目录的 `MIYU_HOME`）：
   `MIYU_HOME=$(mktemp -d) MIYU_RESOURCES=<资源目录> MIYU_TEST_PROGRAM=<miyu> cargo test -- --ignored a_real_core`
   用 `~/.cache/miyu-web-demo/core-target-debug/debug/miyu` 和 `core-fc48ba0c0/resources`（协议同 `d58f7b099`，只加不减）过：起来先报连不上，只靠盯 `run/` 跟上别人拉起的核心，报连上、软件包的入口（出厂的一个都没有）、局域网（这份资源目录没有网页包，报不认），停了报断。
4. 本机 waybar 上实测（不碰真桌面的窗口）：托盘的 `GDK_BACKEND=x11`、`DISPLAY` 指到自己起的 Xvfb（消息框画在那里，不上真桌面），会话的 D-Bus 照旧（图标挂在 waybar 上，测完就退）；`MIYU_PROGRAM` 指到一个记参数的替身，`web …` 只记不开，别的转给真核心；数据根是新的空目录，测完删掉。菜单的结构用 `com.canonical.dbusmenu` 的 `GetLayout` 看，点菜单用 `Event <编号> clicked`，左键用 SNI 的 `Activate`：
   - SNI：`Menu` 是 `/MenuBar`、`ItemIsMenu` 是假；`Activate` 以后替身记到 `web`。
   - 菜单从上往下：状态行、停止、重新启动｜打开网页界面、设置…｜打开托盘测试（装了测试包时）｜局域网访问（装了网页包时）｜开机时启动、打开日志所在的文件夹｜关于 Miyu、退出 Miyu，同样板。
   - 出错了：数据根放了别的文件，点「启动」，状态行「出错了」下面是 `miyu-client` 那一句（数据根认不出），「启动」「重新启动」能点。
   - 没有响应：拿一个握手 15 秒才回的假核心，前 10 秒「已停止」，10 秒以后「没有响应」（「停止」能点），回了以后「运行中」。
   - 软件包挂的入口：装写了 `[tray]` 的测试扩展，马上多一组「打开托盘测试」，点了替身记到 `web --package traytest`；`package.disable` 以后没了、`package.enable` 又出来、`miyu pkg remove` 以后没了。
   - 局域网：装网页包的清单以后出这一组；没设密码点它，替身记到 `web --settings access`、配置没动；`account.setup` 设了密码再点，`web.listen` 变成 `0.0.0.0`，打勾、多出「在手机上打开…」（点了替身记到 `web --settings access`），状态行「运行中 · 局域网」；再点关掉，回到默认；从外面 `config.set` 改、恢复默认，托盘跟着变；「停止」以后这一行灰着。
   - 退出：替身记到 `web --stop`、`core stop`，托盘退了、核心不在听了（核心带着在跑的扩展时要等几秒）。

### 要人手测的（出包以后）

- Linux 别的托盘宿主：KDE Plasma、GNOME 加 AppIndicator 扩展、XFCE 的状态栏插件。左键打开网页界面、右键出菜单；菜单的图标、圆点、灰的样子；状态行「出错了」下面那一句原因换不换行。没有 SNI 宿主的桌面上没有图标（已知）。
- Windows 虚拟机（装这一步的 MSI）：
  - 菜单的新几组：装一个写了 `[tray]` 的包（或者 im），菜单多一行，点了浏览器打开那个包的后台页；卸掉以后那一行没了。
  - 局域网：没设访问密码时点「局域网访问」，浏览器打开设置「通用」并滚到「访问」那一组，勾不打上；在网页里设了密码再点，打勾、多「在手机上打开…」，状态行「运行中 · 局域网」，手机能连上；点「在手机上打开…」浏览器到「访问」那一组、看得到地址和二维码；再点关掉，手机连不上了。点了以后勾会不会闪一下（托盘库先翻、托盘再翻回来）。
  - 「出错了」：数据根指到一个放了别的东西的目录，点「启动」，状态行两行字（Windows 菜单认不认换行）。
  - 左键照旧打开网页界面。
- macOS：同 Windows 那几样；菜单整个换一份（装卸包、局域网开关）时菜单栏的图标不闪、不丢。

### 结果

- 提交：`bdd0e39` 换 ksni、`0704cd3` 状态行、`d17f773` 软件包挂的入口、`257c8dc` 局域网，以及补这一节的那一个。
- CI（`ci.yml`）：运行 41792684256（`257c8dc`）Linux、macOS、Windows 三台都绿。
- 打包（`release.yml`）：运行 41792684367（`257c8dc`）三台都绿。拆开 Linux 的 deb 看：`Depends: libgtk-3-0, bubblewrap, libwebkit2gtk-4.1-0, libgtk-3-0`，没有 libayatana-appindicator 了（`libwebkit2gtk-4.1-0` 是 Tauri 打包工具自己加的，AP-2 记过，留给 AP-5）；PKGBUILD 的 `depends=('gtk3' 'bubblewrap')`。

### Windows 虚拟机上手测（2026-11-04，Win11 26200，装 AP-3 打包运行 41792684367 的中文 MSI）

数据根另指空目录，装一个自己造的测试包（mingw 编的空程序、`start = "manual"`、带后台页和 `[tray]`）。测完删了数据根、计划任务、临时自动登录，虚拟显示器启用回去，关机；持久配置和测之前逐字一样。

- 软件包挂的入口：菜单里多一组「Open tray test」，点了浏览器打开 `#package=traytest`，后台页出来。**名字是英文**：握手的 `locale` 原来交 `miyu_client::open::locale()`，只读环境变量，Windows 上是空的，核心照英文给；菜单别的字照 `sys_locale` 是中文。改成握手也交系统语言（`text::system_locale`），本机核心上核对过：空的回「Open tray test」，`zh-CN` 回「打开托盘测试」。
- 局域网：没设密码点「局域网访问」，打开网页设置「访问」那一组、配置没动；在网页里设了密码（网页设完就打开），托盘跟着打勾、多「在手机上打开…」、状态行「运行中 · 局域网」，从宿主访问虚拟机的 `:8300` 回 200；点「在手机上打开…」打开设置，二维码和 `http://192.168.0.80:8300` 都在（设置页没自己滚到「访问」那一组，要往下滚，记给网页）；托盘里点掉，`web.listen` 拿掉、宿主访问不到、勾和那一行都没了。
- 左键：打开网页界面（日志 `miyu web`）。
- 没测：点「局域网访问」时勾闪不闪（肉眼没看出）、「出错了」两行字换不换行、深色任务栏、注销再登录的开机自启。
