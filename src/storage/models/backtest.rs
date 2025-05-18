use chrono::{DateTime, Utc};
use sqlx::types::{Decimal, Json};
use serde::{Deserialize, Serialize};

/// 回測配置模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BacktestConfig {
    pub config_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub currency: String,
    pub instruments: Vec<i32>,
    pub strategy_id: i32,
    pub execution_settings: Json<serde_json::Value>,
    pub risk_settings: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 回測配置插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfigInsert {
    pub name: String,
    pub description: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub currency: String,
    pub instruments: Vec<i32>,
    pub strategy_id: i32,
    pub execution_settings: Json<serde_json::Value>,
    pub risk_settings: Json<serde_json::Value>,
}

/// 回測結果模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BacktestResult {
    pub result_id: i32,
    pub config_id: i32,
    pub status: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub execution_time: Option<i32>,
    pub metrics: Json<serde_json::Value>,
    pub benchmark_comparison: Option<Json<serde_json::Value>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 回測結果插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResultInsert {
    pub config_id: i32,
    pub status: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub execution_time: Option<i32>,
    pub metrics: Json<serde_json::Value>,
    pub benchmark_comparison: Option<Json<serde_json::Value>>,
    pub error_message: Option<String>,
}

/// 回測交易記錄模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BacktestTrade {
    pub time: DateTime<Utc>,
    pub result_id: i32,
    pub instrument_id: i32,
    pub direction: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
    pub slippage: Option<Decimal>,
    pub trade_id: Option<String>,
    pub position_effect: Option<String>,
    pub order_type: Option<String>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 回測交易記錄插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTradeInsert {
    pub time: DateTime<Utc>,
    pub result_id: i32,
    pub instrument_id: i32,
    pub direction: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
    pub slippage: Option<Decimal>,
    pub trade_id: Option<String>,
    pub position_effect: Option<String>,
    pub order_type: Option<String>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 回測倉位快照模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BacktestPositionSnapshot {
    pub time: DateTime<Utc>,
    pub result_id: i32,
    pub instrument_id: i32,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub unrealized_pl: Decimal,
    pub realized_pl: Decimal,
    pub margin_used: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

/// 回測倉位快照插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestPositionSnapshotInsert {
    pub time: DateTime<Utc>,
    pub result_id: i32,
    pub instrument_id: i32,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub unrealized_pl: Decimal,
    pub realized_pl: Decimal,
    pub margin_used: Option<Decimal>,
}

/// 回測投資組合快照模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BacktestPortfolioSnapshot {
    pub time: DateTime<Utc>,
    pub result_id: i32,
    pub total_value: Decimal,
    pub cash: Decimal,
    pub equity: Decimal,
    pub margin: Option<Decimal>,
    pub daily_pnl: Option<Decimal>,
    pub total_pnl: Option<Decimal>,
    pub daily_return: Option<Decimal>,
    pub total_return: Option<Decimal>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 回測投資組合快照插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestPortfolioSnapshotInsert {
    pub time: DateTime<Utc>,
    pub result_id: i32,
    pub total_value: Decimal,
    pub cash: Decimal,
    pub equity: Decimal,
    pub margin: Option<Decimal>,
    pub daily_pnl: Option<Decimal>,
    pub total_pnl: Option<Decimal>,
    pub daily_return: Option<Decimal>,
    pub total_return: Option<Decimal>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 回測日收益率聚合模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BacktestDailyReturns {
    pub bucket: DateTime<Utc>,
    pub result_id: i32,
    pub daily_return: Decimal,
    pub end_of_day_value: Decimal,
    pub end_of_day_equity: Decimal,
} 