use crate::{for_each_ohlcv_frequency, domain_types::{
    Day, FifteenMinutes, FiveMinutes, Frequency, Hour, Minute, Month, 
    OhlcvSeries, Tick, TickSeries, Week,
}};
use crate::redis::pool::RedisPool;
use crate::storage::{
    models::market_data::{MinuteBar, Tick as DbTick},
    repository::{TimeRange, market_data::{MarketDataRepository, PgMarketDataRepository}},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use polars::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, instrument};

/// 資料載入器的錯誤類型
#[derive(Debug, Error)]
pub enum DataLoaderError {
    #[error("資料庫錯誤: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Redis 錯誤: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("Polars 錯誤: {0}")]
    Polars(#[from] PolarsError),
    
    #[error("儲存器錯誤: {0}")]
    Repository(#[from] anyhow::Error),
    
    #[error("找不到指定的資料: instrument_id={instrument_id}, start={start}, end={end}")]
    DataNotFound {
        instrument_id: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    
    #[error("無效的時間範圍: start={start} 必須早於 end={end}")]
    InvalidTimeRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    
    #[error("不支援的頻率: {0}")]
    InvalidFrequency(String),
}

pub type Result<T> = std::result::Result<T, DataLoaderError>;

/// 定義 AnyOhlcvSeries 枚舉和實現
macro_rules! define_any_ohlcv_series {
    ($($marker:ident => $freq:ident,)*) => {
        /// 包裝不同頻率的 OHLCV 資料
        #[derive(Debug)]
        pub enum AnyOhlcvSeries {
            $($marker(OhlcvSeries<$marker>),)*
        }
        
        impl AnyOhlcvSeries {
            /// 獲取 instrument_id
            pub fn instrument_id(&self) -> &str {
                match self {
                    $(AnyOhlcvSeries::$marker(s) => s.instrument_id(),)*
                }
            }
            
            /// 獲取頻率
            pub fn frequency(&self) -> Frequency {
                match self {
                    $(AnyOhlcvSeries::$marker(_) => Frequency::$freq,)*
                }
            }
            
            /// 獲取 LazyFrame 引用
            pub fn lazy_frame(&self) -> &LazyFrame {
                match self {
                    $(AnyOhlcvSeries::$marker(s) => s.lazy_frame(),)*
                }
            }
            
            /// 執行計算並返回 DataFrame
            pub fn collect(self) -> PolarsResult<DataFrame> {
                match self {
                    $(AnyOhlcvSeries::$marker(s) => s.collect(),)*
                }
            }
        }
    };
}

// 使用宏生成 AnyOhlcvSeries
for_each_ohlcv_frequency!(define_any_ohlcv_series);

/// 定義從分鐘級資料創建任意頻率 OHLCV 的函數
macro_rules! define_create_ohlcv_by_frequency {
    ($($marker:ident => $freq:ident,)*) => {
        fn create_ohlcv_by_frequency(
            minute_ohlcv: OhlcvSeries<Minute>,
            frequency: Frequency,
        ) -> Result<AnyOhlcvSeries> {
            match frequency {
                Frequency::Tick => {
                    Err(DataLoaderError::InvalidFrequency(
                        "無法將 OHLCV 資料轉換為 Tick 頻率".to_string()
                    ))
                }
                $(
                    Frequency::$freq => {
                        // 特殊處理分鐘級資料（不需要重採樣）
                        if stringify!($marker) == "Minute" {
                            Ok(AnyOhlcvSeries::Minute(minute_ohlcv))
                        } else {
                            Ok(AnyOhlcvSeries::$marker(minute_ohlcv.resample_to::<$marker>()?))
                        }
                    }
                )*
            }
        }
    };
}

// 使用宏生成函數
for_each_ohlcv_frequency!(define_create_ohlcv_by_frequency);

/// 資料載入器的通用介面
#[async_trait]
pub trait DataLoader: Send + Sync {
    /// 載入指定頻率的 OHLCV 資料
    async fn load_ohlcv(
        &self,
        instrument_id: i32,
        frequency: Frequency,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<AnyOhlcvSeries>;
    
    /// 載入 Tick 資料
    async fn load_ticks(
        &self,
        instrument_id: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<TickSeries<Tick>>;
}

/// 市場資料載入器實現
pub struct MarketDataLoader {
    database: Arc<PgPool>,
    redis_pool: Option<Arc<dyn RedisPool>>,
    cache_ttl_seconds: u64,
}

impl MarketDataLoader {
    /// 建立新的市場資料載入器
    pub fn new(database: Arc<PgPool>) -> Self {
        Self {
            database,
            redis_pool: None,
            cache_ttl_seconds: 300, // 預設 5 分鐘快取
        }
    }
    
    /// 設定 Redis 連接池以啟用快取功能
    pub fn with_redis(mut self, redis_pool: Arc<dyn RedisPool>) -> Self {
        self.redis_pool = Some(redis_pool);
        self
    }
    
    /// 設定快取過期時間（秒）
    pub fn with_cache_ttl(mut self, ttl_seconds: u64) -> Self {
        self.cache_ttl_seconds = ttl_seconds;
        self
    }
}

#[async_trait]
impl DataLoader for MarketDataLoader {
    #[instrument(skip(self))]
    async fn load_ohlcv(
        &self,
        instrument_id: i32,
        frequency: Frequency,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<AnyOhlcvSeries> {
        // 驗證時間範圍
        if start >= end {
            return Err(DataLoaderError::InvalidTimeRange { start, end });
        }
        
        debug!("載入 {:?} 級 OHLCV 資料: instrument_id={}, start={}, end={}", 
               frequency, instrument_id, start, end);
        
        // 從資料庫載入分鐘級資料作為基礎
        let market_data_repo = PgMarketDataRepository::new((*self.database).clone());
        let time_range = TimeRange::new(start, end);
        let bars = market_data_repo
            .get_minute_bars(instrument_id, time_range, None)
            .await?;
        
        if bars.is_empty() {
            return Err(DataLoaderError::DataNotFound {
                instrument_id,
                start,
                end,
            });
        }
        
        // 轉換為 Polars DataFrame
        let df = minute_bars_to_dataframe(bars)?;
        
        // 建立分鐘級 OHLCV
        let minute_ohlcv = OhlcvSeries::<Minute>::from_lazy(df.lazy(), instrument_id.to_string());
        
        // 根據請求的頻率進行重採樣或直接返回
        create_ohlcv_by_frequency(minute_ohlcv, frequency)
    }
    
    #[instrument(skip(self))]
    async fn load_ticks(
        &self,
        instrument_id: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<TickSeries<Tick>> {
        // 驗證時間範圍
        if start >= end {
            return Err(DataLoaderError::InvalidTimeRange { start, end });
        }
        
        debug!("載入 Tick 資料: instrument_id={}, start={}, end={}", 
               instrument_id, start, end);
        
        // 從資料庫載入資料
        let market_data_repo = PgMarketDataRepository::new((*self.database).clone());
        let time_range = TimeRange::new(start, end);
        let ticks = market_data_repo
            .get_ticks(instrument_id, time_range, None)
            .await?;
        
        if ticks.is_empty() {
            return Err(DataLoaderError::DataNotFound {
                instrument_id,
                start,
                end,
            });
        }
        
        // 轉換為 Polars DataFrame
        let df = ticks_to_dataframe(ticks)?;
        
        // 建立 TickSeries
        Ok(TickSeries::<Tick>::from_lazy(df.lazy(), instrument_id.to_string()))
    }
}

/// 將分鐘線資料轉換為 DataFrame
fn minute_bars_to_dataframe(bars: Vec<MinuteBar>) -> Result<DataFrame> {
    let mut time_vec = Vec::with_capacity(bars.len());
    let mut open_vec = Vec::with_capacity(bars.len());
    let mut high_vec = Vec::with_capacity(bars.len());
    let mut low_vec = Vec::with_capacity(bars.len());
    let mut close_vec = Vec::with_capacity(bars.len());
    let mut volume_vec = Vec::with_capacity(bars.len());
    let mut instrument_id_vec = Vec::with_capacity(bars.len());
    
    for bar in bars {
        time_vec.push(bar.time.timestamp_millis());
        open_vec.push(bar.open.to_f64().unwrap_or(0.0));
        high_vec.push(bar.high.to_f64().unwrap_or(0.0));
        low_vec.push(bar.low.to_f64().unwrap_or(0.0));
        close_vec.push(bar.close.to_f64().unwrap_or(0.0));
        volume_vec.push(bar.volume.to_f64().unwrap_or(0.0));
        instrument_id_vec.push(bar.instrument_id);
    }
    
    let df = DataFrame::new(vec![
        Series::new("time".into(), time_vec).into(),
        Series::new("open".into(), open_vec).into(),
        Series::new("high".into(), high_vec).into(),
        Series::new("low".into(), low_vec).into(),
        Series::new("close".into(), close_vec).into(),
        Series::new("volume".into(), volume_vec).into(),
        Series::new("instrument_id".into(), instrument_id_vec).into(),
    ])?;
    
    Ok(df)
}

/// 將 Tick 資料轉換為 DataFrame
fn ticks_to_dataframe(ticks: Vec<DbTick>) -> Result<DataFrame> {
    let mut time_vec = Vec::with_capacity(ticks.len());
    let mut price_vec = Vec::with_capacity(ticks.len());
    let mut volume_vec = Vec::with_capacity(ticks.len());
    let mut instrument_id_vec = Vec::with_capacity(ticks.len());
    
    for tick in ticks {
        time_vec.push(tick.time.timestamp_millis());
        price_vec.push(tick.price.to_f64().unwrap_or(0.0));
        volume_vec.push(tick.volume.to_f64().unwrap_or(0.0));
        instrument_id_vec.push(tick.instrument_id);
    }
    
    let df = DataFrame::new(vec![
        Series::new("time".into(), time_vec).into(),
        Series::new("price".into(), price_vec).into(),
        Series::new("volume".into(), volume_vec).into(),
        Series::new("instrument_id".into(), instrument_id_vec).into(),
    ])?;
    
    Ok(df)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    
    #[test]
    fn test_minute_bars_to_dataframe() {
        let bars = vec![
            MinuteBar {
                instrument_id: 1001,
                time: DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: Decimal::from_str("100.5").unwrap(),
                high: Decimal::from_str("101.0").unwrap(),
                low: Decimal::from_str("100.0").unwrap(),
                close: Decimal::from_str("100.8").unwrap(),
                volume: Decimal::from_str("1000").unwrap(),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
            MinuteBar {
                instrument_id: 1001,
                time: DateTime::parse_from_rfc3339("2024-01-01T10:01:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: Decimal::from_str("100.8").unwrap(),
                high: Decimal::from_str("101.2").unwrap(),
                low: Decimal::from_str("100.7").unwrap(),
                close: Decimal::from_str("101.0").unwrap(),
                volume: Decimal::from_str("1500").unwrap(),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
        ];
        
        let df = minute_bars_to_dataframe(bars).unwrap();
        
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 7);
        assert!(df.column("time").is_ok());
        assert!(df.column("open").is_ok());
        assert!(df.column("high").is_ok());
        assert!(df.column("low").is_ok());
        assert!(df.column("close").is_ok());
        assert!(df.column("volume").is_ok());
        assert!(df.column("instrument_id").is_ok());
    }
    
    #[test]
    fn test_any_ohlcv_series() {
        use crate::domain_types::types::ColumnName;
        use polars::prelude::*;
        
        // 創建測試 DataFrame
        let df = DataFrame::new(vec![
            Series::new(ColumnName::TIME.into(), &[1704067200000i64, 1704067260000i64]).into(),
            Series::new(ColumnName::OPEN.into(), &[100.0, 101.0]).into(),
            Series::new(ColumnName::HIGH.into(), &[105.0, 106.0]).into(),
            Series::new(ColumnName::LOW.into(), &[95.0, 96.0]).into(),
            Series::new(ColumnName::CLOSE.into(), &[102.0, 103.0]).into(),
            Series::new(ColumnName::VOLUME.into(), &[1000.0, 2000.0]).into(),
        ]).unwrap();
        
        let minute_series = OhlcvSeries::<Minute>::from_lazy(df.lazy(), "TEST".to_string());
        let any_series = AnyOhlcvSeries::Minute(minute_series);
        
        assert_eq!(any_series.instrument_id(), "TEST");
        assert_eq!(any_series.frequency(), Frequency::Minute);
    }
}