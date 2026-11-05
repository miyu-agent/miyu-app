//! 托盘自己的目录只给自己看（原来外壳的第五批审计 D5-2：那时里面有网页视图的设备令牌）：现在放单实例的锁、「请打开」的文件，
//! 别人进得来就能抢先占锁、替人点「打开」。目录收成只有自己进得去，里面的文件写成什么权限都无所谓。

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// 托盘自己的目录在哪：Windows 是 `%LOCALAPPDATA%\miyu-applet`，macOS 是 `~/Library/Application Support/miyu-applet`，别的照 XDG 放在
/// `$XDG_STATE_HOME/miyu-applet`（没设的 `~/.local/state/miyu-applet`）。只认绝对路径；找不到的交 `None`。不放进数据根：数据根第一次
/// 由核心建，没建之前往里写东西，核心就认不出它了。
pub fn own(
    os: &str,
    home: Option<PathBuf>,
    local_app_data: Option<OsString>,
    xdg_state_home: Option<OsString>,
) -> Option<PathBuf> {
    let absolute = |path: PathBuf| path.is_absolute().then_some(path);
    let base = match os {
        "windows" => local_app_data.map(PathBuf::from).and_then(absolute)?,
        "macos" => home
            .and_then(absolute)?
            .join("Library")
            .join("Application Support"),
        _ => match xdg_state_home.map(PathBuf::from).and_then(absolute) {
            Some(state) => state,
            None => home.and_then(absolute)?.join(".local").join("state"),
        },
    };
    Some(base.join("miyu-applet"))
}

/// 照这个进程的环境找托盘自己的目录。
pub fn own_here() -> Option<PathBuf> {
    own(
        std::env::consts::OS,
        std::env::home_dir(),
        std::env::var_os("LOCALAPPDATA"),
        std::env::var_os("XDG_STATE_HOME"),
    )
}

/// 建 `dir`（连同没有的上级），Unix 上新建的和本来就有的 `dir` 都收成 0700（老装的升级上来也收紧）。别的平台照常建：
/// macOS 的 `~/Library`、Windows 的 `%LOCALAPPDATA%` 本来就只给自己。
///
/// # Errors
/// 建不出、改不了权限。
#[cfg(unix)]
pub fn private(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

/// 建 `dir`（连同没有的上级）。
///
/// # Errors
/// 建不出。
#[cfg(not(unix))]
pub fn private(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)
}

#[cfg(test)]
mod tests;
