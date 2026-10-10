//! 面板相关命令

use tauri::{AppHandle, Manager, State};

use crate::config::schema::{Edge, EdgeState, EffectKind, MonitorInfo};
use crate::config::store::emit_config_changed;
use crate::AppState;

/// 切换挂载边
#[tauri::command]
pub fn set_panel_edge(
    app: AppHandle,
    state: State<'_, AppState>,
    edge: String,
) -> Result<(), String> {
    let parsed = Edge::parse(&edge)?;

    state.config.mutate(|cfg| {
        cfg.panel.edge = parsed;
    });

    state.edge.apply_edge_change().map_err(|e| e.to_string())?;

    emit_config_changed(&app, &state.config.get());
    Ok(())
}

/// 调整面板尺寸
#[tauri::command]
pub fn set_panel_size(
    app: AppHandle,
    state: State<'_, AppState>,
    length: Option<u32>,
    thickness: Option<u32>,
) -> Result<(), String> {
    state.config.mutate(|cfg| {
        if let Some(l) = length {
            cfg.panel.length = l.clamp(200, 4000);
        }
        if let Some(t) = thickness {
            cfg.panel.thickness = t.clamp(120, 1200);
        }
    });

    state.edge.apply_size_change().map_err(|e| e.to_string())?;

    emit_config_changed(&app, &state.config.get());
    Ok(())
}

/// 切换材质模式（quality = Acrylic，performance = Blur）
#[tauri::command]
pub fn set_panel_effect(
    app: AppHandle,
    mode: String,
) -> Result<(), String> {
    let window = app
        .get_webview_window("panel")
        .ok_or_else(|| "找不到面板窗口".to_string())?;

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "应用状态未初始化".to_string())?;

    let kind = match mode.as_str() {
        "quality" => state.config.get().appearance.effect,
        "performance" => EffectKind::Blur,
        other => return Err(format!("未知的材质模式：{other}")),
    };

    crate::window::effect::apply_effect(&window, kind).map_err(|e| e.to_string())
}

/// 设置鼠标穿透
#[tauri::command]
pub fn set_passthrough(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .edge
        .set_passthrough(enabled)
        .map_err(|e| e.to_string())
}

/// 强制唤出面板
#[tauri::command]
pub fn summon_panel(state: State<'_, AppState>) -> Result<(), String> {
    state.edge.summon().map_err(|e| e.to_string())
}

/// 强制收起面板
#[tauri::command]
pub fn hide_panel(state: State<'_, AppState>) -> Result<(), String> {
    state.edge.hide_now().map_err(|e| e.to_string())
}

/// 查询当前显隐状态
#[tauri::command]
pub fn get_edge_state(state: State<'_, AppState>) -> Result<EdgeState, String> {
    Ok(state.edge.state())
}

/// 枚举显示器
#[tauri::command]
pub fn list_monitors() -> Result<Vec<MonitorInfo>, String> {
    crate::edge::monitor::enumerate_monitors().map_err(|e| e.to_string())
}

/// 设置拖拽状态（拖拽期间禁止收起）
#[tauri::command]
pub fn set_dragging(state: State<'_, AppState>, dragging: bool) -> Result<(), String> {
    state.edge.set_dragging(dragging);
    Ok(())
}

/// 校验热键字符串是否合法（设置界面即时校验用）
#[tauri::command]
pub fn validate_hotkey(spec: String) -> Result<(), String> {
    crate::window::hotkey::validate(&spec).map_err(|e| e.to_string())
}

/// 应用新的全局热键（配置变更后调用）
#[tauri::command]
pub fn apply_hotkey(
    state: State<'_, AppState>,
    spec: String,
    enabled: bool,
) -> Result<(), String> {
    use crate::window::hotkey;

    if !enabled {
        hotkey::unregister();
        return Ok(());
    }

    // 校验
    hotkey::validate(&spec).map_err(|e| e.to_string())?;

    // 重新注册
    hotkey::reregister(&spec).map_err(|e| e.to_string())?;

    // 记录到配置
    let spec_clone = spec.clone();
    state.config.mutate(|cfg| {
        cfg.edge.summon_hotkey = spec_clone;
        cfg.edge.hotkey_enabled = true;
    });

    Ok(())
}
