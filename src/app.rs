//! 托盘程序的界面线程：收托盘、菜单的事件，收后台线程做完的、连接报来的，改状态、改菜单，派活。

use std::sync::Arc;
use std::time::Instant;

use tao::event_loop::{ControlFlow, EventLoopProxy};
use tray_icon::TrayIconEvent;
use tray_icon::menu::MenuId;

use crate::actions::{self, Ctx};
use crate::instance::Once;
use crate::link::Report;
use crate::status::{self, Busy, Linked, Status};
use crate::text::Text;
use crate::tray::{self, Tray, id};
use crate::{autostart, log};

/// 交给界面线程的事。
#[derive(Debug)]
pub enum Event {
    /// 点了菜单的一项。
    Menu(MenuId),
    /// 点了托盘图标。
    Tray(TrayIconEvent),
    /// 托盘那条连接报来的。
    Link(Report),
    /// 后台开始做一件起停的事（「重新启动」停完了、开始拉起）。
    Begin(Busy),
    /// 后台做完一件起停的事，带做成了没有。
    Done(Busy, bool),
    /// 出错了：拉不起核心、打不开网页界面，带 `miyu` 说的那一句。状态行断着的时候写「出错了」和它。
    Failed(String),
    /// 有人要打开网页界面：第二个起来的托盘、macOS 上点了程序坞或者再开一次。
    Open,
    /// 「退出 Miyu」停完了，现在退。
    Exit,
}

/// 从别的线程把事交给界面线程。
#[derive(Clone)]
pub struct Post(EventLoopProxy<Event>);

impl Post {
    pub fn new(proxy: EventLoopProxy<Event>) -> Self {
        Self(proxy)
    }

    pub fn send(&self, event: Event) {
        if let Err(error) = self.0.send_event(event) {
            log::line(&format!("the event loop is gone: {error}"));
        }
    }
}

/// 界面线程手上的。
pub struct App {
    ctx: Ctx,
    text: Arc<Text>,
    status: Status,
    tray: Option<Tray>,
    /// 起来时开不开浏览器：开机拉起来的不开。
    open_at_start: bool,
    /// 在退了：别的事不再接。
    quitting: bool,
    asked: Once,
}

impl App {
    /// `problem` 是起来时就知道的错（找不到数据根、找不到主程序），状态行照它写「出错了」。
    pub fn new(ctx: Ctx, open_at_start: bool, problem: Option<String>) -> Self {
        let mut status = Status::default();
        if let Some(problem) = problem {
            status.fail(problem);
        }
        Self {
            text: Arc::clone(&ctx.text),
            ctx,
            status,
            tray: None,
            open_at_start,
            quitting: false,
            asked: Once::default(),
        }
    }

    /// 事件循环转起来了：建托盘（macOS 要在这以后建），要开浏览器的开一次。
    pub fn started(&mut self) {
        match tray::create(&self.text, &self.look(), autostart::enabled()) {
            Ok(tray) => self.tray = Some(tray),
            Err(error) => log::line(&format!("no tray icon: {error}")),
        }
        if self.open_at_start {
            actions::open_web(&self.ctx, false);
        }
    }

    pub fn handle(&mut self, event: Event, control_flow: &mut ControlFlow) {
        match event {
            Event::Exit => *control_flow = ControlFlow::Exit,
            _ if self.quitting => {}
            Event::Menu(item) => self.menu(item.as_ref()),
            Event::Tray(event) => self.clicked(&event),
            Event::Link(report) => self.linked(report),
            Event::Begin(busy) => {
                self.status.begin(busy);
                self.redraw();
            }
            Event::Done(busy, ok) => {
                self.status.done(busy, ok);
                self.redraw();
            }
            Event::Failed(reason) => {
                self.status.fail(reason);
                self.redraw();
            }
            Event::Open => {
                if self.asked.allow(Instant::now()) {
                    actions::open_web(&self.ctx, false);
                }
            }
        }
    }

    fn linked(&mut self, report: Report) {
        match report {
            Report::Up => {
                self.status.link(Linked::Up);
                start_log(&self.ctx);
            }
            Report::Slow => self.status.link(Linked::Slow),
            Report::Down => self.status.link(Linked::Down),
            Report::Refused(why) => {
                self.status.link(Linked::Down);
                self.status.fail(why);
            }
        }
        self.redraw();
    }

    fn menu(&mut self, item: &str) {
        match item {
            id::TOGGLE if !self.status.is_busy() => {
                if self.status.is_up() {
                    self.status.begin(Busy::Stopping);
                    actions::stop(&self.ctx);
                } else {
                    self.status.begin(Busy::Starting);
                    actions::start(&self.ctx);
                }
                self.redraw();
            }
            id::RESTART if !self.status.is_busy() => {
                self.status.begin(Busy::Stopping);
                actions::restart(&self.ctx);
                self.redraw();
            }
            id::TOGGLE | id::RESTART => log::line("busy, start/stop ignored"),
            id::OPEN => actions::open_web(&self.ctx, false),
            id::SETTINGS => actions::open_web(&self.ctx, true),
            id::AUTOSTART => {
                let on = actions::toggle_autostart(&self.ctx);
                if let Some(tray) = &self.tray {
                    tray.set_autostart(on);
                }
            }
            id::LOGS => actions::open_logs(&self.ctx),
            id::ABOUT => actions::about(&self.ctx),
            id::QUIT => {
                log::line("quitting with the core");
                self.quitting = true;
                // 先拿掉图标：停核心最多要等一分钟，图标留着像没点上。
                self.tray = None;
                actions::quit(&self.ctx);
            }
            other => log::line(&format!("unknown menu item {other}")),
        }
    }

    /// 点了托盘图标：Windows、Linux 左键「打开网页界面」（右键托盘库、Linux 的托盘宿主自己出菜单）。macOS 点了出菜单。
    fn clicked(&mut self, event: &TrayIconEvent) {
        if cfg!(target_os = "macos") {
            return;
        }
        if let TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        } = event
        {
            actions::open_web(&self.ctx, false);
        }
    }

    fn look(&self) -> status::Look {
        status::look(&self.status.state())
    }

    fn redraw(&self) {
        if let Some(tray) = &self.tray {
            tray.show(&self.text, &self.look());
        }
    }
}

/// 连上核心了：数据根的 `var/log/` 这时一定有了（核心起来时建），日志还没定下的定在那里。
fn start_log(ctx: &Ctx) {
    if log::is_started() {
        return;
    }
    if let Some(logs) = ctx.root.as_ref().map(miyu_client::places::DataRoot::logs)
        && logs.is_dir()
    {
        log::start(logs.join("miyu-app.log"));
        log::line(&format!(
            "logging here from now on, {}",
            env!("CARGO_PKG_VERSION")
        ));
    }
}
