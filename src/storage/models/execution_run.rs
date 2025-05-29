use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

/// 執行任務記錄模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionRun {
    pub run_id: i32,
    pub external_backtest_id: i32,
    pub request_id: Uuid,
    pub strategy_dsl: String,
    pub parameters: Json<serde_json::Value>,
    pub status: String,
    pub progress: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub error_details: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 執行任務插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRunInsert {
    pub external_backtest_id: i32,
    pub request_id: Uuid,
    pub strategy_dsl: String,
    pub parameters: Json<serde_json::Value>,
    pub status: Option<String>,
    pub progress: Option<i32>,
}

/// 執行任務更新模型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionRunUpdate {
    pub status: Option<String>,
    pub progress: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub error_details: Option<Json<serde_json::Value>>,
}

/// 執行狀態枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    #[serde(rename = "INITIALIZING")]
    Initializing,
    #[serde(rename = "RUNNING")]
    Running,
    #[serde(rename = "COMPLETED")]
    Completed,
    #[serde(rename = "FAILED")]
    Failed,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Initializing => "INITIALIZING",
            ExecutionStatus::Running => "RUNNING",
            ExecutionStatus::Completed => "COMPLETED",
            ExecutionStatus::Failed => "FAILED",
        }
    }
}

impl From<String> for ExecutionStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "INITIALIZING" => ExecutionStatus::Initializing,
            "RUNNING" => ExecutionStatus::Running,
            "COMPLETED" => ExecutionStatus::Completed,
            "FAILED" => ExecutionStatus::Failed,
            _ => ExecutionStatus::Initializing,
        }
    }
}

impl From<&str> for ExecutionStatus {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status_conversion() {
        assert_eq!(ExecutionStatus::Initializing.as_str(), "INITIALIZING");
        assert_eq!(ExecutionStatus::Running.as_str(), "RUNNING");
        assert_eq!(ExecutionStatus::Completed.as_str(), "COMPLETED");
        assert_eq!(ExecutionStatus::Failed.as_str(), "FAILED");

        assert_eq!(
            ExecutionStatus::from("INITIALIZING"),
            ExecutionStatus::Initializing
        );
        assert_eq!(ExecutionStatus::from("RUNNING"), ExecutionStatus::Running);
    }
}