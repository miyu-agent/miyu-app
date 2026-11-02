//! 外壳自己的数据目录只给自己看（蓝图「怎么走」第 4 条，第五批审计 D5-2）：里面的网页视图数据有设备令牌，WebKitGTK 照
//! umask 把 `localStorage` 的文件写成别人读得到的。目录收成只有自己进得去，里面的文件写成什么权限都无所谓。

use std::io;
use std::path::Path;

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
