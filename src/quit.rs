//! 叫核心停（图纸 `app.md` 第 4 条「停止」「退出 Miyu」，原来外壳的彻底退出）：调 `miyu core stop`，软件包跟着停。
//! 它最多等 60 秒；退出码 1 的标准错误上有一句「miyu: …」，照它的语言，带回去给人看。
//! 「退出 Miyu」没停成的照样退，记一行日志。
//!
//! 「停止」「退出 Miyu」先 `miyu web --stop` 停网页软件（网页 W-1091）：不停它的话，开着的页面一重连，网页软件照
//! `connect_or_start` 一秒内又把核心拉起来（2026-11-03 在 Windows 虚拟机上测出来的）。停没停成都接着停核心，只记日志：旧网页
//! 不认这个参数（退出码 2），核心停了以后它照自己的规矩空闲了退。「重新启动」不停它，页面接着连新起来的核心。

use std::path::Path;

use crate::log;
use crate::program::{self, RunError};
use crate::text::{Text, key};

/// 叫网页软件停（`miyu web --stop`）：只记日志，不交结果。
pub fn stop_web(miyu: &Path) {
    match program::run(miyu, program::STOP_WEB, program::STOP_WEB_LIMIT) {
        Ok(ran) if ran.code == Some(0) => log::line("web stopped"),
        Ok(ran) if ran.code == Some(program::USAGE) => {
            log::line("the web app does not know --stop, leaving it to idle out");
        }
        Ok(ran) => log::line(&format!(
            "web stop exited {:?}: {}",
            ran.code,
            program::tail(&ran.stderr)
        )),
        Err(RunError::Timeout) => log::line("web stop timed out"),
        Err(RunError::Spawn(error) | RunError::Wait(error)) => {
            log::line(&format!("web stop: {error}"));
        }
    }
}

/// 叫核心停。停了（或者本来就没在跑）交 `Ok`；没停成交给人看的一句。
///
/// # Errors
/// 没停成：`miyu` 说的那几句、超时、起不来。
pub fn stop_core(miyu: &Path, text: &Text) -> Result<(), String> {
    let got = program::run(miyu, program::STOP, program::STOP_LIMIT);
    match &got {
        Ok(ran) if ran.code == Some(0) => {
            log::line("core stopped");
            Ok(())
        }
        Ok(ran) => {
            let said = program::tail(&ran.stderr);
            log::line(&format!("core stop exited {:?}: {said}", ran.code));
            Err(said)
        }
        Err(RunError::Timeout) => {
            log::line("core stop timed out");
            Err(text.get(key::TIMEOUT).to_owned())
        }
        Err(RunError::Spawn(error) | RunError::Wait(error)) => {
            log::line(&format!("core stop: {error}"));
            Err(error.to_string())
        }
    }
}
