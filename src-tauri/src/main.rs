//! AcrylicDesk —— 亚克力桌面管理面板
//!
//! 应用入口。负责组装 Tauri Builder、注册状态、挂载命令与插件。

// Windows 发布版不弹出控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    acrylic_desk_lib::run()
}
