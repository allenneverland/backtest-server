//! 市場數據模組 - 提供金融市場數據的核心結構和操作

pub mod types;
pub mod instrument;
pub mod frame;
pub mod series;
pub mod indicators;
pub mod resampler;

// 重新匯出核心類型
pub use types::{AssetType, Frequency, Column, Direction, OrderType, DomainError, Result};
pub use instrument::Instrument;
pub use frame::{MarketFrame, MarketFrameExt};
pub use series::MarketSeries;
pub use indicators::IndicatorsExt;
pub use resampler::Resampler;
