use crate::config::types::RedisConfig;
use async_trait::async_trait;
use redis::{AsyncCommands, Client as RedisClient, RedisError};
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, error, warn};

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
    async fn set<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
        value: String,
        expiry_secs: Option<u64>,
    ) -> Result<(), RedisClientError>;

    /// 獲取鍵對應的值
    async fn get<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
    ) -> Result<Option<String>, RedisClientError>;

    /// 刪除鍵
    async fn delete<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
    ) -> Result<bool, RedisClientError>;

    /// 檢查鍵是否存在
    async fn exists<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
    ) -> Result<bool, RedisClientError>;

    /// 設置鍵的過期時間
    async fn expire<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
        seconds: u64,
    ) -> Result<bool, RedisClientError>;

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
        let client =
            RedisClient::open(config.url.clone()).map_err(RedisClientError::ConnectionError)?;

        Ok(Self { client, config })
    }

    /// 建立異步連接
    pub async fn get_async_connection(
        &self,
    ) -> Result<redis::aio::MultiplexedConnection, RedisClientError> {
        let connect_timeout = Duration::from_secs(self.config.connection_timeout_secs);

        let connection_future = self.client.get_multiplexed_async_connection();
        match timeout(connect_timeout, connection_future).await {
            Ok(result) => match result {
                Ok(conn) => {
                    debug!("成功建立Redis連接");
                    Ok(conn)
                }
                Err(err) => {
                    error!("無法建立Redis連接: {}", err);
                    Err(RedisClientError::ConnectionError(err))
                }
            },
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
        F: FnOnce(redis::aio::MultiplexedConnection) -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<
                Output = Result<(redis::aio::MultiplexedConnection, T), RedisClientError>,
            > + Send,
        T: Send,
    {
        let mut attempts = 0;
        let max_attempts = self.config.reconnect_attempts as usize + 1; // +1 for initial attempt
        let retry_delay = Duration::from_secs(self.config.reconnect_delay_secs);

        loop {
            attempts += 1;
            let conn = self.get_async_connection().await?;

            match operation.clone()(conn).await {
                Ok((new_conn, result)) => {
                    let _conn = new_conn; // 使用了新的連接，但不需要再保存
                    return Ok(result);
                }
                Err(err) => {
                    if attempts >= max_attempts {
                        return Err(err);
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
    async fn set<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
        value: String,
        expiry_secs: Option<u64>,
    ) -> Result<(), RedisClientError> {
        let key_clone = key.clone();
        let value_clone = value.clone();

        self.with_retry_connection(move |mut conn| {
            let key = key_clone.clone();
            let value = value_clone.clone();
            let expiry = expiry_secs;

            async move {
                debug!("Setting Redis key: {}", key.as_ref());

                // 處理可能的過期時間
                let result = if let Some(seconds) = expiry {
                    // 設置帶過期時間的鍵
                    let mut cmd = redis::cmd("SET");
                    cmd.arg(key.as_ref()).arg(value).arg("EX").arg(seconds);

                    let result: Result<(), RedisError> = cmd.query_async(&mut conn).await;
                    result
                } else {
                    // 設置不帶過期時間的鍵
                    match conn.set::<_, _, ()>(key.as_ref(), value).await {
                        Ok(_) => Ok(()),
                        Err(err) => Err(err),
                    }
                };

                match result {
                    Ok(_) => Ok((conn, ())),
                    Err(err) => {
                        error!("Failed to set Redis key: {}", err);
                        Err(RedisClientError::ConnectionError(err))
                    }
                }
            }
        })
        .await
    }

    async fn get<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
    ) -> Result<Option<String>, RedisClientError> {
        let key_clone = key.clone();

        self.with_retry_connection(move |mut conn| {
            let key = key_clone.clone();

            async move {
                debug!("Getting Redis key: {}", key.as_ref());

                let fut = conn.get::<_, Option<String>>(key.as_ref());
                match fut.await {
                    Ok(value) => Ok((conn, value)),
                    Err(err) => {
                        error!("Failed to get Redis key: {}", err);
                        Err(RedisClientError::ConnectionError(err))
                    }
                }
            }
        })
        .await
    }

    async fn delete<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
    ) -> Result<bool, RedisClientError> {
        let key_clone = key.clone();

        self.with_retry_connection(move |mut conn| {
            let key = key_clone.clone();

            async move {
                debug!("Deleting Redis key: {}", key.as_ref());

                let fut = conn.del::<_, bool>(key.as_ref());
                match fut.await {
                    Ok(value) => Ok((conn, value)),
                    Err(err) => {
                        error!("Failed to delete Redis key: {}", err);
                        Err(RedisClientError::ConnectionError(err))
                    }
                }
            }
        })
        .await
    }

    async fn exists<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
    ) -> Result<bool, RedisClientError> {
        let key_clone = key.clone();

        self.with_retry_connection(move |mut conn| {
            let key = key_clone.clone();

            async move {
                debug!("Checking if Redis key exists: {}", key.as_ref());

                let fut = conn.exists::<_, bool>(key.as_ref());
                match fut.await {
                    Ok(exists) => Ok((conn, exists)),
                    Err(err) => {
                        error!("Failed to check if Redis key exists: {}", err);
                        Err(RedisClientError::ConnectionError(err))
                    }
                }
            }
        })
        .await
    }

    async fn expire<K: AsRef<str> + Send + Sync + Clone>(
        &self,
        key: K,
        seconds: u64,
    ) -> Result<bool, RedisClientError> {
        let key_clone = key.clone();

        self.with_retry_connection(move |mut conn| {
            let key = key_clone.clone();

            async move {
                debug!(
                    "Setting expiry for Redis key: {}, seconds: {}",
                    key.as_ref(),
                    seconds
                );

                let fut = conn.expire::<_, bool>(key.as_ref(), seconds as i64);
                match fut.await {
                    Ok(success) => Ok((conn, success)),
                    Err(err) => {
                        error!("Failed to set expiry for Redis key: {}", err);
                        Err(RedisClientError::ConnectionError(err))
                    }
                }
            }
        })
        .await
    }

    async fn ping(&self) -> Result<String, RedisClientError> {
        self.with_retry_connection(|mut conn| async move {
            debug!("Pinging Redis server");

            // 使用 cmd 直接執行 PING 指令
            let result: Result<String, RedisError> =
                redis::cmd("PING").query_async(&mut conn).await;
            match result {
                Ok(pong) => Ok((conn, pong)),
                Err(err) => {
                    error!("Failed to ping Redis server: {}", err);
                    Err(RedisClientError::ConnectionError(err))
                }
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redis::test_config::RedisTestConfig;

    #[tokio::test]
    async fn test_redis_operations() {
        if RedisTestConfig::skip_if_redis_unavailable("test_redis_operations")
            .await
            .is_none()
        {
            return;
        }

        let config = RedisTestConfig::create_test_config();
        let client = Client::new(config).expect("無法創建Redis客戶端");

        // 測試SET和GET
        client
            .set("test_key", "test_value".to_string(), None)
            .await
            .expect("SET失敗");
        let value = client.get("test_key").await.expect("GET失敗");
        assert_eq!(value, Some("test_value".to_string()));

        // 測試帶過期時間的SET
        client
            .set("test_expire_key", "test_expire_value".to_string(), Some(1))
            .await
            .expect("SET EX失敗");
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
