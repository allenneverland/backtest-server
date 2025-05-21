use crate::messaging::rabbitmq::connection::RabbitMQConnectionError;
use lapin::Error as LapinError;
use serde_json::Error as SerdeError;
use thiserror::Error;

/// RabbitMQ 系統通用錯誤類型
#[derive(Error, Debug)]
pub enum RabbitMQError {
    #[error("Connection error: {0}")]
    Connection(#[from] RabbitMQConnectionError),

    #[error("Lapin error: {0}")]
    Lapin(#[from] LapinError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerdeError),

    #[error("Response timeout")]
    Timeout,

    #[error("No handler registered for routing key: {0}")]
    NoHandler(String),

    #[error("Invalid message format")]
    InvalidMessage,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Queue not found: {0}")]
    QueueNotFound(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// 將標準錯誤轉換為 RabbitMQ 錯誤
impl From<Box<dyn std::error::Error + Send + Sync>> for RabbitMQError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        RabbitMQError::Other(error.to_string())
    }
}

/// 將字符串錯誤轉換為 RabbitMQ 錯誤
impl From<String> for RabbitMQError {
    fn from(error: String) -> Self {
        RabbitMQError::Other(error)
    }
}

/// 將 &str 錯誤轉換為 RabbitMQ 錯誤
impl From<&str> for RabbitMQError {
    fn from(error: &str) -> Self {
        RabbitMQError::Other(error.to_string())
    }
}
