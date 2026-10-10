//! 边缘吸附与唤出（项目技术核心）
//!
//! 方案见 docs/03-窗体与亚克力技术方案.md：
//! - 60Hz 轮询 GetCursorPos（**不用全局钩子**，规避杀软误报）
//! - 四态状态机 Hidden → Showing → Visible → Hiding
//! - 动画期材质降级 Blur，静止时 Acrylic

pub mod geometry;
pub mod monitor;
pub mod state;

pub use geometry::*;
pub use state::EdgeManager;
