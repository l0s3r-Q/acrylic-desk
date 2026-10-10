//! 统一错误类型
//!
//! 所有面向用户的错误信息使用中文描述。
//! 约定见 docs/05-API契约.md §5。

use std::fmt;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    /// 配置读写失败
    Config(String),
    /// 窗口操作失败
    Window(String),
    /// 文件/路径问题
    Io(String),
    /// 图标提取失败
    Icon(String),
    /// 参数非法
    Invalid(String),
    /// 功能未实现
    NotImplemented(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Config(m) => write!(f, "配置错误：{m}"),
            AppError::Window(m) => write!(f, "窗口操作失败：{m}"),
            AppError::Io(m) => write!(f, "文件操作失败：{m}"),
            AppError::Icon(m) => write!(f, "图标处理失败：{m}"),
            AppError::Invalid(m) => write!(f, "{m}"),
            AppError::NotImplemented(m) => write!(f, "功能尚未实现：{m}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Config(format!("JSON 解析失败：{e}"))
    }
}

/// 供 Tauri 命令直接使用：将 AppError 转为 String
impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}
