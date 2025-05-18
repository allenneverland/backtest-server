use std::time::Duration;
use async_trait::async_trait;
use redis::{AsyncCommands, RedisError, Client as RedisClient};
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use crate::config::types::RedisConfig;

/// Redis客戶端錯誤
#[derive(Error, Debug)]
pub enum RedisClientError {
    /// Redis連接錯誤
    #[error("Redis連接錯誤: {0}")]
    ConnectionError(#[from] RedisError),

    /// 操作超時錯誤
    #[error("Redis操作超時: {0}")]
    TimeoutError(String),

    /// 序列化/反序列化錯誤
    #[error("數據序列化錯誤: {0}")]
    SerializationError(String),

    /// 值類型錯誤
    #[error("Redis值類型錯誤: {0}")]
    TypeError(String),

    /// 其他錯誤
    #[error("Redis其他錯誤: {0}")]
    Other(String),
}

/// Redis操作特質
#[async_trait]
pub trait RedisOperations: Send + Sync + 'static {
    /// 設置鍵值對
    async fn set<K: AsRef<str> + Send + Sync, V: AsRef<str> + Send + Sync>(
        &self, 
        key: K, 
        value: V, 
        expiry_secs: Option<u64>
    ) -> Result<(), RedisClientError>;

    /// 獲取鍵對應的值
    async fn get<K: AsRef<str> + Send + Sync>(&self, key: K) -> Result<Option<String>, RedisClientError>;

    /// 刪除鍵
    async fn delete<K: AsRef<str> + Send + Sync>(&self, key: K) -> Result<bool, RedisClientError>;

    /// 檢查鍵是否存在
    async fn exists<K: AsRef<str> + Send + Sync>(&self, key: K) -> Result<bool, RedisClientError>;

    /// 設置鍵的過期時間
    async fn expire<K: AsRef<str> + Send + Sync>(&self, key: K, seconds: u64) -> Result<bool, RedisClientError>;

    /// 執行PING命令
    async fn ping(&self) -> Result<String, RedisClientError>;
}

/// Redis客戶端實現
pub struct Client {
    client: RedisClient,
    config: RedisConfig,
}

impl Client {
    /// 創建新的Redis客戶端
    pub fn new(config: RedisConfig) -> Result<Self, RedisClientError> {
        let client = RedisClient::open(config.url.clone())
            .map_err(RedisClientError::ConnectionError)?;

        Ok(Self {
            client,
            config,
        })
    }

    /// 建立異步連接
    pub async fn get_async_connection(&self) -> Result<redis::aio::Connection, RedisClientError> {
        let connect_timeout = Duration::from_secs(self.config.connection_timeout_secs);

        let connection_future = self.client.get_async_connection();
        match timeout(connect_timeout, connection_future).await {
            Ok(result) => {
                match result {
                    Ok(conn) => {
                        debug!("成功建立Redis連接");
                        Ok(conn)
                    }
                    Err(err) => {
                        error!("無法建立Redis連接: {}", err);
                        Err(RedisClientError::ConnectionError(err))
                    }
                }
            }
            Err(_) => {
                error!("Redis連接超時 ({}秒)", self.config.connection_timeout_secs);
                Err(RedisClientError::TimeoutError(format!(
                    "連接超時 ({}秒)",
                    self.config.connection_timeout_secs
                )))
            }
        }
    }

    /// 測試連接是否有效
    pub async fn test_connection(&self) -> bool {
        match self.ping().await {
            Ok(_) => true,
            Err(err) => {
                warn!("Redis連接測試失敗: {}", err);
                false
            }
        }
    }

    /// 使用重試策略執行Redis操作
    async fn with_retry_connection<F, Fut, T>(&self, operation: F) -> Result<T, RedisClientError>
    where
        F: Fn(redis::aio::Connection) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(redis::aio::Connection, T), RedisError>> + Send,
        T: Send,
    {
        let mut attempts = 0;
        let max_attempts = self.config.reconnect_attempts as usize + 1; // +1 for initial attempt
        let retry_delay = Duration::from_secs(self.config.reconnect_delay_secs);

        loop {
            attempts += 1;
            let mut conn = self.get_async_connection().await?;

            match operation(conn).await {
                Ok((new_conn, result)) => {
                    conn = new_conn;
                    return Ok(result);
                }
                Err(err) => {
                    if attempts >= max_attempts {
                        return Err(RedisClientError::ConnectionError(err));
                    }

                    warn!(
                        "Redis操作失敗 (嘗試 {}/{}): {}. 重試中...",
                        attempts, max_attempts, err
                    );

                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }
}

#[async_trait]
impl RedisOperations for Client {
    async fn set<K: AsRef<str> + Send + Sync, V: AsRef<str> + Send + Sync>(
        &self,
        key: K,
        value: V,
        expiry_secs: Option<u64>,
    ) -> Result<(), RedisClientError> {
        self.with_retry_connection(|mut conn| async move {
            // 設置操作超時
            let op_timeout = Duration::from_secs(self.config.write_timeout_secs);
            
            let result = if let Some(expiry) = expiry_secs {
                // 設置帶過期時間的鍵值對
                let fut = redis::cmd("SET")
                    .arg(key.as_ref())
                    .arg(value.as_ref())
                    .arg("EX")
                    .arg(expiry)
                    .query_async(&mut conn);
                
                timeout(op_timeout, fut).await
            } else {
                // 設置不帶過期時間的鍵值對
                let fut = conn.set(key.as_ref(), value.as_ref());
                timeout(op_timeout, fut).await
            };

            match result {
                Ok(redis_result) => match redis_result {
                    Ok(_) => Ok((conn, ())),
                    Err(err) => Err(err),
                },
                Err(_) => Err(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Redis SET操作超時 ({}秒)", self.config.write_timeout_secs),
                ))),
            }
        })
        .await
    }

    async fn get<K: AsRef<str> + Send + Sync>(&self, key: K) -> Result<Option<String>, RedisClientError> {
        self.with_retry_connection(|mut conn| async move {
            // 設置操作超時
            let op_timeout = Duration::from_secs(self.config.read_timeout_secs);
            
            let fut = conn.get(key.as_ref());
            let result = timeout(op_timeout, fut).await;

            match result {
                Ok(redis_result) => match redis_result {
                    Ok(value) => Ok((conn, value)),
                    Err(err) => Err(err),
                },
                Err(_) => Err(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Redis GET操作超時 ({}秒)", self.config.read_timeout_secs),
                ))),
            }
        })
        .await
    }

    async fn delete<K: AsRef<str> + Send + Sync>(&self, key: K) -> Result<bool, RedisClientError> {
        self.with_retry_connection(|mut conn| async move {
            // 設置操作超時
            let op_timeout = Duration::from_secs(self.config.write_timeout_secs);
            
            let fut = conn.del(key.as_ref());
            let result = timeout(op_timeout, fut).await;

            match result {
                Ok(redis_result) => match redis_result {
                    Ok(deleted) => Ok((conn, deleted)),
                    Err(err) => Err(err),
                },
                Err(_) => Err(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Redis DEL操作超時 ({}秒)", self.config.write_timeout_secs),
                ))),
            }
        })
        .await
    }

    async fn exists<K: AsRef<str> + Send + Sync>(&self, key: K) -> Result<bool, RedisClientError> {
        self.with_retry_connection(|mut conn| async move {
            // 設置操作超時
            let op_timeout = Duration::from_secs(self.config.read_timeout_secs);
            
            let fut = conn.exists(key.as_ref());
            let result = timeout(op_timeout, fut).await;

            match result {
                Ok(redis_result) => match redis_result {
                    Ok(exists) => Ok((conn, exists)),
                    Err(err) => Err(err),
                },
                Err(_) => Err(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Redis EXISTS操作超時 ({}秒)", self.config.read_timeout_secs),
                ))),
            }
        })
        .await
    }

    async fn expire<K: AsRef<str> + Send + Sync>(&self, key: K, seconds: u64) -> Result<bool, RedisClientError> {
        self.with_retry_connection(|mut conn| async move {
            // 設置操作超時
            let op_timeout = Duration::from_secs(self.config.write_timeout_secs);
            
            let fut = conn.expire(key.as_ref(), seconds as usize);
            let result = timeout(op_timeout, fut).await;

            match result {
                Ok(redis_result) => match redis_result {
                    Ok(changed) => Ok((conn, changed)),
                    Err(err) => Err(err),
                },
                Err(_) => Err(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Redis EXPIRE操作超時 ({}秒)", self.config.write_timeout_secs),
                ))),
            }
        })
        .await
    }

    async fn ping(&self) -> Result<String, RedisClientError> {
        self.with_retry_connection(|mut conn| async move {
            // 設置操作超時
            let op_timeout = Duration::from_secs(self.config.read_timeout_secs);
            
            let fut = redis::cmd("PING").query_async(&mut conn);
            let result = timeout(op_timeout, fut).await;

            match result {
                Ok(redis_result) => match redis_result {
                    Ok(pong) => Ok((conn, pong)),
                    Err(err) => Err(err),
                },
                Err(_) => Err(RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Redis PING操作超時 ({}秒)", self.config.read_timeout_secs),
                ))),
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::RedisConfig;

    fn create_test_config() -> RedisConfig {
        RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 5,
            connection_timeout_secs: 5,
            read_timeout_secs: 5,
            write_timeout_secs: 5,
            reconnect_attempts: 3,
            reconnect_delay_secs: 1,
        }
    }

    #[tokio::test]
    async fn test_redis_operations() {
        // 跳過測試，除非環境中有Redis可用
        let redis_available = std::env::var("REDIS_TEST_AVAILABLE").unwrap_or_else(|_| "false".to_string());
        if redis_available != "true" {
            println!("跳過Redis測試 - 無Redis環境可用");
            return;
        }

        let config = create_test_config();
        let client = Client::new(config).expect("無法創建Redis客戶端");

        // 測試SET和GET
        client.set("test_key", "test_value", None).await.expect("SET失敗");
        let value = client.get("test_key").await.expect("GET失敗");
        assert_eq!(value, Some("test_value".to_string()));

        // 測試帶過期時間的SET
        client.set("test_expire_key", "test_expire_value", Some(1)).await.expect("SET EX失敗");
        let value = client.get("test_expire_key").await.expect("GET失敗");
        assert_eq!(value, Some("test_expire_value".to_string()));

        // 等待過期
        tokio::time::sleep(Duration::from_secs(2)).await;
        let value = client.get("test_expire_key").await.expect("GET失敗");
        assert_eq!(value, None);

        // 測試EXISTS
        assert!(client.exists("test_key").await.expect("EXISTS失敗"));
        assert!(!client.exists("non_existent_key").await.expect("EXISTS失敗"));

        // 測試DELETE
        assert!(client.delete("test_key").await.expect("DELETE失敗"));
        assert!(!client.exists("test_key").await.expect("EXISTS失敗"));

        // 測試PING
        let pong = client.ping().await.expect("PING失敗");
        assert_eq!(pong, "PONG");
    }
} 