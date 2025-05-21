use async_trait::async_trait;
use deadpool_redis::redis::{cmd, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
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
}

/// 快取操作實現
pub struct CacheManager<P: RedisPool> {
    pool: P,
}

impl<P: RedisPool> CacheManager<P> {
    /// 創建新的快取管理器
    pub fn new(pool: P) -> Self {
        Self { pool }
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
            .arg(ttl_secs as u64)
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
                .arg(ttl as u64)
                .query_async::<()>(&mut conn)
                .await
            {
                Ok(_) => {
                    debug!("快取設置成功 (帶TTL): {}", prefixed_key);
                    Ok(())
                }
                Err(e) => {
                    error!("快取設置失敗 (帶TTL): {}", e);
                    Err(CacheError::RedisError(RedisClientError::ConnectionError(e)))
                }
            }
        } else {
            // 設置不帶TTL的值
            match conn.set::<_, _, String>(&prefixed_key, &serialized).await {
                Ok(_) => {
                    debug!("快取設置成功: {}", prefixed_key);
                    Ok(())
                }
                Err(e) => {
                    error!("快取設置失敗: {}", e);
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
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.pool.get_conn().await?;

        // 從Redis獲取值
        match conn.get::<_, Option<String>>(&prefixed_key).await {
            Ok(Some(value)) => {
                // 反序列化值
                match serde_json::from_str(&value) {
                    Ok(deserialized) => {
                        debug!("快取命中: {}", prefixed_key);
                        Ok(deserialized)
                    }
                    Err(e) => {
                        warn!("快取值反序列化失敗: {}", e);
                        Err(CacheError::DeserializationError(e.to_string()))
                    }
                }
            }
            Ok(None) => {
                debug!("快取未命中: {}", prefixed_key);
                Err(CacheError::CacheMiss(prefixed_key))
            }
            Err(e) => {
                error!("快取讀取失敗: {}", e);
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
                data: vec![1.0, 2.5, 3.14],
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
}
