use super::*;

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

fn ran(code: i32, stdout: &str, stderr: &str) -> Ran {
    Ran {
        code: Some(code),
        stdout: stdout.into(),
        stderr: stderr.into(),
    }
}

#[test]
fn the_environment_variable_wins() {
    let got = locate(
        Some("/dev/miyu".into()),
        Some(Path::new("/opt/m")),
        None,
        |_| true,
    );
    assert_eq!(got, Some(PathBuf::from("/dev/miyu")));
}

#[test]
fn an_empty_environment_variable_is_not_set() {
    let got = locate(Some("".into()), Some(Path::new("/opt/m")), None, |_| true);
    assert_eq!(got, Some(Path::new("/opt/m").join(exe("miyu"))));
}

#[test]
fn beside_the_shell_comes_before_the_path() {
    let path = std::env::join_paths(["/usr/bin"]).unwrap();
    let got = locate(None, Some(Path::new("/opt/m")), Some(path), |_| true);
    assert_eq!(got, Some(Path::new("/opt/m").join(exe("miyu"))));
}

#[test]
fn the_path_is_searched_in_order() {
    // 绝对目录：Windows 上 `/b` 这种没有盘符的不算绝对，照临时目录拼。
    let root = std::env::temp_dir();
    let path = std::env::join_paths([root.join("a"), root.join("b"), root.join("c")]).unwrap();
    let wanted = root.join("b").join(exe("miyu"));
    let got = locate(None, Some(Path::new("/opt/m")), Some(path), |p| p == wanted);
    assert_eq!(got, Some(wanted));
}

#[test]
fn nowhere_is_none() {
    let path = std::env::join_paths(["/a"]).unwrap();
    assert_eq!(
        locate(None, Some(Path::new("/opt/m")), Some(path), |_| false),
        None
    );
    assert_eq!(locate(None, None, None, |_| true), None);
}

#[test]
fn a_good_reply_gives_the_address_and_its_origin() {
    let line = r#"{"url":"http://127.0.0.1:8300/#ticket=abc","first":true}"#;
    let (url, origin) = read_open(&ran(0, &format!("{line}\n"), "")).unwrap();
    assert_eq!(url.as_str(), "http://127.0.0.1:8300/#ticket=abc");
    assert!(origin.holds(&"http://127.0.0.1:8300/x".parse().unwrap()));
}

#[test]
fn more_fields_are_fine() {
    let line = r#"{"url":"http://127.0.0.1:8300/","first":false,"later":1}"#;
    assert!(read_open(&ran(0, line, "")).is_ok());
}

#[test]
fn a_reply_without_a_loopback_address_is_bad() {
    for line in [
        r#"{"first":true}"#,
        r#"{"url":"https://example.com/#ticket=abc"}"#,
        r#"{"url":"http://127.0.0.1/"}"#,
        r#"{"url":"not a url"}"#,
        "http://127.0.0.1:8300/",
        "",
    ] {
        assert_eq!(
            read_open(&ran(0, line, "")),
            Err(OpenError::BadReply),
            "{line}"
        );
    }
}

#[test]
fn a_failure_brings_back_the_last_lines_of_stderr() {
    let stderr = "a\n\nb\nc\nd\ne\nf\ng\n";
    assert_eq!(
        read_open(&ran(1, "", stderr)),
        Err(OpenError::Failed("b\nc\nd\ne\nf\ng".into()))
    );
}

#[test]
fn killed_by_a_signal_is_a_failure() {
    let killed = Ran {
        code: None,
        stdout: br#"{"url":"http://127.0.0.1:8300/"}"#.to_vec(),
        stderr: Vec::new(),
    };
    assert_eq!(read_open(&killed), Err(OpenError::Failed(String::new())));
}

#[cfg(unix)]
#[test]
fn run_collects_output_and_the_exit_code() {
    let got = run(
        Path::new("/bin/sh"),
        &["-c", "echo out; echo err >&2; exit 3"],
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(got.code, Some(3));
    assert_eq!(got.stdout, b"out\n");
    assert_eq!(got.stderr, b"err\n");
}

#[cfg(unix)]
#[test]
fn run_kills_what_runs_too_long() {
    let started = Instant::now();
    let got = run(
        Path::new("/bin/sh"),
        &["-c", "sleep 30"],
        Duration::from_millis(200),
    );
    assert!(matches!(got, Err(RunError::Timeout)));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn run_says_when_it_cannot_start() {
    let got = run(
        Path::new("/nowhere/miyu-not-here"),
        &[],
        Duration::from_secs(1),
    );
    assert!(matches!(got, Err(RunError::Spawn(_))));
}

#[cfg(unix)]
#[test]
fn run_does_not_wait_for_a_child_that_keeps_the_pipe() {
    let started = Instant::now();
    let got = run(
        Path::new("/bin/sh"),
        &[
            "-c",
            "echo '{\"url\":\"http://127.0.0.1:8300/\"}'; sleep 30 & exit 0",
        ],
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(got.code, Some(0));
    assert!(
        read_open(&got).is_ok(),
        "{:?}",
        String::from_utf8_lossy(&got.stdout)
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

/// 审计第五轮 D5-4（红）：`PATH` 里的空项、`.`、相对目录照外壳的当前目录解析（图形程序的当前目录说不准：从文件管理器、
/// 别的程序拉起来时是那边的目录），那里放一个 `miyu` 就被当成主程序调。只该照绝对目录找。
#[test]
fn audit5_relative_path_entries_are_not_searched() {
    let absolute = std::env::temp_dir();
    let path = std::env::join_paths([
        Path::new(""),
        Path::new("."),
        Path::new("bin"),
        absolute.as_path(),
    ])
    .unwrap();
    let got = locate(None, None, Some(path), |_| true);
    assert_eq!(got, Some(absolute.join(exe("miyu"))));
}
