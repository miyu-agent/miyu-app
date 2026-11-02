use serde_json::Value;

/// 页面能碰到的外壳只有这一个口子（蓝图「怎么走」第 5 条）：谁要往里加东西，先改图纸、找项目主人。
#[test]
fn the_page_may_only_ask_for_notifications() {
    let text = include_str!("../../capabilities/page-notify.json");
    let capability: Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        capability["permissions"],
        serde_json::json!(["allow-notify"])
    );
    assert_eq!(
        capability["remote"]["urls"],
        serde_json::json!(["http://127.0.0.1:*"])
    );
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    assert_eq!(
        capability["platforms"],
        serde_json::json!(["windows", "macOS"])
    );
}

#[test]
fn only_that_capability_is_turned_on() {
    let config: Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    assert_eq!(
        config["app"]["security"]["capabilities"],
        serde_json::json!(["page-notify"])
    );
    assert_eq!(config["app"]["withGlobalTauri"], serde_json::json!(false));
}

#[test]
fn the_stand_in_calls_nothing_but_notify() {
    let script = include_str!("../../resources/notification.js");
    let calls: Vec<&str> = script
        .match_indices("invoke(")
        .map(|(i, _)| &script[i..i + 16])
        .collect();
    assert_eq!(calls, ["invoke('notify',"], "{calls:?}");
}

#[test]
fn the_stand_in_passes_silent_along() {
    let script = include_str!("../../resources/notification.js");
    assert!(script.contains("this.silent = options.silent === true;"));
    assert!(script.contains("body: this.body, silent: this.silent })"));
}
