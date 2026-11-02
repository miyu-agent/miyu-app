//! 窗口（蓝图「怎么走」第 3 到 7、10 条）：先建好空的，再调 `miyu web` 要网址导航过去；导航、新窗口、下载在网页视图外面
//! 接住；关窗口只是藏起来。

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use tauri::webview::{DownloadEvent, NewWindowResponse};
use tauri::{
    AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::navigate::{self, Go, Origin};
use crate::program::{self, OpenError, RunError};
use crate::text::{Text, key};
use crate::{download, log, notify};

/// 唯一那个窗口的标签。
pub const LABEL: &str = "main";

/// 网页软件的来源：要到网址以后才有。导航的判断读它。
pub type Known = Arc<Mutex<Option<Origin>>>;

/// 建窗口：标题 Miyu，第一次 1200×800（以后照窗口状态插件记的），网页视图先开 `about:blank`。
///
/// # Errors
/// Tauri 建不出窗口。
pub fn create(
    app: &AppHandle,
    known: &Known,
    data: Option<PathBuf>,
) -> tauri::Result<WebviewWindow> {
    let blank = WebviewUrl::External(
        Url::parse("about:blank").map_err(|e| tauri::Error::Anyhow(e.into()))?,
    );
    let on_navigation = Arc::clone(known);
    let downloads = app.path().download_dir().ok();
    let first = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.size().to_logical::<f64>(m.scale_factor()))
        .map_or(FALLBACK, |screen| first_size(screen.width, screen.height));
    let mut builder = WebviewWindowBuilder::new(app, LABEL, blank)
        .title("Miyu")
        .inner_size(first.0, first.1)
        .min_inner_size(400.0, 300.0)
        .center()
        .on_navigation(move |url| {
            let origin = *on_navigation.lock().unwrap_or_else(PoisonError::into_inner);
            go(navigate::navigation(origin, url), url)
        })
        .on_new_window(|url, _features| {
            go(navigate::new_window(&url), &url);
            NewWindowResponse::Deny
        })
        .on_download(move |_webview, event| save(downloads.as_deref(), event));
    if let Some(data) = data {
        builder = builder.data_directory(data);
    }
    if let Some(args) = browser_args(debug_port().as_deref()) {
        log::line(&format!(
            "webview debugging port open for diagnostics: {args}"
        ));
        builder = builder.additional_browser_args(&args);
    }
    let window = builder.build()?;
    let hide = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(error) = hide.hide() {
                log::line(&format!("hiding the window: {error}"));
            }
        }
    });
    notify::allow(&window);
    Ok(window)
}

/// 主屏量不出来时第一次开窗口的大小（逻辑像素）。
const FALLBACK: (f64, f64) = (1200.0, 800.0);

/// 第一次开窗口的大小（逻辑像素，以后照窗口状态插件记的）：屏幕的七成五，不小于 1100×760（左栏 252 宽，再窄对话区挤），
/// 也不超过屏幕的九成（小屏幕上照屏幕来）。项目主人 2026-10-30：1200×800 在 2560 宽的屏幕上显得左栏太宽。
pub fn first_size(screen_width: f64, screen_height: f64) -> (f64, f64) {
    let fit = |share: f64, least: f64, screen: f64| (screen * share).max(least).min(screen * 0.9);
    (
        fit(0.75, 1100.0, screen_width).round(),
        fit(0.75, 760.0, screen_height).round(),
    )
}

/// 排查用的调试口子的环境变量 `MIYU_DESKTOP_DEBUG_PORT`：只有 debug 编译认，正式版一律当没写（第五批审计 D5-5：开着的时候
/// 本机任何进程都连得上 DevTools，在页面里拿设备令牌、替人点「允许」；设成用户级、系统级的环境变量容易忘了关）。
fn debug_port() -> Option<String> {
    if cfg!(debug_assertions) {
        std::env::var("MIYU_DESKTOP_DEBUG_PORT").ok()
    } else {
        None
    }
}

/// 排查用（施工 DT-6，Windows 上查动画）：`MIYU_DESKTOP_DEBUG_PORT` 写了 1024 以上的口子，WebView2 开只听本机的调试口子
/// （`--remote-debugging-port`）。只有 WebView2 认这个参数，别的网页视图不理。没写、写错的交 `None`，照常不开。
/// wry 默认的那一句要一起写上：一给参数就整个换掉。
pub fn browser_args(port: Option<&str>) -> Option<String> {
    let port: u16 = port?.trim().parse().ok().filter(|p| *p >= 1024)?;
    Some(format!(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port={port}"
    ))
}

/// 照 `navigate` 的判断：留下的放行，交出去的给系统的浏览器。交回放不放行。
fn go(decision: Go, url: &Url) -> bool {
    match decision {
        Go::Stay => true,
        Go::Elsewhere => {
            if let Err(error) = open::that_detached(url.as_str()) {
                log::line(&format!("opening a {} link outside: {error}", url.scheme()));
            }
            false
        }
        Go::Drop => {
            log::line(&format!("not opening a {} link", url.scheme()));
            false
        }
    }
}

/// 下载存进「下载」目录；找不到这个目录的不下。
fn save(downloads: Option<&std::path::Path>, event: DownloadEvent<'_>) -> bool {
    match event {
        DownloadEvent::Requested { destination, .. } => {
            let Some(dir) = downloads else {
                log::line("no downloads folder, download refused");
                return false;
            };
            let suggested = destination.file_name().map(std::ffi::OsStr::to_os_string);
            *destination =
                download::destination(dir, suggested.as_deref(), std::path::Path::exists);
            true
        }
        DownloadEvent::Finished { success, .. } => {
            log::line(&format!("download finished, success={success}"));
            true
        }
        _ => true,
    }
}

/// 在另一个线程里要网址、导航过去；要不到的弹对话框：「重试」再要，「退出」只退外壳。
pub fn open(app: &AppHandle, known: &Known, miyu: Option<PathBuf>, text: Arc<Text>) {
    let app = app.clone();
    let known = Arc::clone(known);
    std::thread::spawn(move || {
        let Some(miyu) = miyu else {
            log::line("main program not found");
            tell(&app, &text, text.get(key::NO_PROGRAM), false);
            app.exit(1);
            return;
        };
        loop {
            match fetch(&miyu) {
                Ok((url, origin)) => {
                    *known.lock().unwrap_or_else(PoisonError::into_inner) = Some(origin);
                    match app.get_webview_window(LABEL).map(|w| w.navigate(url)) {
                        Some(Ok(())) => {}
                        Some(Err(error)) => log::line(&format!("navigating: {error}")),
                        None => log::line("no window to navigate"),
                    }
                    return;
                }
                Err(why) => {
                    let detail = match &why {
                        Failure::Run(RunError::Timeout) => text.get(key::TIMEOUT).to_owned(),
                        Failure::Run(RunError::Spawn(e) | RunError::Wait(e)) => e.to_string(),
                        Failure::Open(OpenError::Failed(tail)) => tail.clone(),
                        Failure::Open(OpenError::BadReply) => text.get(key::BAD_REPLY).to_owned(),
                    };
                    let message = format!("{}\n\n{detail}", text.get(key::OPEN));
                    if !tell(&app, &text, message.trim_end(), true) {
                        app.exit(1);
                        return;
                    }
                }
            }
        }
    });
}

enum Failure {
    Run(RunError),
    Open(OpenError),
}

/// 调一次 `miyu web --print --format json --ticket`，记下耗时和结果（不记网址）。
fn fetch(miyu: &std::path::Path) -> Result<(Url, Origin), Failure> {
    let started = Instant::now();
    let got = program::run(miyu, program::OPEN, program::OPEN_LIMIT)
        .map_err(Failure::Run)
        .and_then(|ran| program::read_open(&ran).map_err(Failure::Open));
    let outcome = match &got {
        Ok(_) => "ok".to_owned(),
        Err(Failure::Run(e)) => format!("{e:?}"),
        Err(Failure::Open(e)) => format!("{e:?}"),
    };
    log::line(&format!(
        "miyu web took {} ms: {outcome}",
        started.elapsed().as_millis()
    ));
    got
}

/// 弹一个出错的对话框，等人点。`retry` 的有「重试」「退出」两个按钮，交回点没点「重试」；别的只有「退出」。
fn tell(app: &AppHandle, text: &Text, message: &str, retry: bool) -> bool {
    let quit = text.get(key::QUIT).to_owned();
    let buttons = if retry {
        MessageDialogButtons::OkCancelCustom(text.get(key::RETRY).to_owned(), quit)
    } else {
        MessageDialogButtons::OkCustom(quit)
    };
    let pressed_ok = app
        .dialog()
        .message(message)
        .title("Miyu")
        .kind(MessageDialogKind::Error)
        .buttons(buttons)
        .blocking_show();
    log::line(&format!("error dialog closed, first button: {pressed_ok}"));
    retry && pressed_ok
}

/// 把窗口拿出来：显示、取消最小化、放到最前（托盘、第二次开程序、点程序坞的图标、点通知）。
pub fn bring_back(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        log::line("no window to bring back");
        return;
    };
    for (what, done) in [
        ("show", window.show()),
        ("unminimize", window.unminimize()),
        ("focus", window.set_focus()),
    ] {
        if let Err(error) = done {
            log::line(&format!("bringing the window back ({what}): {error}"));
        }
    }
}

#[cfg(test)]
mod tests;
