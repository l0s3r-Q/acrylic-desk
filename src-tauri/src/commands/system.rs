//! 系统相关命令

use tauri::{AppHandle, Manager, State};

use crate::config::schema::AppInfo;
use crate::AppState;

/// 打开设置窗口
///
/// ⚠️ 注意（见 docs/03 §8.7）：
/// 设置窗口在 tauri.conf.json 中配置为 `visible: false`，
/// Tauri 会**延迟 WebView 的首次合成**。若只调用 `show()`，
/// 窗口可能显示为空白，直到用户拖动或改变其大小才触发重绘。
///
/// 因此这里在 show 之后主动做一次位置微调，
/// 强制 WebView 重新布局与合成：
#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "找不到设置窗口".to_string())?;

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    // 强制重绘：位置挪动 1px 再回来
    if let Ok(pos) = window.outer_position() {
        let _ = window.set_position(tauri::PhysicalPosition::new(pos.x + 1, pos.y));
        let _ = window.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
    }

    Ok(())
}

/// 获取应用信息
#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> Result<AppInfo, String> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        os_version: os_version_string(),
        data_dir: state.config.data_dir().display().to_string(),
    })
}

/// 退出应用（先落盘配置）
#[tauri::command]
pub fn quit_app(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let _ = state.config.flush_now();
    app.exit(0);
    Ok(())
}

#[cfg(windows)]
fn os_version_string() -> String {
    // 通过 RtlGetVersion 获取真实版本（GetVersionEx 在 Win8.1+ 会撒谎）
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

    type RtlGetVersionFn = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> i32;

    unsafe {
        let ntdll = match GetModuleHandleW(windows::core::w!("ntdll.dll")) {
            Ok(h) => h,
            Err(_) => return "Windows".into(),
        };

        let proc = match GetProcAddress(ntdll, windows::core::s!("RtlGetVersion")) {
            Some(p) => p,
            None => return "Windows".into(),
        };

        let func: RtlGetVersionFn = std::mem::transmute(proc);

        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };

        if func(&mut info) == 0 {
            format!(
                "Windows {}.{}.{}",
                info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
            )
        } else {
            "Windows".into()
        }
    }
}

#[cfg(not(windows))]
fn os_version_string() -> String {
    std::env::consts::OS.to_string()
}
