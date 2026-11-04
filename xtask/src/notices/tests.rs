use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::json;

use super::{Shipped, page, read, shipped, texts};

/// 一个空的临时目录，测试完删掉。
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("xtask-notices-{name}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("清得掉上次留下的");
        }
        std::fs::create_dir_all(&dir).expect("建得了");
        Scratch(dir)
    }

    fn put(&self, path: &str, text: &str) -> PathBuf {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().expect("有上一级")).expect("建得了");
        std::fs::write(&path, text).expect("写得进");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("临时目录没删掉：{error}");
        }
    }
}

fn package(dir: &Path, license: Option<&str>, authors: &[&str]) -> Shipped {
    Shipped {
        name: "pudding".to_string(),
        version: "1.0.0".to_string(),
        license: license.map(str::to_string),
        authors: authors.iter().map(|a| (*a).to_string()).collect(),
        dir: dir.to_path_buf(),
        license_file: None,
    }
}

#[test]
fn only_what_normal_dependencies_reach_is_shipped() {
    let node = |id: &str, deps: serde_json::Value| json!({"id": id, "deps": deps});
    let dep =
        |pkg: &str, kind: serde_json::Value| json!({"pkg": pkg, "dep_kinds": [{"kind": kind}]});
    let json = json!({
        "workspace_members": ["me"],
        "packages": [
            {"id": "me", "name": "me", "version": "0.0.0", "manifest_path": "/w/me/Cargo.toml"},
            {"id": "a", "name": "a", "version": "1.0.0", "license": "MIT", "authors": ["A <a@x>"],
             "manifest_path": "/r/a-1.0.0/Cargo.toml", "license_file": null},
            {"id": "b", "name": "b", "version": "2.0.0", "manifest_path": "/r/b-2.0.0/Cargo.toml"},
            {"id": "dev", "name": "dev", "version": "1.0.0", "manifest_path": "/r/dev/Cargo.toml"},
            {"id": "build", "name": "build", "version": "1.0.0", "manifest_path": "/r/build/Cargo.toml"},
            {"id": "devdep", "name": "devdep", "version": "1.0.0", "manifest_path": "/r/devdep/Cargo.toml"},
            {"id": "client", "name": "miyu-client", "version": "0.0.0", "manifest_path": "/g/miyu/crates/miyu-client/Cargo.toml",
             "source": "git+https://github.com/miyu-agent/miyu?rev=abc#abc"},
            {"id": "lookalike", "name": "lookalike", "version": "1.0.0", "manifest_path": "/g/miyu-im/Cargo.toml",
             "source": "git+https://github.com/miyu-agent/miyu-im?rev=def#def"},
        ],
        "resolve": {"nodes": [
            node("me", json!([dep("a", json!(null)), dep("dev", json!("dev")), dep("build", json!("build")), dep("client", json!(null)), dep("lookalike", json!(null))])),
            node("a", json!([dep("b", json!(null))])),
            node("b", json!([])),
            node("dev", json!([dep("devdep", json!(null))])),
            node("build", json!([])),
            node("devdep", json!([])),
            node("client", json!([])),
            node("lookalike", json!([])),
        ]},
    });
    let got: Vec<String> = shipped(&json).into_iter().map(|p| p.name).collect();
    // 顺着正常依赖走：a、a 带进来的 b；开发、构建依赖和它们带进来的不算，工作区自己不算，照 git 钉住的核心自己的 crate 不算
    // （来源名字只是开头像的别的仓库照算）。
    assert_eq!(got, ["a", "b", "lookalike"]);
    let a = &shipped(&json)[0];
    assert_eq!(a.dir, Path::new("/r/a-1.0.0"));
    assert_eq!(a.authors, ["A <a@x>"]);
    // 门禁的小程序也在工作区里，它的依赖不编进安装包。
    let mut with_xtask = json.clone();
    with_xtask["workspace_members"] = json!(["me", "xtask"]);
    with_xtask["packages"]
        .as_array_mut()
        .expect("是数组")
        .push(json!({"id": "xtask", "name": "xtask", "version": "0.1.0", "manifest_path": "/w/xtask/Cargo.toml"}));
    with_xtask["resolve"]["nodes"]
        .as_array_mut()
        .expect("是数组")
        .push(node("xtask", json!([dep("devdep", json!(null))])));
    let got: Vec<String> = shipped(&with_xtask).into_iter().map(|p| p.name).collect();
    assert_eq!(got, ["a", "b", "lookalike"]);
}

#[test]
fn license_files_are_read_with_line_endings_made_plain() {
    let scratch = Scratch::new("files");
    scratch.put("p/LICENSE-MIT", "MIT text\r\nline two\r\n\r\n");
    scratch.put("p/LICENSE-APACHE", "Apache text\n");
    scratch.put("p/COPYING", "copying text\n");
    scratch.put("p/LICENSES/extra.txt", "inside a folder\n");
    scratch.put("p/README.md", "not a license\n");
    scratch.put("p/docs/the-license.txt", "named in license-file\n");
    let mut found = package(&scratch.0.join("p"), Some("MIT OR Apache-2.0"), &[]);
    found.license_file = Some("docs/the-license.txt".to_string());
    let got: BTreeSet<String> = texts(&found).expect("读得出").into_iter().collect();
    let want: BTreeSet<String> = [
        "MIT text\nline two",
        "Apache text",
        "copying text",
        "inside a folder",
        "named in license-file",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    assert_eq!(got, want);
}

#[test]
fn a_package_without_files_gets_the_standard_text_with_its_authors() {
    let scratch = Scratch::new("standard");
    scratch.put("p/Cargo.toml", "[package]\n");
    let found = package(
        &scratch.0.join("p"),
        Some("MIT OR Apache-2.0"),
        &["Ann <ann@x>", "Bo"],
    );
    let got = texts(&found).expect("有自带的原文");
    assert_eq!(got.len(), 2, "MIT、Apache-2.0 各一份");
    assert!(
        got[0].starts_with("MIT License\n\nCopyright (c) Ann <ann@x>, Bo\n"),
        "{}",
        got[0]
    );
    assert!(got[1].contains("Apache License"));
    // 没写作者的，版权行写「这个包的作者们」。
    let nobody = package(&scratch.0.join("p"), Some("BSD-3-Clause"), &[]);
    let got = texts(&nobody).expect("有自带的原文");
    assert!(
        got[0].starts_with("Copyright (c) the pudding authors\n"),
        "{}",
        got[0]
    );
    // MPL-2.0 照原文，没有版权行。
    let mpl = package(&scratch.0.join("p"), Some("MPL-2.0"), &[]);
    let got = texts(&mpl).expect("有自带的原文");
    assert!(
        got[0].starts_with("Mozilla Public License Version 2.0\n"),
        "{}",
        got[0]
    );
}

#[test]
fn what_cannot_be_attributed_is_reported() {
    let scratch = Scratch::new("bad");
    scratch.put("none/Cargo.toml", "[package]\n");
    // 没带文件，`license` 里也没有自带原文的那几种。
    let unknown = package(&scratch.0.join("none"), Some("ISC"), &[]);
    let said = texts(&unknown).expect_err("要人看过");
    assert_eq!(
        said,
        "pudding 1.0.0：没带许可证文件，ISC 也没有自带的原文，要人看过"
    );
    let unwritten = package(&scratch.0.join("none"), None, &[]);
    assert!(texts(&unwritten).expect_err("要人看过").contains("没写"));
    // 许可证文件是空的。
    scratch.put("empty/LICENSE", " \n\n");
    let empty = package(&scratch.0.join("empty"), Some("MIT"), &[]);
    assert!(texts(&empty).expect_err("空的").ends_with("是空的"));
    assert_eq!(
        read(&scratch.0.join("empty/LICENSE")),
        Err("是空的".to_string())
    );
}

#[test]
fn identical_texts_share_one_section_in_a_stable_order() {
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (text, user) in [
        ("zeta text", "a 1.0.0 (MIT)"),
        ("shared text", "c 1.0.0 (MIT)"),
        ("shared text", "b 2.0.0 (MIT)"),
    ] {
        groups
            .entry(text.to_string())
            .or_default()
            .insert(user.to_string());
    }
    let got = page(groups.clone());
    assert_eq!(got, page(groups), "同样的输入一字不差");
    // 一段一份原文，段照第一个用它的排：a 那段在前；b、c 共用一段，原文只出现一次。
    let a = got.find("a 1.0.0").expect("有 a");
    let b = got
        .find("b 2.0.0 (MIT)\nc 1.0.0 (MIT)\n\nshared text\n")
        .expect("b、c 一段");
    assert!(a < b);
    assert_eq!(got.matches("shared text").count(), 1);
    assert!(got.starts_with("Third-party software included in the Miyu tray app"));
}

#[test]
fn mpl_packages_say_where_their_source_is() {
    let dir = Path::new("/r/option-ext-0.2.0");
    let mut mpl = package(dir, Some("MPL-2.0"), &[]);
    mpl.name = "option-ext".to_string();
    mpl.version = "0.2.0".to_string();
    assert_eq!(
        mpl.label(),
        "option-ext 0.2.0 (MPL-2.0), source code: https://crates.io/crates/option-ext/0.2.0"
    );
    // 别的许可证照旧只写名字、版本、许可证。
    assert_eq!(
        package(dir, Some("MIT"), &[]).label(),
        "pudding 1.0.0 (MIT)"
    );
}
