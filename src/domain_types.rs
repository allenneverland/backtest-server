//! 市場數據模組 - 提供金融市場數據的核心結構和操作

pub mod frame;
pub mod indicators;
pub mod instrument;
pub mod resampler;
pub mod series;
pub mod types;

// 重新匯出核心類型
pub use frame::{OHLCVFrame, TickFrame};
pub use indicators::IndicatorsExt;
pub use instrument::Instrument;
pub use resampler::Resampler;
pub use series::MarketSeries;
pub use types::{AssetType, ColumnName, Direction, DomainError, Frequency, OrderType, Result};
