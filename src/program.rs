//! 调装在旁边的主程序 `miyu`（蓝图「它是什么」那张表、「怎么走」第 2、3、12 条）：外壳不连核心，只调它。

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use url::Url;

use crate::navigate::Origin;

/// 开窗口时要网址的那一句。
pub const OPEN: &[&str] = &["web", "--print", "--format", "json", "--ticket"];
/// 彻底退出时叫核心停的那一句。
pub const STOP: &[&str] = &["core", "stop"];
/// 要网址最多等多久：第一次要先拉起核心和网页软件。
pub const OPEN_LIMIT: Duration = Duration::from_secs(30);
/// 叫核心停最多等多久：`miyu core stop` 自己最多等 60 秒。
pub const STOP_LIMIT: Duration = Duration::from_secs(70);
/// 出错时带回标准错误的最后几行。
const TAIL_LINES: usize = 6;

/// 找主程序：`MIYU_PROGRAM` 写了的用它；再看外壳自己所在的目录；再照 `PATH` 找，只认里面的绝对目录（第五批审计 D5-4：
/// 空项、`.`、相对目录照外壳的当前目录解析，图形程序的当前目录说不准）。`exists` 说那个文件在不在。
pub fn locate(
    program: Option<OsString>,
    own_dir: Option<&Path>,
    path: Option<OsString>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if let Some(program) = program.filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(program));
    }
    let name = format!("miyu{}", std::env::consts::EXE_SUFFIX);
    let beside = own_dir.map(|dir| dir.join(&name));
    let on_path = path
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .filter(|dir| dir.is_absolute());
    beside
        .into_iter()
        .chain(on_path.map(|dir| dir.join(&name)))
        .find(|p| exists(p))
}

/// 照这个进程的环境找主程序。
pub fn locate_here() -> Option<PathBuf> {
    let own = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    locate(
        std::env::var_os("MIYU_PROGRAM"),
        own.as_deref(),
        std::env::var_os("PATH"),
        Path::is_file,
    )
}

/// 调了一次，跑完了。
#[derive(Debug)]
pub struct Ran {
    /// 退出码；被信号杀掉的没有。
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// 调不成、没跑完。
#[derive(Debug)]
pub enum RunError {
    /// 起不来（找不到、没有权限……）。
    Spawn(std::io::Error),
    /// 等它退出时出错。
    Wait(std::io::Error),
    /// 过了时限还没退，杀掉了。
    Timeout,
}

/// 调 `program args`，最多等 `limit`；标准输入是空的，标准输出、标准错误收下来。Windows 上不开控制台窗口。
///
/// # Errors
/// 起不来、等不了、超时。
pub fn run(program: &Path, args: &[&str], limit: Duration) -> Result<Ran, RunError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_console(&mut command);
    let mut child = command.spawn().map_err(RunError::Spawn)?;
    let stdout = collect(child.stdout.take());
    let stderr = collect(child.stderr.take());
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < limit => thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                if let Err(error) = child.kill().and_then(|()| child.wait().map(drop)) {
                    crate::log::line(&format!("could not kill {}: {error}", program.display()));
                }
                return Err(RunError::Timeout);
            }
            Err(error) => return Err(RunError::Wait(error)),
        }
    };
    Ok(Ran {
        code: status.code(),
        stdout: gather(stdout),
        stderr: gather(stderr),
    })
}

/// 在另一个线程里读一个管道，读到一块交一块。不等读到头：Windows 上它拉起的程序（核心、网页软件）会继承这条管道一直拿着，
/// 读到头要等它们退出（施工 DT-6 在 Windows 上碰到：第一次开窗口时拿不到网址）。
fn collect(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<Vec<u8>> {
    let (send, receive) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => return,
                    Ok(n) => {
                        if send.send(chunk[..n].to_vec()).is_err() {
                            return;
                        }
                    }
                    Err(error) => {
                        crate::log::line(&format!("reading a pipe: {error}"));
                        return;
                    }
                }
            }
        });
    }
    receive
}

/// 程序退出以后收下已经读到的：管道读到头了的立刻交，没到头的（还有别的程序拿着）再有 200 毫秒没新东西就交。
fn gather(receive: mpsc::Receiver<Vec<u8>>) -> Vec<u8> {
    let mut bytes = Vec::new();
    while let Ok(chunk) = receive.recv_timeout(QUIET) {
        bytes.extend_from_slice(&chunk);
    }
    bytes
}

/// 程序退出以后，管道多久没新东西就当读完了。
const QUIET: Duration = Duration::from_millis(200);

#[cfg(windows)]
fn no_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW`：图形程序调命令行程序时不闪一个黑窗口。
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_console(_: &mut Command) {}

/// 要网址没要成。
#[derive(Debug, PartialEq, Eq)]
pub enum OpenError {
    /// 退出码不是 0：带回标准错误的最后几行。
    Failed(String),
    /// 回的那一行读不懂。
    BadReply,
}

/// 读 `miyu web --print --format json --ticket` 跑完的结果：退出码 0、第一行是 `{"url": …}`，网址是网页软件的来源。
///
/// # Errors
/// 见 [`OpenError`]。
pub fn read_open(ran: &Ran) -> Result<(Url, Origin), OpenError> {
    if ran.code != Some(0) {
        return Err(OpenError::Failed(tail(&ran.stderr)));
    }
    let text = String::from_utf8_lossy(&ran.stdout);
    let line = text.lines().next().ok_or(OpenError::BadReply)?;
    #[derive(serde::Deserialize)]
    struct Reply {
        url: String,
    }
    let reply: Reply = serde_json::from_str(line).map_err(|_| OpenError::BadReply)?;
    let url: Url = reply.url.parse().map_err(|_| OpenError::BadReply)?;
    let origin = Origin::of(&url).ok_or(OpenError::BadReply)?;
    Ok((url, origin))
}

/// 标准错误的最后几行（`miyu` 照人的语言说的那几句），去掉空行。
pub fn tail(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n")
}

#[cfg(test)]
mod tests;
