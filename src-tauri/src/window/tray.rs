//! 系统托盘

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::error::Result;

/// 创建托盘图标与菜单
pub fn setup_tray(app: &AppHandle) -> Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "显示 / 隐藏面板", true, None::<&str>)
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;
    let sep1 = PredefinedMenuItem::separator(app)
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;
    let reload = MenuItem::with_id(app, "reload", "重载面板", true, None::<&str>)
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;
    let sep2 = PredefinedMenuItem::separator(app)
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;

    let menu = Menu::with_items(app, &[&toggle, &settings, &sep1, &reload, &sep2, &quit])
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            crate::error::AppError::Window("缺少应用图标".into())
        })?)
        .tooltip("AcrylicDesk")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_menu(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击托盘：切换面板
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_panel(app);
            }
        })
        .build(app)
        .map_err(|e| crate::error::AppError::Window(e.to_string()))?;

    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "toggle" => toggle_panel(app),
        "settings" => {
            // 与 commands::system::open_settings 保持一致：
            // 隐藏窗口首次显示需强制重绘（见 docs/03 §8.7）
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.set_focus();
                if let Ok(pos) = w.outer_position() {
                    let _ = w.set_position(tauri::PhysicalPosition::new(pos.x + 1, pos.y));
                    let _ = w.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
                }
            }
        }
        "reload" => {
            if let Some(w) = app.get_webview_window("panel") {
                let _ = w.eval("window.location.reload()");
            }
        }
        "quit" => {
            // 退出前落盘配置
            if let Some(store) = crate::config::store::store_of(app) {
                let _ = store.flush_now();
            }
            app.exit(0);
        }
        _ => {}
    }
}

fn toggle_panel(app: &AppHandle) {
    if let Some(state) = app.try_state::<crate::AppState>() {
        let _ = state.edge.toggle();
    }
}
