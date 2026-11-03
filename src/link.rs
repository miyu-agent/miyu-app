//! 托盘连核心看状态（图纸 `app.md` 第 4 条状态行、「要核心给的」第一条）：连已经在跑的核心（不拉起），出示本机令牌握手，连着就是
//! 「运行中」，断了、连不上就是「已停止」；不定时去问。连不上的时候盯着数据根的 `run/`（系统的文件变动通知）：别人把核心拉起来了，
//! 核心会在那里换令牌、写套接字的位置，一变就再连一次。
//!
//! 连法同核心门面的 `Core::connect_running`（`miyu-client` 的 `connect::connect` 加同一句 `hello`），只是自己拿着读的那一半：
//! `Core` 只能一问一答，等不了「断了」（`call` 30 秒没回就当没回应），网页软件跟着配置的那条连接（网页 `follow.rs`）也是这样写的。

use std::path::{Path, PathBuf};
use std::time::Duration;

use miyu_client::connect::Connection;
use miyu_client::places::DataRoot;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::log;

/// 握手时报的头。
pub const KIND: &str = "miyu-app";
/// 文件一变不马上连：核心起来那几步（换令牌、听套接字、写位置）一连串，等它们落定。
const SETTLE: Duration = Duration::from_millis(150);
/// 握手最多等多久（同核心门面的一问一答）。
const HELLO_LIMIT: Duration = Duration::from_secs(30);

/// 叫那条连接马上再试一次（托盘自己起停了核心以后）。
#[derive(Clone)]
pub struct Waker(UnboundedSender<()>);

impl Waker {
    pub fn wake(&self) {
        if self.0.send(()).is_err() {
            log::line("core link is gone, not woken");
        }
    }
}

/// 起一个线程连核心、跟着它：每连一次报一次（`true` 连上了，`false` 断了或者连不上），不论和上次一不一样。
pub fn spawn(root: DataRoot, report: impl Fn(bool) + Send + 'static) -> Waker {
    let (send, receive) = unbounded_channel();
    let waker = Waker(send.clone());
    let started = std::thread::Builder::new()
        .name("core-link".into())
        .spawn(move || follow(&root, send, receive, &report));
    if let Err(error) = started {
        log::line(&format!("could not start the core link: {error}"));
    }
    waker
}

fn follow(
    root: &DataRoot,
    send: UnboundedSender<()>,
    mut receive: UnboundedReceiver<()>,
    report: &dyn Fn(bool),
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            log::line(&format!("no runtime for the core link: {error}"));
            return;
        }
    };
    let mut watch = match Watch::new(root, send) {
        Ok(watch) => Some(watch),
        Err(error) => {
            log::line(&format!("not watching {}: {error}", root.run().display()));
            None
        }
    };
    runtime.block_on(async {
        let mut was = None;
        loop {
            if let Some(watch) = watch.as_mut() {
                watch.rearm();
            }
            match open(root).await {
                Ok(link) => {
                    said(&mut was, true, "connected");
                    report(true);
                    let why = link.closed().await;
                    log::line(&format!("core link closed: {why}"));
                    // 断了马上再连一次：核心崩了、被叫停的连不上，报「已停止」；只是这条连接断了的接着连上。
                    tokio::time::sleep(SETTLE).await;
                    drain(&mut receive);
                }
                Err(why) => {
                    said(&mut was, false, &why);
                    report(false);
                    if receive.recv().await.is_none() {
                        return;
                    }
                    tokio::time::sleep(SETTLE).await;
                    drain(&mut receive);
                }
            }
        }
    });
}

/// 变了才记一行。
fn said(was: &mut Option<bool>, up: bool, why: &str) {
    if *was != Some(up) {
        log::line(&format!("core link up={up}: {why}"));
        *was = Some(up);
    }
}

fn drain(receive: &mut UnboundedReceiver<()>) {
    while receive.try_recv().is_ok() {}
}

/// 连着的核心：读到头就是断了。写的那一半拿着不放：放了不碍事，拿着免得哪个平台上半关了连接。
struct Link {
    lines: BufReader<ReadHalf<Connection>>,
    _write: WriteHalf<Connection>,
}

/// 连已经在跑的核心、握手。
async fn open(root: &DataRoot) -> Result<Link, String> {
    let (connection, token) = miyu_client::connect::connect(root)
        .await
        .map_err(|error| error.to_string())?;
    let (read, mut write) = tokio::io::split(connection);
    let hello = hello(&token, &miyu_client::open::locale());
    write
        .write_all(format!("{hello}\n").as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    write.flush().await.map_err(|error| error.to_string())?;
    let mut lines = BufReader::new(read);
    tokio::time::timeout(HELLO_LIMIT, shaken(&mut lines))
        .await
        .map_err(|_| "no answer to hello".to_string())??;
    // AP-4 挂在这里：握手以后调 `notices.claim`（参数空，回 `{}`；核心 TR-1），说「后台通知由托盘来发」，核心按这条连接记、
    // 断了就作废；再订会话状态和 `notices` 流，照它们发系统通知。这一步不发，连接只用来看在不在。
    Ok(Link {
        lines,
        _write: write,
    })
}

/// 等握手的回应。
async fn shaken(lines: &mut BufReader<ReadHalf<Connection>>) -> Result<(), String> {
    let mut line = String::new();
    loop {
        line.clear();
        let read = lines
            .read_line(&mut line)
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("core disconnected".into());
        }
        if let Some(answer) = hello_answer(&line) {
            return answer;
        }
    }
}

impl Link {
    /// 一直读到断，交回为什么断。推来的东西不看（这一步不订推送）。
    async fn closed(mut self) -> String {
        let mut line = String::new();
        loop {
            line.clear();
            match self.lines.read_line(&mut line).await {
                Ok(0) => return "disconnected".into(),
                Ok(_) => {}
                Err(error) => return error.to_string(),
            }
        }
    }
}

/// 握手那一句：出示本机令牌，报自己是托盘（同核心门面 `Core::shake` 的写法）。
fn hello(token: &str, locale: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "hello",
        "method": "hello",
        "params": {
            "protocol": [1, 1],
            "token": token,
            "head": {"kind": KIND, "version": env!("CARGO_PKG_VERSION")},
            "locale": locale,
        },
    })
}

/// 读一行：是握手的回应的，交回成没成（没成的带核心的原话）；别的交 `None`。
fn hello_answer(line: &str) -> Option<Result<(), String>> {
    let reply: Value = serde_json::from_str(line).ok()?;
    if reply["id"] != json!("hello") {
        return None;
    }
    Some(match reply.get("result") {
        Some(_) => Ok(()),
        None => Err(reply["error"]["message"]
            .as_str()
            .unwrap_or("refused")
            .to_string()),
    })
}

/// 盯着 `run/`：没有的盯着数据根，数据根也没有的盯着它的上一级（第一次装上、核心还没起过）。每连一次之前重新看该盯哪里。
struct Watch {
    watcher: RecommendedWatcher,
    root: PathBuf,
    run: PathBuf,
    on: Option<PathBuf>,
}

impl Watch {
    fn new(root: &DataRoot, send: UnboundedSender<()>) -> notify::Result<Watch> {
        let inside = root.path().to_path_buf();
        let watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
                Ok(event) if relevant(&event.kind, &event.paths, &inside) => {
                    if send.send(()).is_err() {
                        log::now_and_then(
                            "watch after link",
                            "core link is gone, change not passed on",
                        );
                    }
                }
                Ok(_) => {}
                Err(error) => log::now_and_then("watch error", &format!("watching run/: {error}")),
            })?;
        Ok(Watch {
            watcher,
            root: root.path().to_path_buf(),
            run: root.run(),
            on: None,
        })
    }

    fn rearm(&mut self) {
        let want = target(&self.run, &self.root, Path::is_dir);
        if want == self.on {
            return;
        }
        if let Some(old) = self.on.take()
            && let Err(error) = self.watcher.unwatch(&old)
        {
            log::line(&format!("unwatching {}: {error}", old.display()));
        }
        if let Some(want) = want {
            match self.watcher.watch(&want, RecursiveMode::NonRecursive) {
                Ok(()) => self.on = Some(want),
                Err(error) => log::line(&format!("watching {}: {error}", want.display())),
            }
        }
    }
}

/// 该盯哪个目录：`run`，没有的退到数据根 `root`，再没有的退到它的上一级；都没有的不盯。
fn target(run: &Path, root: &Path, is_dir: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    [Some(run), Some(root), root.parent()]
        .into_iter()
        .flatten()
        .find(|dir| is_dir(dir))
        .map(Path::to_path_buf)
}

/// 这一下变动要不要再连：数据根里（含数据根本身）有东西新建、改了、删了。只是打开、读的不算（inotify 会报，托盘自己连核心
/// 时读令牌也会报，算了就停不下来）。
fn relevant(kind: &EventKind, paths: &[PathBuf], root: &Path) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    ) && paths.iter().any(|path| path.starts_with(root))
}

#[cfg(test)]
mod tests;
