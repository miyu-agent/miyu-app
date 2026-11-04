## AP-3：托盘补全

状态：做着（2026-11-04）。

### 目的

补上 AP-2 留着位置的几样（图纸 `docs/blueprint/app.md` 第 3、4 条）：Linux 左键打开网页界面（换 `ksni` 后端，2026-11-04 项目主人定换）、软件包挂的入口（核心 TR-2）、局域网一组、状态行的「没有响应」「出错了」。

### 图纸改哪几节

`app.md` 照实际做法改了这几处（都标了「AP-3」）：

- 第 3 条：Linux 换 `ksni` 后端定了、写明日期，没有 StatusNotifierItem 宿主的托盘显示不出来。
- 第 4 条状态行：「没有响应」「出错了」怎么判断、什么时候放下。
- 第 4 条「软件包挂的入口」：怎么跟、哪些算开着、变了怎么换菜单；「要核心给的」TR-2 那一条。
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

### 风险

- 没有 StatusNotifierItem 宿主的托盘（只认 XEmbed 的老托盘、GNOME 没装 AppIndicator 扩展）显示不出图标：libayatana-appindicator 在这种地方会退回 GTK 的老图标，ksni 不会。`ksni` 照 `assume_sni_available(true)` 起，没有宿主不报错，宿主后来起来了照样挂上。
- 「没有响应」只在托盘自己问的时候看得出：连着、核心没推东西、托盘也没问的时候核心卡住了，状态行照旧写「运行中」，直到下一次问或者断了。要随时知道得定时去问，图纸不许。
- 「出错了」的原因有的只有中文：数据根认不出、建不成，连不上核心这几句是核心门面 `miyu-client` 写死的中文（`miyu-store`、`miyu-ipc` 的 `Display`），英文、日文系统上照样是中文；核心拉起时说的原因照核心的语言。要分语言得核心门面给原因码。
- 菜单由宿主照 dbusmenu 画：字号、图标大小、灰的样子照宿主的（KDE、waybar、GNOME 扩展各不一样）；状态行带原因的那两行字在一个菜单项里，宿主认不认换行看它自己。

### 结果

（做完补）
