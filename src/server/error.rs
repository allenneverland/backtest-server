use thiserror::Error;
use crate::messaging::rabbitmq::connection::RabbitMQConnectionError;
use crate::messaging::rabbitmq::broker::BrokerError;

/// 伺服器錯誤類型
#[derive(Error, Debug)]
pub enum ServerError {
    /// 配置錯誤
    #[error("配置錯誤: {0}")]
    Config(String),
    
    /// RabbitMQ 連接錯誤
    #[error("RabbitMQ 連接錯誤: {0}")]
    RabbitMQConnection(#[from] RabbitMQConnectionError),
    
    /// 消息代理錯誤
    #[error("消息代理錯誤: {0}")]
    Broker(#[from] BrokerError),
    
    /// IO 錯誤
    #[error("IO 錯誤: {0}")]
    Io(#[from] std::io::Error),
    
    /// 初始化錯誤
    #[error("初始化錯誤: {0}")]
    Initialization(String),
    
    /// 運行時錯誤
    #[error("運行時錯誤: {0}")]
    Runtime(String),
    
    /// 關閉錯誤
    #[error("關閉錯誤: {0}")]
    Shutdown(String),
}

/// 伺服器結果類型別名
pub type ServerResult<T> = Result<T, ServerError>; 