//! 系统通知（蓝图「怎么走」第 8 条）：页面自己照网页的规矩决定什么时候弹，外壳让它弹得出来、点了把窗口拿回来。一个网页视图一份
//! 做法，关在这里：
//! - Linux（WebKitGTK）：网页视图自己的通知，外壳放行权限、接住点击（`webkitgtk`）。
//! - Windows（WebView2 默认拒绝网页通知）、macOS（WKWebView 没有这个接口）：页面里的 `Notification` 换成外壳的替身
//!   （`resources/notification.js`），替身调命令 [`notify`]，外壳弹系统的通知（`native`）。施工 DT-8，项目主人 2026-10-31 定。

#[cfg(any(windows, target_os = "macos"))]
mod native;
#[cfg(target_os = "linux")]
mod webkitgtk;

/// 放进页面的替身；没有替身的平台是 `None`。
#[cfg(any(windows, target_os = "macos"))]
pub const STAND_IN: Option<&str> = Some(include_str!("../resources/notification.js"));
#[cfg(not(any(windows, target_os = "macos")))]
pub const STAND_IN: Option<&str> = None;

/// 放行这个窗口的网页视图的通知。
#[cfg(target_os = "linux")]
pub fn allow(window: &tauri::WebviewWindow) {
    webkitgtk::allow(window);
}

/// Windows、macOS 走替身，网页视图自己的通知不用放行。
#[cfg(not(target_os = "linux"))]
pub fn allow(_window: &tauri::WebviewWindow) {}

/// 替身调的命令：弹一条系统通知（`silent` 的不出声），点了把窗口拿回来、叫页面里这一条的 `onclick`。页面能调的只有这一条（`capabilities/`）。
///
/// # Errors
/// 系统的通知弹不出来，交回原因（替身照它叫 `onerror`）。
#[cfg(any(windows, target_os = "macos"))]
#[tauri::command]
pub fn notify(
    app: tauri::AppHandle,
    id: u32,
    title: String,
    body: String,
    silent: bool,
) -> Result<(), String> {
    native::show(app, id, title, body, silent).map_err(|error| {
        crate::log::line(&format!("notification not shown: {error}"));
        error
    })
}

#[cfg(test)]
mod tests;
