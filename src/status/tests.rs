use super::*;

#[test]
fn the_link_alone_says_running_or_stopped() {
    let mut status = Status::default();
    assert_eq!(status.state(), State::Stopped);
    status.link(true);
    assert_eq!(status.state(), State::Running);
    status.link(false);
    assert_eq!(status.state(), State::Stopped);
}

#[test]
fn stopping_shows_until_the_stop_is_done_even_after_the_link_drops() {
    let mut status = Status::default();
    status.link(true);
    status.begin(Busy::Stopping);
    assert_eq!(status.state(), State::Stopping);
    status.link(false);
    assert_eq!(status.state(), State::Stopping);
    status.done(Busy::Stopping, true);
    assert_eq!(status.state(), State::Stopped);
}

#[test]
fn a_failed_stop_goes_back_to_what_the_link_says() {
    let mut status = Status::default();
    status.link(true);
    status.begin(Busy::Stopping);
    status.done(Busy::Stopping, false);
    assert_eq!(status.state(), State::Running);
}

#[test]
fn starting_waits_for_the_link_after_the_start_is_done() {
    let mut status = Status::default();
    status.begin(Busy::Starting);
    status.done(Busy::Starting, true);
    // 拉起来了、连接还没报：照旧写「正在启动…」，不闪「已停止」。
    assert_eq!(status.state(), State::Starting);
    status.link(true);
    assert_eq!(status.state(), State::Running);
}

#[test]
fn starting_that_already_has_the_link_ends_at_once() {
    let mut status = Status::default();
    status.begin(Busy::Starting);
    status.link(true);
    assert_eq!(status.state(), State::Starting);
    status.done(Busy::Starting, true);
    assert_eq!(status.state(), State::Running);
}

#[test]
fn a_failed_start_shows_stopped() {
    let mut status = Status::default();
    status.begin(Busy::Starting);
    status.done(Busy::Starting, false);
    assert_eq!(status.state(), State::Stopped);
}

#[test]
fn a_stale_done_does_not_end_the_newer_work() {
    // 「重新启动」：停完了才开始拉起；停的那一句晚到了，不把「正在启动…」放下。
    let mut status = Status::default();
    status.begin(Busy::Starting);
    status.done(Busy::Stopping, true);
    assert_eq!(status.state(), State::Starting);
    assert!(status.is_busy());
}

#[test]
fn running_offers_stop_and_restart() {
    let look = look(&State::Running);
    assert_eq!(look.status, key::STATUS_RUNNING);
    assert_eq!(look.dot, Dot::Green);
    assert_eq!((look.toggle, look.toggle_enabled), (Toggle::Stop, true));
    assert!(look.restart_enabled);
}

#[test]
fn stopped_offers_start_and_greys_restart() {
    let look = look(&State::Stopped);
    assert_eq!(look.dot, Dot::Gray);
    assert_eq!((look.toggle, look.toggle_enabled), (Toggle::Start, true));
    assert!(!look.restart_enabled);
}

#[test]
fn busy_greys_both_lines_and_keeps_them_in_place() {
    let starting = look(&State::Starting);
    assert_eq!(
        (starting.toggle, starting.toggle_enabled),
        (Toggle::Stop, false)
    );
    assert!(!starting.restart_enabled);
    assert_eq!(starting.dot, Dot::Yellow);
    let stopping = look(&State::Stopping);
    assert_eq!(
        (stopping.toggle, stopping.toggle_enabled),
        (Toggle::Start, false)
    );
    assert!(!stopping.restart_enabled);
}

#[test]
fn the_reserved_states_already_have_their_look() {
    let hung = look(&State::Unresponsive);
    assert_eq!((hung.toggle, hung.toggle_enabled), (Toggle::Stop, true));
    assert!(hung.restart_enabled);
    let failed = look(&State::Failed("旧版的数据根".into()));
    assert_eq!(failed.dot, Dot::Red);
    assert_eq!(failed.reason.as_deref(), Some("旧版的数据根"));
    assert_eq!(
        (failed.toggle, failed.toggle_enabled),
        (Toggle::Start, true)
    );
    assert!(failed.restart_enabled);
}
