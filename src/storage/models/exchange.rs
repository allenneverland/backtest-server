use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 交易所模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Exchange {
    pub exchange_id: i32,
    pub code: String,
    pub name: String,
    pub country: String,
    pub timezone: String,
    pub operating_hours: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 交易所插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeInsert {
    pub code: String,
    pub name: String,
    pub country: String,
    pub timezone: String,
    pub operating_hours: Option<JsonValue>,
}
