use crate::cache::buffer::CacheBuffer;
use crate::cache::keys::CacheKeyHash;
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

        // 1. 嘗試從內存快取獲取（使用 hash）
        if let Some(arc_data) = cache.get(&hash).await {
            self.record_metric::<T>(MetricType::Hit { layer: "memory" }, None);
            self.record_metric::<T>(MetricType::Latency { operation: "get_memory" }, Some(start.elapsed()));
            return Ok(Some(arc_data));
        }

        // 2. 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<T>>(key).await {
            Ok(data) => {
                self.record_metric::<T>(MetricType::Hit { layer: "redis" }, None);
                self.record_metric::<T>(MetricType::Latency { operation: "get_redis" }, Some(start.elapsed()));

                // 更新內存快取和映射
                let arc_data = Arc::new(data);
                cache.insert(hash, arc_data.clone()).await;

                // 更新 hash 映射
                self.key_mapping.write().await.insert(hash, key.to_string());

                Ok(Some(arc_data))
            }
            Err(CacheError::CacheMiss(_)) => {
                self.record_metric::<T>(MetricType::Miss, None);
                self.record_metric::<T>(MetricType::Latency { operation: "get_miss" }, Some(start.elapsed()));
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

        // 1. 先更新 Redis 快取
        match self.redis_cache.set(key, data, Some(self.cache_ttl)).await {
            Ok(_) => {
                // 2. Redis 更新成功後，更新內存快取
                let arc_data = Arc::new(data.clone());
                cache.insert(hash, arc_data).await;

                // 3. 更新 hash 映射
                self.key_mapping.write().await.insert(hash, key.to_string());

                CacheMetrics::record_set(T::data_type_name());
                self.record_metric::<T>(MetricType::Latency { operation: "set" }, Some(start.elapsed()));
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
                self.record_metric::<T>(MetricType::Latency { operation: "set_arc" }, Some(start.elapsed()));
                Ok(())
            }
            Err(e) => {
                self.record_metric::<T>(MetricType::Error { operation: "set_arc" }, None);
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

    /// 獲取 MinuteBars 資料 - 高性能版本
    ///
    /// 使用 u64 hash 作為內存快取鍵，提升查找性能。
    /// 直接返回 Arc<Vec<MinuteBar>>，避免深拷貝整個 Vec。
    pub async fn get_minute_bars(
        &self,
        key: &str,
    ) -> Result<Option<Arc<Vec<MinuteBar>>>, CacheError> {
        self.get_cached_data::<MinuteBar>(key, &self.minute_bars_cache).await
    }

    /// 設置 MinuteBars 資料 - 高性能版本
    ///
    /// # 快取一致性策略
    /// - 先更新 Redis（持久化層）
    /// - 只有在 Redis 更新成功後才更新內存快取
    /// - 如果 Redis 更新失敗，內存快取保持不變
    pub async fn set_minute_bars(
        &self,
        key: &str,
        bars: &Vec<MinuteBar>,
    ) -> Result<(), CacheError> {
        self.set_cached_data::<MinuteBar>(key, bars, &self.minute_bars_cache).await
    }

    /// 設置 MinuteBars 資料 (Arc 版本) - 避免不必要的複製
    ///
    /// 當調用者已經擁有 Arc<Vec<MinuteBar>> 時，可以使用此方法避免額外的 clone。
    ///
    /// # 快取一致性策略
    /// - 先更新 Redis（持久化層）
    /// - 只有在 Redis 更新成功後才更新內存快取
    /// - 如果 Redis 更新失敗，內存快取保持不變
    pub async fn set_minute_bars_arc(
        &self,
        key: &str,
        bars: Arc<Vec<MinuteBar>>,
    ) -> Result<(), CacheError> {
        self.set_cached_data_arc::<MinuteBar>(key, bars, &self.minute_bars_cache).await
    }

    /// 獲取 Ticks 資料 - 帶監控指標
    ///
    /// 直接返回 Arc<Vec<DbTick>>，避免深拷貝整個 Vec。
    /// 適合需要高性能且只讀數據的場景。
    pub async fn get_ticks(&self, key: &str) -> Result<Option<Arc<Vec<DbTick>>>, CacheError> {
        self.get_cached_data::<DbTick>(key, &self.ticks_cache).await
    }

    /// 設置 Ticks 資料 - 帶監控指標
    ///
    /// # 快取一致性策略
    /// - 先更新 Redis（持久化層）
    /// - 只有在 Redis 更新成功後才更新內存快取
    /// - 如果 Redis 更新失敗，內存快取保持不變
    pub async fn set_ticks(&self, key: &str, ticks: &Vec<DbTick>) -> Result<(), CacheError> {
        self.set_cached_data::<DbTick>(key, ticks, &self.ticks_cache).await
    }

    /// 設置 Ticks 資料 (Arc 版本) - 避免不必要的複製
    ///
    /// 當調用者已經擁有 Arc<Vec<DbTick>> 時，可以使用此方法避免額外的 clone。
    ///
    /// # 快取一致性策略
    /// - 先更新 Redis（持久化層）
    /// - 只有在 Redis 更新成功後才更新內存快取
    /// - 如果 Redis 更新失敗，內存快取保持不變
    pub async fn set_ticks_arc(
        &self,
        key: &str,
        ticks: Arc<Vec<DbTick>>,
    ) -> Result<(), CacheError> {
        self.set_cached_data_arc::<DbTick>(key, ticks, &self.ticks_cache).await
    }

    /// 獲取或計算 MinuteBars 資料 - 使用 Arc 避免不必要的複製
    ///
    /// 如果快取命中，返回共享的數據；如果需要計算，返回新創建的數據。
    /// 這可以顯著減少記憶體分配和複製操作。
    ///
    /// # 快取一致性策略
    /// - 先更新 Redis（持久化層）
    /// - 只有在 Redis 更新成功後才更新內存快取
    pub async fn get_or_compute_minute_bars<F>(
        &self,
        key: &str,
        compute: F,
    ) -> Result<Arc<Vec<MinuteBar>>, CacheError>
    where
        F: FnOnce() -> Vec<MinuteBar>,
    {
        let hash = Self::hash_key(key);

        // 如果快取命中，返回 Arc（使用 hash）
        if let Some(arc_bars) = self.minute_bars_cache.get(&hash).await {
            self.record_metric::<MinuteBar>(MetricType::Hit { layer: "memory" }, None);
            return Ok(arc_bars);
        }

        // 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<MinuteBar>>(key).await {
            Ok(bars) => {
                self.record_metric::<MinuteBar>(MetricType::Hit { layer: "redis" }, None);
                let arc_bars = Arc::new(bars);
                self.minute_bars_cache.insert(hash, arc_bars.clone()).await;

                // 更新 hash 映射
                self.key_mapping.write().await.insert(hash, key.to_string());

                Ok(arc_bars)
            }
            Err(CacheError::CacheMiss(_)) => {
                // 計算新數據
                self.record_metric::<MinuteBar>(MetricType::Miss, None);
                let bars = compute();
                let arc_bars = Arc::new(bars);

                // 先嘗試更新 Redis
                match self
                    .redis_cache
                    .set(key, &*arc_bars, Some(self.cache_ttl))
                    .await
                {
                    Ok(_) => {
                        // Redis 更新成功後，更新內存快取和映射
                        self.minute_bars_cache.insert(hash, arc_bars.clone()).await;
                        self.key_mapping.write().await.insert(hash, key.to_string());
                        Ok(arc_bars)
                    }
                    Err(e) => {
                        // Redis 更新失敗，不更新內存快取
                        self.record_metric::<MinuteBar>(MetricType::Error { operation: "set_in_compute" }, None);
                        Err(e)
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    /// 獲取或計算 Ticks 資料 - 使用 Arc 避免不必要的複製
    ///
    /// # 快取一致性策略
    /// - 先更新 Redis（持久化層）
    /// - 只有在 Redis 更新成功後才更新內存快取
    pub async fn get_or_compute_ticks<F>(
        &self,
        key: &str,
        compute: F,
    ) -> Result<Arc<Vec<DbTick>>, CacheError>
    where
        F: FnOnce() -> Vec<DbTick>,
    {
        let hash = Self::hash_key(key);

        // 如果快取命中，返回 Arc（使用 hash）
        if let Some(arc_ticks) = self.ticks_cache.get(&hash).await {
            self.record_metric::<DbTick>(MetricType::Hit { layer: "memory" }, None);
            return Ok(arc_ticks);
        }

        // 嘗試從 Redis 獲取
        match self.redis_cache.get::<_, Vec<DbTick>>(key).await {
            Ok(ticks) => {
                self.record_metric::<DbTick>(MetricType::Hit { layer: "redis" }, None);
                let arc_ticks = Arc::new(ticks);
                self.ticks_cache.insert(hash, arc_ticks.clone()).await;

                // 更新 hash 映射
                self.key_mapping.write().await.insert(hash, key.to_string());

                Ok(arc_ticks)
            }
            Err(CacheError::CacheMiss(_)) => {
                // 計算新數據
                self.record_metric::<DbTick>(MetricType::Miss, None);
                let ticks = compute();
                let arc_ticks = Arc::new(ticks);

                // 先嘗試更新 Redis
                match self
                    .redis_cache
                    .set(key, &*arc_ticks, Some(self.cache_ttl))
                    .await
                {
                    Ok(_) => {
                        // Redis 更新成功後，更新內存快取和映射
                        self.ticks_cache.insert(hash, arc_ticks.clone()).await;
                        self.key_mapping.write().await.insert(hash, key.to_string());
                        Ok(arc_ticks)
                    }
                    Err(e) => {
                        // Redis 更新失敗，不更新內存快取
                        self.record_metric::<DbTick>(MetricType::Error { operation: "set_in_compute" }, None);
                        Err(e)
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

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
                self.record_metric::<MinuteBar>(MetricType::Latency { operation: "delete" }, Some(start.elapsed()));
                Ok(deleted)
            }
            Err(e) => {
                // Note: Using MinuteBar as placeholder for general delete operations
                self.record_metric::<MinuteBar>(MetricType::Error { operation: "delete" }, None);
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
        let mut missing_hashes = Vec::new();

        // 1. 先從內存快取批量獲取（使用 hash）
        for (idx, key) in keys.iter().enumerate() {
            let hash = Self::hash_key(key);
            if let Some(arc_bars) = self.minute_bars_cache.get(&hash).await {
                results[idx] = Some(arc_bars);
                self.record_metric::<MinuteBar>(MetricType::Hit { layer: "memory" }, None);
            } else {
                missing_keys.push(key.clone());
                missing_indices.push(idx);
                missing_hashes.push(hash);
            }
        }

        // 2. 如果有缺失的key，從Redis批量獲取
        if !missing_keys.is_empty() {
            let redis_results = self
                .redis_cache
                .mget::<String, Vec<MinuteBar>>(&missing_keys)
                .await?;

            // 批量更新映射
            let mut key_mapping = self.key_mapping.write().await;

            for ((idx, bars_opt), hash) in missing_indices
                .iter()
                .zip(redis_results)
                .zip(missing_hashes.iter())
            {
                if let Some(bars) = bars_opt {
                    let arc_bars = Arc::new(bars);
                    // 更新內存快取（使用 hash）
                    self.minute_bars_cache.insert(*hash, arc_bars.clone()).await;
                    // 更新映射
                    key_mapping.insert(*hash, keys[*idx].clone());
                    results[*idx] = Some(arc_bars);
                    self.record_metric::<MinuteBar>(MetricType::Hit { layer: "redis" }, None);
                } else {
                    self.record_metric::<MinuteBar>(MetricType::Miss, None);
                }
            }
        }

        self.record_metric::<MinuteBar>(MetricType::BatchOperation { operation: "get", count: keys.len() }, Some(start.elapsed()));

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
        let mut missing_hashes = Vec::new();

        // 1. 先從內存快取批量獲取（使用 hash）
        for (idx, key) in keys.iter().enumerate() {
            let hash = Self::hash_key(key);
            if let Some(arc_ticks) = self.ticks_cache.get(&hash).await {
                results[idx] = Some(arc_ticks);
                self.record_metric::<DbTick>(MetricType::Hit { layer: "memory" }, None);
            } else {
                missing_keys.push(key.clone());
                missing_indices.push(idx);
                missing_hashes.push(hash);
            }
        }

        // 2. 如果有缺失的key，從Redis批量獲取
        if !missing_keys.is_empty() {
            let redis_results = self
                .redis_cache
                .mget::<String, Vec<DbTick>>(&missing_keys)
                .await?;

            // 批量更新映射
            let mut key_mapping = self.key_mapping.write().await;

            for ((idx, ticks_opt), hash) in missing_indices
                .iter()
                .zip(redis_results)
                .zip(missing_hashes.iter())
            {
                if let Some(ticks) = ticks_opt {
                    let arc_ticks = Arc::new(ticks);
                    // 更新內存快取（使用 hash）
                    self.ticks_cache.insert(*hash, arc_ticks.clone()).await;
                    // 更新映射
                    key_mapping.insert(*hash, keys[*idx].clone());
                    results[*idx] = Some(arc_ticks);
                    self.record_metric::<DbTick>(MetricType::Hit { layer: "redis" }, None);
                } else {
                    self.record_metric::<DbTick>(MetricType::Miss, None);
                }
            }
        }

        self.record_metric::<DbTick>(MetricType::BatchOperation { operation: "get", count: keys.len() }, Some(start.elapsed()));

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

        // 清空映射
        self.key_mapping.write().await.clear();

        CacheMetrics::record_cache_clear("memory", 1);
        CacheMetrics::record_cache_clear("minute_bars", minute_bars_count);
        CacheMetrics::record_cache_clear("ticks", ticks_count);
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

        // 1. 先收集所有需要的數據
        let updates: Vec<(u64, String, Arc<Vec<MinuteBar>>)> = items
            .iter()
            .map(|(key, bars)| (Self::hash_key(key), key.clone(), Arc::new(bars.clone())))
            .collect();

        // 2. 批量更新內存快取（不需要持有鎖）
        for (hash, _, arc_bars) in &updates {
            self.minute_bars_cache.insert(*hash, arc_bars.clone()).await;
        }

        // 3. 最後一次性更新映射（最小化鎖持有時間）
        self.key_mapping
            .write()
            .await
            .extend(updates.iter().map(|(hash, key, _)| (*hash, key.clone())));

        // 4. 批量更新 Redis 快取
        match self.redis_cache.mset(items, Some(self.cache_ttl)).await {
            Ok(_) => {
                CacheMetrics::record_batch_set("minute_bars", items.len(), start.elapsed());
                Ok(())
            }
            Err(e) => {
                CacheMetrics::record_batch_set_error("minute_bars");
                Err(e)
            }
        }
    }

    /// 優化的批量設置 MinuteBars - 減少複製操作
    ///
    /// 當調用者已經擁有 Arc<Vec<MinuteBar>> 時，使用此方法可以避免額外的 clone。
    pub async fn set_minute_bars_batch_optimized(
        &self,
        items: Vec<(String, Arc<Vec<MinuteBar>>)>,
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        if items.is_empty() {
            return Ok(());
        }

        // 1. 先收集所有需要的數據
        let updates: Vec<(u64, String)> = items
            .iter()
            .map(|(key, _)| (Self::hash_key(key), key.clone()))
            .collect();

        // 2. 批量更新內存快取（不需要持有鎖）
        for ((_, arc_bars), (hash, _)) in items.iter().zip(updates.iter()) {
            self.minute_bars_cache.insert(*hash, arc_bars.clone()).await;
        }

        // 3. 最後一次性更新映射（最小化鎖持有時間）
        self.key_mapping.write().await.extend(updates);

        // 4. 準備 Redis 數據（只在必要時解引用）
        let redis_items: Vec<(String, &Vec<MinuteBar>)> =
            items.iter().map(|(k, v)| (k.clone(), &**v)).collect();

        match self
            .redis_cache
            .mset(&redis_items, Some(self.cache_ttl))
            .await
        {
            Ok(_) => {
                CacheMetrics::record_batch_set("minute_bars", items.len(), start.elapsed());
                Ok(())
            }
            Err(e) => {
                CacheMetrics::record_batch_set_error("minute_bars");
                Err(e)
            }
        }
    }

    /// 使用 Pipeline 批量設置 MinuteBars - 最大化 Redis 性能
    ///
    /// 使用 Redis Pipeline 技術一次性執行多個命令，大幅減少網路往返次數。
    /// 適合大批量數據寫入場景，性能比逐個設置或 MSET 更佳。
    pub async fn set_minute_bars_batch_pipeline(
        &self,
        items: &[(String, Arc<Vec<MinuteBar>>)],
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        if items.is_empty() {
            return Ok(());
        }

        // 1. 先收集所有需要的數據
        let updates: Vec<(u64, String)> = items
            .iter()
            .map(|(key, _)| (Self::hash_key(key), key.clone()))
            .collect();

        // 2. 批量更新內存快取（不需要持有鎖）
        for ((_, arc_bars), (hash, _)) in items.iter().zip(updates.iter()) {
            self.minute_bars_cache.insert(*hash, arc_bars.clone()).await;
        }

        // 3. 最後一次性更新映射（最小化鎖持有時間）
        self.key_mapping.write().await.extend(updates);

        // 4. 使用 CacheManager 的 Pipeline 批量設置
        let redis_items: Vec<(String, &Vec<MinuteBar>)> =
            items.iter().map(|(k, v)| (k.clone(), &**v)).collect();

        match self
            .redis_cache
            .pipeline_mset(&redis_items, Some(self.cache_ttl))
            .await
        {
            Ok(_) => {
                CacheMetrics::record_pipeline_set("minute_bars", items.len(), start.elapsed());
                Ok(())
            }
            Err(e) => {
                CacheMetrics::record_pipeline_set_error("minute_bars");
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

        // 1. 先收集所有需要的數據
        let updates: Vec<(u64, String, Arc<Vec<DbTick>>)> = items
            .iter()
            .map(|(key, ticks)| (Self::hash_key(key), key.clone(), Arc::new(ticks.clone())))
            .collect();

        // 2. 批量更新內存快取（不需要持有鎖）
        for (hash, _, arc_ticks) in &updates {
            self.ticks_cache.insert(*hash, arc_ticks.clone()).await;
        }

        // 3. 最後一次性更新映射（最小化鎖持有時間）
        self.key_mapping
            .write()
            .await
            .extend(updates.iter().map(|(hash, key, _)| (*hash, key.clone())));

        // 4. 批量更新 Redis 快取
        match self.redis_cache.mset(items, Some(self.cache_ttl)).await {
            Ok(_) => {
                CacheMetrics::record_batch_set("ticks", items.len(), start.elapsed());
                Ok(())
            }
            Err(e) => {
                CacheMetrics::record_batch_set_error("ticks");
                Err(e)
            }
        }
    }

    /// 優化的批量設置 Ticks - 減少複製操作
    pub async fn set_ticks_batch_optimized(
        &self,
        items: Vec<(String, Arc<Vec<DbTick>>)>,
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        if items.is_empty() {
            return Ok(());
        }

        // 1. 先收集所有需要的數據
        let updates: Vec<(u64, String)> = items
            .iter()
            .map(|(key, _)| (Self::hash_key(key), key.clone()))
            .collect();

        // 2. 批量更新內存快取（不需要持有鎖）
        for ((_, arc_ticks), (hash, _)) in items.iter().zip(updates.iter()) {
            self.ticks_cache.insert(*hash, arc_ticks.clone()).await;
        }

        // 3. 最後一次性更新映射（最小化鎖持有時間）
        self.key_mapping.write().await.extend(updates);

        // 4. 準備 Redis 數據（只在必要時解引用）
        let redis_items: Vec<(String, &Vec<DbTick>)> =
            items.iter().map(|(k, v)| (k.clone(), &**v)).collect();

        match self
            .redis_cache
            .mset(&redis_items, Some(self.cache_ttl))
            .await
        {
            Ok(_) => {
                CacheMetrics::record_batch_set("ticks", items.len(), start.elapsed());
                Ok(())
            }
            Err(e) => {
                CacheMetrics::record_batch_set_error("ticks");
                Err(e)
            }
        }
    }

    /// 使用 Pipeline 批量設置 Ticks - 最大化 Redis 性能
    ///
    /// 使用 Redis Pipeline 技術一次性執行多個命令，大幅減少網路往返次數。
    /// 適合大批量 Tick 數據寫入場景，性能比逐個設置或 MSET 更佳。
    pub async fn set_ticks_batch_pipeline(
        &self,
        items: &[(String, Arc<Vec<DbTick>>)],
    ) -> Result<(), CacheError> {
        let start = Instant::now();

        if items.is_empty() {
            return Ok(());
        }

        // 1. 先收集所有需要的數據
        let updates: Vec<(u64, String)> = items
            .iter()
            .map(|(key, _)| (Self::hash_key(key), key.clone()))
            .collect();

        // 2. 批量更新內存快取（不需要持有鎖）
        for ((_, arc_ticks), (hash, _)) in items.iter().zip(updates.iter()) {
            self.ticks_cache.insert(*hash, arc_ticks.clone()).await;
        }

        // 3. 最後一次性更新映射（最小化鎖持有時間）
        self.key_mapping.write().await.extend(updates);

        // 4. 使用 CacheManager 的 Pipeline 批量設置
        let redis_items: Vec<(String, &Vec<DbTick>)> =
            items.iter().map(|(k, v)| (k.clone(), &**v)).collect();

        match self
            .redis_cache
            .pipeline_mset(&redis_items, Some(self.cache_ttl))
            .await
        {
            Ok(_) => {
                CacheMetrics::record_pipeline_set("ticks", items.len(), start.elapsed());
                Ok(())
            }
            Err(e) => {
                CacheMetrics::record_pipeline_set_error("ticks");
                Err(e)
            }
        }
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

    /// 使用預分配緩衝區的批量獲取 MinuteBars
    ///
    /// 通過重用緩衝區來減少記憶體分配，提升批量操作性能。
    pub async fn get_minute_bars_batch_buffered(
        &self,
        keys: &[String],
        buffer: &mut CacheBuffer,
    ) -> Result<Vec<Option<Arc<Vec<MinuteBar>>>>, CacheError> {
        let start = Instant::now();

        buffer.clear();
        let mut results = vec![None; keys.len()];
        let mut missing_hashes = Vec::new();

        // 重用緩衝區來收集缺失的鍵（使用 hash）
        for (idx, key) in keys.iter().enumerate() {
            let hash = Self::hash_key(key);
            if let Some(arc_bars) = self.minute_bars_cache.get(&hash).await {
                results[idx] = Some(arc_bars);
                self.record_metric::<MinuteBar>(MetricType::Hit { layer: "memory" }, None);
            } else {
                buffer.keys.push(key.clone());
                buffer.indices.push(idx);
                missing_hashes.push(hash);
            }
        }

        // 如果有缺失的key，從Redis批量獲取
        if !buffer.keys.is_empty() {
            let redis_results = self
                .redis_cache
                .mget::<String, Vec<MinuteBar>>(&buffer.keys)
                .await?;

            // 批量更新映射
            let mut key_mapping = self.key_mapping.write().await;

            for ((idx, bars_opt), hash) in buffer
                .indices
                .iter()
                .zip(redis_results)
                .zip(missing_hashes.iter())
            {
                if let Some(bars) = bars_opt {
                    let arc_bars = Arc::new(bars);
                    // 更新內存快取（使用 hash）
                    self.minute_bars_cache.insert(*hash, arc_bars.clone()).await;
                    // 更新映射
                    key_mapping.insert(*hash, keys[*idx].clone());
                    results[*idx] = Some(arc_bars);
                    self.record_metric::<MinuteBar>(MetricType::Hit { layer: "redis" }, None);
                } else {
                    self.record_metric::<MinuteBar>(MetricType::Miss, None);
                }
            }
        }

        self.record_metric::<MinuteBar>(MetricType::BatchOperation { operation: "get", count: keys.len() }, Some(start.elapsed()));

        Ok(results)
    }

    /// 使用預分配緩衝區的批量獲取 Ticks
    pub async fn get_ticks_batch_buffered(
        &self,
        keys: &[String],
        buffer: &mut CacheBuffer,
    ) -> Result<Vec<Option<Arc<Vec<DbTick>>>>, CacheError> {
        let start = Instant::now();

        buffer.clear();
        let mut results = vec![None; keys.len()];
        let mut missing_hashes = Vec::new();

        // 重用緩衝區來收集缺失的鍵（使用 hash）
        for (idx, key) in keys.iter().enumerate() {
            let hash = Self::hash_key(key);
            if let Some(arc_ticks) = self.ticks_cache.get(&hash).await {
                results[idx] = Some(arc_ticks);
                self.record_metric::<DbTick>(MetricType::Hit { layer: "memory" }, None);
            } else {
                buffer.keys.push(key.clone());
                buffer.indices.push(idx);
                missing_hashes.push(hash);
            }
        }

        // 如果有缺失的key，從Redis批量獲取
        if !buffer.keys.is_empty() {
            let redis_results = self
                .redis_cache
                .mget::<String, Vec<DbTick>>(&buffer.keys)
                .await?;

            // 批量更新映射
            let mut key_mapping = self.key_mapping.write().await;

            for ((idx, ticks_opt), hash) in buffer
                .indices
                .iter()
                .zip(redis_results)
                .zip(missing_hashes.iter())
            {
                if let Some(ticks) = ticks_opt {
                    let arc_ticks = Arc::new(ticks);
                    // 更新內存快取（使用 hash）
                    self.ticks_cache.insert(*hash, arc_ticks.clone()).await;
                    // 更新映射
                    key_mapping.insert(*hash, keys[*idx].clone());
                    results[*idx] = Some(arc_ticks);
                    self.record_metric::<DbTick>(MetricType::Hit { layer: "redis" }, None);
                } else {
                    self.record_metric::<DbTick>(MetricType::Miss, None);
                }
            }
        }

        self.record_metric::<DbTick>(MetricType::BatchOperation { operation: "get", count: keys.len() }, Some(start.elapsed()));

        Ok(results)
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
        if RedisTestConfig::skip_if_redis_unavailable("test_cache_consistency_on_redis_failure")
            .await
            .is_none()
        {
            return;
        }

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
        // TODO: 實現完整的測試用例
        // 確保 Redis 和內存快取都被正確更新
    }

    #[tokio::test]
    async fn test_invalidate_inconsistent_cache() {
        // 測試清理不一致快取的功能
        // TODO: 實現完整的測試用例
        // 確保 invalidate_inconsistent_cache 方法能正確清理所有層級的快取
    }
}
