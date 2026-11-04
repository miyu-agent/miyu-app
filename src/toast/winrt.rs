//! Windows 的通知自己拼（AP-4 补，2026-11-04 Windows 虚拟机上测出来的）：notify-rust 底下的 tauri-winrt-notification 拼的 toast 没有
//! `launch`，点正文、收进通知中心以后再点，Windows 都不交回 `Activated`，只有带 `arguments` 的按钮交回。这里照 WinRT 的接口（`windows`
//! 这个 crate，都是安全的 Rust）自己拼一份：
//! - 正文带 `launch="open"`、「打开」按钮带 `arguments="open"`，都是前台激活：点正文、点按钮、通知中心里点，托盘在跑时都交回 `Activated`。
//! - 同一个 `tag` 的放在同一组（`Tag`、`Group`）：下一条换掉上一条，不再摞起来；被换掉的那一条交回 `Dismissed`（应用收掉的），等它的线程走掉。
//! - 托盘没在跑时点通知中心里的那一条，系统照应用身份拉起托盘（开始菜单快捷方式），不交回到哪个会话：那要 COM 激活器，仓库不许 `unsafe`，不做。

use std::sync::mpsc;

use windows::Data::Xml::Dom::XmlDocument;
use windows::Foundation::TypedEventHandler;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::core::HSTRING;

/// 一条通知后来怎么样了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 点了（正文、按钮）。
    Clicked,
    /// 关了、过期了、被下一条换掉了。
    Closed,
}

/// 同一个应用的通知都放在这一组里，组里照 `tag` 换。
const GROUP: &str = "miyu";

/// 弹一条，等到点了或者收掉。`tag` 最长 64 个字（WinRT 的规矩），长了截掉。
///
/// # Errors
///
/// 拼不成、弹不出来：WinRT 的原话。
pub fn show(
    app_id: &str,
    title: &str,
    body: &str,
    open: &str,
    tag: &str,
) -> Result<Outcome, String> {
    let xml = template(title, body, open);
    let doc = XmlDocument::new().map_err(|error| error.to_string())?;
    doc.LoadXml(&HSTRING::from(xml))
        .map_err(|error| error.to_string())?;
    let toast =
        ToastNotification::CreateToastNotification(&doc).map_err(|error| error.to_string())?;
    let short: String = tag.chars().take(64).collect();
    toast
        .SetTag(&HSTRING::from(short))
        .map_err(|error| error.to_string())?;
    toast
        .SetGroup(&HSTRING::from(GROUP))
        .map_err(|error| error.to_string())?;
    let (sender, outcome) = mpsc::channel();
    let clicked = sender.clone();
    let failed = sender.clone();
    toast
        .Activated(&TypedEventHandler::new(move |_, _| {
            if clicked.send(Outcome::Clicked).is_err() {
                // 等的那头已经走了。
            }
            Ok(())
        }))
        .map_err(|error| error.to_string())?;
    toast
        .Dismissed(&TypedEventHandler::new(move |_, _| {
            if sender.send(Outcome::Closed).is_err() {
                // 等的那头已经走了。
            }
            Ok(())
        }))
        .map_err(|error| error.to_string())?;
    toast
        .Failed(&TypedEventHandler::new(move |_, _| {
            if failed.send(Outcome::Closed).is_err() {
                // 等的那头已经走了。
            }
            Ok(())
        }))
        .map_err(|error| error.to_string())?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
        .map_err(|error| error.to_string())?;
    notifier.Show(&toast).map_err(|error| error.to_string())?;
    // `toast` 留到有了下文：放早了收不到事件
    let got = outcome.recv().unwrap_or(Outcome::Closed);
    drop(toast);
    Ok(got)
}

/// toast 的 XML：标题、正文两行，正文点了和「打开」按钮一样（`launch`、`arguments` 都是 `open`，前台激活）。
pub fn template(title: &str, body: &str, open: &str) -> String {
    format!(
        "<toast launch=\"open\" activationType=\"foreground\"><visual><binding template=\"ToastGeneric\">\
         <text>{}</text><text>{}</text></binding></visual><actions>\
         <action content=\"{}\" arguments=\"open\" activationType=\"foreground\"/></actions></toast>",
        escape(title),
        escape(body),
        escape(open)
    )
}

/// XML 里要转的五个字。
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests;
