//! Windows、macOS 的系统通知：照应用标识（`tauri.conf.json` 的 `identifier`）弹，点了把窗口拿回来、叫页面里那一条的
//! `onclick`（`window.__miyuNotificationClicked`，`resources/notification.js`）。
//! Windows 照应用标识弹 toast：MSI 装的开始菜单快捷方式带同一个 AppUserModelID，系统才认这个程序。macOS 照包的标识发，
//! 第一次系统会问人要不要让 Miyu 发通知。

use tauri::{AppHandle, Manager};

use crate::{log, window};

/// 这一条被点了：拿回窗口，叫页面里的 `onclick`。
fn clicked(app: &AppHandle, id: u32) {
    window::bring_back(app);
    let Some(webview) = app.get_webview_window(window::LABEL) else {
        return;
    };
    let call =
        format!("window.__miyuNotificationClicked && window.__miyuNotificationClicked({id})");
    if let Err(error) = webview.eval(call) {
        log::line(&format!(
            "notification click not passed to the page: {error}"
        ));
    }
}

/// 弹一条；`silent` 的 toast 写成静音。
///
/// # Errors
/// 系统的通知弹不出来。
#[cfg(windows)]
pub fn show(
    app: AppHandle,
    id: u32,
    title: String,
    body: String,
    silent: bool,
) -> Result<(), String> {
    use tauri_winrt_notification::{Sound, Toast};
    let identifier = app.config().identifier.clone();
    Toast::new(&identifier)
        .title(&title)
        .text1(&body)
        .sound((!silent).then_some(Sound::Default))
        .on_activated(move |_| {
            clicked(&app, id);
            Ok(())
        })
        .show()
        .map_err(|error| error.to_string())
}

/// 弹一条：等点的那一下会阻塞，放到另一个线程里。不设声音，`silent` 的本来就不出声。
///
/// # Errors
/// 不会：弹不出来的记一行日志。
#[cfg(target_os = "macos")]
pub fn show(
    app: AppHandle,
    id: u32,
    title: String,
    body: String,
    _silent: bool,
) -> Result<(), String> {
    use mac_notification_sys::{Notification, NotificationResponse, set_application};
    use std::sync::Once;
    static APPLICATION: Once = Once::new();
    let identifier = app.config().identifier.clone();
    APPLICATION.call_once(|| {
        if let Err(error) = set_application(&identifier) {
            log::line(&format!("notifications as {identifier}: {error}"));
        }
    });
    std::thread::spawn(move || {
        let mut notification = Notification::new();
        notification
            .title(&title)
            .message(&body)
            .wait_for_click(true);
        match notification.send() {
            Ok(NotificationResponse::Click | NotificationResponse::ActionButton(_)) => {
                clicked(&app, id)
            }
            Ok(_) => {}
            Err(error) => log::line(&format!("notification not shown: {error}")),
        }
    });
    Ok(())
}
