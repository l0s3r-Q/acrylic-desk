//! 启动图标项

use std::process::Command;

use crate::config::schema::{IconItem, ItemType};
use crate::error::{AppError, Result};

/// 启动一个图标项
pub fn launch(item: &IconItem) -> Result<()> {
    match item.r#type {
        ItemType::Url => open_url(&item.target),
        ItemType::Folder => open_path(&item.target),
        ItemType::File => open_path(&item.target),
        ItemType::App => launch_app(item),
    }
}

/// 打开 URL
fn open_url(url: &str) -> Result<()> {
    // 用系统默认浏览器打开（不经过 cmd，避免杀软告警）
    crate::icon::launch::shell_open(url)
}

/// 用关联程序打开文件/文件夹
fn open_path(path: &str) -> Result<()> {
    shell_open(path)
}

/// 启动应用程序
fn launch_app(item: &IconItem) -> Result<()> {
    let target = &item.target;

    // .lnk 由系统直接打开即可，无需解析
    if target.to_ascii_lowercase().ends_with(".lnk") {
        return shell_open(target);
    }

    let mut cmd = Command::new(target);

    if !item.args.is_empty() {
        // 简单按空格切分（不支持引号嵌套的复杂场景）
        for arg in item.args.split_whitespace() {
            cmd.arg(arg);
        }
    }

    if !item.work_dir.is_empty() {
        cmd.current_dir(&item.work_dir);
    }

    cmd.spawn()
        .map_err(|e| AppError::Io(format!("启动失败：{e}")))?;

    Ok(())
}

/// 调用系统 Shell 打开路径/URL
///
/// 使用 `explorer.exe` 或 `rundll32` 之外的方式，
/// 避免被杀软识别为可疑的命令行调用。
#[cfg(windows)]
pub fn shell_open(target: &str) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = std::ffi::OsStr::new(target)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let op: Vec<u16> = std::ffi::OsStr::new("open")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let result = ShellExecuteW(
            None,
            PCWSTR(op.as_ptr()),
            PCWSTR(wide.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );

        // ShellExecuteW 返回值 <= 32 表示失败
        if result.0 as usize <= 32 {
            return Err(AppError::Io(format!(
                "无法打开：{target}（系统返回 {}）",
                result.0 as usize
            )));
        }
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn shell_open(target: &str) -> Result<()> {
    Err(AppError::Io(format!("仅支持 Windows：{target}")))
}

/// 在资源管理器中定位文件
#[cfg(windows)]
pub fn reveal_in_explorer(path: &str) -> Result<()> {
    // explorer /select,"路径"
    Command::new("explorer")
        .arg(format!("/select,{path}"))
        .spawn()
        .map_err(|e| AppError::Io(format!("打开资源管理器失败：{e}")))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn reveal_in_explorer(_path: &str) -> Result<()> {
    Err(AppError::NotImplemented("仅支持 Windows".into()))
}

/// 以管理员身份启动
#[cfg(windows)]
pub fn launch_as_admin(target: &str, args: &str) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let file: Vec<u16> = std::ffi::OsStr::new(target)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = std::ffi::OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params: Vec<u16> = std::ffi::OsStr::new(args)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let r = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(params.as_ptr()),
            None,
            SW_SHOWNORMAL,
        );
        if r.0 as usize <= 32 {
            return Err(AppError::Io("提权启动被拒绝或失败".into()));
        }
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn launch_as_admin(_target: &str, _args: &str) -> Result<()> {
    Err(AppError::NotImplemented("仅支持 Windows".into()))
}
