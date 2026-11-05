use super::*;

use std::path::PathBuf;

/// 每个测试一个自己的空目录。
fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("miyu-applet-folder-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    dir
}

#[test]
fn a_new_folder_and_the_missing_parents_are_made() {
    let root = scratch("new");
    let dir = root.join("a").join("webview");
    private(&dir).unwrap();
    assert!(dir.is_dir());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn making_it_twice_is_fine() {
    let root = scratch("twice");
    private(&root).unwrap();
    private(&root).unwrap();
    assert!(root.is_dir());
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[cfg(unix)]
#[test]
fn a_new_folder_is_only_for_its_owner() {
    let root = scratch("mode");
    let dir = root.join("webview");
    private(&dir).unwrap();
    assert_eq!(mode(&root), 0o700);
    assert_eq!(mode(&dir), 0o700);
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
#[test]
fn an_old_open_folder_is_closed() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("old");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    private(&root).unwrap();
    assert_eq!(mode(&root), 0o700);
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
#[test]
fn the_own_folder_follows_each_platform() {
    let home = Some(PathBuf::from("/home/u"));
    assert_eq!(
        own("linux", home.clone(), None, None),
        Some(PathBuf::from("/home/u/.local/state/miyu-applet"))
    );
    assert_eq!(
        own("linux", home.clone(), None, Some("/x/state".into())),
        Some(PathBuf::from("/x/state/miyu-applet"))
    );
    assert_eq!(
        own("linux", home.clone(), None, Some("state".into())),
        Some(PathBuf::from("/home/u/.local/state/miyu-applet"))
    );
    assert_eq!(
        own("macos", home.clone(), None, None),
        Some(PathBuf::from(
            "/home/u/Library/Application Support/miyu-applet"
        ))
    );
    assert_eq!(own("linux", None, None, None), None);
    assert_eq!(own("linux", Some("rel".into()), None, None), None);
}

#[test]
fn the_own_folder_on_windows_needs_local_app_data() {
    let local = std::env::temp_dir();
    assert_eq!(
        own("windows", None, Some(local.clone().into_os_string()), None),
        Some(local.join("miyu-applet"))
    );
    assert_eq!(own("windows", Some(local), None, None), None);
}
