use chrono::{DateTime, Utc, NaiveDate};
use sqlx::types::{Decimal, Json};
use serde::{Deserialize, Serialize};

/// 分鐘K線數據結構
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MinuteBar {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub amount: Option<Decimal>,
    pub open_interest: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

/// 分鐘K線數據插入模型 (不包含自動生成的字段)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinuteBarInsert {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub amount: Option<Decimal>,
    pub open_interest: Option<Decimal>,
}

/// Tick數據結構
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tick {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub price: Decimal,
    pub volume: Decimal,
    pub trade_type: Option<i16>,
    pub bid_price_1: Option<Decimal>,
    pub bid_volume_1: Option<Decimal>,
    pub ask_price_1: Option<Decimal>,
    pub ask_volume_1: Option<Decimal>,
    pub bid_prices: Option<Json<serde_json::Value>>,
    pub bid_volumes: Option<Json<serde_json::Value>>,
    pub ask_prices: Option<Json<serde_json::Value>>,
    pub ask_volumes: Option<Json<serde_json::Value>>,
    pub open_interest: Option<Decimal>,
    pub spread: Option<Decimal>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// Tick數據插入模型 (不包含自動生成的字段)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickInsert {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub price: Decimal,
    pub volume: Decimal,
    pub trade_type: Option<i16>,
    pub bid_price_1: Option<Decimal>,
    pub bid_volume_1: Option<Decimal>,
    pub ask_price_1: Option<Decimal>,
    pub ask_volume_1: Option<Decimal>,
    // 擴展盤口數據（使用數組存儲多檔）
    pub bid_prices: Option<Vec<Decimal>>,
    pub bid_volumes: Option<Vec<Decimal>>,
    pub ask_prices: Option<Vec<Decimal>>,
    pub ask_volumes: Option<Vec<Decimal>>,
    // 期貨/選擇權特有
    pub open_interest: Option<Decimal>,
    // 外匯特有
    pub spread: Option<Decimal>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 日K線數據查詢結果
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DailyBar {
    pub date: NaiveDate,
    pub instrument_id: i32,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub amount: Option<Decimal>,
}

/// 技術指標數據
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InstrumentDailyIndicator {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub indicator_id: i32,
    pub parameters: Json<serde_json::Value>,
    pub values: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// 技術指標定義
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TechnicalIndicator {
    pub indicator_id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 基本面指標
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FundamentalIndicator {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub indicator_type: String,
    pub values: Json<serde_json::Value>,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 市場事件插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketEventInsert {
    pub time: DateTime<Utc>,
    pub event_type: String,
    pub title: String,
    pub description: Option<String>,
    pub impact_level: Option<i16>,
    pub related_instruments: Option<Json<serde_json::Value>>,
    pub related_exchanges: Option<Json<serde_json::Value>>,
    pub source: Option<String>,
}

/// 財務報告插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReportInsert {
    pub instrument_id: i32,
    pub report_date: NaiveDate,
    pub report_type: String,
    pub fiscal_year: i32,
    pub publish_date: NaiveDate,
    pub revenue: Option<Decimal>,
    pub net_income: Option<Decimal>,
    pub eps: Option<Decimal>,
    pub pe_ratio: Option<Decimal>,
    pub pb_ratio: Option<Decimal>,
    pub roe: Option<Decimal>,
    pub debt_to_equity: Option<Decimal>,
    pub current_ratio: Option<Decimal>,
    pub metrics: Option<Json<serde_json::Value>>,
}

/// 每小時成交量聚合視圖模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HourlyVolumeByInstrument {
    pub bucket: DateTime<Utc>,
    pub instrument_id: i32,
    pub total_volume: Decimal,
    pub total_amount: Option<Decimal>,
    pub trade_count: i64,
}

/// 每日交易量聚合視圖模型
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

/// 技術指標插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicatorInsert {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Json<serde_json::Value>>,
}

/// 技術指標數據插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentDailyIndicatorInsert {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub indicator_id: i32,
    pub parameters: Json<serde_json::Value>,
    pub values: Json<serde_json::Value>,
}

/// 基本面指標插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundamentalIndicatorInsert {
    pub time: DateTime<Utc>,
    pub instrument_id: i32,
    pub indicator_type: String,
    pub values: Json<serde_json::Value>,
    pub source: Option<String>,
}

/// 市場事件查詢模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketEvent {
    pub time: DateTime<Utc>,
    pub event_type: String,
    pub title: String,
    pub description: Option<String>,
    pub impact_level: Option<i16>,
    pub related_instruments: Option<Json<serde_json::Value>>,
    pub related_exchanges: Option<Json<serde_json::Value>>,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 財務報告查詢模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FinancialReport {
    pub report_id: i32,
    pub instrument_id: i32,
    pub report_date: NaiveDate,
    pub report_type: String,
    pub fiscal_year: i32,
    pub publish_date: NaiveDate,
    pub revenue: Option<Decimal>,
    pub net_income: Option<Decimal>,
    pub eps: Option<Decimal>,
    pub pe_ratio: Option<Decimal>,
    pub pb_ratio: Option<Decimal>,
    pub roe: Option<Decimal>,
    pub debt_to_equity: Option<Decimal>,
    pub current_ratio: Option<Decimal>,
    pub metrics: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
} 