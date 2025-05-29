use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

/// 執行日誌模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionLog {
    pub log_id: i64,
    pub run_id: i32,
    pub timestamp: DateTime<Utc>,
    pub log_level: String,
    pub component: Option<String>,
    pub message: String,
    pub details: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 執行日誌插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogInsert {
    pub run_id: i32,
    pub timestamp: Option<DateTime<Utc>>,
    pub log_level: String,
    pub component: Option<String>,
    pub message: String,
    pub details: Option<Json<serde_json::Value>>,
}

/// 日誌級別枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    #[serde(rename = "DEBUG")]
    Debug,
    #[serde(rename = "INFO")]
    Info,
    #[serde(rename = "WARN")]
    Warn,
    #[serde(rename = "ERROR")]
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

impl From<String> for LogLevel {
    fn from(s: String) -> Self {
        match s.to_uppercase().as_str() {
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

/// 執行日誌查詢過濾器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionLogFilter {
    pub run_id: Option<i32>,
    pub log_levels: Option<Vec<String>>,
    pub component: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");

        assert_eq!(LogLevel::from("DEBUG"), LogLevel::Debug);
        assert_eq!(LogLevel::from("info"), LogLevel::Info);
        assert_eq!(LogLevel::from("WARN"), LogLevel::Warn);
        assert_eq!(LogLevel::from("error"), LogLevel::Error);
    }
}
