//! 边缘显隐状态机与监测线程
//!
//! 状态转换见 docs/03-窗体与亚克力技术方案.md §3.3：
//!
//! ```text
//!   Hidden ──鼠标进触发区──▶ Showing ──动画结束──▶ Visible
//!     ▲                                              │
//!     └──动画结束── Hiding ◀──鼠标离开+延时到期──────┘
//! ```
//!
//! **安全红线**：使用 60Hz 轮询而非全局鼠标钩子，
//! 避免被国产杀软判定为远控行为。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};

use crate::config::schema::{Edge, EdgeState};
use crate::config::ConfigStore;
use crate::edge::geometry::{
    compute_position, ease_out_cubic, is_in_trigger_zone, lerp, panel_rect,
    panel_rect_with_tolerance, work_rect_of, PanelSize,
};
use crate::edge::monitor::{cursor_pos, enumerate_monitors, is_fullscreen_foreground};
use crate::error::{AppError, Result};

/// 轮询间隔（~60Hz）
const POLL_INTERVAL: Duration = Duration::from_millis(16);
/// 动画帧间隔
const FRAME_INTERVAL: Duration = Duration::from_millis(8);
/// 面板矩形命中测试容差 (px)
const HIT_TOLERANCE: i32 = 2;
/// 启动宽限期：应用启动后这段时间内不自动收起面板
///
/// 目的：让用户首次启动时能看到面板出现在哪里，
/// 否则鼠标不在面板内会立刻被收起，用户会以为程序没运行。
const STARTUP_GRACE: Duration = Duration::from_secs(4);

/// 边缘管理器
pub struct EdgeManager {
    app: AppHandle,
    config: Arc<ConfigStore>,

    /// 当前状态
    state: Arc<Mutex<EdgeState>>,
    /// 是否正在拖拽（拖拽期间禁止收起）
    dragging: Arc<AtomicBool>,
    /// 收起计划时间；None 表示无计划
    hide_deadline: Arc<Mutex<Option<Instant>>>,
    /// 是否已启动监测
    started: Arc<AtomicBool>,
    /// 监测开始时间（用于启动宽限期）
    started_at: Arc<Mutex<Option<Instant>>>,
    /// 调试输出节流
    #[cfg(debug_assertions)]
    last_debug: Arc<Mutex<Instant>>,
}

impl EdgeManager {
    pub fn new(app: AppHandle, config: Arc<ConfigStore>) -> Self {
        Self {
            app,
            config,
            state: Arc::new(Mutex::new(EdgeState::Visible)),
            dragging: Arc::new(AtomicBool::new(false)),
            hide_deadline: Arc::new(Mutex::new(None)),
            started: Arc::new(AtomicBool::new(false)),
            started_at: Arc::new(Mutex::new(None)),
            #[cfg(debug_assertions)]
            last_debug: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// 当前状态
    pub fn state(&self) -> EdgeState {
        *self.state.lock().unwrap()
    }

    fn set_state(&self, s: EdgeState) {
        *self.state.lock().unwrap() = s;
    }

    /// 设置拖拽标志（前端在拖拽开始/结束时调用）
    pub fn set_dragging(&self, dragging: bool) {
        self.dragging.store(dragging, Ordering::SeqCst);
        if dragging {
            // 拖拽期间取消任何收起计划
            *self.hide_deadline.lock().unwrap() = None;
        }
    }

    /// 取面板窗口
    fn panel(&self) -> Result<WebviewWindow> {
        self.app
            .get_webview_window("panel")
            .ok_or_else(|| AppError::Window("找不到面板窗口".into()))
    }

    /// 计算当前面板尺寸（物理像素）
    fn panel_size(&self) -> PanelSize {
        let cfg = self.config.get();
        PanelSize::from_edge(cfg.panel.edge, cfg.panel.length, cfg.panel.thickness)
    }

    /// 取挂载显示器的工作区
    ///
    /// ⚠️ DPI 说明（见 docs/03 §2.1）：
    /// Win32 `GetMonitorInfo` 返回的是**物理像素**，
    /// 而 Tauri 的窗口 API 使用**逻辑像素**（DIP）。
    /// 这里把物理工作区换算为逻辑工作区，保证与 Tauri 坐标系一致。
    fn work_rect(&self) -> Result<crate::edge::geometry::Rect> {
        let cfg = self.config.get();
        let monitors = enumerate_monitors()?;
        let mon = crate::edge::monitor::monitor_at(&monitors, cfg.panel.monitor_index)
            .ok_or_else(|| AppError::Window("找不到目标显示器".into()))?;

        // 取窗口当前缩放比
        let scale = self
            .app
            .get_webview_window("panel")
            .and_then(|w| w.scale_factor().ok())
            .unwrap_or(1.0);

        Ok(crate::edge::geometry::scale_rect(
            work_rect_of(mon),
            scale,
        ))
    }

    /// 取鼠标位置（逻辑像素，与 Tauri 坐标系一致）
    fn cursor_pos_logical(&self) -> (i32, i32) {
        let (px, py) = cursor_pos();
        if px < 0 || py < 0 {
            return (px, py);
        }

        let scale = self
            .app
            .get_webview_window("panel")
            .and_then(|w| w.scale_factor().ok())
            .unwrap_or(1.0);

        (
            (px as f64 / scale).round() as i32,
            (py as f64 / scale).round() as i32,
        )
    }

    /// 应用初始布局（启动时调用）
    pub fn apply_initial_layout(&self, window: &WebviewWindow) -> Result<()> {
        let cfg = self.config.get();
        let size = PanelSize::from_edge(cfg.panel.edge, cfg.panel.length, cfg.panel.thickness);

        // 尺寸（逻辑像素，与 Tauri 坐标系一致）
        window
            .set_size(LogicalSize::new(size.width as f64, size.height as f64))
            .map_err(|e| AppError::Window(e.to_string()))?;

        // 位置
        let work = self.work_rect()?;
        let (x, y) = compute_position(cfg.panel.edge, work, size, cfg.panel.offset, true);
        window
            .set_position(LogicalPosition::new(x as f64, y as f64))
            .map_err(|e| AppError::Window(e.to_string()))?;

        // 置顶
        let _ = window.set_always_on_top(cfg.panel.always_on_top);

        // 材质
        crate::window::effect::apply_effect(window, cfg.appearance.effect)?;

        // 首次启动保持可见，避免用户找不到面板
        let _ = window.set_ignore_cursor_events(false);
        self.set_state(EdgeState::Visible);

        Ok(())
    }

    /// 启动边缘监测线程
    pub fn start_monitor(self: &Arc<Self>) {
        if self.started.swap(true, Ordering::SeqCst) {
            return; // 已在运行
        }

        // 记录启动时刻，用于宽限期判定
        *self.started_at.lock().unwrap() = Some(Instant::now());

        let me = self.clone();
        std::thread::spawn(move || {
            me.monitor_loop();
        });
    }

    /// 监测主循环
    fn monitor_loop(&self) {
        loop {
            std::thread::sleep(POLL_INTERVAL);

            let cfg = self.config.get();
            let edge_cfg = cfg.edge.clone();

            // 常驻模式：不做自动收起
            if edge_cfg.always_visible || !edge_cfg.enable_hover {
                // 仍需保证面板位置正确
                continue;
            }

            // 全屏时禁用
            if edge_cfg.disable_on_fullscreen && is_fullscreen_foreground() {
                if self.state().is_visible() {
                    let _ = self.hide_now();
                }
                continue;
            }

            // 启动宽限期：应用刚启动时保持面板可见，
            // 让用户能看到面板在哪，之后再进入自动收起逻辑。
            if let Some(t0) = *self.started_at.lock().unwrap() {
                if t0.elapsed() < STARTUP_GRACE {
                    continue;
                }
            }

            let (mx, my) = self.cursor_pos_logical();
            if mx < 0 || my < 0 {
                continue; // 取不到鼠标位置，跳过本轮
            }

            let work = match self.work_rect() {
                Ok(w) => w,
                Err(_) => continue,
            };

            let size = self.panel_size();
            let edge = cfg.panel.edge;

            let state = self.state();
            let in_trigger = is_in_trigger_zone(
                mx,
                my,
                edge,
                work,
                edge_cfg.trigger_width,
            );
            let in_panel = panel_rect_with_tolerance(
                edge,
                work,
                size,
                cfg.panel.offset,
                HIT_TOLERANCE,
            )
            .contains(mx, my);

            match state {
                EdgeState::Hidden => {
                    if in_trigger {
                        let _ = self.summon();
                    }
                }
                EdgeState::Visible => {
                    if self.dragging.load(Ordering::SeqCst) {
                        *self.hide_deadline.lock().unwrap() = None;
                        continue;
                    }

                    if in_panel {
                        // 鼠标在面板内，取消收起计划
                        *self.hide_deadline.lock().unwrap() = None;
                    } else if in_trigger {
                        // 鼠标回到触发区，也取消
                        *self.hide_deadline.lock().unwrap() = None;
                    } else {
                        // 启动收起倒计时
                        let mut dl = self.hide_deadline.lock().unwrap();
                        if dl.is_none() {
                            *dl = Some(
                                Instant::now()
                                    + Duration::from_millis(edge_cfg.hide_delay),
                            );
                        } else if let Some(deadline) = *dl {
                            if Instant::now() >= deadline {
                                *dl = None;
                                drop(dl);
                                let _ = self.hide_now();
                            }
                        }
                    }
                }
                EdgeState::Showing | EdgeState::Hiding => {
                    // 动画中，不做判定（由动画线程负责推进状态）
                }
            }

            // 调试输出（仅 debug 构建，且带节流避免刷屏）
            #[cfg(debug_assertions)]
            {
                let now = Instant::now();
                let mut last = self.last_debug.lock().unwrap();
                if now.duration_since(*last) >= Duration::from_millis(1000) {
                    *last = now;
                    let rect = panel_rect_with_tolerance(
                        edge,
                        work,
                        size,
                        cfg.panel.offset,
                        HIT_TOLERANCE,
                    );
                    println!(
                        "[edge] state={:?} mouse=({mx},{my}) trigger={in_trigger} inPanel={in_panel} \
                         panelRect=({},{},{},{}) work=({},{},{},{})",
                        self.state(),
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        work.x,
                        work.y,
                        work.width,
                        work.height
                    );
                }
            }
        }
    }

    /// 唤出面板
    pub fn summon(&self) -> Result<()> {
        let state = self.state();
        if state == EdgeState::Visible || state == EdgeState::Showing {
            return Ok(());
        }

        self.set_state(EdgeState::Showing);
        *self.hide_deadline.lock().unwrap() = None;

        let window = self.panel()?;
        let cfg = self.config.get();

        // 立即接管鼠标
        let _ = window.set_ignore_cursor_events(false);

        // 动画期材质降级，避免 Acrylic 拖拽卡顿
        let _ = crate::window::effect::apply_effect(
            &window,
            crate::config::schema::EffectKind::Blur,
        );

        let _ = self.app.emit("edge://summon", cfg.panel.edge);

        self.animate_to(&window, true)?;

        self.set_state(EdgeState::Visible);

        // 恢复高质量材质
        let _ = crate::window::effect::apply_effect(&window, cfg.appearance.effect);

        let _ = self.app.emit("edge://shown", ());
        Ok(())
    }

    /// 收起面板
    pub fn hide_now(&self) -> Result<()> {
        let state = self.state();
        if state == EdgeState::Hidden || state == EdgeState::Hiding {
            return Ok(());
        }

        self.set_state(EdgeState::Hiding);
        self.app
            .emit("edge://hide", ())
            .map_err(|e| AppError::Window(e.to_string()))?;

        let window = self.panel()?;
        let cfg = self.config.get();

        // 动画期材质降级
        let _ = crate::window::effect::apply_effect(
            &window,
            crate::config::schema::EffectKind::Blur,
        );

        self.animate_to(&window, false)?;

        self.set_state(EdgeState::Hidden);

        // 完全收起后开启穿透，不影响桌面操作
        let _ = window.set_ignore_cursor_events(true);

        // 恢复材质（隐藏状态下无所谓，但保持状态一致）
        let _ = crate::window::effect::apply_effect(&window, cfg.appearance.effect);

        let _ = self.app.emit("edge://hidden", ());
        Ok(())
    }

    /// 强制切换
    pub fn toggle(&self) -> Result<()> {
        if self.state().is_visible() {
            self.hide_now()
        } else {
            self.summon()
        }
    }

    /// 窗口位移动画
    fn animate_to(&self, window: &WebviewWindow, visible: bool) -> Result<()> {
        let cfg = self.config.get();
        let work = self.work_rect()?;
        let size = self.panel_size();
        let edge = cfg.panel.edge;

        let (from_x, from_y) = if visible {
            compute_position(edge, work, size, cfg.panel.offset, false)
        } else {
            compute_position(edge, work, size, cfg.panel.offset, true)
        };
        let (to_x, to_y) = compute_position(edge, work, size, cfg.panel.offset, visible);

        let duration = cfg.edge.animation_duration;

        // duration = 0 表示不做动画，直接跳转
        if duration == 0 {
            let _ = window.set_position(LogicalPosition::new(to_x as f64, to_y as f64));
            return Ok(());
        }

        let steps = (duration / FRAME_INTERVAL.as_millis() as u64).max(1);

        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let eased = ease_out_cubic(t);
            let x = lerp(from_x, to_x, eased);
            let y = lerp(from_y, to_y, eased);

            if window
                .set_position(LogicalPosition::new(x as f64, y as f64))
                .is_err()
            {
                break; // 窗口可能已销毁
            }

            std::thread::sleep(FRAME_INTERVAL);
        }

        // 确保精确落位
        let _ = window.set_position(LogicalPosition::new(to_x as f64, to_y as f64));

        Ok(())
    }

    /// 应用挂载边变更（含宽高交换与重定位）
    pub fn apply_edge_change(&self) -> Result<()> {
        let window = self.panel()?;
        let cfg = self.config.get();
        let size = PanelSize::from_edge(cfg.panel.edge, cfg.panel.length, cfg.panel.thickness);

        window
            .set_size(LogicalSize::new(size.width as f64, size.height as f64))
            .map_err(|e| AppError::Window(e.to_string()))?;

        let work = self.work_rect()?;
        let visible = self.state().is_visible();
        let (x, y) = compute_position(cfg.panel.edge, work, size, cfg.panel.offset, visible);
        window
            .set_position(LogicalPosition::new(x as f64, y as f64))
            .map_err(|e| AppError::Window(e.to_string()))?;

        Ok(())
    }

    /// 应用尺寸变更
    pub fn apply_size_change(&self) -> Result<()> {
        self.apply_edge_change()
    }

    /// 设置鼠标穿透
    pub fn set_passthrough(&self, enabled: bool) -> Result<()> {
        let window = self.panel()?;
        window
            .set_ignore_cursor_events(enabled)
            .map_err(|e| AppError::Window(e.to_string()))?;
        Ok(())
    }

    /// 当前面板矩形（供调试/测试）
    pub fn current_rect(&self) -> Result<crate::edge::geometry::Rect> {
        let cfg = self.config.get();
        let work = self.work_rect()?;
        let size = self.panel_size();
        Ok(panel_rect(
            cfg.panel.edge,
            work,
            size,
            cfg.panel.offset,
            self.state().is_visible(),
        ))
    }

    /// 边缘配置引用（供命令层使用）
    pub fn config_store(&self) -> &Arc<ConfigStore> {
        &self.config
    }
}

/// 便捷：从 Edge 与状态推出目标位置（供测试使用）
pub fn target_position(
    edge: Edge,
    work: crate::edge::geometry::Rect,
    size: PanelSize,
    offset: i32,
    visible: bool,
) -> (i32, i32) {
    compute_position(edge, work, size, offset, visible)
}
