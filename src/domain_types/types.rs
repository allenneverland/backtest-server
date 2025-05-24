//! 基本市場數據類型定義

use polars::prelude::{DataFrame, PolarsResult};
use serde::{Deserialize, Serialize};
use std::fmt;
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

// Frequency types are now in the frequency module

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
