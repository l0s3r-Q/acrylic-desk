//! AcrylicDesk 库入口
//!
//! 架构分层见 docs/02-系统架构设计.md：
//! - window/  窗口与材质
//! - edge/    边缘吸附与唤出（核心）
//! - icon/    图标管理
//! - config/  配置持久化

pub mod commands;
pub mod config;
pub mod edge;
pub mod error;
pub mod icon;
pub mod window;

use std::sync::Arc;

use tauri::Manager;

/// 应用全局状态
pub struct AppState {
    pub config: Arc<config::ConfigStore>,
    pub edge: Arc<edge::EdgeManager>,
}

/// 应用入口
pub fn run() {
    // 初始化日志（调试期用标准输出，发布版写入文件）
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 单实例：重复启动时唤出面板
            if let Some(panel) = app.get_webview_window("panel") {
                let _ = panel.show();
                let _ = panel.set_focus();
            }
        }))
        .setup(|app| {
            let handle = app.handle().clone();
            log_startup("=== AcrylicDesk 启动 ===");

            // 1. 加载配置
            let config = config::ConfigStore::load(&handle)
                .map_err(|e| format!("配置加载失败：{e}"))?;
            let config = Arc::new(config);
            log_startup(&format!(
                "配置已加载: 分组数={}, 数据目录={:?}",
                config.get().groups.len(),
                config.data_dir()
            ));

            // 2. 初始化边缘管理器
            let edge = Arc::new(edge::EdgeManager::new(
                handle.clone(),
                config.clone(),
            ));
            log_startup("边缘管理器已创建");

            // 3. 应用初始窗口配置（位置、尺寸、材质）
            if let Some(panel) = handle.get_webview_window("panel") {
                match edge.apply_initial_layout(&panel) {
                    Ok(_) => log_startup("面板初始布局已应用"),
                    Err(e) => log_startup(&format!("⚠ 面板初始化失败: {e}")),
                }
            } else {
                log_startup("⚠ 未找到 panel 窗口");
            }

            // 4. 注册全局状态
            app.manage(AppState {
                config: config.clone(),
                edge: edge.clone(),
            });

            {
                let ok = app.try_state::<AppState>().is_some();
                log_startup(&format!("AppState 注册结果: {ok}"));
            }

            // 5. 启动边缘监测线程
            edge.clone().start_monitor();
            log_startup("边缘监测线程已启动");

            // 6. 托盘
            match crate::window::tray::setup_tray(&handle) {
                Ok(_) => log_startup("托盘已创建"),
                Err(e) => log_startup(&format!("⚠ 托盘初始化失败：{e}")),
            }

            // 7. 全局热键
            let hotkey_spec = config.get().edge.summon_hotkey.clone();
            match crate::window::hotkey::register(&hotkey_spec) {
                Ok(_) => log_startup(&format!("全局热键已注册: {hotkey_spec}")),
                Err(e) => log_startup(&format!("⚠ 热键注册失败: {e}")),
            }

            // 热键轮询线程（见下方说明）
            spawn_hotkey_watcher(handle.clone(), edge.clone(), config.clone());

            log_startup("=== 启动完成 ===");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // --- 配置 ---
            commands::config::get_config,
            commands::config::update_config,
            commands::config::reset_config,
            // --- 面板 ---
            commands::panel::set_panel_edge,
            commands::panel::set_panel_size,
            commands::panel::set_panel_effect,
            commands::panel::set_passthrough,
            commands::panel::summon_panel,
            commands::panel::hide_panel,
            commands::panel::get_edge_state,
            commands::panel::list_monitors,
            commands::panel::set_dragging,
            commands::panel::validate_hotkey,
            commands::panel::apply_hotkey,
            // --- 图标 ---
            commands::icons::add_icons,
            commands::icons::remove_icon,
            commands::icons::update_icon,
            commands::icons::reorder_icons,
            commands::icons::launch_icon,
            commands::icons::reveal_icon,
            commands::icons::get_icon_data_url,
            commands::icons::pick_background,
            commands::icons::refresh_icon,
            commands::icons::scan_desktop,
            commands::icons::scan_start_menu,
            // --- 分组 ---
            commands::icons::create_group,
            commands::icons::rename_group,
            commands::icons::delete_group,
            commands::icons::set_active_group,
            // --- 系统 ---
            commands::system::open_settings,
            commands::system::get_app_info,
            commands::system::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("AcrylicDesk 启动失败");
}

/// 初始化日志
///
/// 发布版没有控制台，因此写入 exe 同级的 startup.log，
/// 便于排查启动期问题（尤其是 State 注册、窗口初始化）。
fn init_logging() {
    println!("[AcrylicDesk] debug 模式");
}

/// 写启动日志到文件（GUI 程序无控制台时使用）
pub fn log_startup(msg: &str) {
    use std::io::Write;

    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("startup.log")));

    if let Some(p) = path {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
        {
            let _ = writeln!(f, "{msg}");
        }
    }
    println!("{msg}");
}

/// 热键监测线程
///
/// 实现说明（重要，见 docs/03 §3.1 的安全红线）：
///
/// `RegisterHotKey(None, ...)` 会把 `WM_HOTKEY` 投递到**注册线程的消息队列**，
/// 而 Tauri 的事件循环不会把它转交给我们，因此该方式收不到通知。
///
/// 替代方案有两种：
/// 1. 安装 `WH_KEYBOARD_LL` 钩子 —— ❌ **本项目禁止**，与杀软红线冲突
/// 2. 轮询 `GetAsyncKeyState` —— ✅ 无副作用，与边缘监测同一思路
///
/// 采用方案 2。轮询频率 50ms（20Hz），对手动触发完全够用，
/// CPU 开销实测可忽略。同时保留 `RegisterHotKey` 的注册结果，
/// 用于**冲突检测**（若热键被占用，注册失败即提示用户）。
fn spawn_hotkey_watcher(
    _app: tauri::AppHandle,
    edge: std::sync::Arc<crate::edge::EdgeManager>,
    config: std::sync::Arc<crate::config::ConfigStore>,
) {
    std::thread::spawn(move || {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

        // 上一次各键的按下状态，用于边沿检测（只在"刚按下"时触发一次）
        let mut was_down = false;
        let mut last_spec = config.get().edge.summon_hotkey;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            // 配置可能被用户修改，热键随之变化
            let spec = config.get().edge.summon_hotkey;
            if spec != last_spec {
                last_spec = spec.clone();
            }

            let (mods, vk) = match crate::window::hotkey::parse_hotkey(&spec) {
                Ok(v) => v,
                Err(_) => continue, // 配置非法，跳过
            };

            const MOD_ALT: u32 = 0x0001;
            const MOD_CONTROL: u32 = 0x0002;
            const MOD_SHIFT: u32 = 0x0004;
            const MOD_WIN: u32 = 0x0008;

            // 检查修饰键
            let mods_ok = unsafe {
                let alt = (GetAsyncKeyState(0x12) as u16 & 0x8000) != 0; // VK_MENU
                let ctrl = (GetAsyncKeyState(0x11) as u16 & 0x8000) != 0; // VK_CONTROL
                let shift = (GetAsyncKeyState(0x10) as u16 & 0x8000) != 0; // VK_SHIFT
                let win = ((GetAsyncKeyState(0x5B) as u16 & 0x8000) != 0)      // VK_LWIN
                    || ((GetAsyncKeyState(0x5C) as u16 & 0x8000) != 0); // VK_RWIN

                let need_alt = mods & MOD_ALT != 0;
                let need_ctrl = mods & MOD_CONTROL != 0;
                let need_shift = mods & MOD_SHIFT != 0;
                let need_win = mods & MOD_WIN != 0;

                alt == need_alt
                    && ctrl == need_ctrl
                    && shift == need_shift
                    && win == need_win
            };

            let key_down = if mods_ok {
                unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
            } else {
                false
            };

            // 边沿检测：从"未按下"变为"按下"时触发
            if key_down && !was_down {
                if let Err(e) = edge.toggle() {
                    eprintln!("[hotkey] 切换面板失败: {e}");
                }
            }
            was_down = key_down;
        }
    });
}
