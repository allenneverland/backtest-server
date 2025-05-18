// 消息系統模組
// 提供基於 RabbitMQ 的消息中間件機制，允許外部系統和用戶與回測伺服器交互

// 導出子模組
pub mod rabbitmq;
pub mod protocol;
pub mod models;
pub mod handlers;

// 重新導出常用類型
pub use protocol::Message;
pub use protocol::ErrorResponse;
pub use rabbitmq::connection::RabbitMQConnectionManager;
pub use rabbitmq::broker::RabbitMQBroker;
pub use rabbitmq::client::RabbitMQClient;
pub use rabbitmq::error::RabbitMQError; 