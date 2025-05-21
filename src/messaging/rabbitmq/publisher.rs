use crate::messaging::protocol::Message;
use crate::messaging::rabbitmq::connection::RabbitMQConnectionManager;
use crate::messaging::rabbitmq::error::RabbitMQError;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    BasicProperties, ExchangeKind,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// 發布者配置
#[derive(Clone, Debug)]
pub struct PublisherConfig {
    pub exchange_name: String,
    pub exchange_type: ExchangeKind,
    pub exchange_durable: bool,
    pub persistent: bool,
    pub mandatory: bool,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            exchange_name: String::new(),
            exchange_type: ExchangeKind::Direct,
            exchange_durable: true,
            persistent: true,
            mandatory: false,
        }
    }
}

/// 消息發布者
pub struct RabbitMQPublisher {
    connection_manager: RabbitMQConnectionManager,
    config: PublisherConfig,
    initialized: Arc<Mutex<bool>>,
}

impl RabbitMQPublisher {
    /// 創建新的消息發布者
    pub fn new(connection_manager: RabbitMQConnectionManager, config: PublisherConfig) -> Self {
        Self {
            connection_manager,
            config,
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// 初始化發布者
    pub async fn initialize(&self) -> Result<(), RabbitMQError> {
        let mut initialized = self.initialized.lock().await;

        if *initialized {
            debug!("Publisher already initialized");
            return Ok(());
        }

        // 如果需要宣告交換機
        if !self.config.exchange_name.is_empty() {
            debug!(
                "Initializing publisher for exchange: {}",
                self.config.exchange_name
            );

            let conn = self.connection_manager.get_connection().await?;
            let channel = conn.create_channel().await?;

            debug!("Declaring exchange: {}", self.config.exchange_name);

            channel
                .exchange_declare(
                    &self.config.exchange_name,
                    self.config.exchange_type.clone(),
                    ExchangeDeclareOptions {
                        durable: self.config.exchange_durable,
                        ..ExchangeDeclareOptions::default()
                    },
                    lapin::types::FieldTable::default(),
                )
                .await?;

            info!(
                "Publisher initialized for exchange: {}",
                self.config.exchange_name
            );
        } else {
            info!("Publisher initialized for default exchange");
        }

        *initialized = true;

        Ok(())
    }

    /// 發布消息
    pub async fn publish<T: Serialize>(
        &self,
        routing_key: &str,
        message: &Message<T>,
    ) -> Result<(), RabbitMQError> {
        // 確保已初始化
        let initialized = *self.initialized.lock().await;

        if !initialized {
            debug!("Publisher not initialized, initializing now");
            self.initialize().await?;
        }

        let conn = self.connection_manager.get_connection().await?;
        let channel = conn.create_channel().await?;

        let exchange_name = if self.config.exchange_name.is_empty() {
            "" // 使用預設交換機
        } else {
            &self.config.exchange_name
        };

        // 序列化消息
        let payload = serde_json::to_vec(message)?;

        // 準備消息屬性
        let mut properties =
            BasicProperties::default().with_content_type("application/json".into());

        // 如果需要持久化
        if self.config.persistent {
            properties = properties.with_delivery_mode(2); // 持久化模式
        }

        // 設置消息ID
        properties = properties.with_message_id(message.message_id.clone().into());

        // 如果有相關ID，設置它
        if let Some(correlation_id) = &message.correlation_id {
            properties = properties.with_correlation_id(correlation_id.clone().into());
        }

        debug!(
            "Publishing message to exchange: {}, routing_key: {}, id: {}",
            exchange_name, routing_key, message.message_id
        );

        // 發布消息
        channel
            .basic_publish(
                exchange_name,
                routing_key,
                BasicPublishOptions {
                    mandatory: self.config.mandatory,
                    ..BasicPublishOptions::default()
                },
                &payload,
                properties,
            )
            .await?;

        debug!("Message published successfully: {}", message.message_id);

        Ok(())
    }

    /// 發布原始消息
    pub async fn publish_raw(
        &self,
        routing_key: &str,
        payload: &[u8],
        properties: BasicProperties,
    ) -> Result<(), RabbitMQError> {
        // 確保已初始化
        let initialized = *self.initialized.lock().await;

        if !initialized {
            debug!("Publisher not initialized, initializing now");
            self.initialize().await?;
        }

        let conn = self.connection_manager.get_connection().await?;
        let channel = conn.create_channel().await?;

        let exchange_name = if self.config.exchange_name.is_empty() {
            "" // 使用預設交換機
        } else {
            &self.config.exchange_name
        };

        debug!(
            "Publishing raw message to exchange: {}, routing_key: {}",
            exchange_name, routing_key
        );

        // 發布消息
        channel
            .basic_publish(
                exchange_name,
                routing_key,
                BasicPublishOptions {
                    mandatory: self.config.mandatory,
                    ..BasicPublishOptions::default()
                },
                payload,
                properties,
            )
            .await?;

        debug!("Raw message published successfully");

        Ok(())
    }

    /// 檢查發布者健康狀態
    pub async fn check_health(&self) -> Result<(), RabbitMQError> {
        let conn = self.connection_manager.get_connection().await?;
        let _ = conn.create_channel().await?;

        Ok(())
    }
}
