//! 配置模块
//!
//! 数据结构定义见 docs/04-数据模型与持久化.md

pub mod schema;
pub mod store;

pub use schema::*;
pub use store::ConfigStore;
