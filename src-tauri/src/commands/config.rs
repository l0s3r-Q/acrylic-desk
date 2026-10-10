//! 配置相关命令

use tauri::{AppHandle, Manager, State};

use crate::config::schema::Config;
use crate::config::store::emit_config_changed;
use crate::AppState;

/// 获取完整配置
#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.get())
}

/// 局部更新配置（深合并）
#[tauri::command]
pub fn update_config(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: serde_json::Value,
) -> Result<Config, String> {
    let cfg = state
        .config
        .mutate(|cfg| {
            let mut current = serde_json::to_value(&*cfg)
                .map_err(|e| format!("配置序列化失败：{e}"))?;
            deep_merge(&mut current, &patch);
            let updated: Config = serde_json::from_value(current)
                .map_err(|e| format!("配置格式错误：{e}"))?;
            *cfg = updated.clone();
            Ok::<Config, String>(updated)
        })
        .map_err(|e: String| e)?;

    // 通知所有窗口
    emit_config_changed(&app, &cfg);

    // 面板相关变更需要立即应用
    if let Err(e) = apply_panel_changes(&app) {
        eprintln!("[warn] 面板变更应用失败：{e}");
    }

    Ok(cfg)
}

/// 恢复默认配置（会先备份）
#[tauri::command]
pub fn reset_config(app: AppHandle, state: State<'_, AppState>) -> Result<Config, String> {
    // 先落盘当前配置到备份
    let _ = state.config.flush_now();

    let cfg = Config::default();
    state.config.set(cfg.clone());

    emit_config_changed(&app, &cfg);

    if let Err(e) = apply_panel_changes(&app) {
        eprintln!("[warn] 面板变更应用失败：{e}");
    }

    Ok(cfg)
}

/// 深合并 JSON（patch 覆盖 target）
fn deep_merge(target: &mut serde_json::Value, patch: &serde_json::Value) {
    use serde_json::Value;

    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                match t.get_mut(k) {
                    Some(existing) => deep_merge(existing, v),
                    None => {
                        t.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (t, p) => {
            *t = p.clone();
        }
    }
}

/// 应用面板相关变更（尺寸、边、材质）
fn apply_panel_changes(app: &AppHandle) -> Result<(), String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "应用状态未初始化".to_string())?;

    let cfg = state.config.get();

    if let Some(window) = app.get_webview_window("panel") {
        // 尺寸与位置
        state
            .edge
            .apply_edge_change()
            .map_err(|e| e.to_string())?;

        // 置顶
        let _ = window.set_always_on_top(cfg.panel.always_on_top);

        // 材质
        let _ = crate::window::effect::apply_radius(
            &window,
            cfg.appearance.effect,
            cfg.panel.corner_radius,
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deep_merge_nested() {
        let mut target = json!({"a": {"b": 1, "c": 2}, "d": 3});
        let patch = json!({"a": {"b": 9}});
        deep_merge(&mut target, &patch);
        assert_eq!(target, json!({"a": {"b": 9, "c": 2}, "d": 3}));
    }

    #[test]
    fn deep_merge_replaces_scalar() {
        let mut target = json!({"a": 1});
        let patch = json!({"a": "text"});
        deep_merge(&mut target, &patch);
        assert_eq!(target, json!({"a": "text"}));
    }

    #[test]
    fn deep_merge_adds_new_key() {
        let mut target = json!({"a": 1});
        let patch = json!({"b": 2});
        deep_merge(&mut target, &patch);
        assert_eq!(target, json!({"a": 1, "b": 2}));
    }
}
