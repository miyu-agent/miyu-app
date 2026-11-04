use super::*;

#[test]
fn the_next_note_with_the_same_tag_replaces_the_open_one() {
    let mut shown = Shown::default();
    assert_eq!(shown.replaces("a:done"), None);
    shown.sent("a:done", 7);
    assert_eq!(shown.replaces("a:done"), Some(7));
    assert_eq!(shown.replaces("a:ask"), None);
}

#[test]
fn only_the_latest_send_of_a_replaced_note_counts_a_click() {
    let mut shown = Shown::default();
    let first = shown.sent("a:done", 7);
    // 换掉：通知服务照旧给编号 7。
    let second = shown.sent("a:done", 7);
    // 点了：两个线程都收到，只有后一次算。
    assert!(!shown.answered(7, first, false));
    assert!(shown.answered(7, second, false));
}

#[test]
fn a_closed_note_is_forgotten_and_the_next_one_starts_fresh() {
    let mut shown = Shown::default();
    let first = shown.sent("a:done", 7);
    let second = shown.sent("a:done", 7);
    // 旧那一次收到关掉了：不算数，也不忘（最新那一次还开着）。
    assert!(!shown.answered(7, first, true));
    assert_eq!(shown.replaces("a:done"), Some(7));
    assert!(shown.answered(7, second, true));
    assert_eq!(shown.replaces("a:done"), None);
}
