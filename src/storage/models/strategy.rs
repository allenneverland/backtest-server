use chrono::{DateTime, Utc};
use sqlx::types::{Decimal, Json};
use sqlx::FromRow;
use serde::{Deserialize, Serialize};

/// 策略定義模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Strategy {
    pub strategy_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub parameters: Json<serde_json::Value>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 策略實例模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StrategyInstance {
    pub instance_id: i32,
    pub strategy_id: i32,
    pub name: String,
    pub parameters: Json<serde_json::Value>,
    pub active: bool,
    pub last_run_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 策略插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInsert {
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub parameters: Json<serde_json::Value>,
    pub active: bool,
}

/// 策略實例插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInstanceInsert {
    pub strategy_id: i32,
    pub name: String,
    pub parameters: Json<serde_json::Value>,
    pub active: bool,
}

/// 策略信號模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StrategySignal {
    pub time: DateTime<Utc>,
    pub instance_id: i32,
    pub instrument_id: i32,
    pub signal_type: String,
    pub price: Option<Decimal>,
    pub quantity: Option<Decimal>,
    pub reason: Option<String>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 策略信號插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySignalInsert {
    pub time: DateTime<Utc>,
    pub instance_id: i32,
    pub instrument_id: i32,
    pub signal_type: String,
    pub price: Option<Decimal>,
    pub quantity: Option<Decimal>,
    pub reason: Option<String>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 交易記錄模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Trade {
    pub trade_id: i32,
    pub time: DateTime<Utc>,
    pub instance_id: i32,
    pub instrument_id: i32,
    pub direction: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
    pub slippage: Option<Decimal>,
    pub contract_month: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub signal_id: Option<i32>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 交易記錄插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeInsert {
    pub time: DateTime<Utc>,
    pub instance_id: i32,
    pub instrument_id: i32,
    pub direction: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
    pub slippage: Option<Decimal>,
    pub contract_month: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub signal_id: Option<i32>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 策略性能聚合視圖模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StrategyPerformance {
    pub bucket: DateTime<Utc>,
    pub instance_id: i32,
    pub total_buy: Decimal,
    pub total_sell: Decimal,
    pub total_commission: Decimal,
    pub trade_count: i64,
} 

/// 策略配置結構
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbStrategyConfig {
    pub strategy_id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub parameters: serde_json::Value,
    pub code_path: Option<String>,
    pub enabled: bool,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub metadata: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}