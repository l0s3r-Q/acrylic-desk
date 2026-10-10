//! 配置数据结构定义
//!
//! 与 docs/04-数据模型与持久化.md §3 严格对应。
//! 前端 TS 类型见 src/types/index.ts，字段名保持一致（camelCase）。

use serde::{Deserialize, Serialize};

/// 当前配置版本，用于迁移
pub const CURRENT_VERSION: u32 = 1;

/// 面板挂载边
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Default for Edge {
    fn default() -> Self {
        Edge::Right
    }
}

impl Edge {
    /// 是否为水平挂载（上/下）
    ///
    /// 水平挂载时：窗口宽 = 长度，窗口高 = 厚度
    /// 垂直挂载时：窗口宽 = 厚度，窗口高 = 长度
    pub fn is_horizontal(self) -> bool {
        matches!(self, Edge::Top | Edge::Bottom)
    }

    /// 将 (长度, 厚度) 转换为窗口的 (宽, 高)
    pub fn to_window_size(self, length: u32, thickness: u32) -> (u32, u32) {
        if self.is_horizontal() {
            (length, thickness)
        } else {
            (thickness, length)
        }
    }

    /// 从字符串解析（前端传入）
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "top" => Ok(Edge::Top),
            "bottom" => Ok(Edge::Bottom),
            "left" => Ok(Edge::Left),
            "right" => Ok(Edge::Right),
            other => Err(format!("无效的挂载边：{other}")),
        }
    }
}

/// 材质类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind {
    Acrylic,
    Mica,
    Blur,
    Solid,
}

impl Default for EffectKind {
    fn default() -> Self {
        EffectKind::Acrylic
    }
}

/// 主题
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
    System,
    Light,
    Dark,
}

impl Default for ThemeKind {
    fn default() -> Self {
        ThemeKind::System
    }
}

/// 背景类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BgType {
    None,
    Image,
    Color,
}

impl Default for BgType {
    fn default() -> Self {
        BgType::None
    }
}

/// 背景图片适配方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BgFit {
    Cover,
    Contain,
    Fill,
    Tile,
    Center,
}

impl Default for BgFit {
    fn default() -> Self {
        BgFit::Cover
    }
}

/// 图标项类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    /// 可执行程序
    App,
    /// 文件夹
    Folder,
    /// 普通文件
    File,
    /// 网址
    Url,
}

/// 面板配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelConfig {
    pub edge: Edge,
    pub monitor_index: usize,
    /// 沿边缘方向的长度 (px)
    pub length: u32,
    /// 垂直边缘方向的厚度 (px)
    pub thickness: u32,
    /// 距屏幕边缘的间距 (px)
    pub offset: i32,
    /// 圆角半径 (px)
    pub corner_radius: f64,
    pub always_on_top: bool,
    pub cover_taskbar: bool,
    pub auto_size: bool,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            edge: Edge::Right,
            monitor_index: 0,
            length: 720,
            thickness: 320,
            offset: 0,
            corner_radius: 12.0,
            always_on_top: true,
            cover_taskbar: false,
            auto_size: false,
        }
    }
}

/// 边缘行为配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeConfig {
    /// 触发区宽度 (px)
    pub trigger_width: i32,
    /// 鼠标离开到收起的延时 (ms)
    pub hide_delay: u64,
    /// 滑入/滑出动画时长 (ms)
    pub animation_duration: u64,
    /// 是否启用碰边唤出
    pub enable_hover: bool,
    /// 全屏应用时是否禁用
    pub disable_on_fullscreen: bool,
    /// 全局唤出热键
    pub summon_hotkey: String,
    /// 是否启用全局热键
    #[serde(default = "default_true")]
    pub hotkey_enabled: bool,
    /// 常驻显示（不自动收起）
    pub always_visible: bool,
}

fn default_true() -> bool {
    true
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            trigger_width: 3,
            hide_delay: 400,
            animation_duration: 220,
            enable_hover: true,
            disable_on_fullscreen: true,
            summon_hotkey: "Alt+`".into(),
            hotkey_enabled: true,
            always_visible: false,
        }
    }
}

/// 背景配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundConfig {
    pub r#type: BgType,
    pub image_path: Option<String>,
    pub fit: BgFit,
    /// 背景暗淡遮罩 0.0–1.0
    pub dim: f64,
    pub color: String,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            r#type: BgType::None,
            image_path: None,
            fit: BgFit::Cover,
            dim: 0.3,
            color: "#1c1c1e".into(),
        }
    }
}

/// 图标样式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconStyleConfig {
    /// 图标尺寸 (px)
    pub size: u32,
    pub show_label: bool,
    /// 标签行数 1 或 2
    pub label_lines: u8,
    /// 图标间距
    pub gap: u32,
    /// 网格列数，0 = 自动
    pub grid_columns: u32,
}

impl Default for IconStyleConfig {
    fn default() -> Self {
        Self {
            size: 48,
            show_label: true,
            label_lines: 1,
            gap: 12,
            grid_columns: 0,
        }
    }
}

/// 外观配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceConfig {
    pub effect: EffectKind,
    /// 整体不透明度 0.3–1.0
    pub opacity: f64,
    pub theme: ThemeKind,
    pub accent_color: String,
    pub background: BackgroundConfig,
    pub icons: IconStyleConfig,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            effect: EffectKind::Acrylic,
            opacity: 0.85,
            theme: ThemeKind::System,
            accent_color: "#0a84ff".into(),
            background: BackgroundConfig::default(),
            icons: IconStyleConfig::default(),
        }
    }
}

/// 图标项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconItem {
    pub id: String,
    pub r#type: ItemType,
    pub name: String,
    /// 目标路径（exe 路径 / 文件夹路径 / 文件路径 / URL）
    pub target: String,
    pub args: String,
    pub work_dir: String,
    /// 图标缓存文件相对路径，如 "icons/a3f2b1c8.png"
    pub icon_path: Option<String>,
    /// 原始快捷方式路径，用于重新提取图标
    pub source_path: Option<String>,
    pub run_as_admin: bool,
    pub order: i32,
    pub added_at: String,
}

/// 分组
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    /// emoji 或图片路径
    pub icon: String,
    pub order: i32,
    pub items: Vec<IconItem>,
}

/// 系统配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfig {
    pub auto_start: bool,
    pub start_minimized: bool,
    pub language: String,
    pub check_update: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            auto_start: false,
            start_minimized: true,
            language: "zh-CN".into(),
            check_update: true,
        }
    }
}

/// 主配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub version: u32,
    pub panel: PanelConfig,
    pub edge: EdgeConfig,
    pub appearance: AppearanceConfig,
    pub groups: Vec<Group>,
    pub active_group_id: Option<String>,
    pub system: SystemConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            panel: PanelConfig::default(),
            edge: EdgeConfig::default(),
            appearance: AppearanceConfig::default(),
            groups: vec![Group {
                id: "g_default".into(),
                name: "默认".into(),
                icon: "📁".into(),
                order: 0,
                items: vec![],
            }],
            active_group_id: Some("g_default".into()),
            system: SystemConfig::default(),
        }
    }
}

/// 显示器信息（供前端展示与选择）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// 工作区（排除任务栏）
    pub work_x: i32,
    pub work_y: i32,
    pub work_width: i32,
    pub work_height: i32,
    pub is_primary: bool,
    pub scale_factor: f64,
}

/// 面板显隐状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeState {
    /// 已收起（屏幕外，鼠标穿透）
    Hidden,
    /// 滑出动画中
    Showing,
    /// 完全展开（可交互）
    Visible,
    /// 滑入动画中
    Hiding,
}

impl EdgeState {
    pub fn is_visible(&self) -> bool {
        matches!(self, EdgeState::Visible | EdgeState::Showing)
    }
}

/// 应用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub tauri_version: String,
    pub os_version: String,
    pub data_dir: String,
}
