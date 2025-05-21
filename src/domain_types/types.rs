//! 基本市場數據類型定義

use polars::prelude::Duration as PolarsDuration;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration as StdDuration;
use thiserror::Error;

/// 金融資產類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    Stock,
    Future,
    Option,
    Forex,
    Crypto,
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetType::Stock => write!(f, "Stock"),
            AssetType::Future => write!(f, "Future"),
            AssetType::Option => write!(f, "Option"),
            AssetType::Forex => write!(f, "Forex"),
            AssetType::Crypto => write!(f, "Crypto"),
        }
    }
}

/// 數據頻率定義
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Frequency {
    Tick,
    Second,
    Minute,
    FiveMinutes,
    FifteenMinutes,
    Hour,
    Day,
    Week,
    Month,
}

impl Frequency {
    /// 轉換為表示該頻率的 std::time::Duration
    pub fn to_std_duration(&self) -> StdDuration {
        match self {
            Frequency::Tick => StdDuration::from_secs(0),
            Frequency::Second => StdDuration::from_secs(1),
            Frequency::Minute => StdDuration::from_secs(60),
            Frequency::FiveMinutes => StdDuration::from_secs(300),
            Frequency::FifteenMinutes => StdDuration::from_secs(900),
            Frequency::Hour => StdDuration::from_secs(3600),
            Frequency::Day => StdDuration::from_secs(86400),
            Frequency::Week => StdDuration::from_secs(604800),
            Frequency::Month => StdDuration::from_secs(2592000), // 簡化，使用30天
        }
    }

    /// 轉換為表示該頻率的 Polars Duration
    pub fn to_duration(&self) -> PolarsDuration {
        match self {
            Frequency::Tick => PolarsDuration::new(0),
            Frequency::Second => PolarsDuration::new(1), // seconds
            Frequency::Minute => PolarsDuration::new(60), // 60 seconds
            Frequency::FiveMinutes => PolarsDuration::new(300), // 300 seconds
            Frequency::FifteenMinutes => PolarsDuration::new(900), // 900 seconds
            Frequency::Hour => PolarsDuration::new(3600), // 3600 seconds
            Frequency::Day => PolarsDuration::new(86400), // 86400 seconds
            Frequency::Week => PolarsDuration::new(604800), // 604800 seconds
            Frequency::Month => PolarsDuration::new(2592000), // 2592000 seconds (30 days)
        }
    }

    /// 轉換為 Polars 可識別的時間字串
    pub fn to_polars_duration_string(&self) -> String {
        match self {
            Frequency::Tick => "ns".to_string(),
            Frequency::Second => "1s".to_string(),
            Frequency::Minute => "1m".to_string(),
            Frequency::FiveMinutes => "5m".to_string(),
            Frequency::FifteenMinutes => "15m".to_string(),
            Frequency::Hour => "1h".to_string(),
            Frequency::Day => "1d".to_string(),
            Frequency::Week => "1w".to_string(),
            Frequency::Month => "1mo".to_string(),
        }
    }
}

/// 交易方向（多/空）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Long,  // 做多
    Short, // 做空
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Long => write!(f, "Long"),
            Direction::Short => write!(f, "Short"),
        }
    }
}

/// 訂單類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    Market,            // 市價單
    Limit,             // 限價單
    Stop,              // 止損單
    StopLimit,         // 止損限價單
    TrailingStop,      // 追蹤止損單
    FillOrKill,        // 全部成交或取消
    ImmediateOrCancel, // 立即成交或取消
    GoodTillCancel,    // 有效至取消
    GoodTillDate,      // 有效至特定日期
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderType::Market => write!(f, "Market"),
            OrderType::Limit => write!(f, "Limit"),
            OrderType::Stop => write!(f, "Stop"),
            OrderType::StopLimit => write!(f, "StopLimit"),
            OrderType::TrailingStop => write!(f, "TrailingStop"),
            OrderType::FillOrKill => write!(f, "FillOrKill"),
            OrderType::ImmediateOrCancel => write!(f, "ImmediateOrCancel"),
            OrderType::GoodTillCancel => write!(f, "GoodTillCancel"),
            OrderType::GoodTillDate => write!(f, "GoodTillDate"),
        }
    }
}

/// 領域錯誤類型
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("無效的資產類型: {0}")]
    InvalidAssetType(String),

    #[error("無效的頻率: {0}")]
    InvalidFrequency(String),

    #[error("無效的交易方向: {0}")]
    InvalidDirection(String),

    #[error("無效的訂單類型: {0}")]
    InvalidOrderType(String),

    #[error("無效的數據格式: {0}")]
    InvalidDataFormat(String),

    #[error("缺少必要欄位: {0}")]
    MissingRequiredField(String),

    #[error("數據範圍錯誤: {0}")]
    DataRangeError(String),

    #[error("時間序列操作錯誤: {0}")]
    TimeSeriesError(String),

    #[error("資料轉換錯誤: {0}")]
    ConversionError(String),

    #[error("未知錯誤: {0}")]
    Unknown(String),
}

/// 領域結果類型
pub type Result<T> = std::result::Result<T, DomainError>;

/// 標準列名定義
pub struct ColumnName;

impl ColumnName {
    pub const TIME: &'static str = "time";
    pub const OPEN: &'static str = "open";
    pub const HIGH: &'static str = "high";
    pub const LOW: &'static str = "low";
    pub const CLOSE: &'static str = "close";
    pub const VOLUME: &'static str = "volume";
    pub const INSTRUMENT_ID: &'static str = "instrument_id";

    // Tick 數據相關
    pub const PRICE: &'static str = "price";
    pub const BID: &'static str = "bid";
    pub const ASK: &'static str = "ask";
    pub const BID_VOLUME: &'static str = "bid_volume";
    pub const ASK_VOLUME: &'static str = "ask_volume";
}
