// monitor.rs - 監控系統模組

pub mod metrics;
pub mod logger;
pub mod alerter;

// 重新導出常用元素，使其可直接從 monitor 模組使用
pub use metrics::{Metrics, MetricsCollector, MetricType};
pub use logger::{Logger, LogLevel, LogEntry};
pub use alerter::{Alerter, AlertConfig, AlertLevel, Alert};

/// 監控系統錯誤類型
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// 指標收集錯誤
    #[error("Metrics error: {0}")]
    MetricsError(String),
    
    /// 日誌記錄錯誤
    #[error("Logger error: {0}")]
    LoggerError(String),
    
    /// 警報系統錯誤
    #[error("Alerter error: {0}")]
    AlerterError(String),
    
    /// 初始化錯誤
    #[error("Initialization error: {0}")]
    InitializationError(String),
    
    /// 其他錯誤
    #[error("Monitor error: {0}")]
    Other(String),
}

/// 監控結果類型
pub type MonitorResult<T> = Result<T, MonitorError>; 