//! 点通知用的网址（AP-4 补）：拼、读、照命令行找；外面交进来的写法不对的不认。

use super::*;

const ID: &str = "01a1a520-30b6-702a-ada5-603e4d8a21e8";

#[test]
fn links_round_trip() {
    assert_eq!(link(Some(ID)), format!("miyu-app://open/?session={ID}"));
    assert_eq!(parse(&link(Some(ID))), Some(Some(ID.to_owned())));
    assert_eq!(link(None), "miyu-app://open/");
    assert_eq!(parse(&link(None)), Some(None));
    assert_eq!(parse("miyu-app://open"), Some(None), "末尾没有斜杠的也认");
}

#[test]
fn strangers_are_refused() {
    assert_eq!(parse("http://open/?session=x"), None);
    assert_eq!(parse("miyu-app://delete/"), None);
    assert_eq!(
        parse("miyu-app://open/?session=../../x"),
        None,
        "编号写法不对"
    );
    assert_eq!(
        parse("miyu-app://open/?session=01a1a520-30b6-702a-ada5-603e4d8a21e8&x=1"),
        None
    );
}

#[test]
fn the_data_root_comes_from_the_command_line() {
    let args = [
        HOME_ARG.to_owned(),
        r"C:\mt\tray-home".to_owned(),
        ARG.to_owned(),
        link(None),
    ];
    assert_eq!(
        home_from_args(args.clone()),
        Some(std::path::PathBuf::from(r"C:\mt\tray-home"))
    );
    assert_eq!(from_args(args), Some(None));
    assert_eq!(
        home_from_args([HOME_ARG.to_owned(), String::new()]),
        None,
        "空的不算"
    );
}

#[test]
fn the_link_comes_from_the_command_line() {
    let args = ["--autostart".to_owned(), ARG.to_owned(), link(Some(ID))];
    assert_eq!(from_args(args), Some(Some(ID.to_owned())));
    assert_eq!(from_args([ARG.to_owned()]), None, "后面没有网址");
    assert_eq!(from_args(["x".to_owned()]), None);
}
