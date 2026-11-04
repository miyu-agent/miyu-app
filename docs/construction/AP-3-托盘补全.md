## AP-3：托盘补全

状态：做着（2026-11-04）。

### 目的

补上 AP-2 留着位置的几样（图纸 `docs/blueprint/app.md` 第 3、4 条）：Linux 左键打开网页界面（换 `ksni` 后端，2026-11-04 项目主人定换）、软件包挂的入口（核心 TR-2）、局域网一组、状态行的「没有响应」「出错了」。

### 图纸改哪几节

`app.md` 照实际做法改了这几处（都标了「AP-3」）：

- 第 3 条：Linux 换 `ksni` 后端定了、写明日期，没有 StatusNotifierItem 宿主的托盘显示不出来。
- 「原来外壳的去留」最后一条：`tray-icon` 的后端。

### 做了什么

1. **Linux 换 `ksni` 后端**：`tray-icon` 关掉默认特性（`libappindicator`），开 `ksni`（它自己带 `muda-snapshot`：菜单照快照交给 ksni 的线程，不用 GTK 画）。托盘库在 Linux 上把宿主调的 `Activate` 报成左键抬起，托盘照原来 Windows 的那一段打开网页界面，代码不用分平台；`with_menu_on_left_click` 在这个后端上不看。
   - 依赖：少了 `libappindicator`、`libappindicator-sys`、`libloading`、`dirs`、`dirs-sys`、`option-ext`（MPL-2.0，原来要在许可证清单里写源码地址的那一个）、`redox_users`、`libredox`；多了 `ksni` 0.3.6（Unlicense）、`zbus` 5.19.0 和 `zvariant`、`zbus_names` 这些（MIT），`async-io`、`async-executor`、`blocking`、`polling` 这一套 smol 的小库（Apache-2.0 OR MIT），`uuid`、`tempfile`、`hex`、`serde_repr`、`enumflags2` 等（MIT OR Apache-2.0），`endi`、`zcheapstr`、`uds_windows`、`tracing-attributes`（MIT）。都能和 AGPL-3.0-or-later 合在一起发。仓库还没有许可证门禁和第三方许可证清单（原 DT-7，挪到 AP-5），这一步照 `cargo metadata` 一个个看的。
   - 打包：deb 去掉 `libayatana-appindicator3-1`，rpm 去掉 `libayatana-appindicator-gtk3`，AUR 去掉 `libayatana-appindicator`；CI、打包流水线不再装 `libayatana-appindicator3-dev`。GTK 3 照旧要。README 照改。

### 风险

- 没有 StatusNotifierItem 宿主的托盘（只认 XEmbed 的老托盘、GNOME 没装 AppIndicator 扩展）显示不出图标：libayatana-appindicator 在这种地方会退回 GTK 的老图标，ksni 不会。`ksni` 照 `assume_sni_available(true)` 起，没有宿主不报错，宿主后来起来了照样挂上。
- 菜单由宿主照 dbusmenu 画：字号、图标大小、灰的样子照宿主的（KDE、waybar、GNOME 扩展各不一样）；状态行带原因的那两行字在一个菜单项里，宿主认不认换行看它自己。

### 结果

（做完补）
