//! 位置计算
//!
//! **坐标约定**（见 docs/03 §2.1）：
//! - Rust 侧全部使用 **物理像素**
//! - 传给前端的尺寸使用逻辑像素
//! - 换算集中在本模块，其他模块禁止自行换算
//!
//! **多显示器注意**：副屏可能在主屏左侧/上方，坐标为**负数**，
//! 因此所有坐标必须使用 `i32` 而非 `u32`。

use crate::config::schema::{Edge, MonitorInfo};

/// 面板尺寸（物理像素）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelSize {
    pub width: i32,
    pub height: i32,
}

impl PanelSize {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    /// 根据挂载边，由 (长度, 厚度) 构造窗口尺寸
    pub fn from_edge(edge: Edge, length: u32, thickness: u32) -> Self {
        let (w, h) = edge.to_window_size(length, thickness);
        Self::new(w as i32, h as i32)
    }
}

/// 矩形区域（物理像素）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    /// 点是否在矩形内（左闭右开）
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }
}

/// 计算面板窗口位置
///
/// # 参数
/// - `edge`: 挂载边
/// - `work`: 显示器工作区（已排除任务栏）
/// - `size`: 面板尺寸
/// - `offset`: 距边缘间距
/// - `visible`: true = 展开位置，false = 隐藏位置（完全移出屏幕）
///
/// # 关键点
/// 隐藏位置必须是 **完全移出屏幕**（如 `x = work.x - size.width`），
/// 而不是留 1px —— 否则会在屏幕边缘露出细线，且可能被鼠标误触。
pub fn compute_position(
    edge: Edge,
    work: Rect,
    size: PanelSize,
    offset: i32,
    visible: bool,
) -> (i32, i32) {
    match edge {
        Edge::Left => {
            let x = if visible {
                work.x + offset
            } else {
                work.x - size.width
            };
            // 垂直居中
            let y = work.y + (work.height - size.height) / 2;
            (x, y)
        }
        Edge::Right => {
            let x = if visible {
                work.x + work.width - size.width - offset
            } else {
                work.x + work.width
            };
            let y = work.y + (work.height - size.height) / 2;
            (x, y)
        }
        Edge::Top => {
            let x = work.x + (work.width - size.width) / 2;
            let y = if visible {
                work.y + offset
            } else {
                work.y - size.height
            };
            (x, y)
        }
        Edge::Bottom => {
            let x = work.x + (work.width - size.width) / 2;
            let y = if visible {
                work.y + work.height - size.height - offset
            } else {
                work.y + work.height
            };
            (x, y)
        }
    }
}

/// 计算面板展开后的完整矩形（用于命中测试）
pub fn panel_rect(
    edge: Edge,
    work: Rect,
    size: PanelSize,
    offset: i32,
    visible: bool,
) -> Rect {
    let (x, y) = compute_position(edge, work, size, offset, visible);
    Rect::new(x, y, size.width, size.height)
}

/// 判断鼠标是否在触发区内
///
/// 触发区位于屏幕边缘，宽度为 `trigger_width`。
/// 使用**工作区**而非全屏区域，这样任务栏在同侧时会自动让开。
pub fn is_in_trigger_zone(
    mx: i32,
    my: i32,
    edge: Edge,
    work: Rect,
    trigger_width: i32,
) -> bool {
    let t = trigger_width.max(1);

    match edge {
        Edge::Left => {
            mx >= work.x && mx < work.x + t && my >= work.y && my < work.bottom()
        }
        Edge::Right => {
            mx >= work.right() - t && mx < work.right() && my >= work.y && my < work.bottom()
        }
        Edge::Top => {
            my >= work.y && my < work.y + t && mx >= work.x && mx < work.right()
        }
        Edge::Bottom => {
            my >= work.bottom() - t && my < work.bottom() && mx >= work.x && mx < work.right()
        }
    }
}

/// 收纳面板矩形（含少量容差，避免边缘抖动导致误收起）
pub fn panel_rect_with_tolerance(
    edge: Edge,
    work: Rect,
    size: PanelSize,
    offset: i32,
    tolerance: i32,
) -> Rect {
    let r = panel_rect(edge, work, size, offset, true);
    Rect::new(
        r.x - tolerance,
        r.y - tolerance,
        r.width + tolerance * 2,
        r.height + tolerance * 2,
    )
}

/// 缓动函数：ease-out-cubic（先快后慢，适合面板滑出）
pub fn ease_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// 线性插值
pub fn lerp(from: i32, to: i32, t: f64) -> i32 {
    (from as f64 + (to - from) as f64 * t).round() as i32
}

/// 从 MonitorInfo 取工作区矩形
pub fn work_rect_of(mon: &MonitorInfo) -> Rect {
    Rect::new(mon.work_x, mon.work_y, mon.work_width, mon.work_height)
}

/// 全屏矩形（含任务栏区域）
pub fn full_rect_of(mon: &MonitorInfo) -> Rect {
    Rect::new(mon.x, mon.y, mon.width, mon.height)
}

/// 物理像素矩形 → 逻辑像素矩形
///
/// Win32 `GetMonitorInfo` 返回物理像素，而 Tauri 窗口 API 使用逻辑像素（DIP）。
/// 用同一个缩放比对 x/y 与宽高做换算，保证两套坐标系一致。
pub fn scale_rect(r: Rect, scale: f64) -> Rect {
    if (scale - 1.0).abs() < f64::EPSILON {
        return r;
    }
    Rect::new(
        (r.x as f64 / scale).round() as i32,
        (r.y as f64 / scale).round() as i32,
        (r.width as f64 / scale).round() as i32,
        (r.height as f64 / scale).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon() -> Rect {
        Rect::new(0, 0, 1920, 1040)
    }

    #[test]
    fn right_edge_visible() {
        let size = PanelSize::new(320, 720);
        let (x, y) = compute_position(Edge::Right, mon(), size, 0, true);
        assert_eq!(x, 1920 - 320);
        assert_eq!(y, (1040 - 720) / 2);
    }

    #[test]
    fn right_edge_hidden_fully_offscreen() {
        let size = PanelSize::new(320, 720);
        let (x, _) = compute_position(Edge::Right, mon(), size, 0, false);
        // 必须完全移出屏幕右侧
        assert_eq!(x, 1920);
    }

    #[test]
    fn left_edge_hidden_fully_offscreen() {
        let size = PanelSize::new(320, 720);
        let (x, _) = compute_position(Edge::Left, mon(), size, 0, false);
        assert_eq!(x, -320);
    }

    #[test]
    fn top_edge_hidden_fully_offscreen() {
        let size = PanelSize::new(720, 320);
        let (_, y) = compute_position(Edge::Top, mon(), size, 0, false);
        assert_eq!(y, -320);
    }

    #[test]
    fn bottom_edge_hidden_fully_offscreen() {
        let size = PanelSize::new(720, 320);
        let (_, y) = compute_position(Edge::Bottom, mon(), size, 0, false);
        assert_eq!(y, 1040);
    }

    #[test]
    fn negative_coordinate_monitor() {
        // 副屏在主屏左侧，坐标为负
        let work = Rect::new(-1920, 0, 1920, 1040);
        let size = PanelSize::new(320, 720);
        let (x, _) = compute_position(Edge::Left, work, size, 0, true);
        assert_eq!(x, -1920);
    }

    #[test]
    fn trigger_zone_right() {
        let w = mon();
        assert!(is_in_trigger_zone(1919, 500, Edge::Right, w, 3));
        assert!(is_in_trigger_zone(1917, 500, Edge::Right, w, 3));
        assert!(!is_in_trigger_zone(1916, 500, Edge::Right, w, 3));
        assert!(!is_in_trigger_zone(1919, 2000, Edge::Right, w, 3));
    }

    #[test]
    fn trigger_zone_left() {
        let w = mon();
        assert!(is_in_trigger_zone(0, 500, Edge::Left, w, 3));
        assert!(is_in_trigger_zone(2, 500, Edge::Left, w, 3));
        assert!(!is_in_trigger_zone(3, 500, Edge::Left, w, 3));
    }

    #[test]
    fn window_size_swaps_on_horizontal() {
        assert_eq!(Edge::Left.to_window_size(720, 320), (320, 720));
        assert_eq!(Edge::Top.to_window_size(720, 320), (720, 320));
    }

    #[test]
    fn ease_out_is_monotonic() {
        let mut prev = -1.0;
        for i in 0..=10 {
            let v = ease_out_cubic(i as f64 / 10.0);
            assert!(v >= prev);
            prev = v;
        }
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-9);
        assert!(ease_out_cubic(0.0).abs() < 1e-9);
    }
}
