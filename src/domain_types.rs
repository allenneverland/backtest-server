//! 市場數據模組 - 提供金融市場數據的核心結構和操作

pub mod frame;
pub mod indicators;
pub mod instrument;
pub mod resampler;
pub mod series;
pub mod types;

// 重新匯出核心類型
pub use frame::{OHLCVFrame, TickFrame}; // 保留向後兼容性
pub use indicators::IndicatorsExt;
pub use instrument::Instrument;
pub use resampler::Resampler;

// 新的泛型時間序列
pub use series::{
    FinancialSeries, OhlcvSeries, TickSeries,
    // 便利類型別名
    DailyOhlcv, MinuteOhlcv, HourlyOhlcv, FiveMinuteOhlcv, FifteenMinuteOhlcv,
    WeeklyOhlcv, MonthlyOhlcv, TickData, SecondTick, MinuteTick,
};

// 基礎類型和trait
pub use types::{
    AssetType, ColumnName, Direction, DomainError, Frequency, OrderType, Result,
    // 新增的頻率標記類型
    FrequencyMarker, Tick, Second, Minute, FiveMinutes, FifteenMinutes, 
    Hour, Day, Week, Month,
    // 數據格式類型
    DataFormat, OhlcvFormat, TickFormat,
};
