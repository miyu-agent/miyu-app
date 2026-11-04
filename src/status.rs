//! 状态行和起停那两行（图纸 `app.md` 第 4 条）：照托盘连核心的那条连接（连上是运行中，问不出话是没有响应，断了是已停止），加上
//! 托盘自己正在做的起停（正在启动、正在停止）和最近一次出的错（出错了），算出菜单上该写什么、哪几行能点。只有算，不碰界面，单测钉住。

use crate::text::key;

/// 托盘手上正在做的事：做完之前状态行照它写。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Busy {
    /// 正在拉起核心（「启动」、「重新启动」的后一半）。
    Starting,
    /// 正在叫核心停（「停止」、「重新启动」的前一半、「退出」）。
    Stopping,
}

/// 那条连接现在怎样。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Linked {
    /// 断了、连不上。
    #[default]
    Down,
    /// 连着、答得上话。
    Up,
    /// 连着，问出去的迟迟不回。
    Slow,
}

/// 状态行写哪一种（图纸第 4 条那一串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Running,
    Starting,
    Stopping,
    Stopped,
    /// 连得上、问不出话。
    Unresponsive,
    /// 出错了，带 `miyu` 说的那一句原因。
    Failed(String),
}

/// 状态行前面那个小圆点的颜色（照样板：绿、黄、灰、红）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dot {
    Green,
    Yellow,
    Gray,
    Red,
}

impl Dot {
    /// 圆点的颜色（RGB）：和样板里浅色菜单上的那几种一样。
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Dot::Green => [0x1f, 0x9d, 0x55],
            Dot::Yellow => [0xc7, 0x7d, 0x12],
            Dot::Gray => [0x8a, 0x8d, 0x96],
            Dot::Red => [0xd2, 0x3c, 0x3c],
        }
    }
}

/// 起停那一行是哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// 「停止」（方块图标）。
    Stop,
    /// 「启动」（播放图标）。
    Start,
}

/// 菜单上跟着状态变的几行该怎么画。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Look {
    /// 状态行那一句的键。
    pub status: &'static str,
    /// 出错了时状态行下面的那一句原因。
    pub reason: Option<String>,
    pub dot: Dot,
    /// 起停那一行是「停止」还是「启动」。
    pub toggle: Toggle,
    /// 起停那一行能不能点。
    pub toggle_enabled: bool,
    /// 「重新启动」能不能点（这一行一直在，不能点时灰着）。
    pub restart_enabled: bool,
    /// 「局域网访问」能不能点：只在运行中（改配置要连得上、答得上的核心）。
    pub lan_enabled: bool,
}

/// 托盘知道的：连没连上核心、手上在做什么、最近出了什么错。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// 托盘那条连接怎样。
    linked: Linked,
    /// 托盘正在做的起停。
    busy: Option<Busy>,
    /// 「启动」做完了，等那条连接报一次再放下「正在启动…」：免得拉起来以后、连上之前闪一下「已停止」。
    waiting_link: bool,
    /// 最近一次出的错（拉不起核心、握手被拒、`miyu` 说的那一句）：连上了、再拉起时放下；断着的时候状态行写「出错了」。
    failed: Option<String>,
}

impl Status {
    /// 那条连接报来：连上了、问不出话、断了或者连不上。
    pub fn link(&mut self, linked: Linked) {
        self.linked = linked;
        if linked != Linked::Down {
            self.failed = None;
        }
        if self.waiting_link {
            self.waiting_link = false;
            self.busy = None;
        }
    }

    /// 托盘开始做一件起停的事。再拉起的放下上一次的错。
    pub fn begin(&mut self, busy: Busy) {
        self.busy = Some(busy);
        self.waiting_link = false;
        if busy == Busy::Starting {
            self.failed = None;
        }
    }

    /// 出错了，`reason` 是给人看的那一句（`miyu` 说的、核心的原话）。连着的时候照旧写「运行中」，断了写「出错了」。
    pub fn fail(&mut self, reason: String) {
        self.failed = Some(reason);
    }

    /// 托盘做完了一件起停的事，`ok` 是做成了没有。拉起来成了的再等连接报一次（见 `waiting_link`）；别的照连接写。
    pub fn done(&mut self, busy: Busy, ok: bool) {
        if self.busy != Some(busy) {
            return;
        }
        if busy == Busy::Starting && ok && self.linked == Linked::Down {
            self.waiting_link = true;
        } else {
            self.busy = None;
        }
    }

    /// 现在该写哪一种。
    pub fn state(&self) -> State {
        match (self.busy, self.linked, &self.failed) {
            (Some(Busy::Starting), ..) => State::Starting,
            (Some(Busy::Stopping), ..) => State::Stopping,
            (None, Linked::Up, _) => State::Running,
            (None, Linked::Slow, _) => State::Unresponsive,
            (None, Linked::Down, Some(reason)) => State::Failed(reason.clone()),
            (None, Linked::Down, None) => State::Stopped,
        }
    }

    /// 手上有没有在做起停：做着的时候不再接新的起停。
    pub fn is_busy(&self) -> bool {
        self.busy.is_some()
    }

    /// 连着没有（问不出话的也算连着：起停那一行是「停止」）。
    pub fn is_up(&self) -> bool {
        self.linked != Linked::Down
    }
}

/// 照一种状态画那几行（照样板：正在启动时那一行是灰的「停止」，正在停止时是灰的「启动」；「重新启动」运行中、没有响应、
/// 出错了时能点）。`lan` 是局域网开着没有：运行中的写「运行中 · 局域网」。
pub fn look(state: &State, lan: bool) -> Look {
    let (status, dot) = match state {
        State::Running if lan => (key::STATUS_RUNNING_LAN, Dot::Green),
        State::Running => (key::STATUS_RUNNING, Dot::Green),
        State::Starting => (key::STATUS_STARTING, Dot::Yellow),
        State::Stopping => (key::STATUS_STOPPING, Dot::Yellow),
        State::Stopped => (key::STATUS_STOPPED, Dot::Gray),
        State::Unresponsive => (key::STATUS_UNRESPONSIVE, Dot::Yellow),
        State::Failed(_) => (key::STATUS_FAILED, Dot::Red),
    };
    let (toggle, toggle_enabled) = match state {
        State::Running | State::Unresponsive => (Toggle::Stop, true),
        State::Stopped | State::Failed(_) => (Toggle::Start, true),
        State::Starting => (Toggle::Stop, false),
        State::Stopping => (Toggle::Start, false),
    };
    Look {
        status,
        reason: match state {
            State::Failed(reason) => Some(reason.clone()),
            _ => None,
        },
        dot,
        toggle,
        toggle_enabled,
        restart_enabled: matches!(
            state,
            State::Running | State::Unresponsive | State::Failed(_)
        ),
        lan_enabled: *state == State::Running,
    }
}

#[cfg(test)]
mod tests;
