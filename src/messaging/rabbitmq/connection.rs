use crate::config::RabbitMQConfig;
use deadpool_lapin::{BuildError, Manager, Object, Pool, PoolError};
use lapin::{ConnectionProperties, Error as LapinError};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// RabbitMQ 連接錯誤類型
#[derive(Error, Debug)]
pub enum RabbitMQConnectionError {
    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),

    #[error("Pool creation error: {0}")]
    BuildError(#[from] BuildError),

    #[error("Lapin error: {0}")]
    Lapin(#[from] LapinError),

    #[error("Connection timeout")]
    Timeout,
}

/// RabbitMQ 連接池類型別名
pub type RabbitMQPool = Pool;
/// RabbitMQ 連接物件類型別名
pub type RabbitMQConnection = Object;

/// RabbitMQ 連接配置
#[derive(Clone, Debug)]
pub struct RabbitMQConnectionConfig {
    /// AMQP URL (例如: "amqp://user:pass@localhost:5672/")
    pub url: String,
    /// 連接池大小
    pub pool_size: usize,
    /// 連接超時（毫秒）
    pub connection_timeout: u64,
    /// 重試間隔（毫秒）
    pub retry_interval: u64,
    /// 最大重試次數
    pub max_retries: usize,
}

impl Default for RabbitMQConnectionConfig {
    fn default() -> Self {
        Self {
            url: "amqp://guest:guest@localhost:5672/".to_string(),
            pool_size: 10,
            connection_timeout: 5000, // 5 秒
            retry_interval: 1000,     // 1 秒
            max_retries: 5,
        }
    }
}

/// RabbitMQ 連接管理器
#[derive(Clone)]
pub struct RabbitMQConnectionManager {
    pool: RabbitMQPool,
    config: RabbitMQConnectionConfig,
}

impl RabbitMQConnectionManager {
    /// 創建新的連接管理器
    pub async fn new(config: RabbitMQConnectionConfig) -> Result<Self, RabbitMQConnectionError> {
        info!("Creating RabbitMQ connection pool to {}", config.url);

        let connection_properties = ConnectionProperties::default()
            .with_connection_name("backtest_server".into())
            .with_executor(tokio_executor_trait::Tokio::current());

        let manager = Manager::new(config.url.clone(), connection_properties);
        let pool = Pool::builder(manager)
            .max_size(config.pool_size)
            .build()
            .map_err(RabbitMQConnectionError::BuildError)?;

        // 嘗試初始連接以確保配置正確
        let mgr = Self {
            pool,
            config: config.clone(),
        };
        mgr.ensure_connected().await?;

        info!(
            "Successfully created RabbitMQ connection pool with {} connections",
            config.pool_size
        );

        Ok(mgr)
    }

    /// 獲取連接池
    pub fn pool(&self) -> RabbitMQPool {
        self.pool.clone()
    }

    /// 獲取連接（帶重試）
    pub async fn get_connection(&self) -> Result<RabbitMQConnection, RabbitMQConnectionError> {
        let mut retries = 0;
        let max_retries = self.config.max_retries;

        loop {
            match self.pool.get().await {
                Ok(conn) => return Ok(conn),
                Err(err) => {
                    retries += 1;
                    if retries > max_retries {
                        error!(
                            "Failed to get RabbitMQ connection after {} retries: {}",
                            max_retries, err
                        );
                        return Err(RabbitMQConnectionError::Pool(err));
                    }

                    let wait_time = Duration::from_millis(self.config.retry_interval);
                    warn!("Failed to get RabbitMQ connection (attempt {}/{}): {}. Retrying in {:?}...", 
                          retries, max_retries, err, wait_time);

                    sleep(wait_time).await;
                }
            }
        }
    }

    /// 確保連接可用
    pub async fn ensure_connected(&self) -> Result<(), RabbitMQConnectionError> {
        let timeout_duration = Duration::from_millis(self.config.connection_timeout);

        let connection_result = tokio::time::timeout(timeout_duration, self.pool.get()).await;

        match connection_result {
            Ok(Ok(_conn)) => {
                debug!("RabbitMQ connection check successful");
                Ok(())
            }
            Ok(Err(err)) => {
                error!("RabbitMQ connection failed: {}", err);
                Err(RabbitMQConnectionError::Pool(err))
            }
            Err(_) => {
                error!("RabbitMQ connection timed out after {:?}", timeout_duration);
                Err(RabbitMQConnectionError::Timeout)
            }
        }
    }

    /// 檢查連接健康狀態
    pub async fn check_health(&self) -> Result<(), RabbitMQConnectionError> {
        self.ensure_connected().await
    }

    /// 從應用配置創建連接管理器
    pub async fn from_config(config: &RabbitMQConfig) -> Result<Self, RabbitMQConnectionError> {
        let connection_config = RabbitMQConnectionConfig {
            url: config.url.clone(),
            pool_size: config.pool_size as usize,
            connection_timeout: config.connection_timeout_secs * 1000, // 轉換為毫秒
            retry_interval: config.retry_interval_secs * 1000,         // 轉換為毫秒
            max_retries: config.max_retries as usize,
        };

        Self::new(connection_config).await
    }

    /// 獲取配置
    pub fn config(&self) -> &RabbitMQConnectionConfig {
        &self.config
    }
}
