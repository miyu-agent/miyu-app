## AP-4：系统通知

状态：已完成（2026-11-04，Linux 单测、真核心的手动测试：私有会话总线上的假通知服务收、点；三平台 CI 见「结果」）。Windows、macOS 上弹不弹得出、点了回不回得来没人手测过，见「要人手测的」。

### 目的

浏览器关了也收得到「回答完了」「等你回答」「等你确认」「定时任务跑完了」（图纸 `docs/blueprint/app.md` 第 5 条）：托盘认领后台通知（核心 TR-1），订会话表和账号级的提醒，没有网页开着的会话由托盘弹系统通知，点了用浏览器打开那个会话。

### 图纸改哪几节

`app.md` 照实际做法改了这几处（都标了「AP-4」）：

- 第 5 条：和网页的分工（认领、断了作废、重连再认领、老核心不发）、订什么、什么时候发、写什么、点了做什么、三个平台各做得到什么。
- 第 4 条起停里「Miyu 意外停止了」那一条：写明还没做、为什么。
- 「要核心给的」TR-1 那一条：AP-4 做了。

### 做了什么

1. **依赖**：`notify-rust` 4.18.2（钉死版本，同托盘、事件循环那几个：点了回不回得来跟着它变）。Linux 走会话 D-Bus（zbus，ksni 本来就带进来了），macOS 走 `mac-notification-sys`，Windows 走 `tauri-winrt-notification`（`windows` crate 本来就有）。新进来的只有 8 个，`cargo metadata` 一个个看的许可证，都能和 AGPL-3.0-or-later 合在一起发：

   | crate | 版本 | 许可证 | 哪个平台 |
   |---|---|---|---|
   | `notify-rust` | 4.18.2 | MIT OR Apache-2.0 | 都有 |
   | `mac-notification-sys` | 0.6.15 | MIT/Apache-2.0 | macOS |
   | `time` | 0.3.55 | MIT OR Apache-2.0 | macOS（`mac-notification-sys` 用） |
   | `time-core` | 0.1.9 | MIT OR Apache-2.0 | 同上 |
   | `deranged` | 0.5.9 | MIT OR Apache-2.0 | 同上 |
   | `num-conv` | 0.2.2 | MIT OR Apache-2.0 | 同上 |
   | `powerfmt` | 0.2.1 | MIT OR Apache-2.0 | 同上 |
   | `tauri-winrt-notification` | 0.8.1 | MIT OR Apache-2.0 | Windows |

   锁文件里 `tempfile` 依赖的 `getrandom` 跟着 `uuid` 统一成 0.4.3（两个版本本来都在）。

2. **认领、订阅**（`src/link.rs`、`src/link/notices.rs`）：握手以后 `notices.claim`；认领成了再订 `sessions`、`notices` 两条流，推来的交给 `alerts` 算，要弹的报 `Report::Note`。被拒的（老核心不认）记一行，这条连接不发通知，别的照用。断了以后照旧重连，每连一次从头来：重新认领、重新订，手上的会话表整份换新。掉了队（`resync`）的重新订。
   - 读推送时有在等标题的通知的，用 `tokio::time::timeout` 等到它的点，到了先发掉再接着读（`rpc` 读一行本来就经得起丢下：读了一半的留在缓冲里）。
   - `waiting` 冒出来时问一句 `view.page {session, turns: 1}`，拿那一轮的事件分在问还是要确认。
   - `notice` 的 `schedule.ran` 那个会话会话表里没有标题、第一句的，问 `schedule.list` 拿定时任务那句话当标题。

3. **发什么**（新的 `src/alerts.rs`，纯的，单测）：照网页 `notify` 包（`model.js`、`index.js`）的规矩：
   - `busy` 去掉是「回答完了」，`waiting` 冒出来是等人；订阅时的整份、第一次见到的会话照「以前没在跑、没在等」算。
   - `viewed` 为真的、子会话（`parent`）、一次性的（`oneshot`）、整理记忆的（`kind: dream`）不发。
   - 在问还是要确认：最后一个还没了结的 `question.asked` / `tool.approval_requested`，看不出的算要确认。
   - 标题：有标题照标题；没有的等最多 5 秒（同网页），会话表推来了标题马上发，到点照第一句，再没有「Miyu」；同一个 `tag` 已经在等的不再等一条。会话删了，等它的扔掉。
   - `tag`：`<会话>:done|ask|approve`；`schedule.ran` 同 `<会话>:done`；`schedules.reset`。
   - 字：`resources/text/` 三份加 `notify.done`、`notify.ask`、`notify.approve`、`notify.schedule_ran`、`notify.schedule_failed`、`notify.schedules_reset`（同网页 `notify` 包清单里的字）和按钮的 `notify.open`（打开、Open、開く）。

4. **弹、点**（新的 `src/toast.rs`）：一条一个后台线程，发出去以后 `wait_for_response` 等人点或者关；点了（默认动作、「打开」按钮）经事件循环交给界面线程，`miyu web --session <编号>`（`actions::Page::Session`），没有会话的 `miyu web`。和平台有关的都关在这个文件的 `platform` 里：
   - Linux：图标是应用图标（`icons/128x128.png`），写进托盘自己的目录的 `notification.png`（通知服务照路径读）；`desktop-entry` 提示 `miyu-app`；记着每个 `tag` 上一条的编号，下一条带着它发（`replaces_id`），关掉了（`NotificationClosed`）的忘掉。换掉以后等旧那一次的线程还在等同一个编号，点了两个线程都收得到，只认最新的那一次（`Shown`），不开两次。
   - Windows：`app_id` 是 `io.github.miyu-agent.miyu-app`（AUMID）。Tauri 打包工具 2.10.4（release 用的 tauri-cli 2.12.4 钉的就是它）的 `main.wxs` 给开始菜单快捷方式写了 `System.AppUserModel.ID = {{bundle_id}}`，就是 `tauri.conf.json` 的 `identifier`，不用另写。
   - macOS：起来时 `set_application("io.github.miyu-agent.miyu-app")`，认不出（没装成 `.app`）的记一行，照库的默认（访达）。

### 风险

- **Windows 点了回不回得来**：toast 的 `Activated` 只在托盘还在跑时回到托盘；通知进了通知中心以后再点，没有注册 COM 激活器的话系统多半只是「打开应用」（不带会话），也可能什么都不做。库（`tauri-winrt-notification`）发完就丢下 toast 对象，`Activated` 还到不到得了也要实测。到不了的话下一步要么注册激活器（要 `unsafe` 的 COM，仓库 `forbid(unsafe_code)`，得换成别的做法），要么认了「点通知只是把托盘叫起来、打开网页界面」。
- **Windows 不装就弹不出**：AUMID 要有带它的开始菜单快捷方式；直接跑 `cargo build` 出来的程序弹不出来，正常。
- **macOS 的身份**：`set_application` 要系统认得 `Miyu.app`（LaunchServices 里有它）；dmg 拖进「应用程序」以后才算。没认出来的照访达的图标和名字发，点了也回得来。用的是标成过时的 NSUserNotificationCenter；第一次弹系统会问要不要允许通知。
- **只有 Linux 换得掉上一条**：Windows、macOS 上同一个会话的「回答完了」会一条条摞在通知中心里。
- **等点的线程**：Linux 上通知服务不发 `NotificationClosed`（有的桌面把通知收进历史、一直不关）时，等它的线程一直在；同一个 `tag` 换掉的旧线程下一次有回音就走。一个会话最多三条在等，平常够用；要真攒多了再换成一条连接、一个线程收全部的信号。
- **GNOME 这类只把信号发给发通知的那条连接的通知服务**：每条通知各开一条 D-Bus 连接（库的做法），换掉一条以后点了的信号可能只到新那条连接上，等它的是新线程，照样打开；旧线程等到关掉才走。
- **在问还是要确认**：脚本里 `miyu call` 调的工具在等确认的，那一轮事件里不一定有 `tool.approval_requested`，照「看不出算要确认」写，对。一轮里先确认完马上又问（`waiting` 一直是真、没落下来）的，只弹第一次。
- **定时任务和「回答完了」同一个 `tag`**：`schedule.ran` 常常先到、「回答完了」等标题晚到，Linux 上留下的是后到的「回答完了」（网页同样）。

### 不做什么

- 「Miyu 意外停止了」（第 4 条起停）：分不清崩了和别的头叫停的，见图纸。
- 会话人格的头像当图标（网页用 `chat.lookOf`）：托盘不读人格的头像，都用应用图标。
- 前台、后台出不出声：托盘发的都是没人开着的会话，照系统默认。
- 托盘菜单里的「通知开关」：系统的通知设置里能关。

### 怎么验

1. 每个提交前 `cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test --locked`（做完是 97 个）过；每个 `.rs` 不超过 500 行（最长的 `link.rs` 380 行）。
2. 单测钉住的：`busy` 去掉是回答完了、`waiting` 冒出来是等人、一样的再推不再弹、订阅时的整份和第一次见到的不算；`viewed`、子会话、一次性、整理记忆的不弹，网页关了以后照弹；标题和三种 `tag`；没标题的等、标题来了马上发、到点照第一句再没有写 Miyu、同一种不等两条、会话删了扔掉；定时任务和「回答完了」同 `tag`、`ok: false` 写没跑完、没标题照定时那句话、网页开着的不弹；记录坏了是账号级的；事件里分在问和要确认（`alerts`）。假核心走一遍订阅、`view.page`、`schedule.list`（`link::notices`）。换掉的只认最新那一次的点、关掉的忘掉（`toast`）。`--session` 的参数（`actions`）；三份字一样的键（`text`）。
3. 本机真核心的手动测试（不碰真桌面）：`dbus-run-session` 起私有的会话总线，里面起一个假的通知服务（Python `dbus-python`：`Notify`、`GetCapabilities`、`GetServerInformation`、`CloseNotification`，收到的记进文件；另有 `test.Control.Invoke`、`Close` 按要求发 `ActionInvoked`、`NotificationClosed`），托盘也起在这条总线里（没有托盘宿主，图标不出来）；消息框画在自己起的 Xvfb 上；`XDG_STATE_HOME`、`XDG_CONFIG_HOME` 指到临时目录（通知的图标写在那里）；`MIYU_PROGRAM` 指到替身，`web …` 只记参数，别的转给真核心（`~/.cache/miyu-web-demo/core-target-debug/debug/miyu`、`core-e9c2b52cd/resources`，协议同 `d58f7b099`，只加不减）；数据根是新的空目录，配一个假的 OpenAI 兼容模型（回一句「好的，做完了。」）。用一个自己写的头（握手 `caps.input` 为真）开会话、说话、`session.viewing`、答确认。测完停了核心、假模型、假通知服务、Xvfb，按进程环境查过没有残留，删了数据根。
   - 认领：托盘连上以后别的头订 `notices` 回 `{"claimed": true}`；托盘退了以后回 `{}`。
   - 没人开着的会话说一句：收到一条 `summary` 是会话标题（核心起的）、`body`「回答完了」、`actions` `["default", "打开"]`、`desktop-entry` `miyu-app`、图标是临时目录里的 `notification.png`。
   - 点了（发 `ActionInvoked 1 default`）：替身记到 `web --session <那个会话>`。
   - 同一个会话再说一句：`replaces_id` 是上一条的编号；再换一次以后点一下，只开一次。
   - 不让起标题的会话（`titles: false`）：2 秒时还没发，5 秒以后照第一句发。
   - 头 `session.viewing` 开着一个会话（会话表 `viewed: true`）时在那里说话：不发；头断了以后再说：照发。
   - 要确认（写工作区外面的文件）：收到「等你确认」（`tag` 是 `<会话>:approve`，另起一条），拒掉以后「回答完了」换掉那个会话上一条「回答完了」。
   - 一次性的会话（`miyu ask`）：不发。
   - 定时任务（65 秒后、每次新开会话）：先收到「汇总一下今天的事 / 定时任务跑完了」，接着那个会话起好标题的「回答完了」换掉它（同一个 `tag`）。
   - 断了重连：`miyu core stop` 以后再拉起核心，托盘连上、别的头看到 `claimed: true`，说一句照样发。
   - 关掉（`NotificationClosed`）以后同一个会话再发的是新的一条（不再带旧编号）。
   - 没实测：「等你回答」（假模型不会调 `ask_user`，靠单测）、`schedules.reset`（要弄坏唤醒表）。

### 要人手测的（出包以后）

- Windows 虚拟机（装这一步的 MSI，浏览器关着，或者开着网页但不在那个会话上）：
  - 在终端（`miyu`）里开一个会话说一句，切走、关掉终端以前它答完：右下角弹「<标题> / 回答完了」，来源写「Miyu」、图标是 Miyu 的；点通知本身、点「打开」按钮，浏览器打开那个会话。
  - 等它进了通知中心（不点，等横幅收起）再点：回不回得到那个会话，还是只打开了网页界面、什么都没做（记下来，决定要不要做激活器）。
  - 网页开着那个会话时：只有网页弹，托盘不弹。
  - 退出托盘以后：网页照原来弹（没人认领了）。
  - 要确认、在问：「等你确认」「等你回答」。
- macOS（`Miyu.app` 拖进「应用程序」以后）：第一次弹时系统问允不允许通知；来源、图标是 Miyu 不是访达；点通知、点「打开」打开那个会话；横幅收起以后在通知中心里点回不回得来；没装进「应用程序」直接跑时照访达的身份发（已知）。
- Linux 别的桌面：GNOME（点了、换掉上一条、通知收进历史以后点）、KDE Plasma（「打开」按钮显示成什么）、mako/dunst（默认动作）。

### 结果

- 提交：`200becf` 依赖、`2fcc10c` 系统通知，以及补这一节的那一个。
- CI（`ci.yml`）：运行 41808854481（`2fcc10c`）Linux、macOS、Windows 三台都绿。
- 打包（`release.yml`）：运行 41808854518（`2fcc10c`）三台都绿。下了 Windows 的中文 MSI 查字符串表：`System.AppUserModel.ID` 和 `io.github.miyu-agent.miyu-app` 都在（开始菜单快捷方式带着应用身份）；装上以后快捷方式上真有没有，手测时用 PowerShell 看一眼（`(New-Object -ComObject Shell.Application)` 读不出这一格，用 `Get-StartApps | ? Name -eq Miyu` 看 `AppID` 是不是它）。

### Windows 虚拟机上手测（2026-11-04，Win11 26200，装打包运行 41813382751 的中文 MSI：AP-4 + 核心 61f1f3ef2 + 网页 0.1.0）

数据根另指空目录，模型照宿主上的假模型（只听宿主的局域网地址，回答慢 20 秒）。网页里说一句、马上关掉浏览器，等通知。测完删了数据根、计划任务、临时自动登录，虚拟显示器启用回去，关机；持久配置和测之前逐字一样。

- `Get-StartApps`：名字「Miyu」，AppID `io.github.miyu-agent.miyu-app`。
- 后台会话答完：右下角弹横幅，来源「Miyu」、Miyu 的图标，标题是那句话（「hi」「again」），正文「回答完了」，下面一个「打开」按钮。
- **点「打开」按钮**：打开那个会话（日志 `notification <会话>:done clicked`，接着 `miyu web --session <会话>`，浏览器里是那一轮）。
- **点横幅本身**：横幅收掉，托盘收不到（日志里没有 clicked）。**收进通知中心以后再点**：同样收不到。库（tauri-winrt-notification 0.8.1）的 toast 没有 `launch`，正文点了 Windows 不交回 `Activated`，只有带 `arguments` 的按钮交回；要点正文也打开，得自己拼带 `launch` 的 toast（WinRT 的接口在 windows 那个 crate 里是安全的 Rust，不用 `unsafe`），排成后续。
- 同一个会话的几条在通知中心里摞在一起（「+2 个通知」），不换掉上一条，同上面「风险」里写的。
- 没测：「等你确认」「等你回答」、网页开着那个会话时只有网页弹、退出托盘以后网页照原来弹。
