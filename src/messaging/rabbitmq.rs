// RabbitMQ 模組
// 提供與 RabbitMQ 通訊的基礎設施

// 導出子模組
pub mod broker;
pub mod client;
pub mod connection;
pub mod consumer;
pub mod error;
pub mod publisher;
pub mod rpc;

// 重新導出常用結構
pub use broker::{MessageHandler, RabbitMQBroker};
pub use client::RabbitMQClient;
pub use connection::{RabbitMQConnectionConfig, RabbitMQConnectionManager};
pub use consumer::{ConsumerConfig, MessageConsumerHandler, RabbitMQConsumer};
pub use error::RabbitMQError;
pub use publisher::{PublisherConfig, RabbitMQPublisher};
pub use rpc::{RpcClient, RpcClientConfig, RpcHandler, RpcServer, RpcServerConfig};
