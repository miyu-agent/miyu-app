use super::*;

fn url(s: &str) -> Url {
    s.parse().unwrap()
}

fn origin() -> Option<Origin> {
    Origin::of(&url("http://127.0.0.1:8300/#ticket=abc"))
}

#[test]
fn the_web_apps_own_pages_stay_in_the_window() {
    assert_eq!(
        navigation(origin(), &url("http://127.0.0.1:8300/")),
        Go::Stay
    );
    assert_eq!(
        navigation(origin(), &url("http://127.0.0.1:8300/p/t/miyu-x/")),
        Go::Stay
    );
}

#[test]
fn another_port_or_host_is_not_the_same_origin() {
    assert_eq!(
        navigation(origin(), &url("http://127.0.0.1:8301/")),
        Go::Drop
    );
    assert_eq!(
        navigation(origin(), &url("http://localhost:8300/")),
        Go::Drop
    );
    assert_eq!(
        navigation(origin(), &url("https://127.0.0.1:8300/")),
        Go::Drop
    );
}

#[test]
fn before_the_address_is_known_only_blank_pages_stay() {
    assert_eq!(navigation(None, &url("about:blank")), Go::Stay);
    assert_eq!(navigation(None, &url("http://127.0.0.1:8300/")), Go::Drop);
}

#[test]
fn pages_the_web_view_makes_itself_stay() {
    assert_eq!(
        navigation(origin(), &url("blob:http://127.0.0.1:8300/x")),
        Go::Stay
    );
    assert_eq!(navigation(origin(), &url("data:text/plain,hi")), Go::Stay);
}

#[test]
fn links_out_in_the_window_are_not_opened() {
    assert_eq!(
        navigation(origin(), &url("https://example.com/a")),
        Go::Drop
    );
    assert_eq!(navigation(origin(), &url("mailto:a@example.com")), Go::Drop);
}

#[test]
fn other_schemes_are_not_opened() {
    assert_eq!(navigation(origin(), &url("file:///etc/passwd")), Go::Drop);
    assert_eq!(navigation(origin(), &url("javascript:alert(1)")), Go::Drop);
    assert_eq!(navigation(origin(), &url("miyu://x")), Go::Drop);
}

#[test]
fn new_windows_always_leave_even_for_the_same_origin() {
    assert_eq!(new_window(&url("http://127.0.0.1:8300/")), Go::Elsewhere);
    assert_eq!(new_window(&url("https://example.com/")), Go::Elsewhere);
    assert_eq!(new_window(&url("file:///tmp/x")), Go::Drop);
}

#[test]
fn only_a_loopback_http_address_with_a_port_is_an_origin() {
    assert!(Origin::of(&url("http://127.0.0.1/")).is_none());
    assert!(Origin::of(&url("http://192.168.1.2:8300/")).is_none());
    assert!(Origin::of(&url("https://127.0.0.1:8300/")).is_none());
}

/// 审计第五轮 D5-1（红）：WebKitGTK、WKWebView 上 `on_navigation` 也收到子框架自己的跳转（产物 HTML 的沙盒框、软件后台页的框），
/// 不要人点，被拒以后框里的脚本接着跑（WebKitGTK 2.54 实测：一个沙盒 srcdoc 框 3 秒里来了 104 次）。现在每次都交给系统浏览器，
/// 框里的脚本能不停地叫起浏览器、带着网址出去。网页里往外的链接都写了 `target=_blank`（走 `new_window`，要人点），
/// 窗口里看到的导航不该再交出去。
#[test]
fn audit5_a_navigation_seen_by_the_window_is_not_handed_to_the_browser() {
    for link in [
        "https://example.com/leak?n=1",
        "http://127.0.0.1:8301/",
        "mailto:a@example.com",
    ] {
        assert_ne!(navigation(origin(), &url(link)), Go::Elsewhere, "{link}");
    }
}
