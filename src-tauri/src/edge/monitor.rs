//! 显示器枚举
//!
//! 使用 Win32 `EnumDisplayMonitors` + `GetMonitorInfoW`。
//! 多显示器坐标可能为负（副屏在主屏左侧/上方）。

use crate::config::schema::MonitorInfo;
use crate::edge::geometry::Rect;
use crate::error::{AppError, Result};

/// 枚举所有显示器
#[cfg(windows)]
pub fn enumerate_monitors() -> Result<Vec<MonitorInfo>> {
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

    struct Ctx {
        list: Vec<MonitorInfo>,
        index: usize,
    }

    unsafe extern "system" fn callback(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let ctx = &mut *(data.0 as *mut Ctx);

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            let is_primary = (info.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY
            let r = info.rcMonitor;
            let w = info.rcWork;

            ctx.list.push(MonitorInfo {
                index: ctx.index,
                name: format!("显示器 {}", ctx.index + 1),
                x: r.left,
                y: r.top,
                width: r.right - r.left,
                height: r.bottom - r.top,
                work_x: w.left,
                work_y: w.top,
                work_width: w.right - w.left,
                work_height: w.bottom - w.top,
                is_primary,
                // DPI 缩放在 apply 阶段按窗口实际值填充
                scale_factor: 1.0,
            });
            ctx.index += 1;
        }

        TRUE
    }

    let mut ctx = Ctx {
        list: Vec::new(),
        index: 0,
    };

    unsafe {
        let ok = EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM(&mut ctx as *mut Ctx as isize),
        );
        if !ok.as_bool() {
            return Err(AppError::Window("显示器枚举失败".into()));
        }
    }

    if ctx.list.is_empty() {
        return Err(AppError::Window("未检测到任何显示器".into()));
    }

    // 主显示器排到 index 0，方便前端默认选择
    ctx.list.sort_by_key(|m| (!m.is_primary) as u8);
    for (i, m) in ctx.list.iter_mut().enumerate() {
        m.index = i;
    }

    Ok(ctx.list)
}

#[cfg(not(windows))]
pub fn enumerate_monitors() -> Result<Vec<MonitorInfo>> {
    Err(AppError::Window("仅支持 Windows".into()))
}

/// 按索引取显示器；索引越界时回退到主显示器
pub fn monitor_at(monitors: &[MonitorInfo], index: usize) -> Option<&MonitorInfo> {
    monitors.get(index).or_else(|| monitors.first())
}

/// 取指定显示器的工作区
pub fn work_rect(monitors: &[MonitorInfo], index: usize) -> Result<Rect> {
    let mon = monitor_at(monitors, index)
        .ok_or_else(|| AppError::Window("找不到指定显示器".into()))?;
    Ok(crate::edge::geometry::work_rect_of(mon))
}

#[cfg(windows)]
pub fn cursor_pos() -> (i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut p = POINT::default();
    unsafe {
        if GetCursorPos(&mut p).is_ok() {
            (p.x, p.y)
        } else {
            (-1, -1)
        }
    }
}

#[cfg(not(windows))]
pub fn cursor_pos() -> (i32, i32) {
    (-1, -1)
}

/// 检测当前是否有全屏应用在前台
///
/// 简化实现：取前台窗口矩形，若覆盖整个显示器则认为全屏。
#[cfg(windows)]
pub fn is_fullscreen_foreground() -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowRect, IsWindowVisible,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() || !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r).is_err() {
            return false;
        }

        // 获取该窗口所在显示器
        let monitors = match enumerate_monitors() {
            Ok(m) => m,
            Err(_) => return false,
        };

        monitors.iter().any(|m| {
            r.left <= m.x
                && r.top <= m.y
                && r.right >= m.x + m.width
                && r.bottom >= m.y + m.height
        })
    }
}

#[cfg(not(windows))]
pub fn is_fullscreen_foreground() -> bool {
    false
}
