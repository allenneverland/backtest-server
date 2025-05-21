use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 策略版本模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StrategyVersion {
    pub version_id: i32,
    pub strategy_id: i32,
    pub version: String,
    pub source_path: String,
    pub description: Option<String>,
    pub is_stable: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 策略版本插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyVersionInsert {
    pub strategy_id: i32,
    pub version: String,
    pub source_path: String,
    pub description: Option<String>,
    pub is_stable: bool,
    pub created_by: String,
}
