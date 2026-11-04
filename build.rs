//! 编译期准备（施工 AP-5）：Windows 上把 Miyu 的图标和版本信息编进 `miyu-app.exe`，资源管理器、任务管理器、「属性」里看到的是
//! Miyu 的图标和名字。原来是 `tauri-build` 编进去的，AP-2 去掉 Tauri 以后程序本身没有图标了（安装包、开始菜单的图标照旧由打包工具给）。
//! 编给别的平台什么都不做：macOS、Linux 的图标在安装包里。

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    // 看编给哪个平台，不看 build.rs 自己跑在哪（本机交叉编 Windows 的也要编进去）
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if let Err(problem) = windows_resources() {
        eprintln!("Windows 的图标、版本信息编不进去：{problem}");
        std::process::exit(1);
    }
}

/// 写一份资源脚本，交给 `embed-resource` 照平台找资源编译器（MSVC 上是 Windows SDK 的 `rc.exe`，别的是 `windres`）编、链进程序。
fn windows_resources() -> Result<(), String> {
    let icon = Path::new(&env("CARGO_MANIFEST_DIR")?)
        .join("icons")
        .join("icon.ico");
    let script = Path::new(&env("OUT_DIR")?).join("miyu-app.rc");
    let version = [
        env("CARGO_PKG_VERSION_MAJOR")?,
        env("CARGO_PKG_VERSION_MINOR")?,
        env("CARGO_PKG_VERSION_PATCH")?,
    ];
    let text = resource_script(
        &icon.to_string_lossy(),
        &version,
        &env("CARGO_PKG_VERSION")?,
    );
    std::fs::write(&script, text).map_err(|e| format!("写不了 {}：{e}", script.display()))?;
    embed_resource::compile_for(&script, ["miyu-app"], embed_resource::NONE)
        .manifest_required()
        .map_err(|e| e.to_string())
}

fn env(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{name}：{e}"))
}

/// 资源脚本：图标（编号 1，资源管理器拿编号最小的那一组当程序的图标），版本信息（任务管理器的「名称」一栏照 `FileDescription`
/// 写，不写就是程序文件名）。给人看的地方写「Miyu」（图纸 app.md「为什么换」），语言写「不分语言」。
fn resource_script(icon: &str, version: &[String; 3], full: &str) -> String {
    // 资源脚本的字符串里反斜杠要写两个、引号写两个
    let icon = icon.replace('\\', "\\\\").replace('"', "\"\"");
    let [major, minor, patch] = version;
    format!(
        r#"1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "000004b0"
    BEGIN
      VALUE "CompanyName", "miyu-agent"
      VALUE "FileDescription", "Miyu"
      VALUE "FileVersion", "{full}"
      VALUE "InternalName", "miyu-app"
      VALUE "OriginalFilename", "miyu-app.exe"
      VALUE "ProductName", "Miyu"
      VALUE "ProductVersion", "{full}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0, 0x4b0
  END
END
"#
    )
}
