use super::*;

use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

#[test]
fn hello_shows_the_local_token_and_says_it_is_the_tray() {
    let hello = hello("t0k", "zh_CN.UTF-8");
    assert_eq!(hello["method"], "hello");
    assert_eq!(hello["id"], "hello");
    assert_eq!(hello["params"]["token"], "t0k");
    assert_eq!(hello["params"]["protocol"], json!([1, 1]));
    assert_eq!(hello["params"]["head"]["kind"], "miyu-app");
    assert_eq!(hello["params"]["locale"], "zh_CN.UTF-8");
}

#[test]
fn only_the_hello_reply_answers_the_hello() {
    assert_eq!(
        hello_answer(r#"{"jsonrpc":"2.0","id":"hello","result":{"language":"zh"}}"#),
        Some(Ok(()))
    );
    assert_eq!(
        hello_answer(r#"{"jsonrpc":"2.0","id":"hello","error":{"message":"bad token"}}"#),
        Some(Err("bad token".into()))
    );
    assert_eq!(
        hello_answer(r#"{"jsonrpc":"2.0","id":"hello","error":{}}"#),
        Some(Err("refused".into()))
    );
    assert_eq!(
        hello_answer(r#"{"jsonrpc":"2.0","method":"config.changed","params":{}}"#),
        None
    );
    assert_eq!(hello_answer("not json"), None);
}

#[test]
fn the_watch_falls_back_from_run_to_the_root_to_its_parent() {
    let root = Path::new("/home/u/.miyu");
    let run = root.join("run");
    let all = |_: &Path| true;
    assert_eq!(target(&run, root, all), Some(run.clone()));
    let no_run = |p: &Path| p != run;
    assert_eq!(target(&run, root, no_run), Some(root.to_path_buf()));
    let only_parent = |p: &Path| p == Path::new("/home/u");
    assert_eq!(
        target(&run, root, only_parent),
        Some(PathBuf::from("/home/u"))
    );
    assert_eq!(target(&run, root, |_| false), None);
}

#[test]
fn changes_inside_the_data_root_count() {
    let root = Path::new("/home/u/.miyu");
    let socket = vec![root.join("run").join("socket")];
    for kind in [
        EventKind::Create(CreateKind::File),
        EventKind::Modify(ModifyKind::Any),
        EventKind::Remove(RemoveKind::File),
        EventKind::Any,
    ] {
        assert!(relevant(&kind, &socket, root), "{kind:?}");
    }
    // 数据根本身刚建出来（盯的是上一级）。
    assert!(relevant(
        &EventKind::Create(CreateKind::Folder),
        &[root.to_path_buf()],
        root
    ));
}

#[test]
fn reads_and_things_outside_do_not_count() {
    let root = Path::new("/home/u/.miyu");
    let token = vec![root.join("run").join("token")];
    assert!(!relevant(&EventKind::Access(AccessKind::Any), &token, root));
    assert!(!relevant(
        &EventKind::Create(CreateKind::File),
        &[PathBuf::from("/home/u/notes.txt")],
        root
    ));
    assert!(!relevant(
        &EventKind::Create(CreateKind::File),
        &[PathBuf::from("/home/u/.miyu-old")],
        root
    ));
}

/// 两个真核心的测试用同一个数据根，一个一个跑。
static REAL_CORE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 拿真的核心走一遍：拉起、托盘那条连接连上、`miyu core stop`、连接断。要人手跑，而且只认指到临时目录的 `MIYU_HOME`
/// （拉起的核心照环境找数据根，不设就是主人的 `~/.miyu`）：
/// `MIYU_HOME=$(mktemp -d) MIYU_TEST_PROGRAM=<miyu> cargo test -- --ignored a_real_core`。
#[test]
#[ignore = "要真的核心：MIYU_TEST_PROGRAM 指到 miyu，MIYU_HOME 指到临时的空目录"]
fn a_real_core_goes_up_and_down() {
    let _one = REAL_CORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let miyu = PathBuf::from(std::env::var_os("MIYU_TEST_PROGRAM").expect("MIYU_TEST_PROGRAM"));
    let home = PathBuf::from(std::env::var_os("MIYU_HOME").expect("MIYU_HOME"));
    assert!(
        home.starts_with(std::env::temp_dir()),
        "MIYU_HOME 要在临时目录里"
    );
    let root = DataRoot::locate(&miyu_client::places::Env::current()).unwrap();
    assert_eq!(root.path(), home);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(runtime.block_on(open(&root)).is_err(), "还没拉起就连上了");
    crate::actions::start_core(&miyu, &root).unwrap();
    let link = runtime.block_on(open(&root)).unwrap();
    let text = crate::text::Text::of(crate::text::Lang::En);
    crate::quit::stop_core(&miyu, &text).unwrap();
    let why = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(10), link.closed())
            .await
            .expect("停了以后连接要断")
    });
    assert!(!why.is_empty());
    assert!(runtime.block_on(open(&root)).is_err(), "停了还连得上");
}

/// 拿真的核心看「别人拉起来了」：托盘那条连接没人叫醒，只靠盯 `run/` 的文件变动跟上；停了报断。跑法同上：
/// `MIYU_HOME=$(mktemp -d) MIYU_TEST_PROGRAM=<miyu> cargo test -- --ignored a_real_core`。
#[test]
#[ignore = "要真的核心：MIYU_TEST_PROGRAM 指到 miyu，MIYU_HOME 指到临时的空目录"]
fn a_real_core_started_by_someone_else_is_seen() {
    let _one = REAL_CORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let miyu = PathBuf::from(std::env::var_os("MIYU_TEST_PROGRAM").expect("MIYU_TEST_PROGRAM"));
    let home = PathBuf::from(std::env::var_os("MIYU_HOME").expect("MIYU_HOME"));
    assert!(
        home.starts_with(std::env::temp_dir()),
        "MIYU_HOME 要在临时目录里"
    );
    let root = DataRoot::locate(&miyu_client::places::Env::current()).unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let _waker = spawn(root.clone(), move |up| send.send(up).unwrap());
    let next = || receive.recv_timeout(Duration::from_secs(20)).unwrap();
    assert!(!next(), "起来先报连不上");
    crate::actions::start_core(&miyu, &root).unwrap();
    while !next() {}
    let text = crate::text::Text::of(crate::text::Lang::En);
    crate::quit::stop_core(&miyu, &text).unwrap();
    assert!(!next(), "停了报断");
}
