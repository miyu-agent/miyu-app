//! toast 的 XML（AP-4 补）：正文和按钮都带 `open`、前台激活；字照 XML 转义。

use super::*;

#[test]
fn the_body_and_the_button_both_activate_with_open() {
    let xml = template("修 bug", "回答完了", "打开");
    assert!(
        xml.starts_with("<toast launch=\"open\" activationType=\"foreground\">"),
        "{xml}"
    );
    assert!(
        xml.contains("<text>修 bug</text><text>回答完了</text>"),
        "{xml}"
    );
    assert!(
        xml.contains("<action content=\"打开\" arguments=\"open\" activationType=\"foreground\"/>"),
        "{xml}"
    );
}

#[test]
fn text_is_escaped_for_xml() {
    let xml = template("a<b> & \"c\" 'd'", "x", "y");
    assert!(
        xml.contains("<text>a&lt;b&gt; &amp; &quot;c&quot; &apos;d&apos;</text>"),
        "{xml}"
    );
}

#[test]
fn the_document_loads() {
    let doc = XmlDocument::new().expect("XmlDocument");
    doc.LoadXml(&HSTRING::from(template("t", "b", "o")))
        .expect("拼的 XML 读得进");
}
