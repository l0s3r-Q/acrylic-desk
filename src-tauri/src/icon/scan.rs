//! 扫描桌面与开始菜单

use std::path::PathBuf;

use crate::config::schema::ItemType;
use crate::error::{AppError, Result};

/// 一个待导入的候选项目
#[derive(Debug, Clone)]
pub struct ScannedItem {
    pub name: String,
    pub path: String,
    pub item_type: ItemType,
    pub icon_path: Option<String>,
    pub source_path: Option<String>,
}

/// 扫描桌面
///
/// 覆盖两个位置：
/// - 用户桌面 `%USERPROFILE%\Desktop`
/// - 公共桌面 `%PUBLIC%\Desktop`
pub fn scan_desktop() -> Result<Vec<ScannedItem>> {
    let mut out = Vec::new();

    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(&profile).join("Desktop");
        collect_dir(&p, &mut out);
    }

    if let Ok(public) = std::env::var("PUBLIC") {
        let p = PathBuf::from(&public).join("Desktop");
        collect_dir(&p, &mut out);
    }

    // 常见便携桌面路径（OneDrive 接管桌面的情况）
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(&profile)
            .join("OneDrive")
            .join("Desktop");
        if p.exists() {
            collect_dir(&p, &mut out);
        }
    }

    // 去重（同一文件可能在多个位置出现）
    out.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    out.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));

    Ok(out)
}

/// 扫描开始菜单
pub fn scan_start_menu() -> Result<Vec<ScannedItem>> {
    let mut out = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(&appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        collect_dir(&p, &mut out);
    }

    if let Ok(pd) = std::env::var("ProgramData") {
        let p = PathBuf::from(&pd)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        collect_dir(&p, &mut out);
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));

    Ok(out)
}

/// 递归收集目录中的可启动项
fn collect_dir(dir: &PathBuf, out: &mut Vec<ScannedItem>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // 跳过隐藏文件
        if name.starts_with('.') {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            // 递归子目录
            collect_dir(&path, out);
            continue;
        }

        // 只收快捷方式和可执行文件
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        if ext != "lnk" && ext != "exe" && ext != "url" {
            continue;
        }

        // 跳过 uninstall / 卸载类
        let lower = name.to_lowercase();
        if lower.contains("uninstall") || name.contains("卸载") {
            continue;
        }

        let display = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());

        let path_str = path.to_string_lossy().to_string();

        out.push(ScannedItem {
            name: display,
            path: path_str.clone(),
            item_type: ItemType::App,
            icon_path: None,
            source_path: Some(path_str),
        });
    }
}

/// 解析桌面 .lnk 的真实目标（可选，用于显示更准确的信息）
pub fn resolve_if_lnk(path: &str) -> Option<(String, String, String)> {
    if path.to_ascii_lowercase().ends_with(".lnk") {
        crate::icon::extract::resolve_lnk(path).ok()
    } else {
        None
    }
}

/// 确保扫描路径存在
pub fn desktop_exists() -> bool {
    std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Desktop").exists())
        .unwrap_or(false)
}

#[allow(dead_code)]
fn _unused(_: AppError) {}
