//! 核心起不来的原因写给人看（核心 LC-1，61f1f3ef2）：数据根认不出的那几种带代码（`data_root_not_ours`、`data_root_old_layout`、
//! `data_root_other_layout`），照核心资源目录里的说法 `core/start/<代码>`、托盘的语言写，`{path}` 填数据根的位置；没代码的（读写出错、
//! 老核心）、资源目录找不到、那一句没有的，照核心给的原话。原来一律是原话，原话写死中文，英文、日文系统上也是中文（AP-3 测出来的）。

use std::collections::BTreeMap;

use miyu_client::places::{DataRoot, Env, ResourceRoot};
use miyu_client::protocol::Said;
use miyu_client::texts::Human;

use crate::text::Lang;

/// 带 `code` 的照说法写，别的照 `reason`。
pub fn say(code: Option<&str>, reason: &str, root: &DataRoot, lang: Lang) -> String {
    code.and_then(|code| coded(code, root, lang))
        .unwrap_or_else(|| reason.to_owned())
}

/// 照代码找那一句；资源目录、那种语言的字、那一句有一样没有的是空的。
fn coded(code: &str, root: &DataRoot, lang: Lang) -> Option<String> {
    let resources = ResourceRoot::locate(&Env::current()).ok()?;
    let human = Human::load(&resources, lang.code()).ok()?;
    human.say(&said(code, root))
}

/// `core/start/<代码>` 那一句，`path` 是数据根的位置。
fn said(code: &str, root: &DataRoot) -> Said {
    Said {
        key: format!("core/start/{code}"),
        fields: BTreeMap::from([("path".to_owned(), root.path().display().to_string())]),
    }
}

#[cfg(test)]
mod tests;
