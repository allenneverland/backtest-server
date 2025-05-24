//! 市場數據模組 - 提供金融市場數據的核心結構和操作

pub mod frequency;
pub mod indicators;
pub mod instrument;
pub mod resampler;
pub mod series;
pub mod types;

// 重新匯出核心類型
pub use indicators::IndicatorsExt;
pub use instrument::Instrument;
pub use resampler::Resampler;

// 新的泛型時間序列
pub use series::{FinancialSeries, OhlcvSeries, TickSeries, TickData};

// 基礎類型和trait
pub use types::{
    AssetType, ColumnName, Direction, DomainError, OrderType, Result,
    // 數據格式類型
    DataFormat, OhlcvFormat, TickFormat,
};

// 頻率相關類型
pub use frequency::*;
