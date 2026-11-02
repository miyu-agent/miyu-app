use super::*;

/// 每个测试一个自己的空目录。
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-desktop-log-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_small_log_stays() {
    let dir = scratch("small");
    let log = dir.join("desktop.log");
    std::fs::write(&log, "abc").unwrap();
    rotate(&log, 10).unwrap();
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "abc");
    assert!(!old(&log).exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_big_log_becomes_the_old_one_and_replaces_it() {
    let dir = scratch("big");
    let log = dir.join("desktop.log");
    std::fs::write(&log, "0123456789").unwrap();
    std::fs::write(old(&log), "older").unwrap();
    rotate(&log, 10).unwrap();
    assert!(!log.exists());
    assert_eq!(
        std::fs::read_to_string(dir.join("desktop.log.1")).unwrap(),
        "0123456789"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn no_log_yet_is_fine() {
    let dir = scratch("none");
    rotate(&dir.join("desktop.log"), 10).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_same_kind_is_logged_once_a_minute_with_the_count() {
    let mut quiet = Quiet::new();
    let start = Instant::now();
    assert_eq!(quiet.allow("dropped", start), Some(0));
    for n in 1..=103 {
        assert_eq!(
            quiet.allow("dropped", start + Duration::from_millis(n * 30)),
            None
        );
    }
    assert_eq!(quiet.allow("dropped", start + QUIET_FOR), Some(103));
    assert_eq!(quiet.allow("dropped", start + QUIET_FOR), None);
}

#[test]
fn kinds_are_counted_apart() {
    let mut quiet = Quiet::new();
    let now = Instant::now();
    assert_eq!(quiet.allow("dropped", now), Some(0));
    assert_eq!(quiet.allow("other", now), Some(0));
    assert_eq!(quiet.allow("dropped", now), None);
}
