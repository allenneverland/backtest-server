use deadpool_lapin::{Manager, Pool, PoolError};
use lapin::{ConnectionProperties, Error as LapinError};
use std::sync::Arc;
use tracing::{error, info};

pub type RabbitMQPool = Pool;

#[derive(Clone)]
pub struct RabbitMQConnection {
    pool: RabbitMQPool,
}

impl RabbitMQConnection {
    /// 創建新的連接池
    pub async fn new(amqp_url: &str, pool_size: usize) -> Result<Self, PoolError> {
        info!("Creating RabbitMQ connection pool to {}", amqp_url);
        
        let manager = Manager::new(amqp_url.to_string(), ConnectionProperties::default());
        let pool = Pool::builder(manager)
            .max_size(pool_size)
            .build()?;
        
        // 測試連接
        let _ = pool.get().await?;
        info!("Successfully connected to RabbitMQ");
        
        Ok(Self { pool })
    }
    
    /// 獲取連接池
    pub fn pool(&self) -> RabbitMQPool {
        self.pool.clone()
    }
    
    /// 檢查連接狀態
    pub async fn check_health(&self) -> Result<(), PoolError> {
        let _ = self.pool.get().await?;
        Ok(())
    }
}