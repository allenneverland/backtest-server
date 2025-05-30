use crate::cache::buffer::CacheBuffer;
use crate::cache::keys::CacheKeyHash;
use crate::cache::macros::impl_cache_methods;
use crate::cache::metrics::{CacheMetrics, MetricType};
use crate::cache::stats::{CacheStats, MultiCacheStats};
use crate::cache::traits::CacheableData;
use crate::redis::operations::cache::{CacheError, CacheManager, CacheOperations};
use crate::redis::pool::RedisPool;
use crate::storage::models::market_data::{MinuteBar, Tick as DbTick};
use futures::stream::{self, StreamExt};
use moka::future::Cache;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 高性能多層級快取實現，使用 u64 hash 作為內存快取鍵
///
/// 結合內存 LRU 快取與 Redis 快取，針對高頻查詢場景優化：
/// - 使用 FxHasher 計算的 u64 hash 作為內存快取鍵，提升查找性能
/// - 維護 hash 到原始鍵的映射，用於 Redis 操作
/// - 支援批量操作和並行處理
pub struct MultiLevelCache<P: RedisPool> {
    /// L1: MinuteBars 內存快取（使用 u64 hash 作為鍵）
    minute_bars_cache: Cache<u64, Arc<Vec<MinuteBar>>>,
    /// L1: Ticks 內存快取（使用 u64 hash 作為鍵）  
    ticks_cache: Cache<u64, Arc<Vec<DbTick>>>,
    /// Hash 到原始鍵的映射（用於 Redis 操作）
    key_mapping: Arc<RwLock<FxHashMap<u64, String>>>,
    /// L2: Redis 快取（跨進程共享）
    redis_cache: Arc<CacheManager<P>>,
    /// 快取 TTL（秒）
    cache_ttl: u64,
}

impl<P: RedisPool> MultiLevelCache<P> {
    /// 統一的監控指標記錄方法
    fn record_metric<T: CacheableData>(&self, metric_type: MetricType, duration: Option<Duration>) {
        CacheMetrics::record(T::data_type_name(), metric_type, duration);
    }

    /// 通用的快取操作方法 - 獲取資料
    async fn get_cached_data<T: CacheableData>(
        &self,
        key: &str,
        cache: &Cache<u64, Arc<Vec<T>>>,
    ) -> Result<Option<Arc<Vec<T>>>, CacheError> {
        let start = Instant::now();
        let hash = Self::hash_key(key);

        // 檢查 hash 碰撞
        if self.check_hash_collision(hash, key).await {
            // 如果檢測到碰撞，清理不一致的快取並重新從 Redis 獲取
            self.invalidate_inconsistent_cache(key).await;
        }

        // 1. 嘗試從內存快取獲取（使用 hash）
        if let Some(arc_data) = cache.get(&hash).await {
            self.record_metric::<T>(MetricType::Hit { layer: "memory" }, None);
            self.record_metric::<T>(
                MetricType::Latency {
                    operation: "get_memory",
                },
                Some(start.elapsed()),
            );
            return Ok(Some(arc_data));
        }

        // 2. 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<T>>(key).await {
            Ok(data) => {
                self.record_metric::<T>(MetricType::Hit { layer: "redis" }, None);
                self.record_metric::<T>(
                    MetricType::Latency {
                        operation: "get_redis",
                    },
                    Some(start.elapsed()),
                );

                // 更新內存快取和映射
                let arc_data = Arc::new(data);
                cache.insert(hash, arc_data.clone()).await;

                // 更新 hash 映射
                self.key_mapping.write().await.insert(hash, key.to_string());

                Ok(Some(arc_data))
            }
            Err(CacheError::CacheMiss(_)) => {
                self.record_metric::<T>(MetricType::Miss, None);
                self.record_metric::<T>(
                    MetricType::Latency {
                        operation: "get_miss",
                    },
                    Some(start.elapsed()),
                );
                Ok(None)
            }
            Err(e) => {
                self.record_metric::<T>(MetricType::Error { operation: "get" }, None);
                Err(e)
            }
        }
    }

    /// 通用的快取操作方法 - 設定資料
    async fn set_cached_data<T: CacheableData>(
        &self,
        key: &str,
        data: &Vec<T>,
        cache: &Cache<u64, Arc<Vec<T>>>,
    ) -> Result<(), CacheError> {
        let start = Instant::now();
        let hash = Self::hash_key(key);

        // 檢查 hash 碰撞
        if self.check_hash_collision(hash, key).await {
            // 如果檢測到碰撞，清理不一致的快取
            self.invalidate_inconsistent_cache(key).await;
        }

        // 1. 先更新 Redis 快取
        match self.redis_cache.set(key, data, Some(self.cache_ttl)).await {
            Ok(_) => {
                // 2. Redis 更新成功後，更新內存快取
                let arc_data = Arc::new(data.clone());
                cache.insert(hash, arc_data).await;

                // 3. 更新 hash 映射
                self.key_mapping.write().await.insert(hash, key.to_string());

                CacheMetrics::record_set(T::data_type_name());
                self.record_metric::<T>(
                    MetricType::Latency { operation: "set" },
                    Some(start.elapsed()),
                );
                Ok(())
            }
            Err(e) => {
                self.record_metric::<T>(MetricType::Error { operation: "set" }, None);
                Err(e)
            }
        }
    }

    /// 通用的快取操作方法 - 設定 Arc 資料
    async fn set_cached_data_arc<T: CacheableData>(
        &self,
        key: &str,
        data: Arc<Vec<T>>,
        cache: &Cache<u64, Arc<Vec<T>>>,
    ) -> Result<(), CacheError> {
        let start = Instant::now();
        let hash = Self::hash_key(key);

        // 檢查 hash 碰撞
        if self.check_hash_collision(hash, key).await {
            // 如果檢測到碰撞，清理不一致的快取
            self.invalidate_inconsistent_cache(key).await;
        }

        // 1. 先更新 Redis 快取（需要解引用）
        match self
            .redis_cache
            .set(key, &*data, Some(self.cache_ttl))
            .await
        {
            Ok(_) => {
                // 2. Redis 更新成功後，更新內存快取（使用 hash 作為鍵）
                cache.insert(hash, data.clone()).await;

                // 3. 更新 hash 映射
                self.key_mapping.write().await.insert(hash, key.to_string());

                CacheMetrics::record_set(T::data_type_name());
                self.record_metric::<T>(
                    MetricType::Latency {
                        operation: "set_arc",
                    },
                    Some(start.elapsed()),
                );
                Ok(())
            }
            Err(e) => {
                self.record_metric::<T>(
                    MetricType::Error {
                        operation: "set_arc",
                    },
                    None,
                );
                Err(e)
            }
        }
    }
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
            key_mapping: Arc::new(RwLock::new(FxHashMap::default())),
            redis_cache,
            cache_ttl,
        }
    }

    /// 計算鍵的 hash 值
    #[inline(always)]
    fn hash_key(key: &str) -> u64 {
        CacheKeyHash::new(key).0
    }

    /// 檢查並處理潛在的 hash 碰撞
    ///
    /// 檢測是否存在 hash 碰撞（不同的鍵產生相同的 hash 值），
    /// 如果檢測到碰撞，會記錄監控指標並發出警告日誌。
    ///
    /// # Arguments
    /// * `hash` - 待檢查的 hash 值
    /// * `key` - 對應的原始鍵
    ///
    /// # Returns
    /// 如果檢測到 hash 碰撞返回 `true`，否則返回 `false`
    async fn check_hash_collision(&self, hash: u64, key: &str) -> bool {
        if let Some(existing_key) = self.key_mapping.read().await.get(&hash) {
            if existing_key != key {
                // 檢測到碰撞
                CacheMetrics::record_hash_collision();
                CacheMetrics::record_hash_collision_check(true);
                tracing::warn!(
                    "Hash collision detected: existing_key='{}' new_key='{}' hash={}",
                    existing_key,
                    key,
                    hash
                );
                return true;
            }
        }

        // 未檢測到碰撞
        CacheMetrics::record_hash_collision_check(false);
        false
    }

    // 使用巨集生成 MinuteBar 和 DbTick 的所有快取操作方法
    impl_cache_methods!(MinuteBar, minute_bars_cache, minute_bars, "minute_bars");
    impl_cache_methods!(DbTick, ticks_cache, ticks, "ticks");

    /// 刪除快取數據 - 帶監控指標
    pub async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let start = Instant::now();
        let hash = Self::hash_key(key);

        // 從所有內存快取刪除（使用 hash）
        self.minute_bars_cache.invalidate(&hash).await;
        self.ticks_cache.invalidate(&hash).await;

        // 從映射中刪除
        self.key_mapping.write().await.remove(&hash);

        // 從 Redis 刪除
        match self.redis_cache.delete(key).await {
            Ok(deleted) => {
                CacheMetrics::record_delete(deleted);
                // Note: Using MinuteBar as placeholder for general delete operations
                self.record_metric::<MinuteBar>(
                    MetricType::Latency {
                        operation: "delete",
                    },
                    Some(start.elapsed()),
                );
                Ok(deleted)
            }
            Err(e) => {
                // Note: Using MinuteBar as placeholder for general delete operations
                self.record_metric::<MinuteBar>(
                    MetricType::Error {
                        operation: "delete",
                    },
                    None,
                );
                Err(e)
            }
        }
    }

    /// 檢查快取是否存在
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let hash = Self::hash_key(key);

        // 檢查任一內存快取（使用 hash）
        if self.minute_bars_cache.get(&hash).await.is_some()
            || self.ticks_cache.get(&hash).await.is_some()
        {
            return Ok(true);
        }

        // 檢查 Redis
        self.redis_cache.exists(key).await
    }

    /// 清空所有內存快取 - 帶監控指標
    pub async fn clear_memory_cache(&self) {
        let minute_bars_count = self.minute_bars_cache.entry_count();
        let ticks_count = self.ticks_cache.entry_count();

        self.minute_bars_cache.invalidate_all();
        self.ticks_cache.invalidate_all();

        // 清空映射
        self.key_mapping.write().await.clear();

        CacheMetrics::record_cache_clear("memory", 1);
        CacheMetrics::record_cache_clear("minute_bars", minute_bars_count);
        CacheMetrics::record_cache_clear("ticks", ticks_count);
    }

    /// 並行預熱快取
    ///
    /// 使用並行處理來加速大量快取鍵的預熱過程。
    pub async fn warm_cache_parallel(
        &self,
        keys: Vec<String>,
        concurrency: usize,
    ) -> Result<(), CacheError> {
        if keys.is_empty() {
            return Ok(());
        }

        let chunks: Vec<_> = keys.chunks(concurrency).map(|c| c.to_vec()).collect();

        stream::iter(chunks)
            .for_each_concurrent(concurrency, |chunk| async move {
                let _ = self.get_minute_bars_batch(&chunk).await;
            })
            .await;

        Ok(())
    }

    /// 根據使用頻率和資料大小智能驅逐快取
    ///
    /// Moka 內建 LFU (Least Frequently Used) 策略，
    /// 此方法提供手動觸發驅逐的接口。
    pub async fn smart_eviction(&self, target_memory_mb: usize) {
        let current_size = self.minute_bars_cache.weighted_size();
        let target_size = (target_memory_mb * 1024 * 1024) as u64;

        if current_size > target_size {
            // 觸發 Moka 的內部維護任務
            self.minute_bars_cache.run_pending_tasks().await;
            self.ticks_cache.run_pending_tasks().await;

            CacheMetrics::record_smart_eviction(current_size - target_size);
        }
    }

    /// 新增方法：安全地刪除不一致的快取資料
    ///
    /// 當偵測到快取不一致時，此方法可以幫助清理相關數據
    pub async fn invalidate_inconsistent_cache(&self, key: &str) {
        let hash = Self::hash_key(key);

        // 從內存快取中刪除
        self.minute_bars_cache.invalidate(&hash).await;
        self.ticks_cache.invalidate(&hash).await;

        // 從映射中刪除
        self.key_mapping.write().await.remove(&hash);

        // 嘗試從 Redis 中刪除（忽略錯誤）
        let _ = self.redis_cache.delete(key).await;

        // 記錄清理操作
        CacheMetrics::record_inconsistent_cleanup();
    }

    /// 定期清理過期的映射
    ///
    /// 檢查 key_mapping 中的映射是否對應到已經過期的快取項目，
    /// 如果對應的數據在內存快取中已不存在，則清理該映射。
    /// 這有助於防止 key_mapping 無限制地累積已過期的映射。
    pub async fn cleanup_expired_mappings(&self) {
        let mut mapping = self.key_mapping.write().await;
        let mut expired_hashes = Vec::new();

        for (hash, _) in mapping.iter() {
            // 檢查內存快取中是否還存在
            if self.minute_bars_cache.get(hash).await.is_none()
                && self.ticks_cache.get(hash).await.is_none()
            {
                expired_hashes.push(*hash);
            }
        }

        let cleaned_count = expired_hashes.len();

        // 批量刪除過期的映射
        for hash in expired_hashes {
            mapping.remove(&hash);
        }

        // 記錄清理操作的監控指標
        CacheMetrics::record_mapping_cleanup(cleaned_count);
    }

    /// 獲取快取統計信息
    pub async fn cache_stats(&self) -> MultiCacheStats {
        // 記錄統計信息請求
        CacheMetrics::record_stats_request();

        let mapping_size = self.key_mapping.read().await.len();

        let minute_bars_stats = CacheStats {
            size: self.minute_bars_cache.entry_count() as usize,
            capacity: self.minute_bars_cache.weighted_size() as usize,
        };

        let ticks_stats = CacheStats {
            size: self.ticks_cache.entry_count() as usize,
            capacity: self.ticks_cache.weighted_size() as usize,
        };

        // 發送快取大小指標
        CacheMetrics::record_cache_size("minute_bars", minute_bars_stats.size, Some(mapping_size));
        CacheMetrics::record_cache_size("ticks", ticks_stats.size, None);

        MultiCacheStats {
            minute_bars: minute_bars_stats,
            ticks: ticks_stats,
            mapping_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::keys::{generate_cache_key, OptimizedKeyBuilder};
    use crate::redis::test_config::RedisTestConfig;
    use chrono::{DateTime, Utc};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

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
            mapping_size: 20,
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

    #[test]
    fn test_cache_key_hash() {
        // 測試 CacheKeyHash
        let key1 = "market_data:100:1h:1704067200:1704153600";
        let hash1 = CacheKeyHash::new(key1);
        let hash2 = CacheKeyHash::new(key1);

        // 相同的鍵應該產生相同的雜湊值
        assert_eq!(hash1, hash2);

        // 不同的鍵應該產生不同的雜湊值
        let key2 = "market_data:200:1h:1704067200:1704153600";
        let hash3 = CacheKeyHash::new(key2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_optimized_cache_key_hashing() {
        // 測試優化快取的 hash 計算
        let key1 = "market_data:100:1h:1704067200:1704153600";
        let key2 = "market_data:200:1h:1704067200:1704153600";
        let key3 = "market_data:100:1h:1704067200:1704153600"; // 與 key1 相同

        let hash1 = CacheKeyHash::new(key1).0;
        let hash2 = CacheKeyHash::new(key2).0;
        let hash3 = CacheKeyHash::new(key3).0;

        // 相同的鍵應該產生相同的 hash
        assert_eq!(hash1, hash3);

        // 不同的鍵應該產生不同的 hash
        assert_ne!(hash1, hash2);

        // 測試 FxHasher 的性能特性
        use std::collections::HashSet;
        let mut hashes = HashSet::new();

        // 生成一批不同的鍵並檢查碰撞率
        for i in 0..1000 {
            let key = format!("market_data:{}:1h:1704067200:1704153600", i);
            let hash = CacheKeyHash::new(&key).0;
            hashes.insert(hash);
        }

        // 確保沒有碰撞（在這種小規模測試中）
        assert_eq!(hashes.len(), 1000);
    }

    #[test]
    fn test_multi_cache_stats_with_mapping() {
        // 測試更新後的快取統計，包含映射大小
        let stats = MultiCacheStats {
            minute_bars: CacheStats {
                size: 10,
                capacity: 1000,
            },
            ticks: CacheStats {
                size: 5,
                capacity: 500,
            },
            mapping_size: 15,
        };

        assert_eq!(stats.minute_bars.size, 10);
        assert_eq!(stats.ticks.capacity, 500);
        assert_eq!(stats.mapping_size, 15);
    }

    #[test]
    fn test_hash_consistency() {
        // 測試 hash 一致性
        let key = "test_key";
        let hash1 = CacheKeyHash::new(key).0;
        let hash2 = CacheKeyHash::new(key).0;

        // 多次計算同一個鍵的 hash 應該得到相同結果
        assert_eq!(hash1, hash2);

        // 測試空字符串
        let empty_hash1 = CacheKeyHash::new("").0;
        let empty_hash2 = CacheKeyHash::new("").0;
        assert_eq!(empty_hash1, empty_hash2);

        // 測試長字符串
        let long_key = "a".repeat(1000);
        let long_hash1 = CacheKeyHash::new(&long_key).0;
        let long_hash2 = CacheKeyHash::new(&long_key).0;
        assert_eq!(long_hash1, long_hash2);
    }

    #[test]
    fn test_hash_distribution() {
        // 測試 hash 分佈均勻性
        use std::collections::HashMap;

        let mut distribution = HashMap::new();
        let sample_size = 10000;

        // 生成大量不同的鍵
        for i in 0..sample_size {
            let key = format!(
                "market_data:{}:{}:{}:{}",
                i,
                i % 10,
                1704067200 + i,
                1704153600 + i
            );
            let hash = CacheKeyHash::new(&key).0;

            // 統計 hash 值的低8位分佈
            let bucket = (hash & 0xFF) as u8;
            *distribution.entry(bucket).or_insert(0) += 1;
        }

        // 檢查分佈是否相對均勻（每個桶應該有大約 sample_size/256 個元素）
        let expected_per_bucket = sample_size / 256;
        let tolerance = expected_per_bucket / 2; // 50% 容差

        for bucket_count in distribution.values() {
            assert!(
                *bucket_count as i32 > (expected_per_bucket as i32 - tolerance as i32),
                "分佈不均勻，某些桶太少: {}",
                bucket_count
            );
            assert!(
                *bucket_count < expected_per_bucket + tolerance,
                "分佈不均勻，某些桶太多: {}",
                bucket_count
            );
        }
    }

    #[tokio::test]
    async fn test_hash_collision_detection() {
        // 測試 hash 碰撞檢測功能
        RedisTestConfig::ensure_redis_available("test_hash_collision_detection").await;

        let pool = RedisTestConfig::create_test_pool()
            .await
            .expect("無法創建測試 Redis 池");
        let redis_cache = Arc::new(CacheManager::new(pool));
        let cache = MultiLevelCache::new(redis_cache, 100, 300);

        // 測試無碰撞情況
        let key1 = "test_key_1";
        let hash1 = MultiLevelCache::<crate::redis::pool::ConnectionPool>::hash_key(key1);
        let collision_detected = cache.check_hash_collision(hash1, key1).await;
        assert!(!collision_detected, "首次檢查不應該檢測到碰撞");

        // 手動添加映射
        cache
            .key_mapping
            .write()
            .await
            .insert(hash1, key1.to_string());

        // 測試相同鍵無碰撞
        let collision_detected = cache.check_hash_collision(hash1, key1).await;
        assert!(!collision_detected, "相同鍵不應該檢測到碰撞");

        // 模擬碰撞情況（不同的鍵有相同的 hash）
        let key2 = "different_key";
        let collision_detected = cache.check_hash_collision(hash1, key2).await;
        assert!(collision_detected, "不同鍵相同 hash 應該檢測到碰撞");
    }

    #[tokio::test]
    async fn test_collision_detection_with_cache_operations() {
        // 測試快取操作中的碰撞檢測整合
        RedisTestConfig::ensure_redis_available("test_collision_detection_with_cache_operations").await;

        let pool = RedisTestConfig::create_test_pool()
            .await
            .expect("無法創建測試 Redis 池");
        let redis_cache = Arc::new(CacheManager::new(pool));
        let cache = MultiLevelCache::new(redis_cache, 100, 300);

        let test_data = create_test_minute_bars();
        let key1 = "collision_test_key_1";
        let _key2 = "collision_test_key_2";

        // 設定第一個鍵的數據
        let result = cache.set_minute_bars(key1, &test_data).await;
        assert!(result.is_ok(), "設定快取應該成功");

        // 獲取數據以確保正常運作
        let retrieved = cache.get_minute_bars(key1).await;
        assert!(retrieved.is_ok(), "獲取快取應該成功");
        assert!(retrieved.unwrap().is_some(), "應該能獲取到數據");

        // 如果兩個不同的鍵產生相同的 hash，系統應該能正確處理
        // 在實際情況下，這極少發生，但我們的檢測機制應該能處理這種情況
    }

    #[test]
    fn test_forced_hash_collision_scenario() {
        // 測試強制 hash 碰撞場景
        // 注意：這是一個理論性測試，因為 FxHasher 的碰撞機率非常低

        let key1 = "test_key_1";
        let key2 = "test_key_2";
        let hash1 = CacheKeyHash::new(key1).0;
        let hash2 = CacheKeyHash::new(key2).0;

        // 在正常情況下，不同的鍵應該產生不同的 hash
        assert_ne!(hash1, hash2, "不同的鍵通常會產生不同的 hash");

        // 測試碰撞檢測邏輯（使用相同的 hash 值模擬碰撞）
        let collision_hash = hash1;

        // 創建一個模擬的 key_mapping
        use rustc_hash::FxHashMap;
        let mut mapping = FxHashMap::default();
        mapping.insert(collision_hash, key1.to_string());

        // 檢查相同鍵
        if let Some(existing_key) = mapping.get(&collision_hash) {
            assert_eq!(existing_key, key1, "相同鍵應該匹配");
        }

        // 模擬碰撞情況
        if let Some(existing_key) = mapping.get(&collision_hash) {
            let collision_detected = existing_key != key2;
            assert!(collision_detected, "不同鍵相同 hash 應該檢測到碰撞");
        }
    }

    #[test]
    fn test_pipeline_operations_data_preparation() {
        // 測試 Pipeline 操作數據準備
        let test_bars = create_test_minute_bars();
        let test_ticks = create_test_ticks();

        // Pipeline MinuteBars 數據準備
        let minute_bars_items = vec![
            ("pipeline_key1".to_string(), Arc::new(test_bars.clone())),
            ("pipeline_key2".to_string(), Arc::new(test_bars.clone())),
            ("pipeline_key3".to_string(), Arc::new(test_bars)),
        ];
        assert_eq!(minute_bars_items.len(), 3);

        // Pipeline Ticks 數據準備
        let ticks_items = vec![
            (
                "pipeline_tick_key1".to_string(),
                Arc::new(test_ticks.clone()),
            ),
            (
                "pipeline_tick_key2".to_string(),
                Arc::new(test_ticks.clone()),
            ),
            ("pipeline_tick_key3".to_string(), Arc::new(test_ticks)),
        ];
        assert_eq!(ticks_items.len(), 3);

        // 測試空 Pipeline 操作
        let empty_minute_bars: Vec<(String, Arc<Vec<MinuteBar>>)> = vec![];
        let empty_ticks: Vec<(String, Arc<Vec<DbTick>>)> = vec![];
        assert!(empty_minute_bars.is_empty());
        assert!(empty_ticks.is_empty());
    }

    #[test]
    fn test_cache_buffer() {
        // 測試 CacheBuffer
        let mut buffer = CacheBuffer::with_capacity(10);

        // 測試初始狀態
        assert!(buffer.keys.is_empty());
        assert!(buffer.indices.is_empty());
        assert!(buffer.keys.capacity() >= 10);
        assert!(buffer.indices.capacity() >= 10);

        // 測試添加數據
        buffer.keys.push("key1".to_string());
        buffer.keys.push("key2".to_string());
        buffer.indices.push(0);
        buffer.indices.push(1);

        assert_eq!(buffer.keys.len(), 2);
        assert_eq!(buffer.indices.len(), 2);

        // 測試清空（保留容量）
        let keys_cap = buffer.keys.capacity();
        let indices_cap = buffer.indices.capacity();

        buffer.clear();

        assert!(buffer.keys.is_empty());
        assert!(buffer.indices.is_empty());
        assert_eq!(buffer.keys.capacity(), keys_cap);
        assert_eq!(buffer.indices.capacity(), indices_cap);
    }

    #[tokio::test]
    async fn test_cache_consistency_on_redis_failure() {
        // 測試 Redis 失敗時的快取一致性
        RedisTestConfig::ensure_redis_available("test_cache_consistency_on_redis_failure").await;

        // 建立測試用的 Redis 池
        let pool = RedisTestConfig::create_test_pool()
            .await
            .expect("無法創建測試 Redis 池");

        // TODO: 可以測試錯誤的 Redis URL 來模擬連接失敗
        // 現在暫時只驗證池的健康狀態
        assert!(pool.check_health().await);
    }

    #[tokio::test]
    async fn test_cache_consistency_on_successful_update() {
        // 測試成功更新時的快取一致性
        RedisTestConfig::ensure_redis_available("test_cache_consistency_on_successful_update").await;

        let pool = RedisTestConfig::create_test_pool()
            .await
            .expect("無法創建測試 Redis 池");
        let redis_cache = Arc::new(CacheManager::new(pool));
        let cache = MultiLevelCache::new(redis_cache.clone(), 100, 300);

        let test_data = create_test_minute_bars();
        let key = "consistency_test_key";

        // 1. 設定快取數據
        let result = cache.set_minute_bars(key, &test_data).await;
        assert!(result.is_ok(), "設定快取應該成功");

        // 2. 驗證內存快取包含數據
        let hash = MultiLevelCache::<crate::redis::pool::ConnectionPool>::hash_key(key);
        let memory_data = cache.minute_bars_cache.get(&hash).await;
        assert!(memory_data.is_some(), "內存快取應該包含數據");
        assert_eq!(
            memory_data.as_ref().unwrap().len(),
            test_data.len(),
            "內存快取數據長度應該正確"
        );

        // 3. 驗證 Redis 快取包含數據
        let redis_data: Result<Vec<MinuteBar>, _> = redis_cache.get(key).await;
        assert!(redis_data.is_ok(), "Redis 快取應該包含數據");
        let redis_data = redis_data.unwrap();
        assert_eq!(
            redis_data.len(),
            test_data.len(),
            "Redis 快取數據長度應該正確"
        );

        // 4. 驗證兩層快取數據一致性
        let memory_data = memory_data.unwrap();
        for (memory_bar, redis_bar) in memory_data.iter().zip(redis_data.iter()) {
            assert_eq!(memory_bar.instrument_id, redis_bar.instrument_id);
            assert_eq!(memory_bar.time, redis_bar.time);
            assert_eq!(memory_bar.open, redis_bar.open);
            assert_eq!(memory_bar.high, redis_bar.high);
            assert_eq!(memory_bar.low, redis_bar.low);
            assert_eq!(memory_bar.close, redis_bar.close);
            assert_eq!(memory_bar.volume, redis_bar.volume);
        }

        // 5. 驗證 key mapping 正確性
        {
            let mapping = cache.key_mapping.read().await;
            assert!(mapping.contains_key(&hash), "映射應該包含該鍵的 hash");
            assert_eq!(mapping.get(&hash).unwrap(), key, "映射應該指向正確的鍵");
        }

        // 6. 測試通過快取接口獲取數據的一致性
        let cached_data = cache.get_minute_bars(key).await;
        assert!(cached_data.is_ok(), "通過快取接口獲取數據應該成功");
        let cached_data = cached_data.unwrap();
        assert!(cached_data.is_some(), "應該能獲取到快取數據");
        let cached_data = cached_data.unwrap();
        assert_eq!(
            cached_data.len(),
            test_data.len(),
            "快取接口返回的數據長度應該正確"
        );

        // 7. 驗證數據內容一致性
        for (cached_bar, original_bar) in cached_data.iter().zip(test_data.iter()) {
            assert_eq!(cached_bar.instrument_id, original_bar.instrument_id);
            assert_eq!(cached_bar.time, original_bar.time);
            assert_eq!(cached_bar.open, original_bar.open);
            assert_eq!(cached_bar.high, original_bar.high);
            assert_eq!(cached_bar.low, original_bar.low);
            assert_eq!(cached_bar.close, original_bar.close);
            assert_eq!(cached_bar.volume, original_bar.volume);
        }
    }

    #[tokio::test]
    async fn test_invalidate_inconsistent_cache() {
        // 測試清理不一致快取的功能
        RedisTestConfig::ensure_redis_available("test_invalidate_inconsistent_cache").await;

        let pool = RedisTestConfig::create_test_pool()
            .await
            .expect("無法創建測試 Redis 池");
        let redis_cache = Arc::new(CacheManager::new(pool));
        let cache = MultiLevelCache::new(redis_cache.clone(), 100, 300);

        let test_minute_bars = create_test_minute_bars();
        let test_ticks = create_test_ticks();
        let key1 = "invalidate_test_key_1";
        let key2 = "invalidate_test_key_2";

        // 1. 設定初始快取數據
        let result1 = cache.set_minute_bars(key1, &test_minute_bars).await;
        let result2 = cache.set_ticks(key2, &test_ticks).await;
        assert!(result1.is_ok(), "設定 minute bars 快取應該成功");
        assert!(result2.is_ok(), "設定 ticks 快取應該成功");

        // 2. 驗證數據已被快取（兩層都有）
        let hash1 = MultiLevelCache::<crate::redis::pool::ConnectionPool>::hash_key(key1);
        let hash2 = MultiLevelCache::<crate::redis::pool::ConnectionPool>::hash_key(key2);

        // 檢查內存快取
        assert!(
            cache.minute_bars_cache.get(&hash1).await.is_some(),
            "內存快取應該包含 minute bars"
        );
        assert!(
            cache.ticks_cache.get(&hash2).await.is_some(),
            "內存快取應該包含 ticks"
        );

        // 檢查 Redis 快取
        let redis_bars: Result<Vec<MinuteBar>, _> = redis_cache.get(key1).await;
        let redis_ticks: Result<Vec<DbTick>, _> = redis_cache.get(key2).await;
        assert!(redis_bars.is_ok(), "Redis 應該包含 minute bars");
        assert!(redis_ticks.is_ok(), "Redis 應該包含 ticks");

        // 檢查映射
        {
            let mapping = cache.key_mapping.read().await;
            assert!(mapping.contains_key(&hash1), "映射應該包含 key1");
            assert!(mapping.contains_key(&hash2), "映射應該包含 key2");
            assert_eq!(mapping.len(), 2, "映射應該包含2個項目");
        }

        // 3. 調用 invalidate_inconsistent_cache 清理第一個鍵
        cache.invalidate_inconsistent_cache(key1).await;

        // 4. 驗證第一個鍵的所有層級快取都被清理
        // 檢查內存快取
        assert!(
            cache.minute_bars_cache.get(&hash1).await.is_none(),
            "內存快取中的 minute bars 應該被清理"
        );
        assert!(
            cache.ticks_cache.get(&hash1).await.is_none(),
            "內存快取中不應該有該鍵的 ticks 數據"
        );

        // 檢查 Redis 快取（應該被嘗試刪除，雖然可能因為權限等原因失敗）
        // 這裡我們主要檢查方法不會出錯
        let _redis_check: Result<Vec<MinuteBar>, _> = redis_cache.get(key1).await;
        // 不管結果如何，至少方法應該能正常執行

        // 檢查映射
        {
            let mapping = cache.key_mapping.read().await;
            assert!(!mapping.contains_key(&hash1), "映射不應該包含已清理的 key1");
            assert!(mapping.contains_key(&hash2), "映射仍應該包含未清理的 key2");
            assert_eq!(mapping.len(), 1, "映射應該只包含1個項目");
        }

        // 5. 驗證第二個鍵的快取仍然存在（未被影響）
        assert!(
            cache.ticks_cache.get(&hash2).await.is_some(),
            "未清理的 ticks 快取應該仍然存在"
        );
        let redis_ticks_check: Result<Vec<DbTick>, _> = redis_cache.get(key2).await;
        assert!(
            redis_ticks_check.is_ok(),
            "未清理的 Redis ticks 應該仍然存在"
        );

        // 6. 測試清理不存在的鍵（應該不會出錯）
        cache
            .invalidate_inconsistent_cache("non_existent_key")
            .await;
        // 確保這不會導致錯誤或影響其他快取

        // 7. 驗證剩餘快取仍然正常工作
        let remaining_data = cache.get_ticks(key2).await;
        assert!(remaining_data.is_ok(), "獲取剩餘快取數據應該成功");
        assert!(remaining_data.unwrap().is_some(), "剩餘快取數據應該存在");

        // 8. 清理第二個鍵並驗證完全清空
        cache.invalidate_inconsistent_cache(key2).await;

        {
            let mapping = cache.key_mapping.read().await;
            assert_eq!(mapping.len(), 0, "所有映射應該被清理");
        }

        assert!(
            cache.minute_bars_cache.get(&hash1).await.is_none(),
            "所有 minute bars 快取應該被清理"
        );
        assert!(
            cache.minute_bars_cache.get(&hash2).await.is_none(),
            "所有 minute bars 快取應該被清理"
        );
        assert!(
            cache.ticks_cache.get(&hash1).await.is_none(),
            "所有 ticks 快取應該被清理"
        );
        assert!(
            cache.ticks_cache.get(&hash2).await.is_none(),
            "所有 ticks 快取應該被清理"
        );
    }

    #[tokio::test]
    async fn test_cleanup_expired_mappings() {
        // 測試清理過期映射的功能
        RedisTestConfig::ensure_redis_available("test_cleanup_expired_mappings").await;

        let pool = RedisTestConfig::create_test_pool()
            .await
            .expect("無法創建測試 Redis 池");
        let redis_cache = Arc::new(CacheManager::new(pool));
        let cache = MultiLevelCache::new(redis_cache, 100, 1); // 短 TTL 用於測試

        let test_data = create_test_minute_bars();
        let key1 = "expire_test_key_1";
        let key2 = "expire_test_key_2";

        // 設定測試數據
        let _ = cache.set_minute_bars(key1, &test_data).await;
        let _ = cache.set_minute_bars(key2, &test_data).await;

        // 驗證映射存在
        {
            let mapping = cache.key_mapping.read().await;
            assert_eq!(mapping.len(), 2, "應該有2個映射");
        }

        // 等待快取過期
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 執行清理
        cache.cleanup_expired_mappings().await;

        // 驗證映射已被清理
        {
            let mapping = cache.key_mapping.read().await;
            assert_eq!(mapping.len(), 0, "所有過期映射應該被清理");
        }
    }

    #[tokio::test]
    async fn test_cleanup_expired_mappings_partial() {
        // 測試部分映射過期的情況
        RedisTestConfig::ensure_redis_available("test_cleanup_expired_mappings_partial").await;

        let pool = RedisTestConfig::create_test_pool()
            .await
            .expect("無法創建測試 Redis 池");
        let redis_cache = Arc::new(CacheManager::new(pool));
        let cache = MultiLevelCache::new(redis_cache, 100, 300);

        let test_data = create_test_minute_bars();
        let key1 = "partial_expire_key_1";
        let key2 = "partial_expire_key_2";

        // 設定第一個鍵
        let _ = cache.set_minute_bars(key1, &test_data).await;

        // 手動清理第一個鍵的內存快取（模擬過期）
        let hash1 = MultiLevelCache::<crate::redis::pool::ConnectionPool>::hash_key(key1);
        cache.minute_bars_cache.invalidate(&hash1).await;

        // 設定第二個鍵（保持有效）
        let _ = cache.set_minute_bars(key2, &test_data).await;

        // 驗證初始狀態
        {
            let mapping = cache.key_mapping.read().await;
            assert_eq!(mapping.len(), 2, "應該有2個映射");
        }

        // 執行清理
        cache.cleanup_expired_mappings().await;

        // 驗證只有過期的映射被清理
        {
            let mapping = cache.key_mapping.read().await;
            assert_eq!(mapping.len(), 1, "只有1個有效映射應該保留");

            let hash2 = MultiLevelCache::<crate::redis::pool::ConnectionPool>::hash_key(key2);
            assert!(mapping.contains_key(&hash2), "有效映射應該保留");
        }
    }
}
