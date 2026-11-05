//! 起不来的原因（核心 LC-1）：没代码的照原话；说法的编号、字段照核心的写法。

use super::*;

#[test]
fn without_a_code_the_reason_is_kept() {
    let root = DataRoot::at(std::env::temp_dir().join("miyu-applet-reason-none"));
    assert_eq!(say(None, "磁盘满了", &root, Lang::En), "磁盘满了");
}

#[test]
fn the_phrase_is_named_after_the_code_and_carries_the_path() {
    let root = DataRoot::at(std::path::PathBuf::from("/data/miyu"));
    let got = said("data_root_old_layout", &root);
    assert_eq!(got.key, "core/start/data_root_old_layout");
    assert_eq!(
        got.fields.get("path").map(String::as_str),
        Some(
            std::path::Path::new("/data/miyu")
                .display()
                .to_string()
                .as_str()
        )
    );
}

/// 照真的资源目录找那一句（要 `MIYU_RESOURCES` 指到核心检出的 resources/，`cargo test -- --ignored`）。
#[test]
#[ignore = "要核心的资源目录"]
fn a_coded_reason_reads_the_core_phrase_in_the_tray_language() {
    let root = DataRoot::at(std::path::PathBuf::from("/data/miyu"));
    let en = say(Some("data_root_old_layout"), "原话", &root, Lang::En);
    let ja = say(Some("data_root_old_layout"), "原话", &root, Lang::Ja);
    println!("en: {en}\nja: {ja}");
    assert!(en.contains("/data/miyu") && en != "原话", "{en}");
    assert!(ja.contains("/data/miyu") && ja != en, "{ja}");
    assert_eq!(
        say(Some("no_such_code"), "原话", &root, Lang::En),
        "原话",
        "没有那一句的照原话"
    );
}
