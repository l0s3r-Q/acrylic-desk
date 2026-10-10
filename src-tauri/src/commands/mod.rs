//! Tauri 命令层
//!
//! 契约见 docs/05-API契约.md。
//! 所有命令返回 `Result<T, String>`，错误信息为中文。

pub mod config;
pub mod icons;
pub mod panel;
pub mod system;
