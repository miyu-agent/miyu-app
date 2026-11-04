use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-app-instance-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    dir
}

#[test]
fn the_key_is_stable_and_differs_by_root() {
    let a = key(Path::new("/home/u/.miyu"));
    assert_eq!(a, key(Path::new("/home/u/.miyu")));
    assert_ne!(a, key(Path::new("/tmp/other")));
    assert_eq!(a.len(), 16);
}

#[test]
fn the_second_claim_on_the_same_root_is_second() {
    let dir = scratch("claim");
    let first = claim(&dir, "k").unwrap();
    assert!(matches!(first, Claim::First(_)));
    assert!(matches!(claim(&dir, "k").unwrap(), Claim::Second));
    // 别的数据根不碍事。
    assert!(matches!(claim(&dir, "other").unwrap(), Claim::First(_)));
    drop(first);
    // 第一个退了，再起来的又是第一个。
    assert!(matches!(claim(&dir, "k").unwrap(), Claim::First(_)));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn asking_writes_something_new_each_time() {
    let dir = scratch("ask");
    std::fs::create_dir_all(&dir).unwrap();
    ask(&dir, "k", None).unwrap();
    let first = std::fs::read_to_string(request(&dir, "k")).unwrap();
    std::thread::sleep(Duration::from_millis(2));
    ask(&dir, "k", None).unwrap();
    assert_ne!(first, std::fs::read_to_string(request(&dir, "k")).unwrap());
    assert_eq!(wanted_session(&first), None, "打开网页界面");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_click_on_a_notification_asks_for_its_session() {
    let dir = scratch("ask-session");
    std::fs::create_dir_all(&dir).unwrap();
    let id = "01a1a520-30b6-702a-ada5-603e4d8a21e8";
    ask(&dir, "k", Some(id)).unwrap();
    let line = std::fs::read_to_string(request(&dir, "k")).unwrap();
    assert_eq!(wanted_session(&line), Some(id.to_owned()));
    assert_eq!(
        wanted_session("session ../x 1"),
        None,
        "编号写法不对的照打开网页界面"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn only_writes_count_as_asking() {
    use notify::event::{AccessKind, CreateKind, DataChange, ModifyKind, RemoveKind};
    assert!(written(&EventKind::Create(CreateKind::File)));
    assert!(written(&EventKind::Modify(ModifyKind::Data(
        DataChange::Any
    ))));
    assert!(!written(&EventKind::Access(AccessKind::Any)));
    assert!(!written(&EventKind::Remove(RemoveKind::File)));
}

#[test]
fn a_burst_counts_once() {
    let mut once = Once::default();
    let start = Instant::now();
    assert!(once.allow(start));
    assert!(!once.allow(start + Duration::from_millis(10)));
    assert!(!once.allow(start + Duration::from_millis(900)));
    assert!(once.allow(start + Duration::from_millis(1100)));
}
