use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc, TimeZone};
use sqlx::PgPool;
use std::sync::Arc;
use rust_decimal::prelude::ToPrimitive;

use crate::domain_types::{
    AssetType, DataType, Frequency, TimeSeries,
    data_point::{OHLCVPoint, TickPoint, TradeType},
};
use crate::storage::{
    database,
    models::market_data::{DailyBar, FundamentalIndicator, InstrumentDailyIndicator, MinuteBar, Tick},
    repository::{market_data::PgMarketDataRepository, TimeRange, MarketDataRepository},
};

/// 數據加載器特性 - 定義數據提供模組的核心接口
#[async_trait]
pub trait DataLoader: Send + Sync {
    /// 加載 OHLCV 數據
    async fn load_ohlcv(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        frequency: Frequency,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<OHLCVPoint>>;

    /// 加載 Tick 數據
    async fn load_tick(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<TickPoint>>;

    /// 加載日級 OHLCV 數據 (從連續聚合)
    async fn load_daily_ohlcv(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<TimeSeries<OHLCVPoint>>;

    /// 加載技術指標數據
    async fn load_technical_indicator(
        &self,
        instrument_id: i32,
        indicator_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<InstrumentDailyIndicator>>;

    /// 加載基本面指標數據
    async fn load_fundamental_indicator(
        &self,
        instrument_id: i32,
        indicator_type: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<FundamentalIndicator>>;

    /// 獲取數據的可用時間範圍
    async fn get_data_time_range(
        &self,
        instrument_id: i32,
        data_type: DataType,
    ) -> Result<Option<TimeRange>>;
}

/// 實現 DataLoader 特性的具體類型，提供從數據庫加載數據的功能
pub struct DatabaseDataLoader {
    market_data_repo: Arc<PgMarketDataRepository>,
}

impl DatabaseDataLoader {
    /// 創建新的數據庫數據加載器
    pub async fn new() -> Result<Self> {
        let pool = database::get_db_pool(false).await?;
        let market_data_repo = Arc::new(PgMarketDataRepository::new(pool.clone()));
        
        Ok(Self {
            market_data_repo,
        })
    }
    
    /// 從現有的數據庫連接池創建數據加載器
    pub fn with_pool(pool: PgPool) -> Self {
        let market_data_repo = Arc::new(PgMarketDataRepository::new(pool));
        
        Self {
            market_data_repo,
        }
    }
    
    /// 將 MinuteBar 轉換為 OHLCVPoint
    fn minute_bar_to_ohlcv_point(bar: &MinuteBar) -> OHLCVPoint {
        OHLCVPoint {
            timestamp: bar.time,
            open: bar.open.to_f64().unwrap_or_default(),
            high: bar.high.to_f64().unwrap_or_default(),
            low: bar.low.to_f64().unwrap_or_default(),
            close: bar.close.to_f64().unwrap_or_default(),
            volume: bar.volume.to_f64().unwrap_or_default(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    /// 將 DailyBar 轉換為 OHLCVPoint
    fn daily_bar_to_ohlcv_point(bar: &DailyBar) -> OHLCVPoint {
        OHLCVPoint {
            timestamp: Utc.from_utc_datetime(
                &bar.date.and_hms_opt(0, 0, 0).unwrap_or_default()
            ),
            open: bar.open.to_f64().unwrap_or_default(),
            high: bar.high.to_f64().unwrap_or_default(),
            low: bar.low.to_f64().unwrap_or_default(),
            close: bar.close.to_f64().unwrap_or_default(),
            volume: bar.volume.to_f64().unwrap_or_default(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    /// 將 Tick 轉換為 TickPoint
    fn tick_to_tick_point(tick: &Tick) -> TickPoint {
        // 將整數交易類型轉換為 TradeType 枚舉
        let trade_type = match tick.trade_type {
            Some(1) => TradeType::Buy,
            Some(2) => TradeType::Sell,
            Some(3) => TradeType::Cross,
            Some(0) => TradeType::Neutral,
            _ => TradeType::Unknown,
        };
        
        TickPoint {
            timestamp: tick.time,
            price: tick.price.to_f64().unwrap_or_default(),
            volume: tick.volume.to_f64().unwrap_or_default(),
            trade_type,
            bid_price_1: tick.bid_price_1.as_ref().map_or(0.0, |v| v.to_f64().unwrap_or_default()),
            bid_price_2: 0.0, // 以下字段在 Tick 模型中不存在，使用默認值
            bid_price_3: 0.0,
            bid_price_4: 0.0,
            bid_price_5: 0.0,
            bid_volume_1: tick.bid_volume_1.as_ref().map_or(0.0, |v| v.to_f64().unwrap_or_default()),
            bid_volume_2: 0.0,
            bid_volume_3: 0.0,
            bid_volume_4: 0.0,
            bid_volume_5: 0.0,
            ask_price_1: tick.ask_price_1.as_ref().map_or(0.0, |v| v.to_f64().unwrap_or_default()),
            ask_price_2: 0.0,
            ask_price_3: 0.0,
            ask_price_4: 0.0,
            ask_price_5: 0.0,
            ask_volume_1: tick.ask_volume_1.as_ref().map_or(0.0, |v| v.to_f64().unwrap_or_default()),
            ask_volume_2: 0.0,
            ask_volume_3: 0.0,
            ask_volume_4: 0.0,
            ask_volume_5: 0.0,
            metadata: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl DataLoader for DatabaseDataLoader {
    async fn load_ohlcv(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        frequency: Frequency,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        // 创建 TimeSeries 对象
        let mut time_series = TimeSeries::new_ohlcv(
            symbol.to_string(),
            asset_type,
            Some(frequency),
            "UTC".to_string(),
        );
        
        // 根据频率选择数据源
        match frequency {
            Frequency::Minute(_) => {
                let time_range = TimeRange::new(start_time, end_time);
                let minute_bars = self.market_data_repo.get_minute_bars(instrument_id, time_range, limit).await?;
                
                // 转换为 OHLCV 点并添加到时间序列
                for bar in minute_bars {
                    time_series.add_point(Self::minute_bar_to_ohlcv_point(&bar));
                }
            },
            Frequency::Day => {
                let time_range = TimeRange::new(start_time, end_time);
                let daily_bars = self.market_data_repo.get_daily_bars(instrument_id, time_range).await?;
                
                // 转换为 OHLCV 点并添加到时间序列
                for bar in daily_bars {
                    time_series.add_point(Self::daily_bar_to_ohlcv_point(&bar));
                }
            },
            _ => {
                anyhow::bail!("Unsupported frequency: {:?}", frequency);
            }
        }
        
        Ok(time_series)
    }
    
    async fn load_tick(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<TickPoint>> {
        // 创建 Tick 时间序列
        let mut time_series = TimeSeries::new_tick(
            symbol.to_string(),
            asset_type,
            "UTC".to_string(),
        );
        
        // 获取 Tick 数据
        let time_range = TimeRange::new(start_time, end_time);
        let ticks = self.market_data_repo.get_ticks(instrument_id, time_range, limit).await?;
        
        // 转换为 TickPoint 并添加到时间序列
        for tick in ticks {
            time_series.add_point(Self::tick_to_tick_point(&tick));
        }
        
        Ok(time_series)
    }
    
    async fn load_daily_ohlcv(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        // 创建 TimeSeries 对象
        let mut time_series = TimeSeries::new_ohlcv(
            symbol.to_string(),
            asset_type,
            Some(Frequency::Day),
            "UTC".to_string(),
        );
        
        // 获取日级 OHLCV 数据
        let time_range = TimeRange::new(start_time, end_time);
        let daily_bars = self.market_data_repo.get_daily_bars(instrument_id, time_range).await?;
        
        // 转换为 OHLCV 点并添加到时间序列
        for bar in daily_bars {
            time_series.add_point(Self::daily_bar_to_ohlcv_point(&bar));
        }
        
        Ok(time_series)
    }
    
    async fn load_technical_indicator(
        &self,
        instrument_id: i32,
        indicator_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<InstrumentDailyIndicator>> {
        let time_range = TimeRange::new(start_time, end_time);
        let indicators = self.market_data_repo.get_instrument_daily_indicators(
            instrument_id,
            indicator_id,
            time_range,
        ).await?;
        
        Ok(indicators)
    }
    
    async fn load_fundamental_indicator(
        &self,
        instrument_id: i32,
        indicator_type: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<FundamentalIndicator>> {
        let time_range = TimeRange::new(start_time, end_time);
        let indicators = self.market_data_repo.get_fundamental_indicators(
            instrument_id,
            indicator_type,
            time_range,
        ).await?;
        
        Ok(indicators)
    }
    
    async fn get_data_time_range(
        &self,
        instrument_id: i32,
        data_type: DataType,
    ) -> Result<Option<TimeRange>> {
        // 使用專用的方法來獲取不同數據類型的時間範圍
        match data_type {
            DataType::OHLCV => {
                self.market_data_repo.get_ohlcv_time_range_for_stock(instrument_id).await
            },
            DataType::Tick => {
                self.market_data_repo.get_tick_time_range_for_stock(instrument_id).await
            },
            _ => {
                // 對於其他數據類型，返回 None
                Ok(None)
            }
        }
    }
}

/// 快取數據加載器，增強數據加載性能
pub struct CachedDataLoader {
    inner: Arc<dyn DataLoader>,
    // 這裡可以添加快取實現，如 LRU 快取
    // cache: Arc<RwLock<LruCache<CacheKey, CacheValue>>>,
}

impl CachedDataLoader {
    /// 創建新的快取數據加載器
    pub fn new(inner: Arc<dyn DataLoader>) -> Self {
        Self {
            inner,
            // 初始化快取
        }
    }
}

#[async_trait]
impl DataLoader for CachedDataLoader {
    // 實現所有方法，加入快取邏輯
    async fn load_ohlcv(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        frequency: Frequency,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        // 這裡可以先檢查快取，如果快取中有數據則直接返回
        // 否則調用內部 loader 並將結果存入快取
        
        // 目前先直接轉發到內部 loader
        self.inner.load_ohlcv(
            instrument_id,
            symbol,
            asset_type,
            frequency,
            start_time,
            end_time,
            limit,
        ).await
    }
    
    async fn load_tick(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<TickPoint>> {
        self.inner.load_tick(
            instrument_id,
            symbol,
            asset_type,
            start_time,
            end_time,
            limit,
        ).await
    }
    
    async fn load_daily_ohlcv(
        &self,
        instrument_id: i32,
        symbol: &str,
        asset_type: AssetType,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        self.inner.load_daily_ohlcv(
            instrument_id,
            symbol,
            asset_type,
            start_time,
            end_time,
        ).await
    }
    
    async fn load_technical_indicator(
        &self,
        instrument_id: i32,
        indicator_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<InstrumentDailyIndicator>> {
        self.inner.load_technical_indicator(
            instrument_id,
            indicator_id,
            start_time,
            end_time,
        ).await
    }
    
    async fn load_fundamental_indicator(
        &self,
        instrument_id: i32,
        indicator_type: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<FundamentalIndicator>> {
        self.inner.load_fundamental_indicator(
            instrument_id,
            indicator_type,
            start_time,
            end_time,
        ).await
    }
    
    async fn get_data_time_range(
        &self,
        instrument_id: i32,
        data_type: DataType,
    ) -> Result<Option<TimeRange>> {
        self.inner.get_data_time_range(instrument_id, data_type).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    
    // Note: For actual testing, you would use a more efficient mocking approach.
    // For now, we'll comment out the mocking tests since we need to set up the proper test environment.
}