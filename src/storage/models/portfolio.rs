use chrono::{DateTime, Utc, NaiveDate};
use sqlx::types::{Decimal, Json};
use serde::{Deserialize, Serialize};

/// 投資組合模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Portfolio {
    pub portfolio_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub initial_capital: Decimal,
    pub currency: String,
    pub risk_tolerance: Option<i16>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub is_active: bool,
    pub strategy_instance: Option<Json<serde_json::Value>>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 投資組合插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioInsert {
    pub name: String,
    pub description: Option<String>,
    pub initial_capital: Decimal,
    pub currency: String,
    pub risk_tolerance: Option<i16>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub is_active: bool,
    pub strategy_instance: Option<Json<serde_json::Value>>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 投資組合持倉模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PortfolioHolding {
    pub time: DateTime<Utc>,
    pub portfolio_id: i32,
    pub instrument_id: i32,
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub market_value: Decimal,
    pub profit_loss: Option<Decimal>,
    pub allocation_percentage: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

/// 投資組合持倉插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioHoldingInsert {
    pub time: DateTime<Utc>,
    pub portfolio_id: i32,
    pub instrument_id: i32,
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub market_value: Decimal,
    pub profit_loss: Option<Decimal>,
    pub allocation_percentage: Option<Decimal>,
}

/// 投資組合表現聚合視圖模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PortfolioPerformance {
    pub bucket: DateTime<Utc>,
    pub portfolio_id: i32,
    pub total_value: Decimal,
    pub total_profit_loss: Decimal,
    pub avg_allocation: Option<Decimal>,
    pub instrument_count: i64,
} 