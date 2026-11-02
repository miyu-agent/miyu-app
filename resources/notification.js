// 页面里的 Notification 换成外壳的替身（施工 DT-8，蓝图 desktop.md「怎么走」第 8 条）：只在 Windows、macOS 上放进去。
// WebView2 默认拒绝网页通知的权限、WKWebView 没有这个接口，页面照浏览器的写法调 new Notification(…)，这里转给外壳弹系统的
// 通知；点了外壳把窗口拿回来，再叫这一条的 onclick。权限一律是 granted（外壳替页面要系统的权限）。
// 只经 Tauri 的进程间通道调一条命令 notify，别的什么都不碰；页面自己的代码不用改，也不用知道它在桌面窗口里。
(() => {
  const internals = window.__TAURI_INTERNALS__;
  if (!internals || typeof internals.invoke !== 'function') return;
  const live = new Map();
  let next = 0;
  class Notification extends EventTarget {
    static get permission() { return 'granted'; }
    static requestPermission(callback) {
      if (typeof callback === 'function') callback('granted');
      return Promise.resolve('granted');
    }
    constructor(title, options = {}) {
      super();
      this.title = String(title);
      this.body = options.body == null ? '' : String(options.body);
      this.tag = options.tag == null ? '' : String(options.tag);
      this.icon = options.icon == null ? '' : String(options.icon);
      this.silent = options.silent === true;
      this.data = options.data;
      this.onclick = null;
      this.onclose = null;
      this.onerror = null;
      this.onshow = null;
      this.id = ++next;
      live.set(this.id, this);
      internals.invoke('notify', { id: this.id, title: this.title, body: this.body, silent: this.silent }).then(
        () => this.fire('show'),
        () => { live.delete(this.id); this.fire('error'); },
      );
    }
    fire(type) {
      const event = new Event(type);
      const handler = this['on' + type];
      if (typeof handler === 'function') handler.call(this, event);
      this.dispatchEvent(event);
    }
    close() {
      if (live.delete(this.id)) this.fire('close');
    }
  }
  // 外壳在这一条被点了以后叫它（window.rs 里 eval）。
  Object.defineProperty(window, '__miyuNotificationClicked', {
    value: (id) => { const n = live.get(id); if (n) n.fire('click'); },
  });
  Object.defineProperty(window, 'Notification', { value: Notification, configurable: true, writable: true });
})();
