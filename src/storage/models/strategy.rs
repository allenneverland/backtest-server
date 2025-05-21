use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;

/// 策略定義模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Strategy {
    pub strategy_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub code: String,
    pub code_path: Option<String>,
    pub parameters: Json<serde_json::Value>,
    pub active: bool,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub metadata: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 策略插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInsert {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub code: String,
    pub code_path: Option<String>,
    pub parameters: Json<serde_json::Value>,
    pub active: bool,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub metadata: Json<serde_json::Value>,
}
