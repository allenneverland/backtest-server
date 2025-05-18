use std::time::Duration;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tracing::{debug, error, warn};
use redis::AsyncCommands;

use crate::storage::redis::pool::{RedisPool, RedisPoolError};
use crate::storage::redis::client::RedisClientError;

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
    async fn get_or_set<K, V, F, Fut>(&self, key: K, ttl_secs: Option<u64>, generator: F) -> Result<V, CacheError>
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
            let result: redis::RedisResult<()> = redis::cmd("SET")
                .arg(&prefixed_key)
                .arg(&serialized)
                .arg("EX")
                .arg(ttl)
                .query_async(&mut *conn)
                .await;

            if let Err(e) = result {
                error!("快取設置失敗 (帶TTL): {}", e);
                return Err(CacheError::RedisError(RedisClientError::ConnectionError(e)));
            }
        } else {
            // 設置不帶TTL的值
            let result: redis::RedisResult<()> = conn.set(&prefixed_key, &serialized).await;
            if let Err(e) = result {
                error!("快取設置失敗: {}", e);
                return Err(CacheError::RedisError(RedisClientError::ConnectionError(e)));
            }
        }

        debug!("快取設置成功: {}", prefixed_key);
        Ok(())
    }

    async fn get<K, V>(&self, key: K) -> Result<V, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Send + Sync,
    {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.pool.get_conn().await?;

        // 從Redis獲取值
        let result: redis::RedisResult<Option<String>> = conn.get(&prefixed_key).await;
        
        match result {
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

        match conn.exists(&prefixed_key).await {
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

        match conn.del(&prefixed_key).await {
            Ok(deleted) => {
                debug!("快取刪除 {}: {}", prefixed_key, if deleted { "成功" } else { "鍵不存在" });
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

        match conn.expire(&prefixed_key, ttl_secs as usize).await {
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

    async fn get_or_set<K, V, F, Fut>(&self, key: K, ttl_secs: Option<u64>, generator: F) -> Result<V, CacheError>
    where
        K: AsRef<str> + Send + Sync,
        V: DeserializeOwned + Serialize + Send + Sync,
        F: FnOnce() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<V, CacheError>> + Send,
    {
        // 先嘗試從快取獲取
        match self.get::<_, V>(key.as_ref()).await {
            Ok(value) => {
                debug!("快取命中: {}", key.as_ref());
                Ok(value)
            }
            Err(CacheError::CacheMiss(_)) => {
                debug!("快取未命中，生成新值: {}", key.as_ref());
                // 生成新值
                let value = generator().await?;
                
                // 存儲到快取
                self.set(key.as_ref(), &value, ttl_secs).await?;
                
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::redis::pool::test_helpers::create_test_pool;
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
                data: vec![1.1, 2.2, 3.3],
            }
        }
    }
    
    #[tokio::test]
    async fn test_cache_operations() {
        // 跳過測試，除非環境中有Redis可用
        let redis_available = std::env::var("REDIS_TEST_AVAILABLE").unwrap_or_else(|_| "false".to_string());
        if redis_available != "true" {
            println!("跳過Redis快取測試 - 無Redis環境可用");
            return;
        }
        
        let pool = create_test_pool().await.expect("無法創建測試連接池");
        let cache = CacheManager::new(pool);
        
        // 測試對象
        let test_obj = TestObject::new(42, "test_object");
        let cache_key = "test_cache_key";
        
        // 測試SET和GET
        cache.set(cache_key, &test_obj, None).await.expect("設置快取失敗");
        let retrieved: TestObject = cache.get(cache_key).await.expect("獲取快取失敗");
        assert_eq!(retrieved, test_obj);
        
        // 測試EXISTS
        assert!(cache.exists(cache_key).await.expect("檢查快取失敗"));
        assert!(!cache.exists("non_existent_key").await.expect("檢查快取失敗"));
        
        // 測試過期
        let expire_key = "test_expire_key";
        cache.set(expire_key, &test_obj, Some(1)).await.expect("設置快取失敗");
        assert!(cache.exists(expire_key).await.expect("檢查快取失敗"));
        
        // 等待過期
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(!cache.exists(expire_key).await.expect("檢查快取失敗"));
        
        // 測試DELETE
        assert!(cache.delete(cache_key).await.expect("刪除快取失敗"));
        assert!(!cache.exists(cache_key).await.expect("檢查快取失敗"));
        
        // 測試GET_OR_SET
        let gen_key = "test_generate_key";
        let generated: TestObject = cache
            .get_or_set(gen_key, Some(60), || async {
                // 模擬生成新值
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(TestObject::new(99, "generated_object"))
            })
            .await
            .expect("get_or_set失敗");
        
        assert_eq!(generated.id, 99);
        
        // 再次調用應該從快取獲取
        let cached: TestObject = cache
            .get_or_set(gen_key, Some(60), || async {
                // 這個不應該被調用
                Ok(TestObject::new(88, "should_not_be_called"))
            })
            .await
            .expect("get_or_set失敗");
        
        assert_eq!(cached.id, 99); // 應該返回之前生成的值
        
        // 清理
        let _ = cache.delete(gen_key).await;
    }
} 