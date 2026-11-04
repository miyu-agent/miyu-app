//! 托盘那条连接上的系统通知（图纸 `app.md` 第 5 条，施工 AP-4）：认领了后台通知（`notices.claim`，核心 TR-1）的连接订会话表
//! （`sessions`）和账号级的提醒（`notices`），照它们报 [`Report::Note`]，弹不弹、写什么在 `alerts` 里定。掉了队（`resync`）的
//! 重新订：会话表整份换上，掉队那一段里的变化不补；提醒本来就不补。
//!
//! 会话表只说「在等人」（`waiting`），不说在问还是要确认：冒出来的时候读那个会话最新一轮的事件（`view.page {turns: 1}`）看。

use std::time::Instant;

use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};

use super::{Report, soft};
use crate::alerts::{self, Rise, Sessions};
use crate::rpc::{Failure, Rpc};

/// 跟着会话表、提醒的那一份。
#[derive(Default)]
pub(super) struct Follow {
    sessions: Sessions,
}

impl Follow {
    /// 订会话表和提醒。订不上的（老核心）记一行，这条连接照用。
    pub(super) async fn start<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        rpc: &mut Rpc<R, W>,
        slow: &mut dyn FnMut(bool),
    ) -> Result<Self, Failure> {
        let mut follow = Self::default();
        follow.subscribe_sessions(rpc, slow).await?;
        subscribe_notices(rpc, slow).await?;
        Ok(follow)
    }

    async fn subscribe_sessions<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &mut self,
        rpc: &mut Rpc<R, W>,
        slow: &mut dyn FnMut(bool),
    ) -> Result<(), Failure> {
        let params = json!({"stream": "sessions"});
        if let Some(reply) = soft(
            "subscribe sessions",
            rpc.call("subscribe", params, slow).await,
        )? {
            self.sessions.replace(&reply);
        }
        Ok(())
    }

    /// 最早一条在等标题的等到什么时候。
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.sessions.next_due()
    }

    /// 到点了还没等到标题的，照第一句发。
    pub(super) fn due(&mut self, report: &dyn Fn(Report)) {
        for note in self.sessions.due(Instant::now()) {
            report(Report::Note(note));
        }
    }

    /// 推来的一条：会话表的变动、提醒、掉了队。别的不理。
    pub(super) async fn push<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &mut self,
        rpc: &mut Rpc<R, W>,
        push: &Value,
        report: &dyn Fn(Report),
        slow: &mut dyn FnMut(bool),
    ) -> Result<(), Failure> {
        let params = &push["params"];
        match push["method"].as_str().unwrap_or_default() {
            "sessions.changed" => {
                let changed = self.sessions.change(params);
                for note in changed.ready {
                    report(Report::Note(note));
                }
                let note = match changed.rise {
                    Some(Rise::Done(id)) => {
                        self.sessions.note(&id, alerts::Why::Done, Instant::now())
                    }
                    Some(Rise::Waiting(id)) => {
                        let page = json!({"session": id, "turns": 1});
                        let got = soft("view.page", rpc.call("view.page", page, slow).await)?;
                        let why = alerts::waiting_why(&got.unwrap_or_default());
                        self.sessions.note(&id, why, Instant::now())
                    }
                    None => None,
                };
                if let Some(note) = note {
                    report(Report::Note(note));
                }
            }
            "notice" => {
                let prompt = match self.sessions.wants_prompt(params) {
                    Some(schedule) => prompt_of(rpc, &schedule, slow).await?,
                    None => None,
                };
                if let Some(note) = self.sessions.notice(params, prompt.as_deref()) {
                    report(Report::Note(note));
                }
            }
            "resync" => match params["stream"].as_str() {
                Some("sessions") => self.subscribe_sessions(rpc, slow).await?,
                Some("notices") => subscribe_notices(rpc, slow).await?,
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

/// 订提醒。回应里的 `claimed`（有没有人认领）托盘不用看：认领的就是它自己。
async fn subscribe_notices<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    slow: &mut dyn FnMut(bool),
) -> Result<(), Failure> {
    let params = json!({"stream": "notices"});
    soft(
        "subscribe notices",
        rpc.call("subscribe", params, slow).await,
    )?;
    Ok(())
}

/// 定时任务那句话（会话表里还没有那个会话的标题时当标题）：照 `schedule.list` 找；找不到的是 `None`。
async fn prompt_of<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    schedule: &str,
    slow: &mut dyn FnMut(bool),
) -> Result<Option<String>, Failure> {
    let list = soft(
        "schedule.list",
        rpc.call("schedule.list", json!({}), slow).await,
    )?;
    Ok(list.and_then(|list| {
        list["schedules"]
            .as_array()?
            .iter()
            .find(|entry| entry["id"] == schedule)?["prompt"]
            .as_str()
            .map(str::to_owned)
    }))
}
#[cfg(test)]
mod tests;
