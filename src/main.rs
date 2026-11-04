//! Miyu 的托盘程序 `miyu-app`（图纸 `docs/blueprint/app.md`）：只有托盘图标（macOS 上是菜单栏图标），没有窗口。界面都在网页里，
//! 托盘调装在旁边的主程序 `miyu` 用浏览器打开它；另照网页服务的样子连核心（不拉起），看它在不在跑。
// 正式版不开控制台窗口（Windows）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod about;
mod actions;
mod app;
mod autostart;
mod extras;
mod folder;
mod icon;
mod instance;
mod lan;
mod link;
mod log;
mod program;
mod quit;
mod rpc;
mod status;
mod text;
mod tray;

use std::sync::Arc;

use miyu_client::places::{DataRoot, Env};
use tao::event::{Event as LoopEvent, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};

use crate::actions::Ctx;
use crate::app::{App, Event, Post};
use crate::instance::Claim;
use crate::text::Text;

fn main() {
    let text = Arc::new(Text::system());
    let autostarted = std::env::args().skip(1).any(|arg| arg == autostart::ARG);
    let mut problem = None;
    let root = match DataRoot::locate(&Env::current()) {
        Ok(root) => Some(root),
        Err(error) => {
            log::line(&format!("no data root: {error}"));
            problem = Some(error.to_string());
            None
        }
    };
    // 数据根的 `var/log/` 核心起过一次才有；没有的等连上核心再定（`app::start_log`）。
    if let Some(logs) = root.as_ref().map(DataRoot::logs)
        && logs.is_dir()
    {
        log::start(logs.join("miyu-app.log"));
    }
    log::line(&format!(
        "starting {}, autostarted: {autostarted}",
        env!("CARGO_PKG_VERSION")
    ));

    // 单实例：一个数据根一个托盘。第二个起来的叫第一个打开网页界面，自己退。
    let own = folder::own_here();
    let key = instance::key(
        root.as_ref()
            .map_or(std::path::Path::new(""), DataRoot::path),
    );
    let _held = match own.as_deref().map(|dir| (dir, instance::claim(dir, &key))) {
        Some((_, Ok(Claim::First(lock)))) => Some(lock),
        Some((dir, Ok(Claim::Second))) => {
            if autostarted {
                log::line("already running, autostart does nothing");
            } else if let Err(error) = instance::ask(dir, &key) {
                log::line(&format!("could not ask the running tray to open: {error}"));
            } else {
                log::line("already running, asked it to open the web interface");
            }
            return;
        }
        Some((dir, Err(error))) => {
            log::line(&format!("no single instance in {}: {error}", dir.display()));
            None
        }
        None => {
            log::line("no folder of our own, no single instance");
            None
        }
    };

    #[cfg_attr(
        not(target_os = "macos"),
        allow(unused_mut, reason = "只有 macOS 要改")
    )]
    let mut event_loop = EventLoopBuilder::<Event>::with_user_event().build();
    // macOS：不占程序坞、不出应用菜单，只在菜单栏（打包时的 `LSUIElement` 是 AP-5 的事，这里先在运行时设）。
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }
    let post = Post::new(event_loop.create_proxy());
    let on_tray = post.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        on_tray.send(Event::Tray(event));
    }));
    let on_menu = post.clone();
    tray_icon::menu::MenuEvent::set_event_handler(Some(
        move |event: tray_icon::menu::MenuEvent| {
            on_menu.send(Event::Menu(event.id));
        },
    ));

    let _listening = own.as_deref().and_then(|dir| {
        let asked = post.clone();
        instance::listen(dir, &key, move || asked.send(Event::Open))
            .inspect_err(|error| log::line(&format!("not listening for a second start: {error}")))
            .ok()
    });
    let on_link = post.clone();
    let waker = root
        .clone()
        .map(|root| link::spawn(root, move |up| on_link.send(Event::Link(up))));
    let miyu = program::locate_here();
    log::line(&format!("main program found: {}", miyu.is_some()));
    if miyu.is_none() {
        problem.get_or_insert_with(|| text.get(crate::text::key::NO_PROGRAM).to_owned());
    }
    let ctx = Ctx {
        miyu,
        root,
        text: Arc::clone(&text),
        post,
        waker,
    };
    let mut app = App::new(ctx, !autostarted, problem);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            LoopEvent::NewEvents(StartCause::Init) => app.started(),
            LoopEvent::UserEvent(event) => app.handle(event, control_flow),
            #[cfg(target_os = "macos")]
            LoopEvent::Reopen { .. } => app.handle(Event::Open, control_flow),
            _ => {}
        }
    });
}
