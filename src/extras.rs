//! 菜单上照核心来的那几行（图纸 `app.md` 第 4 条）：软件包挂的入口（核心 TR-2）。只有读和算，不碰界面、不碰连接，单测钉住。
//!
//! - 软件包：`subscribe {stream: "packages"}` 的回应是整份列表，之后推 `package.changed {package, entry}`（没有了的 `entry` 是 `null`）。
//!   一项带 `tray: {name}`（有后台页、清单写了 `[tray]`，名字照连接的语言挑好的）、开着的才列，一个包一行，照编号排（同 `package.list`）。

use std::collections::BTreeMap;

use serde_json::Value;

/// 托盘菜单里一个软件包的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// 包的编号：点了 `miyu web --package <编号>`。
    pub package: String,
    /// 那一行的字。
    pub name: String,
}

/// 照 `packages` 流记着的软件包列表（只留算入口要的那一项项）。
#[derive(Debug, Default)]
pub struct Packages {
    items: BTreeMap<String, Value>,
}

impl Packages {
    /// 订阅的回应：整份换掉。
    pub fn replace(&mut self, reply: &Value) {
        self.items = reply["packages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| Some((item["package"].as_str()?.to_string(), item.clone())))
            .collect();
    }

    /// 推来的 `package.changed` 的参数：换掉、删掉那一项。
    pub fn change(&mut self, params: &Value) {
        let Some(package) = params["package"].as_str() else {
            return;
        };
        match &params["entry"] {
            Value::Null => {
                self.items.remove(package);
            }
            entry => {
                self.items.insert(package.to_string(), entry.clone());
            }
        }
    }

    /// 托盘菜单该列的那几行，照编号排。
    pub fn entries(&self) -> Vec<Entry> {
        self.items
            .iter()
            .filter(|(_, item)| switched_on(item))
            .filter_map(|(package, item)| {
                let name = item["tray"]["name"].as_str()?.trim();
                (!name.is_empty()).then(|| Entry {
                    package: package.clone(),
                    name: name.to_string(),
                })
            })
            .collect()
    }
}

/// 开着没有：开关关着的（`enabled: false`）、卸掉了的出厂包、`status` 是关着、程序不在、缺依赖的不算；没有开关的（内置包）算开着。
fn switched_on(item: &Value) -> bool {
    item["enabled"] != Value::Bool(false)
        && item["removed"] != Value::Bool(true)
        && !matches!(
            item["status"].as_str(),
            Some("off" | "program_missing" | "depends_missing")
        )
}

#[cfg(test)]
mod tests;
