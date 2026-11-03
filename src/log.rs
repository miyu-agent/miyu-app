//! 托盘自己的日志（图纸 `app.md` 第 6 条）：一行一条，写进数据根的 `var/log/miyu-app.log`，和核心、网页的日志放在一起（菜单
//! 「打开日志所在的文件夹」打开的就是那里）。只记起来、调 `miyu` 的结果和耗时、连核心的变化、退出。数据根的 `var/log/` 由核心建：
//! 还没有的（核心没起过）先写到标准错误上，等连上核心再定下文件（[`start`] 只认第一次）。
//! 文件到 1 MiB 挪成 `miyu-app.log.1`（只留一份旧的）；会一下来很多的事同一种一分钟只记一行（原来外壳第五批审计 D5-6）。

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static FILE: OnceLock<PathBuf> = OnceLock::new();
static WRITING: Mutex<()> = Mutex::new(());
static QUIET: Mutex<Quiet> = Mutex::new(Quiet::new());

/// 日志文件到这么大就挪成旧的。
const LIMIT: u64 = 1024 * 1024;
/// 同一种吵的事多久最多记一行。
const QUIET_FOR: Duration = Duration::from_secs(60);

/// 定下日志文件。只认第一次。
pub fn start(path: PathBuf) {
    if let Err(path) = FILE.set(path) {
        line(&format!(
            "log already started, not moving to {}",
            path.display()
        ));
    }
}

/// 日志文件定下了没有。
pub fn is_started() -> bool {
    FILE.get().is_some()
}

/// 记一行：前面是 Unix 时间（秒，三位小数）。
pub fn line(message: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let entry = format!("{}.{:03} {message}\n", now.as_secs(), now.subsec_millis());
    let Some(path) = FILE.get() else {
        eprint!("{entry}");
        return;
    };
    let _guard = WRITING.lock().unwrap_or_else(PoisonError::into_inner);
    if let Err(error) = rotate(path, LIMIT) {
        eprintln!("(not rotating {}: {error})", path.display());
    }
    let written = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(entry.as_bytes()));
    if let Err(error) = written {
        eprintln!("{entry}(not written to {}: {error})", path.display());
    }
}

/// 会一下来很多的事记一行：同一种（`kind`）`QUIET_FOR` 里只记第一次，下次记的时候带上中间没记的次数。
pub fn now_and_then(kind: &'static str, message: &str) {
    let allowed = QUIET
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .allow(kind, Instant::now());
    match allowed {
        None => {}
        Some(0) => line(message),
        Some(skipped) => line(&format!("{message} ({skipped} more like this not logged)")),
    }
}

/// `path` 到 `limit` 字节就挪成 `<path>.1`（原来的 `.1` 换掉）；没有这个文件的不管。
///
/// # Errors
/// 读不了大小、挪不动。
fn rotate(path: &Path, limit: u64) -> io::Result<()> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.len() >= limit => std::fs::rename(path, old(path)),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// 旧的那一份：`miyu-app.log` → `miyu-app.log.1`。
fn old(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".1");
    PathBuf::from(name)
}

/// 记着每一种吵的事上次记是什么时候、之后又来了几次没记。种类是代码里写死的几个，不会无限多。
struct Quiet {
    seen: Vec<(&'static str, Instant, u64)>,
}

impl Quiet {
    const fn new() -> Self {
        Self { seen: Vec::new() }
    }

    /// 这一次记不记：记的交回上次以来没记的次数，不记的交 `None`。
    fn allow(&mut self, kind: &'static str, now: Instant) -> Option<u64> {
        let Some(entry) = self.seen.iter_mut().find(|(k, _, _)| *k == kind) else {
            self.seen.push((kind, now, 0));
            return Some(0);
        };
        if now.duration_since(entry.1) < QUIET_FOR {
            entry.2 += 1;
            return None;
        }
        let skipped = entry.2;
        entry.1 = now;
        entry.2 = 0;
        Some(skipped)
    }
}

#[cfg(test)]
mod tests;
