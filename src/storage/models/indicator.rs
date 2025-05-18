use chrono::{DateTime, Utc};
use sqlx::types::Json;
use serde::{Deserialize, Serialize};

/// 技術指標定義模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TechnicalIndicator {
    pub indicator_id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 技術指標定義插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicatorInsert {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Json<serde_json::Value>,
}

/// 商品日級指標數據模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InstrumentDailyIndicator {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub indicator_id: i32,
    pub parameters: Json<serde_json::Value>,
    pub values: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// 商品日級指標數據插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentDailyIndicatorInsert {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub indicator_id: i32,
    pub parameters: Json<serde_json::Value>,
    pub values: Json<serde_json::Value>,
} 