use super::*;

use std::sync::Mutex;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf};

use crate::alerts::{Note, Why};

/// 另一头当核心：读托盘问的，回它。
struct Core {
    read: BufReader<ReadHalf<DuplexStream>>,
    write: WriteHalf<DuplexStream>,
}

impl Core {
    async fn asked(&mut self, method: &str) -> Value {
        let mut line = String::new();
        self.read.read_line(&mut line).await.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], method, "{request}");
        request
    }

    async fn reply(&mut self, request: &Value, result: Value) {
        self.say(&json!({"jsonrpc": "2.0", "id": request["id"], "result": result}))
            .await;
    }

    async fn push(&mut self, method: &str, params: Value) {
        self.say(&json!({"jsonrpc": "2.0", "method": method, "params": params}))
            .await;
    }

    async fn say(&mut self, message: &Value) {
        let line = format!("{message}\n");
        self.write.write_all(line.as_bytes()).await.unwrap();
    }
}

/// 托盘这头读推送、报通知，直到报够 `count` 条。
async fn follow_until(
    rpc: &mut Rpc<ReadHalf<DuplexStream>, WriteHalf<DuplexStream>>,
    count: usize,
) -> Vec<Note> {
    let notes = Mutex::new(Vec::new());
    let report = |got: Report| {
        if let Report::Note(note) = got {
            notes.lock().unwrap().push(note);
        }
    };
    let mut slow = |_: bool| {};
    let mut notices = Some(Follow::start(rpc, &mut slow).await.unwrap());
    while notes.lock().unwrap().len() < count {
        let push = super::super::next_push(rpc, &mut notices, &report)
            .await
            .unwrap();
        let follow = notices.as_mut().unwrap();
        follow.push(rpc, &push, &report, &mut slow).await.unwrap();
    }
    notes.into_inner().unwrap()
}

fn note(session: Option<&str>, why: Why, title: &str, tag: &str) -> Note {
    Note {
        session: session.map(str::to_owned),
        why,
        title: title.into(),
        tag: tag.into(),
    }
}

#[test]
fn the_session_table_and_notices_become_notes() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (ours, theirs) = tokio::io::duplex(64 * 1024);
        let (read, write) = tokio::io::split(ours);
        let mut rpc = Rpc::new(read, write);
        let (core_read, core_write) = tokio::io::split(theirs);
        let mut core = Core {
            read: BufReader::new(core_read),
            write: core_write,
        };
        let core = async move {
            let asked = core.asked("subscribe").await;
            assert_eq!(asked["params"], json!({"stream": "sessions"}));
            let table = json!({"sessions": [
                {"session": "a", "busy": true, "title": "修 CI"},
                {"session": "b", "busy": true, "title": "看日志", "viewed": true},
            ]});
            core.reply(&asked, table).await;
            let asked = core.asked("subscribe").await;
            assert_eq!(asked["params"], json!({"stream": "notices"}));
            core.reply(&asked, json!({"claimed": true})).await;
            // 网页开着的 b 做完了：不弹。a 在问：托盘读那一轮的事件看是哪一种。
            core.push(
                "sessions.changed",
                json!({"session": "b", "entry": {"title": "看日志", "viewed": true}}),
            )
            .await;
            core.push(
                "sessions.changed",
                json!({"session": "a", "entry": {"busy": true, "waiting": true, "title": "修 CI"}}),
            )
            .await;
            let asked = core.asked("view.page").await;
            assert_eq!(asked["params"], json!({"session": "a", "turns": 1}));
            let events = json!({"events": [
                {"kind": "question.asked", "body": {"call_id": "c1"}},
            ]});
            core.reply(&asked, events).await;
            core.push(
                "sessions.changed",
                json!({"session": "a", "entry": {"title": "修 CI"}}),
            )
            .await;
            // 定时任务那个会话会话表里还没有：照 `schedule.list` 找那句话。
            core.push(
                "notice",
                json!({"kind": "schedule.ran", "schedule": "t1", "session": "s", "ok": true}),
            )
            .await;
            let asked = core.asked("schedule.list").await;
            let schedules = json!({"schedules": [{"id": "t1", "prompt": "汇总昨天的提交"}]});
            core.reply(&asked, schedules).await;
            core.push("notice", json!({"kind": "schedules.reset", "at": "…"}))
                .await;
            core
        };
        let (notes, _core) = tokio::join!(follow_until(&mut rpc, 4), core);
        assert_eq!(
            notes,
            [
                note(Some("a"), Why::Ask, "修 CI", "a:ask"),
                note(Some("a"), Why::Done, "修 CI", "a:done"),
                note(Some("s"), Why::ScheduleRan, "汇总昨天的提交", "s:done"),
                note(None, Why::SchedulesReset, "Miyu", "schedules.reset"),
            ]
        );
    });
}
