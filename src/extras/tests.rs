use super::*;

use serde_json::json;

fn listed() -> Value {
    json!({"packages": [
        {"package": "miyu-im", "kind": "process", "enabled": true, "status": "running", "page": true, "tray": {"name": "打开通讯平台接入界面"}},
        {"package": "hello", "kind": "process", "enabled": false, "status": "off", "page": true, "tray": {"name": "打开打招呼"}},
        {"package": "aaa", "kind": "builtin", "page": true, "status": "ready", "tray": {"name": "  看一眼  "}},
        {"package": "no-tray", "kind": "process", "enabled": true, "status": "running", "page": true},
        {"package": "broken", "layer": "home", "code": "bad_tray", "problem": "…", "line": 3},
    ]})
}

fn names(packages: &Packages) -> Vec<(String, String)> {
    packages
        .entries()
        .into_iter()
        .map(|entry| (entry.package, entry.name))
        .collect()
}

#[test]
fn only_switched_on_packages_with_a_tray_line_are_listed_by_id() {
    let mut packages = Packages::default();
    packages.replace(&listed());
    assert_eq!(
        names(&packages),
        [
            ("aaa".to_string(), "看一眼".to_string()),
            ("miyu-im".to_string(), "打开通讯平台接入界面".to_string()),
        ]
    );
}

#[test]
fn a_change_replaces_or_removes_one_package() {
    let mut packages = Packages::default();
    packages.replace(&listed());
    packages.change(&json!({"package": "hello", "entry": {"package": "hello", "enabled": true, "status": "starting", "tray": {"name": "打开打招呼"}}}));
    packages.change(&json!({"package": "miyu-im", "entry": null}));
    packages.change(&json!({"package": "aaa", "entry": {"package": "aaa", "status": "program_missing", "tray": {"name": "看一眼"}}}));
    assert_eq!(
        names(&packages),
        [("hello".to_string(), "打开打招呼".to_string())]
    );
    // 认不出的推送不动列表。
    packages.change(&json!({"entry": null}));
    assert_eq!(packages.entries().len(), 1);
}

#[test]
fn an_empty_or_odd_reply_lists_nothing() {
    let mut packages = Packages::default();
    packages.replace(&json!({}));
    assert!(packages.entries().is_empty());
    packages.replace(&json!({"packages": [{"package": "x", "tray": {"name": ""}}, {"tray": {"name": "没编号"}}]}));
    assert!(packages.entries().is_empty());
}
