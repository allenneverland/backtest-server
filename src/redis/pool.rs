use super::client::RedisClientError;
use crate::config::types::RedisConfig;
use async_trait::async_trait;
use deadpool::managed::QueueMode;
use deadpool_redis::{
    redis::{cmd, RedisError},
    Config, Connection, CreatePoolError, Pool, PoolConfig, PoolError, Runtime, Timeouts,
};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info};

/// Redis連接池錯誤
#[derive(Error, Debug)]
pub enum RedisPoolError {
    /// 連接池初始化錯誤
    #[error("Redis連接池初始化錯誤: {0}")]
    PoolInitError(String),

    /// 無法獲取連接
    #[error("無法從連接池獲取連接: {0}")]
    GetConnectionError(String),

    /// Redis操作錯誤
    #[error("Redis操作錯誤: {0}")]
    RedisError(#[from] RedisClientError),

    /// Redis原生錯誤
    #[error("Redis原生錯誤: {0}")]
    NativeRedisError(#[from] RedisError),

    /// 其他錯誤
    #[error("Redis連接池其他錯誤: {0}")]
    Other(String),
}

/// 從deadpool-redis錯誤轉換為RedisPoolError
impl From<PoolError> for RedisPoolError {
    fn from(error: PoolError) -> Self {
        RedisPoolError::GetConnectionError(error.to_string())
    }
}

/// 從deadpool-redis創建錯誤轉換為RedisPoolError
impl From<CreatePoolError> for RedisPoolError {
    fn from(error: CreatePoolError) -> Self {
        RedisPoolError::PoolInitError(error.to_string())
    }
}

/// Redis連接池接口
#[async_trait]
pub trait RedisPool: Send + Sync + 'static {
    /// 獲取連接
    async fn get_conn(&self) -> Result<Connection, RedisPoolError>;

    /// 檢查連接池健康狀態
    async fn check_health(&self) -> bool;

    /// 獲取連接池大小
    fn pool_size(&self) -> u32;
}

/// Redis連接池實現
pub struct ConnectionPool {
    pool: Pool,
    config: RedisConfig,
}

impl ConnectionPool {
    /// 創建新的Redis連接池
    pub async fn new(config: RedisConfig) -> Result<Self, RedisPoolError> {
        // 創建連接池配置
        let mut cfg = Config::from_url(&config.url);

        // 設置連接池大小和超時
        cfg.pool = Some(PoolConfig {
            max_size: config.pool_size as usize,
            timeouts: Timeouts {
                wait: Some(Duration::from_secs(config.connection_timeout_secs)),
                create: Some(Duration::from_secs(config.connection_timeout_secs)),
                recycle: Some(Duration::from_secs(60)),
            },
            queue_mode: QueueMode::Fifo,
        });

        // 創建連接池 - 使用 deadpool_redis 的 create_pool 方法
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        info!("Redis連接池初始化完成，大小: {}", config.pool_size);

        Ok(Self { pool, config })
    }
}

#[async_trait]
impl RedisPool for ConnectionPool {
    async fn get_conn(&self) -> Result<Connection, RedisPoolError> {
        match self.pool.get().await {
            Ok(conn) => {
                debug!("從Redis連接池獲取連接成功");
                Ok(conn)
            }
            Err(e) => {
                error!("無法從Redis連接池獲取連接: {}", e);
                Err(RedisPoolError::GetConnectionError(e.to_string()))
            }
        }
    }

    async fn check_health(&self) -> bool {
        match self.pool.get().await {
            Ok(mut conn) => {
                // 直接執行 PING 命令
                let result: Result<String, RedisError> = cmd("PING").query_async(&mut conn).await;
                match result {
                    Ok(pong) => pong == "PONG",
                    Err(e) => {
                        error!("Redis健康檢查錯誤: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Redis健康檢查無法獲取連接: {}", e);
                false
            }
        }
    }

    fn pool_size(&self) -> u32 {
        self.config.pool_size
    }
}

/// Arc<ConnectionPool> 也實現 RedisPool trait，便於共享連接池
#[async_trait]
impl RedisPool for Arc<ConnectionPool> {
    async fn get_conn(&self) -> Result<Connection, RedisPoolError> {
        (**self).get_conn().await
    }

    async fn check_health(&self) -> bool {
        (**self).check_health().await
    }

    fn pool_size(&self) -> u32 {
        (**self).pool_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redis::test_config::RedisTestConfig;
    use deadpool_redis::redis::AsyncCommands;

    #[tokio::test]
    async fn test_connection_pool() {
        RedisTestConfig::ensure_redis_available("test_connection_pool").await;

        // 創建連接池
        let config = RedisTestConfig::create_test_config();
        let pool = ConnectionPool::new(config)
            .await
            .expect("無法創建Redis連接池");

        // 測試連接池健康狀態
        assert!(pool.check_health().await);

        // 測試獲取連接和基本操作
        let mut conn = pool.get_conn().await.expect("無法獲取連接");

        // 測試SET和GET
        let key = "pool_test_key";
        let value = "pool_test_value";

        let _: () = conn.set(key, value).await.expect("SET失敗");
        let result: String = conn.get(key).await.expect("GET失敗");

        assert_eq!(result, value);

        // 清理
        let _: bool = conn.del(key).await.expect("DEL失敗");
    }
}
