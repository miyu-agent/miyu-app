//! 「关于 Miyu」（图纸 `app.md` 第 4 条）：托盘、核心、网页三样的版本和钉的提交号，系统原生的消息框里写出来。
//!
//! - 托盘：这个程序编的时候的版本。
//! - 核心：`miyu --version` 回的那一行。
//! - 网页：资源目录里网页包清单的 `version`（打包时 `packaging/stage.py` 照网页仓库写进去的）。
//! - 钉的提交号：`pins.toml` 编进程序，和打包流水线照着编核心、网页的是同一份。

use std::path::Path;

use crate::program;
use crate::text::{Text, key};

/// 网页包的编号：资源目录 `packages/` 下它的文件夹名（`packaging/stage.py` 的 `WEB_PACKAGE`）。
const WEB_PACKAGE: &str = "miyu-webui";

/// 钉的一样：仓库和提交号。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub repo: String,
    pub rev: String,
}

/// 钉的两样。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pins {
    pub core: Option<Pin>,
    pub web: Option<Pin>,
}

/// 读 `pins.toml`：缺的、写错的那一样是 `None`。
pub fn pins(text: &str) -> Pins {
    let Ok(table) = text.parse::<toml::Table>() else {
        return Pins::default();
    };
    let pin = |part: &str| {
        let part = table.get(part)?.as_table()?;
        Some(Pin {
            repo: part.get("repo")?.as_str()?.to_owned(),
            rev: part.get("rev")?.as_str()?.to_owned(),
        })
    };
    Pins {
        core: pin("core"),
        web: pin("web"),
    }
}

/// `miyu --version` 回的：第一行去掉开头的程序名（`miyu 0.0.0` → `0.0.0`）。空的交 `None`。
pub fn core_version(stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stdout);
    let mut words = text.lines().next()?.split_whitespace().peekable();
    if words.peek() == Some(&"miyu") {
        words.next();
    }
    let version = words.collect::<Vec<_>>().join(" ");
    (!version.is_empty()).then_some(version)
}

/// 网页包清单里 `[package]` 的 `version`。
pub fn web_version(manifest: &str) -> Option<String> {
    let table = manifest.parse::<toml::Table>().ok()?;
    Some(
        table
            .get("package")?
            .as_table()?
            .get("version")?
            .as_str()?
            .to_owned(),
    )
}

/// 拼消息框里的字：一样一行，钉了提交号的下面再一行。
pub fn describe(
    text: &Text,
    app: &str,
    core: Option<&str>,
    web: Option<&str>,
    pins: &Pins,
) -> String {
    let unknown = text.get(key::ABOUT_UNKNOWN);
    let mut lines = vec![format!("{}: {app}", text.get(key::ABOUT_APP))];
    for (name, version, pin) in [
        (key::ABOUT_CORE, core, pins.core.as_ref()),
        (key::ABOUT_WEB, web, pins.web.as_ref()),
    ] {
        lines.push(format!(
            "{}: {}",
            text.get(name),
            version.unwrap_or(unknown)
        ));
        if let Some(pin) = pin {
            lines.push(format!("  {}@{}", pin.repo, pin.rev));
        }
    }
    lines.join("\n")
}

/// 照这台机器找齐三样，拼好。要调一次 `miyu --version`，在后台线程里调。
pub fn gather(text: &Text, miyu: Option<&Path>) -> String {
    let core =
        miyu.and_then(
            |miyu| match program::run(miyu, program::VERSION, program::VERSION_LIMIT) {
                Ok(ran) if ran.code == Some(0) => core_version(&ran.stdout),
                Ok(ran) => {
                    crate::log::line(&format!("miyu --version exited {:?}", ran.code));
                    None
                }
                Err(error) => {
                    crate::log::line(&format!("miyu --version: {error:?}"));
                    None
                }
            },
        );
    let web = web_manifest().as_deref().and_then(web_version);
    describe(
        text,
        env!("CARGO_PKG_VERSION"),
        core.as_deref(),
        web.as_deref(),
        &pins(include_str!("../pins.toml")),
    )
}

/// 资源目录里网页包的清单；资源目录照核心找它的办法找（程序旁边的 `resources/`、上一级的 `share/miyu/`……）。
fn web_manifest() -> Option<String> {
    let env = miyu_client::places::Env::current();
    let resources = miyu_client::places::ResourceRoot::locate(&env).ok()?;
    let path = resources
        .path()
        .join("packages")
        .join(WEB_PACKAGE)
        .join("package.toml");
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests;
