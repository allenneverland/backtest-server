// RabbitMQ 模組
// 提供與 RabbitMQ 通訊的基礎設施

// 導出子模組
pub mod connection;
pub mod broker;
pub mod client;
pub mod consumer;
pub mod publisher;
pub mod rpc;
pub mod error;

// 重新導出常用結構
pub use connection::{RabbitMQConnectionManager, RabbitMQConnectionConfig};
pub use broker::{RabbitMQBroker, MessageHandler};
pub use client::{RabbitMQClient};
pub use consumer::{RabbitMQConsumer, MessageConsumerHandler, ConsumerConfig};
pub use publisher::{RabbitMQPublisher, PublisherConfig};
pub use rpc::{RpcClient, RpcServer, RpcHandler, RpcClientConfig, RpcServerConfig};
pub use error::RabbitMQError; 