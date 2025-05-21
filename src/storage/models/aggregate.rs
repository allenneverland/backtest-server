use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Decimal;

/// 日級成交量聚合視圖模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DailyVolumeByInstrument {
    pub bucket: DateTime<Utc>,
    pub instrument_id: i32,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub total_volume: Decimal,
    pub total_amount: Option<Decimal>,
    pub max_open_interest: Option<Decimal>,
}

/// 回測日收益率聚合視圖模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BacktestDailyReturns {
    pub bucket: DateTime<Utc>,
    pub result_id: i32,
    pub daily_return: Decimal,
    pub end_of_day_value: Decimal,
    pub end_of_day_equity: Decimal,
}
