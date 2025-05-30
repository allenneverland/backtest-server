use async_trait::async_trait;
use deadpool_redis::redis::{cmd, AsyncCommands};
use metrics::{counter, histogram};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, error, warn};

use crate::redis::client::RedisClientError;
use crate::redis::pool::{RedisPool, RedisPoolError};

/// 快取操作錯誤
#[derive(Error, Debug)]
pub enum CacheError {
    /// Redis連接錯誤
    #[error("Redis連接錯誤: {0}")]
    ConnectionError(#[from] RedisPoolError),

    /// Redis操作錯誤
    #[error("Redis操作錯誤: {0}")]
    RedisError(#[from] RedisClientError),

    /// 序列化錯誤
    #[error("數據序列化錯誤: {0}")]
    SerializationError(String),

    /// 反序列化錯誤
    #[error("數據反序列化錯誤: {0}")]
    DeserializationError(String),

    /// 快取miss
    #[error("快取未命中: {0}")]
    CacheMiss(String),

    /// 其他錯誤
    #[error("快取操作其他錯誤: {0}")]
    Other(String),
}

/// 快取操作接口
#[async_trait]
pub trait CacheOperations: Send + Sync + 'static {
    /// 存儲值到快取
    async fn set<K, V>(&self, key: K, value: &V, ttl_secs: Option<u64>) -> Result<(), CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: Serialize + Send + Sync;

    /// 從快取獲取值
    async fn get<K, V>(&self, key: K) -> Result<V, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Send + Sync;

    /// 檢查快取中是否存在鍵
    async fn exists<K>(&self, key: K) -> Result<bool, CacheError>
    where
        K: AsRef<str> + Send + Sync;

    /// 從快取中刪除鍵
    async fn delete<K>(&self, key: K) -> Result<bool, CacheError>
    where
        K: AsRef<str> + Send + Sync;

    /// 設置鍵的過期時間
    async fn expire<K>(&self, key: K, ttl_secs: u64) -> Result<bool, CacheError>
    where
        K: AsRef<str> + Send + Sync;

    /// 從快取獲取值，如果不存在則使用提供的函數生成
    async fn get_or_set<K, V, F, Fut>(
        &self,
        key: K,
        ttl_secs: Option<u64>,
        generator: F,
    ) -> Result<V, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Serialize + Send + Sync,
        F: FnOnce() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<V, CacheError>> + Send;

    /// 批量獲取多個鍵的值
    async fn mget<K, V>(&self, keys: &[K]) -> Result<Vec<Option<V>>, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Send + Sync;

    /// 批量設置多個鍵值對
    async fn mset<K, V>(&self, items: &[(K, V)], ttl_secs: Option<u64>) -> Result<(), CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: Serialize + Send + Sync;
}

/// Redis 快取監控指標命名空間
const REDIS_METRIC_NAMESPACE: &str = "redis_cache";

/// 快取操作實現
pub struct CacheManager<P: RedisPool> {
    pool: P,
}

impl<P: RedisPool> CacheManager<P> {
    /// 記錄 Redis 操作指標
    fn record_redis_operation(
        &self,
        operation: &str,
        success: bool,
        duration: std::time::Duration,
    ) {
        let status = if success { "success" } else { "error" };

        counter!(
            format!("{}.operation", REDIS_METRIC_NAMESPACE),
            "operation" => operation.to_string(),
            "status" => status.to_string()
        )
        .increment(1);

        histogram!(
            format!("{}.latency_ns", REDIS_METRIC_NAMESPACE),
            "operation" => operation.to_string()
        )
        .record(duration.as_nanos() as f64);
    }

    /// 創建新的快取管理器
    pub fn new(pool: P) -> Self {
        Self { pool }
    }

    /// 獲取 Redis 連接 - 用於 Pipeline 操作
    pub async fn get_connection(&self) -> Result<deadpool_redis::Connection, CacheError> {
        self.pool
            .get_conn()
            .await
            .map_err(CacheError::ConnectionError)
    }

    /// 生成快取鍵前綴
    fn prefix_key<K: AsRef<str>>(&self, key: K) -> String {
        // 可以在這裡添加應用名稱或其他前綴
        format!("cache:{}", key.as_ref())
    }

    /// 設置緩存，帶過期時間
    pub async fn set_with_ttl<K: AsRef<str>, V: Serialize>(
        &self,
        key: K,
        value: &V,
        ttl_secs: u64,
    ) -> Result<(), CacheError> {
        let prefixed_key = self.prefix_key(key);
        let serialized = serde_json::to_string(value)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;

        debug!("設置緩存 [{}] 帶過期時間: {}秒", prefixed_key, ttl_secs);

        let mut conn = self.pool.get_conn().await?;

        // 使用標準的 Redis set 命令帶過期時間
        match cmd("SET")
            .arg(&prefixed_key)
            .arg(&serialized)
            .arg("EX")
            .arg(ttl_secs)
            .query_async::<()>(&mut conn)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("緩存設置失敗: {}", e);
                Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
            }
        }
    }
}

#[async_trait]
impl<P: RedisPool> CacheOperations for CacheManager<P> {
    async fn set<K, V>(&self, key: K, value: &V, ttl_secs: Option<u64>) -> Result<(), CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: Serialize + Send + Sync,
    {
        let start = Instant::now();

        // 序列化值
        let serialized = serde_json::to_string(value)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;

        let prefixed_key = self.prefix_key(key);
        let mut conn = self.pool.get_conn().await?;

        if let Some(ttl) = ttl_secs {
            // 設置帶TTL的值
            match cmd("SET")
                .arg(&prefixed_key)
                .arg(&serialized)
                .arg("EX")
                .arg(ttl)
                .query_async::<()>(&mut conn)
                .await
            {
                Ok(_) => {
                    debug!("快取設置成功 (帶TTL): {}", prefixed_key);
                    self.record_redis_operation("set_with_ttl", true, start.elapsed());
                    Ok(())
                }
                Err(e) => {
                    error!("快取設置失敗 (帶TTL): {}", e);
                    self.record_redis_operation("set_with_ttl", false, start.elapsed());
                    Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
                }
            }
        } else {
            // 設置不帶TTL的值
            match conn.set::<_, _, String>(&prefixed_key, &serialized).await {
                Ok(_) => {
                    debug!("快取設置成功: {}", prefixed_key);
                    self.record_redis_operation("set", true, start.elapsed());
                    Ok(())
                }
                Err(e) => {
                    error!("快取設置失敗: {}", e);
                    self.record_redis_operation("set", false, start.elapsed());
                    Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
                }
            }
        }
    }

    async fn get<K, V>(&self, key: K) -> Result<V, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Send + Sync,
    {
        let start = Instant::now();
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.pool.get_conn().await?;

        // 從Redis獲取值
        match conn.get::<_, Option<String>>(&prefixed_key).await {
            Ok(Some(value)) => {
                // 反序列化值
                match serde_json::from_str(&value) {
                    Ok(deserialized) => {
                        debug!("快取命中: {}", prefixed_key);
                        self.record_redis_operation("get_hit", true, start.elapsed());
                        Ok(deserialized)
                    }
                    Err(e) => {
                        warn!("快取值反序列化失敗: {}", e);
                        self.record_redis_operation(
                            "get_deserialization_error",
                            false,
                            start.elapsed(),
                        );
                        Err(CacheError::DeserializationError(e.to_string()))
                    }
                }
            }
            Ok(None) => {
                debug!("快取未命中: {}", prefixed_key);
                self.record_redis_operation("get_miss", true, start.elapsed());
                Err(CacheError::CacheMiss(prefixed_key))
            }
            Err(e) => {
                error!("快取讀取失敗: {}", e);
                self.record_redis_operation("get_error", false, start.elapsed());
                Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
            }
        }
    }

    async fn exists<K>(&self, key: K) -> Result<bool, CacheError>
    where
        K: AsRef<str> + Send + Sync,
    {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.pool.get_conn().await?;

        match conn.exists::<_, bool>(&prefixed_key).await {
            Ok(exists) => Ok(exists),
            Err(e) => {
                error!("快取鍵檢查失敗: {}", e);
                Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
            }
        }
    }

    async fn delete<K>(&self, key: K) -> Result<bool, CacheError>
    where
        K: AsRef<str> + Send + Sync,
    {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.pool.get_conn().await?;

        match conn.del::<_, bool>(&prefixed_key).await {
            Ok(deleted) => {
                debug!(
                    "快取刪除 {}: {}",
                    prefixed_key,
                    if deleted { "成功" } else { "鍵不存在" }
                );
                Ok(deleted)
            }
            Err(e) => {
                error!("快取刪除失敗: {}", e);
                Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
            }
        }
    }

    async fn expire<K>(&self, key: K, ttl_secs: u64) -> Result<bool, CacheError>
    where
        K: AsRef<str> + Send + Sync,
    {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.pool.get_conn().await?;

        match conn.expire::<_, bool>(&prefixed_key, ttl_secs as i64).await {
            Ok(set) => {
                debug!(
                    "快取過期時間設置 {}: {} ({}秒)",
                    prefixed_key,
                    if set { "成功" } else { "鍵不存在" },
                    ttl_secs
                );
                Ok(set)
            }
            Err(e) => {
                error!("快取過期時間設置失敗: {}", e);
                Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
            }
        }
    }

    async fn get_or_set<K, V, F, Fut>(
        &self,
        key: K,
        ttl_secs: Option<u64>,
        generator: F,
    ) -> Result<V, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Serialize + Send + Sync,
        F: FnOnce() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<V, CacheError>> + Send,
    {
        let prefixed_key = self.prefix_key(key);

        // 先嘗試從快取獲取
        match self.get::<_, V>(&prefixed_key).await {
            Ok(value) => {
                debug!("快取命中(get_or_set): {}", prefixed_key);
                Ok(value)
            }
            Err(CacheError::CacheMiss(_)) => {
                // 快取未命中，執行生成函數
                debug!("快取未命中(get_or_set)，執行生成函數: {}", prefixed_key);
                let value = generator().await?;

                // 將結果存入快取
                self.set(&prefixed_key, &value, ttl_secs).await?;

                Ok(value)
            }
            Err(e) => Err(e), // 其他錯誤直接返回
        }
    }

    async fn mget<K, V>(&self, keys: &[K]) -> Result<Vec<Option<V>>, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Send + Sync,
    {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let prefixed_keys: Vec<String> = keys.iter().map(|k| self.prefix_key(k)).collect();
        let mut conn = self.pool.get_conn().await?;

        match conn.get::<_, Vec<Option<String>>>(&prefixed_keys).await {
            Ok(values) => {
                let mut results = Vec::with_capacity(values.len());

                for (idx, value) in values.into_iter().enumerate() {
                    match value {
                        Some(serialized) => match serde_json::from_str::<V>(&serialized) {
                            Ok(deserialized) => {
                                debug!("批量快取命中: {}", prefixed_keys[idx]);
                                results.push(Some(deserialized));
                            }
                            Err(e) => {
                                warn!("批量快取值反序列化失敗: {}", e);
                                results.push(None);
                            }
                        },
                        None => {
                            debug!("批量快取未命中: {}", prefixed_keys[idx]);
                            results.push(None);
                        }
                    }
                }

                Ok(results)
            }
            Err(e) => {
                error!("批量快取讀取失敗: {}", e);
                Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
            }
        }
    }

    async fn mset<K, V>(&self, items: &[(K, V)], ttl_secs: Option<u64>) -> Result<(), CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: Serialize + Send + Sync,
    {
        if items.is_empty() {
            return Ok(());
        }

        let mut conn = self.pool.get_conn().await?;

        // 準備批量設置的數據
        let mut prefixed_items = Vec::with_capacity(items.len());
        for (key, value) in items {
            let prefixed_key = self.prefix_key(key);
            let serialized = serde_json::to_string(value)
                .map_err(|e| CacheError::SerializationError(e.to_string()))?;
            prefixed_items.push((prefixed_key, serialized));
        }

        if let Some(ttl) = ttl_secs {
            // 使用 pipeline 進行批量設置帶TTL
            let mut pipe = deadpool_redis::redis::pipe();
            for (key, value) in &prefixed_items {
                pipe.cmd("SET").arg(key).arg(value).arg("EX").arg(ttl);
            }

            match pipe.query_async::<()>(&mut conn).await {
                Ok(_) => {
                    debug!("批量快取設置成功 (帶TTL): {} 項", prefixed_items.len());
                    Ok(())
                }
                Err(e) => {
                    error!("批量快取設置失敗 (帶TTL): {}", e);
                    Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
                }
            }
        } else {
            // 使用 MSET 進行批量設置不帶TTL
            match conn.mset::<String, String, ()>(&prefixed_items).await {
                Ok(_) => {
                    debug!("批量快取設置成功: {} 項", items.len());
                    Ok(())
                }
                Err(e) => {
                    error!("批量快取設置失敗: {}", e);
                    Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestObject {
        id: i32,
        name: String,
        data: Vec<f64>,
    }

    impl TestObject {
        fn new(id: i32, name: &str) -> Self {
            Self {
                id,
                name: name.to_string(),
                data: vec![1.0, 2.5, 3.5],
            }
        }
    }

    #[tokio::test]
    async fn test_cache_operations() {
        // 跳過測試，除非環境中有Redis可用
        let redis_available =
            std::env::var("REDIS_TEST_AVAILABLE").unwrap_or_else(|_| "false".to_string());
        if redis_available != "true" {
            println!("跳過Redis快取測試 - 無Redis環境可用");
            return;
        }

        // 創建Redis連接池和快取管理器
        let pool = crate::redis::pool::test_helpers::create_test_pool()
            .await
            .expect("無法創建測試Redis連接池");

        let cache = CacheManager::new(pool);

        // 測試對象
        let test_key = "test_cache_key";
        let test_obj = TestObject::new(42, "測試對象");

        // 測試設置和獲取
        cache
            .set(test_key, &test_obj, Some(60))
            .await
            .expect("設置快取失敗");

        let retrieved: TestObject = cache.get(test_key).await.expect("獲取快取失敗");
        assert_eq!(retrieved, test_obj);

        // 測試存在性檢查
        let exists = cache.exists(test_key).await.expect("檢查快取存在性失敗");
        assert!(exists);

        // 測試過期時間設置
        let expired = cache.expire(test_key, 300).await.expect("設置過期時間失敗");
        assert!(expired);

        // 測試刪除
        let deleted = cache.delete(test_key).await.expect("刪除快取失敗");
        assert!(deleted);

        // 驗證刪除後不存在
        let exists_after = cache.exists(test_key).await.expect("檢查快取存在性失敗");
        assert!(!exists_after);

        // 測試 get_or_set 功能
        let result: TestObject = cache
            .get_or_set("new_key", Some(30), || async {
                Ok(TestObject::new(99, "動態生成的對象"))
            })
            .await
            .expect("get_or_set 失敗");

        assert_eq!(result.id, 99);

        // 再次使用 get_or_set 應該從快取獲取
        let cached: TestObject = cache
            .get_or_set("new_key", Some(30), || async {
                Ok(TestObject::new(100, "這不應該被返回"))
            })
            .await
            .expect("get_or_set 失敗");

        assert_eq!(cached.id, 99); // 應該返回快取的結果，而不是新生成的

        // 清理
        let _ = cache.delete("new_key").await;
    }

    #[tokio::test]
    async fn test_batch_operations() {
        // 跳過測試，除非環境中有Redis可用
        let redis_available =
            std::env::var("REDIS_TEST_AVAILABLE").unwrap_or_else(|_| "false".to_string());
        if redis_available != "true" {
            println!("跳過Redis批量操作測試 - 無Redis環境可用");
            return;
        }

        // 創建Redis連接池和快取管理器
        let pool = crate::redis::pool::test_helpers::create_test_pool()
            .await
            .expect("無法創建測試Redis連接池");

        let cache = CacheManager::new(pool);

        // 準備測試數據
        let test_items = vec![
            ("batch_key_1".to_string(), TestObject::new(1, "批量測試1")),
            ("batch_key_2".to_string(), TestObject::new(2, "批量測試2")),
            ("batch_key_3".to_string(), TestObject::new(3, "批量測試3")),
        ];

        // 測試批量設置
        cache
            .mset(&test_items, Some(60))
            .await
            .expect("批量設置失敗");

        // 測試批量獲取
        let keys: Vec<String> = test_items.iter().map(|(k, _)| k.clone()).collect();
        let results: Vec<Option<TestObject>> = cache.mget(&keys).await.expect("批量獲取失敗");

        // 驗證結果
        assert_eq!(results.len(), 3);
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_some());
            if let Some(obj) = result {
                assert_eq!(obj.id, (i + 1) as i32);
            }
        }

        // 測試部分存在的情況
        let mixed_keys = vec![
            "batch_key_1".to_string(),
            "non_existent_key".to_string(),
            "batch_key_3".to_string(),
        ];
        let mixed_results: Vec<Option<TestObject>> =
            cache.mget(&mixed_keys).await.expect("混合批量獲取失敗");

        assert_eq!(mixed_results.len(), 3);
        assert!(mixed_results[0].is_some()); // batch_key_1 存在
        assert!(mixed_results[1].is_none()); // non_existent_key 不存在
        assert!(mixed_results[2].is_some()); // batch_key_3 存在

        // 清理測試數據
        for (key, _) in &test_items {
            let _ = cache.delete(key).await;
        }
    }

    #[tokio::test]
    async fn test_empty_batch_operations() {
        // 跳過測試，除非環境中有Redis可用
        let redis_available =
            std::env::var("REDIS_TEST_AVAILABLE").unwrap_or_else(|_| "false".to_string());
        if redis_available != "true" {
            println!("跳過Redis空批量操作測試 - 無Redis環境可用");
            return;
        }

        let pool = crate::redis::pool::test_helpers::create_test_pool()
            .await
            .expect("無法創建測試Redis連接池");

        let cache = CacheManager::new(pool);

        // 測試空的批量操作
        let empty_items: Vec<(String, TestObject)> = vec![];
        let empty_keys: Vec<String> = vec![];

        // 空批量設置應該成功
        cache
            .mset(&empty_items, None)
            .await
            .expect("空批量設置失敗");

        // 空批量獲取應該返回空結果
        let empty_results: Vec<Option<TestObject>> =
            cache.mget(&empty_keys).await.expect("空批量獲取失敗");
        assert!(empty_results.is_empty());
    }
}
