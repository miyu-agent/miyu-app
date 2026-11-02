//! Miyu 的桌面端外壳（蓝图 `docs/blueprint/desktop.md`）：一个窗口开网页软件给的本机页面，一个托盘图标。外壳不连核心，
//! 只调装在旁边的主程序 `miyu`：开窗口时要网址，彻底退出时叫核心停。
// 正式版不开控制台窗口（Windows）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod download;
mod folder;
mod log;
#[cfg(target_os = "macos")]
mod menu;
mod navigate;
mod notify;
mod program;
mod quit;
mod text;
mod tray;
mod window;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use tauri_plugin_window_state::StateFlags;

use crate::text::Text;

fn main() {
    let text = Arc::new(Text::system());
    let miyu = program::locate_here();
    let known: window::Known = Arc::new(Mutex::new(None));
    let on_menu = miyu.clone();
    let built = tauri::Builder::default()
        // 单实例要第一个注册：第二次开程序只叫已经在跑的那一个把窗口拿出来（蓝图「怎么走」第 1 条）。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::bring_back(app)
        }))
        // 只记大小、位置、最大化；「藏着」不记，不然下次起来窗口不出来。
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .on_menu_event(move |app, event| match event.id().as_ref() {
            tray::SHOW => window::bring_back(app),
            tray::QUIT => quit::everything(app, on_menu.clone()),
            #[cfg(target_os = "macos")]
            menu::CLOSE => hide(app),
            other => log::line(&format!("unknown menu item {other}")),
        })
        .setup(move |app| {
            let handle = app.handle();
            let data = start_log(handle);
            log::line(&format!(
                "starting {}, main program found: {}",
                env!("CARGO_PKG_VERSION"),
                miyu.is_some()
            ));
            #[cfg(target_os = "macos")]
            handle.set_menu(menu::create(handle, &text)?)?;
            window::create(handle, &known, data)?;
            if let Err(error) = tray::create(handle, &text) {
                log::line(&format!("no tray icon: {error}"));
            }
            window::open(handle, &known, miyu.clone(), Arc::clone(&text));
            Ok(())
        })
        .build(tauri::generate_context!());
    match built {
        Ok(app) => app.run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                window::bring_back(_app);
            }
        }),
        Err(error) => {
            log::line(&format!("could not start: {error}"));
            std::process::exit(1);
        }
    }
}

/// 外壳自己的数据目录：日志 `desktop.log` 放这里，网页视图的数据放下面的 `webview/`（蓝图「怎么走」第 4、15 条）。两个都
/// 只给自己看（第五批审计 D5-2：`webview/` 里有设备令牌）。交回网页视图的那个目录；找不到、建不出的交 `None`，网页视图用
/// Tauri 默认的地方。
fn start_log(app: &AppHandle) -> Option<PathBuf> {
    let dir = match app.path().app_local_data_dir() {
        Ok(dir) => dir,
        Err(error) => {
            log::line(&format!("no data folder: {error}"));
            return None;
        }
    };
    if let Err(error) = folder::private(&dir) {
        log::line(&format!("creating {}: {error}", dir.display()));
        return None;
    }
    log::start(dir.join("desktop.log"));
    let webview = dir.join("webview");
    // 建不出也照样用：上一级已经只给自己看了，网页视图自己会建。
    if let Err(error) = folder::private(&webview) {
        log::line(&format!("creating {}: {error}", webview.display()));
    }
    Some(webview)
}

/// 关窗口：藏起来（macOS「窗口」一栏的「关闭窗口」）。
#[cfg(target_os = "macos")]
fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(window::LABEL)
        && let Err(error) = window.hide()
    {
        log::line(&format!("hiding the window: {error}"));
    }
}
