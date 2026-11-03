//! 托盘图标和菜单（图纸 `app.md` 第 3、4 条，2026-11-03 项目主人看过的样板）：从上往下
//!
//! 1. 状态行（灰、不能点，前面一个小圆点）；
//! 2. 「停止」或「启动」、「重新启动」（带图标；不能用时灰着，不藏）；
//! 3. 「打开网页界面」（默认项）、「设置…」（macOS 带 ⌘,）；
//! 4. （软件包挂的入口：等核心 TR-2，见 [`create`] 里留的位置）
//! 5. （局域网：AP-3，同上）
//! 6. 「开机时启动」（打勾）、「打开日志所在的文件夹」；
//! 7. 「关于 Miyu」、「退出 Miyu」（带图标；macOS 带 ⌘Q）。
//!
//! 点图标：Windows 左键「打开网页界面」、右键出菜单；macOS 点了出菜单。Linux 的 AppIndicator 不给程序图标的点击，左键、右键都是
//! 出菜单（托盘库的限制，`tray-icon` 文档写明）。菜单的事件在 `app` 里统一接。

use tray_icon::menu::{
    CheckMenuItem, Icon as MenuIcon, IconMenuItem, Menu, MenuItem, PredefinedMenuItem,
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::icon::{self, MENU_SIZE};
use crate::status::{Look, Toggle};
use crate::text::{Text, key};

/// 菜单项的编号。
pub mod id {
    pub const STATUS: &str = "status";
    pub const TOGGLE: &str = "toggle";
    pub const RESTART: &str = "restart";
    pub const OPEN: &str = "open";
    pub const SETTINGS: &str = "settings";
    pub const AUTOSTART: &str = "autostart";
    pub const LOGS: &str = "logs";
    pub const ABOUT: &str = "about";
    pub const QUIT: &str = "quit";
}

/// 建好的托盘：跟着状态变的几行拿着，好改。
pub struct Tray {
    _icon: TrayIcon,
    status: IconMenuItem,
    toggle: IconMenuItem,
    restart: IconMenuItem,
    autostart: CheckMenuItem,
    stop_icon: Option<MenuIcon>,
    start_icon: Option<MenuIcon>,
}

/// 解一张编进程序的菜单图标；解不出的（不会：测试查过）不带图标。
fn menu_icon(bytes: &[u8]) -> Option<MenuIcon> {
    let image = icon::decode(bytes).ok()?;
    MenuIcon::from_rgba(image.pixels, image.width, image.height).ok()
}

/// 给一行放图标：macOS 上当模板图（系统照菜单着色），别的照原样画。
fn put_icon(item: &IconMenuItem, icon: Option<MenuIcon>) {
    #[cfg(target_os = "macos")]
    item.set_icon_templated(icon);
    #[cfg(not(target_os = "macos"))]
    item.set_icon(icon);
}

/// macOS 上的快捷键（只在菜单里显示、菜单开着时按得到）；别的平台托盘菜单不带。
#[cfg(target_os = "macos")]
fn shortcut(keys: &str) -> Option<tray_icon::menu::accelerator::Accelerator> {
    keys.parse().ok()
}
#[cfg(not(target_os = "macos"))]
fn shortcut(_: &str) -> Option<tray_icon::menu::accelerator::Accelerator> {
    None
}

/// 建托盘图标和菜单，先照 `look` 画一遍。
///
/// # Errors
/// 建不出（Linux 上没有 `libayatana-appindicator` 这些、没有托盘）。
pub fn create(text: &Text, look: &Look, autostart: bool) -> Result<Tray, String> {
    let status = IconMenuItem::with_id(id::STATUS, "", false, None, None);
    let toggle = IconMenuItem::with_id(id::TOGGLE, "", false, None, None);
    let restart = IconMenuItem::with_id(id::RESTART, text.get(key::RESTART), false, None, None);
    put_icon(&restart, menu_icon(icon::RESTART));
    let open = MenuItem::with_id(id::OPEN, text.get(key::OPEN), true, None);
    let settings = MenuItem::with_id(
        id::SETTINGS,
        text.get(key::SETTINGS),
        true,
        shortcut("CmdOrCtrl+Comma"),
    );
    let autostart_item = CheckMenuItem::with_id(
        id::AUTOSTART,
        text.get(key::AUTOSTART),
        true,
        autostart,
        None,
    );
    let logs = MenuItem::with_id(id::LOGS, text.get(key::LOGS), true, None);
    let about = MenuItem::with_id(id::ABOUT, text.get(key::ABOUT), true, None);
    let quit = IconMenuItem::with_id(
        id::QUIT,
        text.get(key::QUIT),
        true,
        None,
        shortcut("CmdOrCtrl+Q"),
    );
    put_icon(&quit, menu_icon(icon::QUIT));

    let menu = Menu::new();
    let separator = PredefinedMenuItem::separator;
    let added = [
        menu.append(&status),
        menu.append(&toggle),
        menu.append(&restart),
        menu.append(&separator()),
        menu.append(&open),
        menu.append(&settings),
        // 软件包挂的入口（图纸第 4 条，AP-3 做；核心 TR-2）：清单写 `[tray] name = {zh, en}` 的包，`package.list` 和 packages
        // 流里多一格 `"tray": {"name": …}`（照连接的语言挑好的）；照 enabled、status 只列开着的，一个包一行，点了跑
        // `miyu web --package <编号>`。前面一根分隔线，一个都没有的不出这一组。
        // 局域网（AP-3）：「局域网访问」打勾的开关，开着时多一行「在手机上打开…」（用浏览器打开网页设置里显示地址和二维码的
        // 那一组），前面一根分隔线。
        menu.append(&separator()),
        menu.append(&autostart_item),
        menu.append(&logs),
        menu.append(&separator()),
        menu.append(&about),
        menu.append(&quit),
    ];
    if let Some(error) = added.into_iter().find_map(Result::err) {
        return Err(error.to_string());
    }

    let image = icon::decode(icon::TRAY)?;
    let tray_icon =
        Icon::from_rgba(image.pixels, image.width, image.height).map_err(|e| e.to_string())?;
    // macOS 上是模板图，系统照菜单栏着色。
    #[cfg(target_os = "macos")]
    let builder = TrayIconBuilder::new().with_icon_templated(tray_icon);
    #[cfg(not(target_os = "macos"))]
    let builder = TrayIconBuilder::new().with_icon(tray_icon);
    let built = builder
        .with_id("miyu")
        .with_tooltip("Miyu")
        .with_menu(Box::new(menu))
        // macOS 点了出菜单；Windows 左键交给程序（打开网页界面），右键出菜单。
        .with_menu_on_left_click(cfg!(target_os = "macos"))
        .build()
        .map_err(|error| error.to_string())?;
    let tray = Tray {
        _icon: built,
        status,
        toggle,
        restart,
        autostart: autostart_item,
        stop_icon: menu_icon(icon::STOP),
        start_icon: menu_icon(icon::START),
    };
    tray.show(text, look);
    Ok(tray)
}

impl Tray {
    /// 照 `look` 改状态行和起停那两行。
    pub fn show(&self, text: &Text, look: &Look) {
        let status = match &look.reason {
            Some(reason) => format!("{}\n{reason}", text.get(look.status)),
            None => text.get(look.status).to_owned(),
        };
        self.status.set_text(status);
        let dot = icon::dot(look.dot.rgb(), MENU_SIZE);
        // 圆点带颜色，不当模板图。
        self.status
            .set_icon(MenuIcon::from_rgba(dot.pixels, dot.width, dot.height).ok());
        let (label, image) = match look.toggle {
            Toggle::Stop => (key::STOP, &self.stop_icon),
            Toggle::Start => (key::START, &self.start_icon),
        };
        self.toggle.set_text(text.get(label));
        put_icon(&self.toggle, image.clone());
        self.toggle.set_enabled(look.toggle_enabled);
        self.restart.set_enabled(look.restart_enabled);
    }

    /// 「开机时启动」打不打勾。
    pub fn set_autostart(&self, on: bool) {
        self.autostart.set_checked(on);
    }
}
