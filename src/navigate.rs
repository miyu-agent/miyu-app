//! 导航、新窗口交给谁（蓝图「怎么走」第 6 条）：只有网页软件的那个来源留在窗口里；窗口里别的导航不开，只有新窗口交系统的
//! 浏览器。

use url::Url;

/// 网页软件的来源：`http://127.0.0.1:<口子>`。开窗口要到网址以后才知道口子。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    port: u16,
}

impl Origin {
    /// 网址是网页软件的来源的（`http`、主机 `127.0.0.1`、写明了口子）交回它，别的交 `None`。
    pub fn of(url: &Url) -> Option<Self> {
        if url.scheme() != "http" || url.host_str() != Some("127.0.0.1") {
            return None;
        }
        url.port().map(|port| Self { port })
    }

    /// 网址和这个来源同协议、同主机、同口子。
    pub fn holds(self, url: &Url) -> bool {
        Self::of(url) == Some(self)
    }
}

/// 一个网址该去哪。
#[derive(Debug, PartialEq, Eq)]
pub enum Go {
    /// 留在窗口里开。
    Stay,
    /// 交给系统的浏览器（邮件程序）。
    Elsewhere,
    /// 哪儿都不开。
    Drop,
}

/// 窗口里要开 `url`：网页软件的来源、`about:`、`blob:`、`data:` 留下；别的不开，也不交出去（第五批审计 D5-1）：WebKitGTK、
/// WKWebView 把子框架自己的跳转也交到这里，框里的脚本（产物 HTML、软件后台页）不用人点就能一直跳，交出去就是一直叫起系统
/// 的浏览器、带着网址出去。往外的链接走 `new_window`。
pub fn navigation(origin: Option<Origin>, url: &Url) -> Go {
    if origin.is_some_and(|origin| origin.holds(url)) {
        return Go::Stay;
    }
    match url.scheme() {
        "about" | "blob" | "data" => Go::Stay,
        _ => Go::Drop,
    }
}

/// 页面要开新窗口（`target=_blank`、`window.open`）：`http`、`https`、`mailto` 交系统的浏览器（一个窗口一份状态，同来源的
/// 也交），别的不开。
pub fn new_window(url: &Url) -> Go {
    match url.scheme() {
        "http" | "https" | "mailto" => Go::Elsewhere,
        _ => Go::Drop,
    }
}

#[cfg(test)]
mod tests;
