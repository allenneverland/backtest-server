use crate::data_provider::cache::{generate_cache_key, MultiLevelCache};
use crate::domain_types::*;
use crate::redis::operations::cache::{CacheError, CacheManager};
use crate::redis::pool::{ConnectionPool, RedisPool};
use crate::storage::{
    models::market_data::{MinuteBar, Tick as DbTick},
    repository::{
        market_data::{MarketDataRepository, PgMarketDataRepository},
        TimeRange,
    },
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use polars::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, instrument, warn};

/// 資料載入器的錯誤類型
#[derive(Debug, Error)]
pub enum DataLoaderError {
    #[error("資料庫錯誤: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis 錯誤: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("快取錯誤: {0}")]
    Cache(#[from] CacheError),

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

/// 資料載入器的通用介面
#[async_trait]
pub trait DataLoader: Send + Sync {
    /// 載入指定頻率的 OHLCV 資料
    async fn load_ohlcv<F: FrequencyMarker>(
        &self,
        instrument_id: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<OhlcvSeries<F>>;

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
    redis_pool: Option<Arc<ConnectionPool>>,
    cache: Option<Arc<MultiLevelCache<ConnectionPool>>>,
    cache_ttl_seconds: u64,
}

impl MarketDataLoader {
    /// 建立新的市場資料載入器
    pub fn new(database: Arc<PgPool>) -> Self {
        Self {
            database,
            redis_pool: None,
            cache: None,
            cache_ttl_seconds: 300, // 預設 5 分鐘快取
        }
    }

    /// 設定 Redis 連接池以啟用快取功能
    pub fn with_redis(mut self, redis_pool: Arc<ConnectionPool>) -> Self {
        self.redis_pool = Some(redis_pool.clone());

        // 創建多層級快取
        let cache_manager = Arc::new(CacheManager::new(redis_pool));
        let multi_level_cache = Arc::new(MultiLevelCache::new(
            cache_manager,
            1000, // 內存快取容量
            self.cache_ttl_seconds,
        ));
        self.cache = Some(multi_level_cache);

        self
    }

    /// 設定快取過期時間（秒）
    pub fn with_cache_ttl(mut self, ttl_seconds: u64) -> Self {
        self.cache_ttl_seconds = ttl_seconds;
        self
    }

    /// 預熱快取
    ///
    /// 載入常用資料到快取中，以提高後續查詢效能
    pub async fn warm_cache(
        &self,
        instrument_ids: &[i32],
        frequencies: &[&str],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<()> {
        if let Some(ref cache) = self.cache {
            let mut cache_keys = Vec::new();

            // 生成所有需要預熱的快取鍵
            for &instrument_id in instrument_ids {
                for &frequency in frequencies {
                    let key = generate_cache_key(
                        instrument_id,
                        frequency,
                        start.timestamp_millis(),
                        end.timestamp_millis(),
                    );
                    cache_keys.push(key);
                }
            }

            // 批量預熱快取
            cache.warm_cache(cache_keys).await?;

            debug!(
                "快取預熱完成: {} 個項目",
                instrument_ids.len() * frequencies.len()
            );
        }

        Ok(())
    }
}

#[async_trait]
impl DataLoader for MarketDataLoader {
    #[instrument(skip(self))]
    async fn load_ohlcv<F: FrequencyMarker>(
        &self,
        instrument_id: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<OhlcvSeries<F>> {
        // 驗證時間範圍
        if start >= end {
            return Err(DataLoaderError::InvalidTimeRange { start, end });
        }

        debug!(
            "載入 {} 級 OHLCV 資料: instrument_id={}, start={}, end={}",
            F::name(),
            instrument_id,
            start,
            end
        );

        // 生成快取鍵
        let cache_key = generate_cache_key(
            instrument_id,
            F::name(),
            start.timestamp_millis(),
            end.timestamp_millis(),
        );

        // 如果快取可用，嘗試從快取獲取
        if let Some(ref cache) = self.cache {
            match cache.get::<Vec<MinuteBar>>(&cache_key).await {
                Ok(Some(cached_bars)) => {
                    debug!("從快取獲取 OHLCV 資料: {}", cache_key);
                    // 轉換快取的資料並返回
                    let df = minute_bars_to_dataframe(cached_bars)?;
                    let minute_ohlcv =
                        OhlcvSeries::<Minute>::from_lazy(df.lazy(), instrument_id.to_string());

                    if F::to_frequency() == Frequency::Minute {
                        let df_collected = minute_ohlcv.collect()?;
                        return Ok(OhlcvSeries::<F>::from_dataframe_unchecked(
                            df_collected,
                            instrument_id.to_string(),
                        ));
                    } else if F::to_frequency().is_ohlcv() {
                        return Ok(minute_ohlcv.resample_to::<F>()?);
                    }
                }
                Ok(None) => {
                    debug!("快取未命中: {}", cache_key);
                }
                Err(e) => {
                    warn!("快取讀取錯誤: {}, 將直接從資料庫讀取", e);
                }
            }
        }

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

        // 如果快取可用，將資料存入快取
        if let Some(ref cache) = self.cache {
            if let Err(e) = cache.set(&cache_key, &bars).await {
                warn!("快取寫入錯誤: {}", e);
            }
        }

        // 轉換為 Polars DataFrame
        let df = minute_bars_to_dataframe(bars)?;

        // 建立分鐘級 OHLCV
        let minute_ohlcv = OhlcvSeries::<Minute>::from_lazy(df.lazy(), instrument_id.to_string());

        // 根據目標頻率進行重採樣
        if F::to_frequency() == Frequency::Minute {
            // 如果目標是分鐘級，直接返回（需要類型轉換）
            // 這裡需要 unsafe 或重新創建，因為 Rust 不知道 F 和 Minute 是同一類型
            let df_collected = minute_ohlcv.collect()?;
            Ok(OhlcvSeries::<F>::from_dataframe_unchecked(
                df_collected,
                instrument_id.to_string(),
            ))
        } else if F::to_frequency().is_ohlcv() {
            // 對於其他 OHLCV 頻率，進行重採樣
            Ok(minute_ohlcv.resample_to::<F>()?)
        } else {
            Err(DataLoaderError::InvalidFrequency(format!(
                "無法載入 {} 頻率的 OHLCV 資料",
                F::name()
            )))
        }
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

        debug!(
            "載入 Tick 資料: instrument_id={}, start={}, end={}",
            instrument_id, start, end
        );

        // 生成快取鍵
        let cache_key = generate_cache_key(
            instrument_id,
            "tick",
            start.timestamp_millis(),
            end.timestamp_millis(),
        );

        // 如果快取可用，嘗試從快取獲取
        if let Some(ref cache) = self.cache {
            match cache.get::<Vec<DbTick>>(&cache_key).await {
                Ok(Some(cached_ticks)) => {
                    debug!("從快取獲取 Tick 資料: {}", cache_key);
                    // 轉換快取的資料並返回
                    let df = ticks_to_dataframe(cached_ticks)?;
                    return Ok(TickSeries::<Tick>::from_lazy(
                        df.lazy(),
                        instrument_id.to_string(),
                    ));
                }
                Ok(None) => {
                    debug!("快取未命中: {}", cache_key);
                }
                Err(e) => {
                    warn!("快取讀取錯誤: {}, 將直接從資料庫讀取", e);
                }
            }
        }

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

        // 如果快取可用，將資料存入快取
        if let Some(ref cache) = self.cache {
            if let Err(e) = cache.set(&cache_key, &ticks).await {
                warn!("快取寫入錯誤: {}", e);
            }
        }

        // 轉換為 Polars DataFrame
        let df = ticks_to_dataframe(ticks)?;

        // 建立 TickSeries
        Ok(TickSeries::<Tick>::from_lazy(
            df.lazy(),
            instrument_id.to_string(),
        ))
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
    fn test_ohlcv_series_generic() {
        use crate::domain_types::types::ColumnName;
        use polars::prelude::*;

        // 創建測試 DataFrame
        let _df = DataFrame::new(vec![
            Series::new(
                ColumnName::TIME.into(),
                &[1704067200000i64, 1704067260000i64],
            )
            .into(),
            Series::new(ColumnName::OPEN.into(), &[100.0, 101.0]).into(),
            Series::new(ColumnName::HIGH.into(), &[105.0, 106.0]).into(),
            Series::new(ColumnName::LOW.into(), &[95.0, 96.0]).into(),
            Series::new(ColumnName::CLOSE.into(), &[102.0, 103.0]).into(),
            Series::new(ColumnName::VOLUME.into(), &[1000.0, 2000.0]).into(),
        ])
        .unwrap();
    }
}
