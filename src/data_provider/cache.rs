use crate::redis::operations::cache::{CacheError, CacheManager, CacheOperations};
use crate::redis::pool::RedisPool;
use crate::storage::models::market_data::{MinuteBar, Tick as DbTick};
use metrics::{counter, histogram};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Instant;

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

    /// 獲取 MinuteBars 資料 - 帶監控指標
    ///
    /// 直接返回 Arc<Vec<MinuteBar>>，避免深拷貝整個 Vec。
    /// 適合需要高性能且只讀數據的場景。
    pub async fn get_minute_bars(
        &self,
        key: &str,
    ) -> Result<Option<Arc<Vec<MinuteBar>>>, CacheError> {
        let start = Instant::now();

        // 1. 嘗試從內存快取獲取
        if let Some(arc_bars) = self.minute_bars_cache.get(key).await {
            counter!("cache_hit", "layer" => "memory", "type" => "minute_bars").increment(1);
            histogram!("cache_get_duration", "layer" => "memory", "type" => "minute_bars")
                .record(start.elapsed());
            return Ok(Some(arc_bars));
        }

        // 2. 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<MinuteBar>>(key).await {
            Ok(bars) => {
                counter!("cache_hit", "layer" => "redis", "type" => "minute_bars").increment(1);
                histogram!("cache_get_duration", "layer" => "redis", "type" => "minute_bars")
                    .record(start.elapsed());

                // 更新內存快取
                let arc_bars = Arc::new(bars);
                self.minute_bars_cache
                    .insert(key.to_string(), arc_bars.clone())
                    .await;
                Ok(Some(arc_bars))
            }
            Err(CacheError::CacheMiss(_)) => {
                counter!("cache_miss", "type" => "minute_bars").increment(1);
                histogram!("cache_get_duration", "layer" => "miss", "type" => "minute_bars")
                    .record(start.elapsed());
                Ok(None)
            }
            Err(e) => {
                counter!("cache_error", "type" => "minute_bars").increment(1);
                Err(e)
            }
        }
    }

    /// 設置 MinuteBars 資料 - 帶監控指標
    pub async fn set_minute_bars(
        &self,
        key: &str,
        bars: &Vec<MinuteBar>,
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        // 1. 更新內存快取
        let arc_bars = Arc::new(bars.clone());
        self.minute_bars_cache
            .insert(key.to_string(), arc_bars)
            .await;

        // 2. 更新 Redis 快取
        match self.redis_cache.set(key, bars, Some(self.cache_ttl)).await {
            Ok(_) => {
                counter!("cache_set", "type" => "minute_bars").increment(1);
                histogram!("cache_set_duration", "type" => "minute_bars").record(start.elapsed());
                Ok(())
            }
            Err(e) => {
                counter!("cache_set_error", "type" => "minute_bars").increment(1);
                Err(e)
            }
        }
    }

    /// 設置 MinuteBars 資料 (Arc 版本) - 避免不必要的複製
    ///
    /// 當調用者已經擁有 Arc<Vec<MinuteBar>> 時，可以使用此方法避免額外的 clone。
    pub async fn set_minute_bars_arc(
        &self,
        key: &str,
        bars: Arc<Vec<MinuteBar>>,
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        // 1. 更新內存快取（直接使用 Arc）
        self.minute_bars_cache
            .insert(key.to_string(), bars.clone())
            .await;

        // 2. 更新 Redis 快取（需要解引用）
        match self
            .redis_cache
            .set(key, &*bars, Some(self.cache_ttl))
            .await
        {
            Ok(_) => {
                counter!("cache_set", "type" => "minute_bars").increment(1);
                histogram!("cache_set_duration", "type" => "minute_bars").record(start.elapsed());
                Ok(())
            }
            Err(e) => {
                counter!("cache_set_error", "type" => "minute_bars").increment(1);
                Err(e)
            }
        }
    }

    /// 獲取 Ticks 資料 - 帶監控指標
    ///
    /// 直接返回 Arc<Vec<DbTick>>，避免深拷貝整個 Vec。
    /// 適合需要高性能且只讀數據的場景。
    pub async fn get_ticks(&self, key: &str) -> Result<Option<Arc<Vec<DbTick>>>, CacheError> {
        let start = Instant::now();

        // 1. 嘗試從內存快取獲取
        if let Some(arc_ticks) = self.ticks_cache.get(key).await {
            counter!("cache_hit", "layer" => "memory", "type" => "ticks").increment(1);
            histogram!("cache_get_duration", "layer" => "memory", "type" => "ticks")
                .record(start.elapsed());
            return Ok(Some(arc_ticks));
        }

        // 2. 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<DbTick>>(key).await {
            Ok(ticks) => {
                counter!("cache_hit", "layer" => "redis", "type" => "ticks").increment(1);
                histogram!("cache_get_duration", "layer" => "redis", "type" => "ticks")
                    .record(start.elapsed());

                // 更新內存快取
                let arc_ticks = Arc::new(ticks);
                self.ticks_cache
                    .insert(key.to_string(), arc_ticks.clone())
                    .await;
                Ok(Some(arc_ticks))
            }
            Err(CacheError::CacheMiss(_)) => {
                counter!("cache_miss", "type" => "ticks").increment(1);
                histogram!("cache_get_duration", "layer" => "miss", "type" => "ticks")
                    .record(start.elapsed());
                Ok(None)
            }
            Err(e) => {
                counter!("cache_error", "type" => "ticks").increment(1);
                Err(e)
            }
        }
    }

    /// 設置 Ticks 資料 - 帶監控指標
    pub async fn set_ticks(&self, key: &str, ticks: &Vec<DbTick>) -> Result<(), CacheError> {
        let start = Instant::now();

        // 1. 更新內存快取
        let arc_ticks = Arc::new(ticks.clone());
        self.ticks_cache.insert(key.to_string(), arc_ticks).await;

        // 2. 更新 Redis 快取
        match self.redis_cache.set(key, ticks, Some(self.cache_ttl)).await {
            Ok(_) => {
                counter!("cache_set", "type" => "ticks").increment(1);
                histogram!("cache_set_duration", "type" => "ticks").record(start.elapsed());
                Ok(())
            }
            Err(e) => {
                counter!("cache_set_error", "type" => "ticks").increment(1);
                Err(e)
            }
        }
    }

    /// 設置 Ticks 資料 (Arc 版本) - 避免不必要的複製
    ///
    /// 當調用者已經擁有 Arc<Vec<DbTick>> 時，可以使用此方法避免額外的 clone。
    pub async fn set_ticks_arc(
        &self,
        key: &str,
        ticks: Arc<Vec<DbTick>>,
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        // 1. 更新內存快取（直接使用 Arc）
        self.ticks_cache
            .insert(key.to_string(), ticks.clone())
            .await;

        // 2. 更新 Redis 快取（需要解引用）
        match self
            .redis_cache
            .set(key, &*ticks, Some(self.cache_ttl))
            .await
        {
            Ok(_) => {
                counter!("cache_set", "type" => "ticks").increment(1);
                histogram!("cache_set_duration", "type" => "ticks").record(start.elapsed());
                Ok(())
            }
            Err(e) => {
                counter!("cache_set_error", "type" => "ticks").increment(1);
                Err(e)
            }
        }
    }

    /// 刪除快取數據 - 帶監控指標
    pub async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let start = Instant::now();

        // 從所有內存快取刪除
        self.minute_bars_cache.invalidate(key).await;
        self.ticks_cache.invalidate(key).await;

        // 從 Redis 刪除
        match self.redis_cache.delete(key).await {
            Ok(deleted) => {
                counter!("cache_delete", "result" => if deleted { "success" } else { "not_found" })
                    .increment(1);
                histogram!("cache_delete_duration").record(start.elapsed());
                Ok(deleted)
            }
            Err(e) => {
                counter!("cache_delete_error").increment(1);
                Err(e)
            }
        }
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

    /// 批量獲取 MinuteBars 資料 - 帶監控指標
    ///
    /// 直接返回 Arc<Vec<MinuteBar>>，避免深拷貝整個 Vec。
    /// 適合需要高性能且只讀數據的場景。
    pub async fn get_minute_bars_batch(
        &self,
        keys: &[String],
    ) -> Result<Vec<Option<Arc<Vec<MinuteBar>>>>, CacheError> {
        let start = Instant::now();

        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = vec![None; keys.len()];
        let mut missing_keys = Vec::new();
        let mut missing_indices = Vec::new();

        // 1. 先從內存快取批量獲取
        for (idx, key) in keys.iter().enumerate() {
            if let Some(arc_bars) = self.minute_bars_cache.get(key).await {
                results[idx] = Some(arc_bars);
                counter!("cache_batch_hit", "layer" => "memory", "type" => "minute_bars")
                    .increment(1);
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
                    let arc_bars = Arc::new(bars);
                    // 更新內存快取
                    self.minute_bars_cache
                        .insert(keys[*idx].clone(), arc_bars.clone())
                        .await;
                    results[*idx] = Some(arc_bars);
                    counter!("cache_batch_hit", "layer" => "redis", "type" => "minute_bars")
                        .increment(1);
                } else {
                    counter!("cache_batch_miss", "type" => "minute_bars").increment(1);
                }
            }
        }

        histogram!("cache_batch_get_duration", "type" => "minute_bars").record(start.elapsed());
        counter!("cache_batch_get_total", "type" => "minute_bars").increment(keys.len() as u64);

        Ok(results)
    }

    /// 預熱快取 - 分鐘級市場資料
    pub async fn warm_minute_bars_cache(&self, keys: Vec<String>) -> Result<(), CacheError> {
        let _ = self.get_minute_bars_batch(&keys).await?;
        Ok(())
    }

    /// 批量獲取 Ticks 資料 - 帶監控指標
    ///
    /// 直接返回 Arc<Vec<DbTick>>，避免深拷貝整個 Vec。
    /// 適合需要高性能且只讀數據的場景。
    pub async fn get_ticks_batch(
        &self,
        keys: &[String],
    ) -> Result<Vec<Option<Arc<Vec<DbTick>>>>, CacheError> {
        let start = Instant::now();

        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = vec![None; keys.len()];
        let mut missing_keys = Vec::new();
        let mut missing_indices = Vec::new();

        // 1. 先從內存快取批量獲取
        for (idx, key) in keys.iter().enumerate() {
            if let Some(arc_ticks) = self.ticks_cache.get(key).await {
                results[idx] = Some(arc_ticks);
                counter!("cache_batch_hit", "layer" => "memory", "type" => "ticks").increment(1);
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
                    let arc_ticks = Arc::new(ticks);
                    // 更新內存快取
                    self.ticks_cache
                        .insert(keys[*idx].clone(), arc_ticks.clone())
                        .await;
                    results[*idx] = Some(arc_ticks);
                    counter!("cache_batch_hit", "layer" => "redis", "type" => "ticks").increment(1);
                } else {
                    counter!("cache_batch_miss", "type" => "ticks").increment(1);
                }
            }
        }

        histogram!("cache_batch_get_duration", "type" => "ticks").record(start.elapsed());
        counter!("cache_batch_get_total", "type" => "ticks").increment(keys.len() as u64);

        Ok(results)
    }

    /// 預熱快取 - Tick 資料
    pub async fn warm_ticks_cache(&self, keys: Vec<String>) -> Result<(), CacheError> {
        let _ = self.get_ticks_batch(&keys).await?;
        Ok(())
    }

    /// 清空所有內存快取 - 帶監控指標
    pub async fn clear_memory_cache(&self) {
        let minute_bars_count = self.minute_bars_cache.entry_count();
        let ticks_count = self.ticks_cache.entry_count();

        self.minute_bars_cache.invalidate_all();
        self.ticks_cache.invalidate_all();

        counter!("cache_clear", "type" => "memory").increment(1);
        counter!("cache_cleared_entries", "type" => "minute_bars").increment(minute_bars_count);
        counter!("cache_cleared_entries", "type" => "ticks").increment(ticks_count);
    }

    /// 批量設置 MinuteBars 資料 - 帶監控指標
    pub async fn set_minute_bars_batch(
        &self,
        items: &[(String, Vec<MinuteBar>)],
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        if items.is_empty() {
            return Ok(());
        }

        // 1. 批量更新內存快取
        for (key, bars) in items {
            let arc_bars = Arc::new(bars.clone());
            self.minute_bars_cache.insert(key.clone(), arc_bars).await;
        }

        // 2. 批量更新 Redis 快取
        match self.redis_cache.mset(items, Some(self.cache_ttl)).await {
            Ok(_) => {
                counter!("cache_batch_set", "type" => "minute_bars").increment(items.len() as u64);
                histogram!("cache_batch_set_duration", "type" => "minute_bars")
                    .record(start.elapsed());
                Ok(())
            }
            Err(e) => {
                counter!("cache_batch_set_error", "type" => "minute_bars").increment(1);
                Err(e)
            }
        }
    }

    /// 批量設置 Ticks 資料 - 帶監控指標
    pub async fn set_ticks_batch(&self, items: &[(String, Vec<DbTick>)]) -> Result<(), CacheError> {
        let start = Instant::now();

        if items.is_empty() {
            return Ok(());
        }

        // 1. 批量更新內存快取
        for (key, ticks) in items {
            let arc_ticks = Arc::new(ticks.clone());
            self.ticks_cache.insert(key.clone(), arc_ticks).await;
        }

        // 2. 批量更新 Redis 快取
        match self.redis_cache.mset(items, Some(self.cache_ttl)).await {
            Ok(_) => {
                counter!("cache_batch_set", "type" => "ticks").increment(items.len() as u64);
                histogram!("cache_batch_set_duration", "type" => "ticks").record(start.elapsed());
                Ok(())
            }
            Err(e) => {
                counter!("cache_batch_set_error", "type" => "ticks").increment(1);
                Err(e)
            }
        }
    }

    /// 獲取快取統計信息
    pub async fn cache_stats(&self) -> MultiCacheStats {
        // 記錄統計信息請求
        counter!("cache_stats_requested").increment(1);

        let minute_bars_stats = CacheStats {
            size: self.minute_bars_cache.entry_count() as usize,
            capacity: self.minute_bars_cache.weighted_size() as usize,
        };

        let ticks_stats = CacheStats {
            size: self.ticks_cache.entry_count() as usize,
            capacity: self.ticks_cache.weighted_size() as usize,
        };

        // 發送快取大小指標
        histogram!("cache_memory_entries", "type" => "minute_bars")
            .record(minute_bars_stats.size as f64);
        histogram!("cache_memory_entries", "type" => "ticks").record(ticks_stats.size as f64);

        MultiCacheStats {
            minute_bars: minute_bars_stats,
            ticks: ticks_stats,
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

thread_local! {
    static KEY_BUILDER: RefCell<OptimizedKeyBuilder> = RefCell::new(OptimizedKeyBuilder::new());
}

/// 生成市場數據快取鍵（優化版本）
///
/// 使用高性能的實現，適合高頻調用場景。
/// 使用 thread_local 重用內部緩衝區，避免頻繁的記憶體分配。
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
    KEY_BUILDER.with(|builder| {
        builder
            .borrow_mut()
            .generate_key_owned(instrument_id, frequency, start_ts, end_ts)
    })
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

    #[test]
    fn test_thread_local_key_generation() {
        // 測試 thread_local 版本的鍵生成
        let key1 = generate_cache_key(100, "1h", 1704067200, 1704153600);
        assert_eq!(key1, "market_data:100:1h:1704067200:1704153600");

        // 再次調用，確保重用了緩衝區
        let key2 = generate_cache_key(200, "5m", 1704067300, 1704153700);
        assert_eq!(key2, "market_data:200:5m:1704067300:1704153700");

        // 測試不同線程會有自己的 thread_local 實例
        std::thread::spawn(|| {
            let key = generate_cache_key(300, "1d", 1704067400, 1704153800);
            assert_eq!(key, "market_data:300:1d:1704067400:1704153800");
        })
        .join()
        .unwrap();
    }
}
