use crate::redis::operations::cache::{CacheError, CacheManager, CacheOperations};
use crate::redis::pool::RedisPool;
use lru::LruCache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Arc;

/// 多層級快取實現，結合內存 LRU 快取與 Redis 快取
pub struct MultiLevelCache<P: RedisPool> {
    /// L1: 內存 LRU 快取（最快）
    memory_cache: Arc<RwLock<LruCache<String, Vec<u8>>>>,
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
    pub fn new(
        redis_cache: Arc<CacheManager<P>>,
        memory_capacity: usize,
        cache_ttl: u64,
    ) -> Self {
        let capacity = NonZeroUsize::new(memory_capacity).unwrap_or(NonZeroUsize::new(1000).unwrap());
        Self {
            memory_cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            redis_cache,
            cache_ttl,
        }
    }

    /// 從快取獲取數據
    ///
    /// 查詢順序：
    /// 1. 內存快取（L1）
    /// 2. Redis 快取（L2）
    /// 3. 如果 L2 命中，更新 L1
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, CacheError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
    {
        // 1. 嘗試從內存快取獲取
        {
            let mut cache = self.memory_cache.write();
            if let Some(data) = cache.get(key) {
                // 反序列化數據
                let config = bincode::config::standard();
                let value: T = bincode::decode_from_slice(data, config)
                    .map_err(|e| CacheError::SerializationError(e.to_string()))?
                    .0;
                return Ok(Some(value));
            }
        }

        // 2. 嘗試從 Redis 獲取
        if let Some(value) = self.redis_cache.get::<_, T>(key).await? {
            // 序列化以存入內存快取
            let config = bincode::config::standard();
            let serialized = bincode::encode_to_vec(&value, config)
                .map_err(|e| CacheError::SerializationError(e.to_string()))?;
            
            // 更新內存快取
            {
                let mut cache = self.memory_cache.write();
                cache.put(key.to_string(), serialized);
            }
            
            return Ok(Some(value));
        }

        Ok(None)
    }

    /// 設置快取數據
    ///
    /// 同時更新 L1 和 L2 快取
    pub async fn set<T>(&self, key: &str, value: &T) -> Result<(), CacheError>
    where
        T: Serialize + Send + Sync,
    {
        // 序列化數據
        let config = bincode::config::standard();
        let serialized = bincode::encode_to_vec(value, config)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;

        // 1. 更新內存快取
        {
            let mut cache = self.memory_cache.write();
            cache.put(key.to_string(), serialized);
        }

        // 2. 更新 Redis 快取
        self.redis_cache.set(key, value, Some(self.cache_ttl)).await?;

        Ok(())
    }

    /// 刪除快取數據
    ///
    /// 同時從 L1 和 L2 刪除
    pub async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        // 1. 從內存快取刪除
        let memory_deleted = {
            let mut cache = self.memory_cache.write();
            cache.pop(key).is_some()
        };

        // 2. 從 Redis 刪除
        let redis_deleted = self.redis_cache.delete(key).await?;

        Ok(memory_deleted || redis_deleted)
    }

    /// 檢查快取是否存在
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        // 1. 檢查內存快取
        {
            let cache = self.memory_cache.read();
            if cache.contains(key) {
                return Ok(true);
            }
        }

        // 2. 檢查 Redis
        self.redis_cache.exists(key).await
    }

    /// 獲取或設置快取
    ///
    /// 如果快取不存在，則執行提供的函數並快取結果
    pub async fn get_or_set<T, F, Fut>(&self, key: &str, f: F) -> Result<T, CacheError>
    where
        T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, CacheError>> + Send,
    {
        // 嘗試從快取獲取
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        // 執行函數獲取數據
        let value = f().await?;
        
        // 快取結果
        self.set(key, &value).await?;
        
        Ok(value)
    }

    /// 預熱快取
    ///
    /// 批量從 Redis 加載數據到內存快取
    pub async fn warm_cache(&self, keys: Vec<String>) -> Result<(), CacheError> {
        for key in keys {
            // 嘗試從 Redis 獲取並加載到內存
            let _: Option<Vec<u8>> = self.get(&key).await?;
        }
        Ok(())
    }

    /// 清空內存快取
    pub fn clear_memory_cache(&self) {
        let mut cache = self.memory_cache.write();
        cache.clear();
    }

    /// 獲取內存快取統計信息
    pub fn memory_cache_stats(&self) -> CacheStats {
        let cache = self.memory_cache.read();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
        }
    }
}

/// 快取統計信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 當前快取項目數
    pub size: usize,
    /// 快取容量
    pub capacity: usize,
}

/// 生成市場數據快取鍵
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
    format!("market_data:{}:{}:{}:{}", instrument_id, frequency, start_ts, end_ts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redis::operations::cache::CacheOperations;
    use crate::storage::models::market_data::MarketData;
    use chrono::{DateTime, Utc};
    use mockall::{mock, predicate::*};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    mock! {
        CacheOps {}
        
        #[async_trait]
        impl CacheOperations for CacheOps {
            async fn get<T: serde::de::DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>, CacheError>;
            async fn set<T: serde::Serialize + Send + Sync>(&self, key: &str, value: &T, ttl: Option<u64>) -> Result<(), CacheError>;
            async fn exists(&self, key: &str) -> Result<bool, CacheError>;
            async fn delete(&self, key: &str) -> Result<bool, CacheError>;
            async fn expire(&self, key: &str, seconds: u64) -> Result<bool, CacheError>;
            async fn get_or_set<T, F, Fut>(&self, key: &str, f: F, ttl: Option<u64>) -> Result<T, CacheError>
            where
                T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
                F: FnOnce() -> Fut + Send,
                Fut: std::future::Future<Output = Result<T, CacheError>> + Send;
        }
    }

    fn create_test_market_data() -> Vec<MarketData> {
        vec![
            MarketData {
                id: 1,
                instrument_id: 100,
                timestamp: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
                open: dec!(100.0),
                high: dec!(105.0),
                low: dec!(99.0),
                close: dec!(104.0),
                volume: dec!(1000000),
                frequency_seconds: 3600,
            },
            MarketData {
                id: 2,
                instrument_id: 100,
                timestamp: DateTime::parse_from_rfc3339("2024-01-01T01:00:00Z").unwrap().with_timezone(&Utc),
                open: dec!(104.0),
                high: dec!(106.0),
                low: dec!(103.0),
                close: dec!(105.5),
                volume: dec!(1200000),
                frequency_seconds: 3600,
            },
        ]
    }

    #[tokio::test]
    async fn test_multi_level_cache_get_from_memory() {
        let redis_cache = Arc::new(MockCacheOps::new());
        let cache = MultiLevelCache::new(redis_cache, 100, 300);
        
        let key = "test_key";
        let data = create_test_market_data();
        
        // Set data in memory cache
        cache.set(key, &data).await.unwrap();
        
        // Get should retrieve from memory without hitting Redis
        let result: Vec<MarketData> = cache.get(key).await.unwrap().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].close, dec!(104.0));
    }

    #[tokio::test]
    async fn test_multi_level_cache_get_from_redis() {
        let mut redis_cache = MockCacheOps::new();
        let data = create_test_market_data();
        let data_clone = data.clone();
        
        redis_cache
            .expect_get()
            .with(eq("test_key"))
            .times(1)
            .returning(move |_| Ok(Some(data_clone.clone())));
            
        let cache = MultiLevelCache::new(Arc::new(redis_cache), 100, 300);
        
        // Get should retrieve from Redis and populate memory cache
        let result: Vec<MarketData> = cache.get("test_key").await.unwrap().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].close, dec!(104.0));
        
        // Second get should come from memory cache
        let result2: Vec<MarketData> = cache.get("test_key").await.unwrap().unwrap();
        assert_eq!(result2.len(), 2);
    }

    #[tokio::test]
    async fn test_multi_level_cache_set() {
        let mut redis_cache = MockCacheOps::new();
        
        redis_cache
            .expect_set()
            .with(eq("test_key"), always(), eq(Some(300)))
            .times(1)
            .returning(|_, _, _| Ok(()));
            
        let cache = MultiLevelCache::new(Arc::new(redis_cache), 100, 300);
        let data = create_test_market_data();
        
        // Set should update both memory and Redis
        cache.set("test_key", &data).await.unwrap();
        
        // Verify data is in memory cache
        let result: Vec<MarketData> = cache.get("test_key").await.unwrap().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let redis_cache = Arc::new(MockCacheOps::new());
        let cache = MultiLevelCache::new(redis_cache, 2, 300); // Small capacity for testing
        
        // Add 3 items to trigger eviction
        cache.set("key1", &vec![1]).await.unwrap();
        cache.set("key2", &vec![2]).await.unwrap();
        cache.set("key3", &vec![3]).await.unwrap();
        
        // key1 should be evicted
        assert!(cache.get::<Vec<i32>>("key1").await.unwrap().is_none());
        // key2 and key3 should still be in memory
        assert_eq!(cache.get::<Vec<i32>>("key2").await.unwrap().unwrap(), vec![2]);
        assert_eq!(cache.get::<Vec<i32>>("key3").await.unwrap().unwrap(), vec![3]);
    }

    #[tokio::test]
    async fn test_cache_warming() {
        let mut redis_cache = MockCacheOps::new();
        let data1 = vec![1, 2, 3];
        let data2 = vec![4, 5, 6];
        let data1_clone = data1.clone();
        let data2_clone = data2.clone();
        
        redis_cache
            .expect_get()
            .with(eq("warm_key1"))
            .times(1)
            .returning(move |_| Ok(Some(data1_clone.clone())));
            
        redis_cache
            .expect_get()
            .with(eq("warm_key2"))
            .times(1)
            .returning(move |_| Ok(Some(data2_clone.clone())));
            
        let cache = MultiLevelCache::new(Arc::new(redis_cache), 100, 300);
        
        // Warm the cache
        cache.warm_cache(vec!["warm_key1".to_string(), "warm_key2".to_string()]).await.unwrap();
        
        // Data should be in memory cache now
        assert_eq!(cache.get::<Vec<i32>>("warm_key1").await.unwrap().unwrap(), vec![1, 2, 3]);
        assert_eq!(cache.get::<Vec<i32>>("warm_key2").await.unwrap().unwrap(), vec![4, 5, 6]);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let key = generate_cache_key(100, "1h", 1704067200, 1704153600);
        assert_eq!(key, "market_data:100:1h:1704067200:1704153600");
    }

    #[tokio::test]
    async fn test_cache_consistency() {
        let mut redis_cache = MockCacheOps::new();
        
        // Redis set should be called when setting data
        redis_cache
            .expect_set()
            .times(1)
            .returning(|_, _, _| Ok(()));
            
        // Redis delete should be called when deleting data
        redis_cache
            .expect_delete()
            .with(eq("test_key"))
            .times(1)
            .returning(|_| Ok(true));
            
        let cache = MultiLevelCache::new(Arc::new(redis_cache), 100, 300);
        let data = vec![1, 2, 3];
        
        // Set data
        cache.set("test_key", &data).await.unwrap();
        
        // Delete should remove from both layers
        cache.delete("test_key").await.unwrap();
        
        // Verify data is removed from memory
        assert!(cache.get::<Vec<i32>>("test_key").await.unwrap().is_none());
    }
}