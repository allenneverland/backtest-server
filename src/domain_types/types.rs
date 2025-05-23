//! 基本市場數據類型定義

use polars::prelude::{Duration as PolarsDuration, DataFrame, PolarsResult};
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
            Frequency::Tick => PolarsDuration::parse("0i"),
            Frequency::Second => PolarsDuration::parse("1000i"),        // 1 second = 1,000 ms
            Frequency::Minute => PolarsDuration::parse("60000i"),       // 1 minute = 60,000 ms
            Frequency::FiveMinutes => PolarsDuration::parse("300000i"), // 5 minutes = 300,000 ms
            Frequency::FifteenMinutes => PolarsDuration::parse("900000i"), // 15 minutes = 900,000 ms
            Frequency::Hour => PolarsDuration::parse("3600000i"),       // 1 hour = 3,600,000 ms
            Frequency::Day => PolarsDuration::parse("86400000i"),       // 1 day = 86,400,000 ms
            Frequency::Week => PolarsDuration::parse("604800000i"),     // 1 week = 604,800,000 ms
            Frequency::Month => PolarsDuration::parse("2592000000i"),   // 30 days = 2,592,000,000 ms
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
    pub const TIME: &'static str = "time"; // 以毫秒為單位的 i64 時間戳
    pub const OPEN: &'static str = "open"; // 開盤價
    pub const HIGH: &'static str = "high"; // 最高價
    pub const LOW: &'static str = "low"; // 最低價
    pub const CLOSE: &'static str = "close"; // 收盤價
    pub const VOLUME: &'static str = "volume"; // 成交量
    pub const INSTRUMENT_ID: &'static str = "instrument_id"; // 商品代碼

    // Tick 數據相關
    pub const PRICE: &'static str = "price"; // 價格
    pub const BID: &'static str = "bid"; // 買一價
    pub const ASK: &'static str = "ask"; // 賣一價
    pub const BID_VOLUME: &'static str = "bid_volume"; // 買一量
    pub const ASK_VOLUME: &'static str = "ask_volume"; // 賣一量
}

// ========== 頻率標記類型 ==========

/// 頻率標記 trait
pub trait FrequencyMarker {
    fn to_frequency() -> Frequency;
    fn name() -> &'static str;
}

/// Tick 頻率標記
pub struct Tick;
impl FrequencyMarker for Tick {
    fn to_frequency() -> Frequency { Frequency::Tick }
    fn name() -> &'static str { "Tick" }
}

/// 秒級頻率標記
pub struct Second;
impl FrequencyMarker for Second {
    fn to_frequency() -> Frequency { Frequency::Second }
    fn name() -> &'static str { "Second" }
}

/// 分鐘級頻率標記
pub struct Minute;
impl FrequencyMarker for Minute {
    fn to_frequency() -> Frequency { Frequency::Minute }
    fn name() -> &'static str { "Minute" }
}

/// 5分鐘級頻率標記
pub struct FiveMinutes;
impl FrequencyMarker for FiveMinutes {
    fn to_frequency() -> Frequency { Frequency::FiveMinutes }
    fn name() -> &'static str { "FiveMinutes" }
}

/// 15分鐘級頻率標記
pub struct FifteenMinutes;
impl FrequencyMarker for FifteenMinutes {
    fn to_frequency() -> Frequency { Frequency::FifteenMinutes }
    fn name() -> &'static str { "FifteenMinutes" }
}

/// 小時級頻率標記
pub struct Hour;
impl FrequencyMarker for Hour {
    fn to_frequency() -> Frequency { Frequency::Hour }
    fn name() -> &'static str { "Hour" }
}

/// 日級頻率標記
pub struct Day;
impl FrequencyMarker for Day {
    fn to_frequency() -> Frequency { Frequency::Day }
    fn name() -> &'static str { "Day" }
}

/// 週級頻率標記
pub struct Week;
impl FrequencyMarker for Week {
    fn to_frequency() -> Frequency { Frequency::Week }
    fn name() -> &'static str { "Week" }
}

/// 月級頻率標記
pub struct Month;
impl FrequencyMarker for Month {
    fn to_frequency() -> Frequency { Frequency::Month }
    fn name() -> &'static str { "Month" }
}

// ========== 數據格式 trait ==========

/// 數據格式 trait - 定義不同金融數據格式的要求
pub trait DataFormat {
    /// 獲取此格式所需的必要列名
    fn required_columns() -> &'static [&'static str];
    
    /// 驗證 DataFrame 是否符合此格式的要求
    fn validate_dataframe(df: &DataFrame) -> PolarsResult<()> {
        let required = Self::required_columns();
        let schema = df.schema();
        
        for &col in required {
            if !schema.contains(col) {
                return Err(polars::prelude::PolarsError::ColumnNotFound(
                    format!("Required column '{}' not found", col).into()
                ));
            }
        }
        Ok(())
    }
    
    /// 格式名稱，用於調試和錯誤訊息
    fn format_name() -> &'static str;
}

/// OHLCV 數據格式
pub struct OhlcvFormat;

impl DataFormat for OhlcvFormat {
    fn required_columns() -> &'static [&'static str] {
        &[
            ColumnName::TIME,
            ColumnName::OPEN,
            ColumnName::HIGH,
            ColumnName::LOW,
            ColumnName::CLOSE,
            ColumnName::VOLUME,
        ]
    }
    
    fn format_name() -> &'static str {
        "OHLCV"
    }
}

/// Tick 數據格式
pub struct TickFormat;

impl DataFormat for TickFormat {
    fn required_columns() -> &'static [&'static str] {
        &[
            ColumnName::TIME,
            ColumnName::PRICE,
            ColumnName::VOLUME,
        ]
    }
    
    fn format_name() -> &'static str {
        "Tick"
    }
}
