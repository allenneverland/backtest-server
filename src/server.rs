// server.rs - 伺服器核心組件，宣告子模組
//
// 伺服器模組提供HTTP服務和管理系統生命週期，實現：
// - HTTP服務器實現與路由管理
// - 伺服器生命週期管理與優雅啟停
// - 請求處理邏輯與中間件
// - 構建器模式的伺服器配置

/// 伺服器構建器實現
pub mod builder;
/// 伺服器級別錯誤處理
pub mod error;

// 重新導出核心組件，簡化外部使用
pub use crate::config::types::ServerConfig;
pub use builder::ServerBuilder;
pub use error::{ServerError, ServerResult};

/// 伺服器狀態枚舉
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServerState {
    /// 伺服器正在初始化
    Initializing,
    /// 伺服器正在運行
    Running,
    /// 伺服器正在關閉
    ShuttingDown,
    /// 伺服器已停止
    Stopped,
}
