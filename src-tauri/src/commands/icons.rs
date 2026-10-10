//! 图标与文件相关命令
//!
//! P1 阶段先提供接口骨架，P2 阶段补全完整实现。

use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::config::schema::{Group, IconItem};
use crate::config::store::emit_config_changed;
use crate::AppState;

/// 生成唯一 ID
fn gen_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // 加一个计数避免同一纳秒内碰撞
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{prefix}_{nanos:x}{n:x}")
}

/// 当前时间 ISO8601
fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// 从路径推导显示名称
fn display_name(path: &str) -> String {
    let p = PathBuf::from(path);
    p.file_stem()
        .or_else(|| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// 批量添加图标
///
/// 会同步解析 .lnk 目标并提取图标（PNG 落盘到 data/icons/）。
#[tauri::command]
pub fn add_icons(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: String,
    paths: Vec<String>,
) -> Result<Vec<IconItem>, String> {
    if paths.is_empty() {
        return Err("未选择任何文件".into());
    }

    let data_dir = state.config.data_dir().to_path_buf();

    let new_items: Vec<IconItem> = paths
        .iter()
        .map(|path| build_icon_item(path, &data_dir))
        .collect();

    let added = new_items.clone();

    state.config.mutate(|cfg| {
        if let Some(g) = cfg.groups.iter_mut().find(|g| g.id == group_id) {
            let base = g.items.len() as i32;
            for (i, item) in new_items.into_iter().enumerate() {
                let mut it = item;
                it.order = base + i as i32;
                g.items.push(it);
            }
        }
    });

    emit_config_changed(&app, &state.config.get());
    Ok(added)
}

/// 由路径构建一个图标项（解析快捷方式 + 提取图标）
fn build_icon_item(path: &str, data_dir: &std::path::Path) -> IconItem {
    let item_type = crate::icon::extract::detect_item_type(path);

    // .lnk 解析出真实目标与参数
    let (target, args, work_dir, display) = if item_type == crate::config::schema::ItemType::App
        && path.to_ascii_lowercase().ends_with(".lnk")
    {
        match crate::icon::extract::resolve_lnk(path) {
            Ok((t, a, w)) => {
                let name = std::path::Path::new(&t)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| display_name(path));
                (t, a, w, name)
            }
            // 解析失败则仍按该 .lnk 本身处理
            Err(_) => (path.to_string(), String::new(), String::new(), display_name(path)),
        }
    } else {
        (
            path.to_string(),
            String::new(),
            String::new(),
            display_name(path),
        )
    };

    // 提取图标（对 .lnk 用原始路径更容易拿到正确图标）
    let icon_source = if item_type == crate::config::schema::ItemType::App
        && path.to_ascii_lowercase().ends_with(".lnk")
    {
        path
    } else {
        &target
    };

    let icon_path = match crate::icon::extract::extract_icon_png(icon_source) {
        Ok(png) => {
            let rel = crate::icon::extract::icon_cache_name(icon_source);
            let full = data_dir.join(&rel);
            if let Some(parent) = full.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&full, &png) {
                Ok(_) => Some(rel),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };

    IconItem {
        id: gen_id("i"),
        r#type: item_type,
        name: display,
        target,
        args,
        work_dir,
        icon_path,
        source_path: Some(path.to_string()),
        run_as_admin: false,
        order: 0,
        added_at: now_iso(),
    }
}

/// 删除图标（仅移除引用）
#[tauri::command]
pub fn remove_icon(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: String,
    item_id: String,
) -> Result<(), String> {
    let mut found = false;

    state.config.mutate(|cfg| {
        if let Some(g) = cfg.groups.iter_mut().find(|g| g.id == group_id) {
            let before = g.items.len();
            g.items.retain(|i| i.id != item_id);
            found = g.items.len() != before;

            // 重排序号
            for (i, item) in g.items.iter_mut().enumerate() {
                item.order = i as i32;
            }
        }
    });

    if !found {
        return Err("找不到该图标项".into());
    }

    emit_config_changed(&app, &state.config.get());
    Ok(())
}

/// 更新图标项
#[tauri::command]
pub fn update_icon(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: String,
    item_id: String,
    patch: serde_json::Value,
) -> Result<IconItem, String> {
    let updated = state
        .config
        .mutate(|cfg| -> Result<IconItem, String> {
            let g = cfg
                .groups
                .iter_mut()
                .find(|g| g.id == group_id)
                .ok_or_else(|| "找不到分组".to_string())?;

            let item = g
                .items
                .iter_mut()
                .find(|i| i.id == item_id)
                .ok_or_else(|| "找不到该图标项".to_string())?;

            // 只允许修改特定字段
            if let Some(v) = patch.get("name").and_then(|v| v.as_str()) {
                item.name = v.to_string();
            }
            if let Some(v) = patch.get("args").and_then(|v| v.as_str()) {
                item.args = v.to_string();
            }
            if let Some(v) = patch.get("workDir").and_then(|v| v.as_str()) {
                item.work_dir = v.to_string();
            }
            if let Some(v) = patch.get("runAsAdmin").and_then(|v| v.as_bool()) {
                item.run_as_admin = v;
            }

            Ok(item.clone())
        })?;

    emit_config_changed(&app, &state.config.get());
    Ok(updated)
}

/// 重排图标顺序
#[tauri::command]
pub fn reorder_icons(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), String> {
    state.config.mutate(|cfg| {
        if let Some(g) = cfg.groups.iter_mut().find(|g| g.id == group_id) {
            // 按给定顺序设置 order
            for (idx, id) in ordered_ids.iter().enumerate() {
                if let Some(item) = g.items.iter_mut().find(|i| &i.id == id) {
                    item.order = idx as i32;
                }
            }
        }
    });

    emit_config_changed(&app, &state.config.get());
    Ok(())
}

/// 启动图标项
#[tauri::command]
pub fn launch_icon(state: State<'_, AppState>, item_id: String) -> Result<(), String> {
    let cfg = state.config.get();

    let item = cfg
        .groups
        .iter()
        .flat_map(|g| g.items.iter())
        .find(|i| i.id == item_id)
        .ok_or_else(|| "找不到该图标项".to_string())?
        .clone();

    if item.run_as_admin {
        crate::icon::launch::launch_as_admin(&item.target, &item.args)
            .map_err(|e| e.to_string())
    } else {
        crate::icon::launch::launch(&item).map_err(|e| e.to_string())
    }
}

/// 在资源管理器中显示
#[tauri::command]
pub fn reveal_icon(state: State<'_, AppState>, item_id: String) -> Result<(), String> {
    let cfg = state.config.get();

    let item = cfg
        .groups
        .iter()
        .flat_map(|g| g.items.iter())
        .find(|i| i.id == item_id)
        .ok_or_else(|| "找不到该图标项".to_string())?;

    crate::icon::launch::reveal_in_explorer(&item.target).map_err(|e| e.to_string())
}

/// 取图标为 data URL（供前端 img 使用）
#[tauri::command]
pub fn get_icon_data_url(state: State<'_, AppState>, item_id: String) -> Result<String, String> {
    let cfg = state.config.get();
    let data_dir = state.config.data_dir().to_path_buf();

    let item = cfg
        .groups
        .iter()
        .flat_map(|g| g.items.iter())
        .find(|i| i.id == item_id)
        .ok_or_else(|| "找不到该图标项".to_string())?;

    let rel = item
        .icon_path
        .as_ref()
        .ok_or_else(|| "该图标项尚未提取图标".to_string())?;

    let full = data_dir.join(rel);
    let bytes = std::fs::read(&full).map_err(|e| format!("读取图标失败：{e}"))?;

    let b64 = base64_encode(&bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// 简易 base64 编码（避免引入额外依赖）
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);

        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

/* ---------------- 分组命令 ---------------- */

/// 新建分组
#[tauri::command]
pub fn create_group(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    icon: Option<String>,
) -> Result<Group, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("分组名称不能为空".into());
    }

    let group = Group {
        id: gen_id("g"),
        name,
        icon: icon.unwrap_or_else(|| "📁".into()),
        order: 0,
        items: vec![],
    };

    let created = group.clone();

    state.config.mutate(|cfg| {
        let order = cfg.groups.len() as i32;
        let mut g = group;
        g.order = order;
        cfg.groups.push(g);
    });

    emit_config_changed(&app, &state.config.get());
    Ok(created)
}

/// 重命名分组
#[tauri::command]
pub fn rename_group(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: String,
    name: String,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("分组名称不能为空".into());
    }

    state.config.mutate(|cfg| {
        if let Some(g) = cfg.groups.iter_mut().find(|g| g.id == group_id) {
            g.name = name;
        }
    });

    emit_config_changed(&app, &state.config.get());
    Ok(())
}

/// 删除分组
#[tauri::command]
pub fn delete_group(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: String,
    move_items_to: Option<String>,
) -> Result<(), String> {
    let mut err: Option<String> = None;

    state.config.mutate(|cfg| {
        if cfg.groups.len() <= 1 {
            err = Some("至少需要保留一个分组".into());
            return;
        }

        let idx = match cfg.groups.iter().position(|g| g.id == group_id) {
            Some(i) => i,
            None => {
                err = Some("找不到该分组".into());
                return;
            }
        };

        let removed = cfg.groups.remove(idx);

        // 迁移图标到目标分组
        if let Some(target) = move_items_to {
            if let Some(g) = cfg.groups.iter_mut().find(|g| g.id == target) {
                let base = g.items.len() as i32;
                for (i, mut item) in removed.items.into_iter().enumerate() {
                    item.order = base + i as i32;
                    g.items.push(item);
                }
            }
        }

        // 若删除的是当前激活分组，切换
        if cfg.active_group_id.as_deref() == Some(removed.id.as_str()) {
            cfg.active_group_id = cfg.groups.first().map(|g| g.id.clone());
        }

        // 重排序号
        for (i, g) in cfg.groups.iter_mut().enumerate() {
            g.order = i as i32;
        }
    });

    if let Some(e) = err {
        return Err(e);
    }

    emit_config_changed(&app, &state.config.get());
    Ok(())
}

/// 切换当前分组
#[tauri::command]
pub fn set_active_group(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: String,
) -> Result<(), String> {
    let ok = state.config.mutate(|cfg| {
        if cfg.groups.iter().any(|g| g.id == group_id) {
            cfg.active_group_id = Some(group_id.clone());
            true
        } else {
            false
        }
    });

    if !ok {
        return Err("找不到该分组".into());
    }

    emit_config_changed(&app, &state.config.get());
    Ok(())
}

/// 选择背景图片（打开文件选择器）
#[tauri::command]
pub async fn pick_background(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();

    app.dialog()
        .file()
        .add_filter("图片", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });

    let picked = rx
        .recv()
        .map_err(|_| "文件选择被取消".to_string())?;

    Ok(picked.map(|p| p.to_string()))
}

/* ---------------- 扫描命令 ---------------- */

/// 扫描结果项（供前端选择导入）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
}

/// 扫描桌面图标
#[tauri::command]
pub fn scan_desktop() -> Result<Vec<ScanEntry>, String> {
    let items = crate::icon::scan::scan_desktop().map_err(|e| e.to_string())?;
    Ok(items
        .into_iter()
        .map(|i| ScanEntry {
            name: i.name,
            path: i.path,
            kind: "desktop".into(),
        })
        .collect())
}

/// 扫描开始菜单
#[tauri::command]
pub fn scan_start_menu() -> Result<Vec<ScanEntry>, String> {
    let items = crate::icon::scan::scan_start_menu().map_err(|e| e.to_string())?;
    Ok(items
        .into_iter()
        .map(|i| ScanEntry {
            name: i.name,
            path: i.path,
            kind: "startmenu".into(),
        })
        .collect())
}

/// 刷新单个图标项的图标（目标文件变化后重新提取）
#[tauri::command]
pub fn refresh_icon(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
) -> Result<String, String> {
    let data_dir = state.config.data_dir().to_path_buf();
    let cfg = state.config.get();

    let item = cfg
        .groups
        .iter()
        .flat_map(|g| g.items.iter())
        .find(|i| i.id == item_id)
        .ok_or_else(|| "找不到该图标项".to_string())?
        .clone();

    let source = item
        .source_path
        .clone()
        .unwrap_or_else(|| item.target.clone());

    let png = crate::icon::extract::extract_icon_png(&source)
        .map_err(|e| format!("提取图标失败：{e}"))?;

    let rel = crate::icon::extract::icon_cache_name(&source);
    let full = data_dir.join(&rel);

    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{e}"))?;
    }

    std::fs::write(&full, &png).map_err(|e| format!("写入图标失败：{e}"))?;

    // 更新配置中的 iconPath
    state.config.mutate(|cfg| {
        for g in cfg.groups.iter_mut() {
            if let Some(it) = g.items.iter_mut().find(|i| i.id == item_id) {
                it.icon_path = Some(rel.clone());
            }
        }
    });

    emit_config_changed(&app, &state.config.get());
    Ok(rel)
}
