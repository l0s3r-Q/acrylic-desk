//! 窗口材质控制
//!
//! 依据 docs/00-技术选型报告.md §4.1：
//! `WindowEffect::Acrylic` 是 tauri-utils 的官方枚举值。
//!
//! ⚠️ 已知问题（见 docs/03 §1.3）：
//! Acrylic 在窗口拖拽/动画期间性能较差，因此动画期降级为 Blur，
//! 动画结束后恢复。
//!
//! API 路径（已核对 tauri 2.12.2 源码）：
//! - `tauri::window::Effect`（枚举，re-export 自 tauri_utils::WindowEffect）
//! - `tauri::utils::config::WindowEffectsConfig`
//!
//! 注：`WindowEffectsConfig` 是 `#[non_exhaustive]` 风格的结构，
//! 构造时统一使用 `..Default::default()`，避免 Tauri 新增字段时破坏编译。

use tauri::utils::config::WindowEffectsConfig;
use tauri::window::Effect;
use tauri::WebviewWindow;

use crate::config::schema::EffectKind;
use crate::error::{AppError, Result};

/// 将配置的材质类型转换为 Tauri Effect
///
/// 返回 `None` 表示不使用系统材质（纯色模式）。
pub fn to_window_effect(kind: EffectKind) -> Option<Effect> {
    match kind {
        EffectKind::Acrylic => Some(Effect::Acrylic),
        EffectKind::Mica => Some(Effect::Mica),
        EffectKind::Blur => Some(Effect::Blur),
        // 纯色：不应用任何系统材质，由前端 CSS 绘制背景
        EffectKind::Solid => None,
    }
}

/// 构造 effects 配置
fn build_config(effects: Vec<Effect>, radius: Option<f64>) -> WindowEffectsConfig {
    WindowEffectsConfig {
        effects,
        radius,
        ..Default::default()
    }
}

/// 应用材质
pub fn apply_effect(window: &WebviewWindow, kind: EffectKind) -> Result<()> {
    let effects = match to_window_effect(kind) {
        // 纯色模式：清空 effects 列表
        None => vec![],
        Some(effect) => vec![effect],
    };

    window
        .set_effects(build_config(effects, None))
        .map_err(|e| AppError::Window(format!("材质应用失败：{e}")))
}

/// 应用材质 + 圆角
///
/// 注意：`radius` 字段官方标注为 macOS Only。
/// Windows 上面板的可见圆角实际由前端 CSS `border-radius` 呈现，
/// 这里仍然传递以便在支持的平台上生效。
pub fn apply_radius(window: &WebviewWindow, kind: EffectKind, radius: f64) -> Result<()> {
    let effects = match to_window_effect(kind) {
        None => vec![],
        Some(effect) => vec![effect],
    };

    window
        .set_effects(build_config(effects, Some(radius)))
        .map_err(|e| AppError::Window(format!("材质应用失败：{e}")))
}

/// 动画期材质降级 / 恢复
///
/// 见 docs/03 §1.3：Acrylic 在窗口移动时性能较差，
/// 动画期间改用 Blur，动画结束后恢复用户设置的材质。
pub fn set_performance_mode(
    window: &WebviewWindow,
    performance: bool,
    restore_to: EffectKind,
) -> Result<()> {
    if performance {
        apply_effect(window, EffectKind::Blur)
    } else {
        apply_effect(window, restore_to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_mapping() {
        assert!(matches!(
            to_window_effect(EffectKind::Acrylic),
            Some(Effect::Acrylic)
        ));
        assert!(matches!(
            to_window_effect(EffectKind::Mica),
            Some(Effect::Mica)
        ));
        assert!(matches!(
            to_window_effect(EffectKind::Blur),
            Some(Effect::Blur)
        ));
        assert!(to_window_effect(EffectKind::Solid).is_none());
    }
}
