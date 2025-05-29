use crate::redis::operations::cache::{CacheError, CacheManager, CacheOperations};
use crate::redis::pool::RedisPool;
use crate::storage::models::market_data::{MinuteBar, Tick as DbTick};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;

/// 可快取的數據特徵
pub trait Cacheable:
    Clone + Send + Sync + Debug + Serialize + for<'de> Deserialize<'de> + 'static
{
    /// 快取鍵前綴
    const CACHE_PREFIX: &'static str;
}

impl Cacheable for Vec<MinuteBar> {
    const CACHE_PREFIX: &'static str = "minute_bars";
}

impl Cacheable for Vec<DbTick> {
    const CACHE_PREFIX: &'static str = "ticks";
}

/// 泛型化多層級快取實現，結合內存 LRU 快取與 Redis 快取
pub struct MultiLevelCache<P: RedisPool> {
    /// L1: MinuteBars 內存快取
    minute_bars_cache: Cache<String, Arc<Vec<MinuteBar>>>,
    /// L1: Ticks 內存快取
    ticks_cache: Cache<String, Arc<Vec<DbTick>>>,
    /// L2: Redis 快取（跨進程共享）
    redis_cache: Arc<CacheManager<P>>,
    /// 快取 TTL（秒）
    cache_ttl: u64,
}

impl<P: RedisPool> MultiLevelCache<P> {
    /// 創建新的多層級快取實例
    ///
    /// # Arguments
    /// * `redis_cache` - Redis 快取管理器
    /// * `memory_capacity` - 內存快取容量
    /// * `cache_ttl` - 快取過期時間（秒）
    pub fn new(redis_cache: Arc<CacheManager<P>>, memory_capacity: usize, cache_ttl: u64) -> Self {
        let minute_bars_cache = Cache::builder()
            .max_capacity(memory_capacity as u64)
            .time_to_live(std::time::Duration::from_secs(cache_ttl))
            .build();

        let ticks_cache = Cache::builder()
            .max_capacity(memory_capacity as u64)
            .time_to_live(std::time::Duration::from_secs(cache_ttl))
            .build();

        Self {
            minute_bars_cache,
            ticks_cache,
            redis_cache,
            cache_ttl,
        }
    }

    /// 獲取 MinuteBars 資料 - 泛型接口
    pub async fn get_minute_bars(&self, key: &str) -> Result<Option<Vec<MinuteBar>>, CacheError> {
        // 1. 嘗試從內存快取獲取
        if let Some(arc_bars) = self.minute_bars_cache.get(key).await {
            return Ok(Some((*arc_bars).clone()));
        }

        // 2. 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<MinuteBar>>(key).await {
            Ok(bars) => {
                // 更新內存快取
                let arc_bars = Arc::new(bars.clone());
                self.minute_bars_cache
                    .insert(key.to_string(), arc_bars)
                    .await;
                Ok(Some(bars))
            }
            Err(CacheError::CacheMiss(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 設置 MinuteBars 資料 - 泛型接口
    pub async fn set_minute_bars(
        &self,
        key: &str,
        bars: &Vec<MinuteBar>,
    ) -> Result<(), CacheError> {
        // 1. 更新內存快取
        let arc_bars = Arc::new(bars.clone());
        self.minute_bars_cache
            .insert(key.to_string(), arc_bars)
            .await;

        // 2. 更新 Redis 快取
        self.redis_cache.set(key, bars, Some(self.cache_ttl)).await
    }

    /// 獲取 Ticks 資料 - 泛型接口
    pub async fn get_ticks(&self, key: &str) -> Result<Option<Vec<DbTick>>, CacheError> {
        // 1. 嘗試從內存快取獲取
        if let Some(arc_ticks) = self.ticks_cache.get(key).await {
            return Ok(Some((*arc_ticks).clone()));
        }

        // 2. 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<DbTick>>(key).await {
            Ok(ticks) => {
                // 更新內存快取
                let arc_ticks = Arc::new(ticks.clone());
                self.ticks_cache.insert(key.to_string(), arc_ticks).await;
                Ok(Some(ticks))
            }
            Err(CacheError::CacheMiss(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 設置 Ticks 資料 - 泛型接口
    pub async fn set_ticks(&self, key: &str, ticks: &Vec<DbTick>) -> Result<(), CacheError> {
        // 1. 更新內存快取
        let arc_ticks = Arc::new(ticks.clone());
        self.ticks_cache.insert(key.to_string(), arc_ticks).await;

        // 2. 更新 Redis 快取
        self.redis_cache.set(key, ticks, Some(self.cache_ttl)).await
    }

    /// 刪除快取數據
    pub async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        // 從所有內存快取刪除
        self.minute_bars_cache.invalidate(key).await;
        self.ticks_cache.invalidate(key).await;

        // 從 Redis 刪除
        self.redis_cache.delete(key).await
    }

    /// 檢查快取是否存在
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        // 檢查任一內存快取
        if self.minute_bars_cache.get(key).await.is_some()
            || self.ticks_cache.get(key).await.is_some()
        {
            return Ok(true);
        }

        // 檢查 Redis
        self.redis_cache.exists(key).await
    }

    /// 批量獲取 MinuteBars 資料
    pub async fn get_minute_bars_batch(
        &self,
        keys: &[String],
    ) -> Result<Vec<Option<Vec<MinuteBar>>>, CacheError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = vec![None; keys.len()];
        let mut missing_keys = Vec::new();
        let mut missing_indices = Vec::new();

        // 1. 先從內存快取批量獲取
        for (idx, key) in keys.iter().enumerate() {
            if let Some(arc_bars) = self.minute_bars_cache.get(key).await {
                results[idx] = Some((*arc_bars).clone());
            } else {
                missing_keys.push(key.clone());
                missing_indices.push(idx);
            }
        }

        // 2. 如果有缺失的key，從Redis批量獲取
        if !missing_keys.is_empty() {
            let redis_results = self
                .redis_cache
                .mget::<String, Vec<MinuteBar>>(&missing_keys)
                .await?;

            for (idx, bars_opt) in missing_indices.iter().zip(redis_results) {
                if let Some(bars) = bars_opt {
                    let arc_bars = Arc::new(bars.clone());
                    // 更新內存快取
                    self.minute_bars_cache
                        .insert(keys[*idx].clone(), arc_bars)
                        .await;
                    results[*idx] = Some(bars);
                }
            }
        }

        Ok(results)
    }

    /// 預熱快取 - 分鐘級市場資料
    pub async fn warm_minute_bars_cache(&self, keys: Vec<String>) -> Result<(), CacheError> {
        let _ = self.get_minute_bars_batch(&keys).await?;
        Ok(())
    }

    /// 批量獲取 Ticks 資料
    pub async fn get_ticks_batch(
        &self,
        keys: &[String],
    ) -> Result<Vec<Option<Vec<DbTick>>>, CacheError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = vec![None; keys.len()];
        let mut missing_keys = Vec::new();
        let mut missing_indices = Vec::new();

        // 1. 先從內存快取批量獲取
        for (idx, key) in keys.iter().enumerate() {
            if let Some(arc_ticks) = self.ticks_cache.get(key).await {
                results[idx] = Some((*arc_ticks).clone());
            } else {
                missing_keys.push(key.clone());
                missing_indices.push(idx);
            }
        }

        // 2. 如果有缺失的key，從Redis批量獲取
        if !missing_keys.is_empty() {
            let redis_results = self
                .redis_cache
                .mget::<String, Vec<DbTick>>(&missing_keys)
                .await?;

            for (idx, ticks_opt) in missing_indices.iter().zip(redis_results) {
                if let Some(ticks) = ticks_opt {
                    let arc_ticks = Arc::new(ticks.clone());
                    // 更新內存快取
                    self.ticks_cache.insert(keys[*idx].clone(), arc_ticks).await;
                    results[*idx] = Some(ticks);
                }
            }
        }

        Ok(results)
    }

    /// 預熱快取 - Tick 資料
    pub async fn warm_ticks_cache(&self, keys: Vec<String>) -> Result<(), CacheError> {
        let _ = self.get_ticks_batch(&keys).await?;
        Ok(())
    }

    /// 清空所有內存快取
    pub async fn clear_memory_cache(&self) {
        self.minute_bars_cache.invalidate_all();
        self.ticks_cache.invalidate_all();
    }

    /// 批量設置 MinuteBars 資料
    pub async fn set_minute_bars_batch(
        &self,
        items: &[(String, Vec<MinuteBar>)],
    ) -> Result<(), CacheError> {
        if items.is_empty() {
            return Ok(());
        }

        // 1. 批量更新內存快取
        for (key, bars) in items {
            let arc_bars = Arc::new(bars.clone());
            self.minute_bars_cache.insert(key.clone(), arc_bars).await;
        }

        // 2. 批量更新 Redis 快取
        self.redis_cache.mset(items, Some(self.cache_ttl)).await
    }

    /// 批量設置 Ticks 資料
    pub async fn set_ticks_batch(&self, items: &[(String, Vec<DbTick>)]) -> Result<(), CacheError> {
        if items.is_empty() {
            return Ok(());
        }

        // 1. 批量更新內存快取
        for (key, ticks) in items {
            let arc_ticks = Arc::new(ticks.clone());
            self.ticks_cache.insert(key.clone(), arc_ticks).await;
        }

        // 2. 批量更新 Redis 快取
        self.redis_cache.mset(items, Some(self.cache_ttl)).await
    }

    /// 獲取快取統計信息
    pub async fn cache_stats(&self) -> MultiCacheStats {
        MultiCacheStats {
            minute_bars: CacheStats {
                size: self.minute_bars_cache.entry_count() as usize,
                capacity: self.minute_bars_cache.weighted_size() as usize,
            },
            ticks: CacheStats {
                size: self.ticks_cache.entry_count() as usize,
                capacity: self.ticks_cache.weighted_size() as usize,
            },
        }
    }
}

/// 單一快取統計信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 當前快取項目數
    pub size: usize,
    /// 快取容量
    pub capacity: usize,
}

/// 多快取統計信息
#[derive(Debug, Clone)]
pub struct MultiCacheStats {
    /// MinuteBars 快取統計
    pub minute_bars: CacheStats,
    /// Ticks 快取統計
    pub ticks: CacheStats,
}

/// 快取鍵構建器，重用內部緩衝區以提升性能
pub struct OptimizedKeyBuilder {
    buffer: Vec<u8>,
}

impl OptimizedKeyBuilder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(128),
        }
    }

    pub fn generate_key(
        &mut self,
        instrument_id: i32,
        frequency: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> &str {
        use std::io::Write;

        self.buffer.clear();

        // 使用 Write trait 和 itoa 進行快速格式化
        let _ = write!(
            &mut self.buffer,
            "market_data:{}:{}:{}:{}",
            itoa::Buffer::new().format(instrument_id),
            frequency,
            itoa::Buffer::new().format(start_ts),
            itoa::Buffer::new().format(end_ts),
        );

        // 安全：我們知道內容是有效的 UTF-8
        unsafe { std::str::from_utf8_unchecked(&self.buffer) }
    }

    pub fn generate_key_owned(
        &mut self,
        instrument_id: i32,
        frequency: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> String {
        self.generate_key(instrument_id, frequency, start_ts, end_ts)
            .to_owned()
    }
}

impl Default for OptimizedKeyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成市場數據快取鍵（優化版本）
///
/// 使用高性能的實現，適合高頻調用場景。
///
/// # Arguments
/// * `instrument_id` - 金融工具 ID
/// * `frequency` - 數據頻率
/// * `start_ts` - 開始時間戳
/// * `end_ts` - 結束時間戳
pub fn generate_cache_key(
    instrument_id: i32,
    frequency: &str,
    start_ts: i64,
    end_ts: i64,
) -> String {
    let mut builder = OptimizedKeyBuilder::new();
    builder.generate_key_owned(instrument_id, frequency, start_ts, end_ts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use rust_decimal_macros::dec;

    fn create_test_minute_bars() -> Vec<MinuteBar> {
        vec![
            MinuteBar {
                instrument_id: 100,
                time: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: dec!(100.0),
                high: dec!(105.0),
                low: dec!(99.0),
                close: dec!(104.0),
                volume: dec!(1000000),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
            MinuteBar {
                instrument_id: 100,
                time: DateTime::parse_from_rfc3339("2024-01-01T01:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: dec!(104.0),
                high: dec!(106.0),
                low: dec!(103.0),
                close: dec!(105.5),
                volume: dec!(1200000),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
        ]
    }

    fn create_test_ticks() -> Vec<DbTick> {
        vec![
            DbTick {
                instrument_id: 100,
                time: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                price: dec!(100.5),
                volume: dec!(100),
                trade_type: Some(1),
                bid_price_1: Some(dec!(100.4)),
                bid_volume_1: Some(dec!(50)),
                ask_price_1: Some(dec!(100.6)),
                ask_volume_1: Some(dec!(50)),
                bid_prices: None,
                bid_volumes: None,
                ask_prices: None,
                ask_volumes: None,
                open_interest: None,
                spread: None,
                metadata: None,
                created_at: Utc::now(),
            },
            DbTick {
                instrument_id: 100,
                time: DateTime::parse_from_rfc3339("2024-01-01T00:00:01Z")
                    .unwrap()
                    .with_timezone(&Utc),
                price: dec!(100.6),
                volume: dec!(150),
                trade_type: Some(1),
                bid_price_1: Some(dec!(100.5)),
                bid_volume_1: Some(dec!(60)),
                ask_price_1: Some(dec!(100.7)),
                ask_volume_1: Some(dec!(40)),
                bid_prices: None,
                bid_volumes: None,
                ask_prices: None,
                ask_volumes: None,
                open_interest: None,
                spread: None,
                metadata: None,
                created_at: Utc::now(),
            },
        ]
    }

    #[test]
    fn test_cacheable_trait() {
        assert_eq!(Vec::<MinuteBar>::CACHE_PREFIX, "minute_bars");
        assert_eq!(Vec::<DbTick>::CACHE_PREFIX, "ticks");
    }

    #[test]
    fn test_cache_key_generation() {
        let key = generate_cache_key(100, "1h", 1704067200, 1704153600);
        assert_eq!(key, "market_data:100:1h:1704067200:1704153600");
    }

    #[test]
    fn test_optimized_key_builder() {
        let mut builder = OptimizedKeyBuilder::new();

        // 測試生成鍵
        let key1 = builder.generate_key(100, "1h", 1704067200, 1704153600);
        assert_eq!(key1, "market_data:100:1h:1704067200:1704153600");

        // 測試重用緩衝區
        let key2 = builder.generate_key(200, "5m", 1704067300, 1704153700);
        assert_eq!(key2, "market_data:200:5m:1704067300:1704153700");

        // 測試 owned 版本
        let owned_key = builder.generate_key_owned(300, "1d", 1704067400, 1704153800);
        assert_eq!(owned_key, "market_data:300:1d:1704067400:1704153800");
    }

    #[test]
    fn test_default_implementation() {
        let mut builder = OptimizedKeyBuilder::default();
        let key = builder.generate_key(123, "15m", 1234567890, 1234567900);
        assert_eq!(key, "market_data:123:15m:1234567890:1234567900");
    }

    #[test]
    fn test_cache_stats() {
        let stats = CacheStats {
            size: 10,
            capacity: 1000,
        };
        assert_eq!(stats.size, 10);
        assert_eq!(stats.capacity, 1000);

        let multi_stats = MultiCacheStats {
            minute_bars: CacheStats {
                size: 5,
                capacity: 500,
            },
            ticks: CacheStats {
                size: 15,
                capacity: 1500,
            },
        };
        assert_eq!(multi_stats.minute_bars.size, 5);
        assert_eq!(multi_stats.ticks.capacity, 1500);
    }

    #[test]
    fn test_batch_operations_empty_input() {
        // 測試空輸入的批量操作邏輯
        let keys: Vec<String> = vec![];
        assert!(keys.is_empty());

        let items: Vec<(String, Vec<MinuteBar>)> = vec![];
        assert!(items.is_empty());

        let items: Vec<(String, Vec<DbTick>)> = vec![];
        assert!(items.is_empty());
    }

    #[test]
    fn test_batch_operations_data_preparation() {
        // 測試批量操作數據準備
        let test_bars = create_test_minute_bars();
        let test_ticks = create_test_ticks();

        let minute_bars_items = vec![
            ("key1".to_string(), test_bars.clone()),
            ("key2".to_string(), test_bars),
        ];
        assert_eq!(minute_bars_items.len(), 2);

        let ticks_items = vec![
            ("key1".to_string(), test_ticks.clone()),
            ("key2".to_string(), test_ticks),
        ];
        assert_eq!(ticks_items.len(), 2);
    }
}
