use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Consumer,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::messaging::protocol::Message;
use crate::messaging::rabbitmq::connection::{RabbitMQConnectionError, RabbitMQConnectionManager};
use thiserror::Error;

type LapinError = lapin::Error;

/// RabbitMQ 客戶端錯誤
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Connection error: {0}")]
    Connection(#[from] RabbitMQConnectionError),

    #[error("Lapin error: {0}")]
    Lapin(#[from] LapinError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Response timeout")]
    Timeout,

    #[error("Response channel closed")]
    ChannelClosed,

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

// Add implementation for oneshot channel error conversion
impl From<oneshot::error::RecvError> for ClientError {
    fn from(_: oneshot::error::RecvError) -> Self {
        ClientError::ChannelClosed
    }
}

/// RabbitMQ 客戶端
pub struct RabbitMQClient {
    connection_manager: RabbitMQConnectionManager,
    channel: Arc<Mutex<Option<Channel>>>,
    reply_queue: Arc<Mutex<Option<String>>>,
    pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Vec<u8>>>>>,
}

impl RabbitMQClient {
    /// 創建新的客戶端
    pub fn new(connection_manager: RabbitMQConnectionManager) -> Self {
        Self {
            connection_manager,
            channel: Arc::new(Mutex::new(None)),
            reply_queue: Arc::new(Mutex::new(None)),
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 初始化客戶端
    pub async fn initialize(&self) -> Result<(), ClientError> {
        info!("Initializing RabbitMQ client");

        // 獲取連接並創建通道
        let conn = self.connection_manager.get_connection().await?;
        let channel = conn.create_channel().await?;

        // 創建臨時回應佇列
        let queue = channel
            .queue_declare(
                "", // 空名稱 = 生成臨時佇列
                QueueDeclareOptions {
                    exclusive: true,
                    auto_delete: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await?;

        let reply_queue = queue.name().to_string();
        debug!("Created reply queue: {}", reply_queue);

        // 保存通道和回應佇列
        {
            let mut channel_guard = self.channel.lock().await;
            *channel_guard = Some(channel.clone());

            let mut reply_queue_guard = self.reply_queue.lock().await;
            *reply_queue_guard = Some(reply_queue.clone());
        }

        // 開始監聽回應
        self.start_response_consumer(channel, reply_queue).await?;

        info!("RabbitMQ client initialized");

        Ok(())
    }

    /// 確保客戶端已初始化並取得通道
    async fn ensure_channel(&self) -> Result<Channel, ClientError> {
        let channel_option = {
            let channel_guard = self.channel.lock().await;
            channel_guard.clone()
        };

        match channel_option {
            Some(channel) => Ok(channel),
            None => {
                // 通道不存在，嘗試初始化
                debug!("Channel not initialized, initializing client");
                self.initialize().await?;

                let channel_guard = self.channel.lock().await;
                match channel_guard.clone() {
                    Some(channel) => Ok(channel),
                    None => {
                        error!("Failed to initialize channel");
                        Err(ClientError::Connection(RabbitMQConnectionError::Timeout))
                    }
                }
            }
        }
    }

    /// 獲取回應佇列名稱
    async fn get_reply_queue(&self) -> Result<String, ClientError> {
        let reply_queue_option = {
            let reply_queue_guard = self.reply_queue.lock().await;
            reply_queue_guard.clone()
        };

        match reply_queue_option {
            Some(queue) => Ok(queue),
            None => {
                // 回應佇列未初始化，嘗試初始化
                debug!("Reply queue not initialized, initializing client");
                self.initialize().await?;

                let reply_queue_guard = self.reply_queue.lock().await;
                match reply_queue_guard.clone() {
                    Some(queue) => Ok(queue),
                    None => {
                        error!("Failed to initialize reply queue");
                        Err(ClientError::Connection(RabbitMQConnectionError::Timeout))
                    }
                }
            }
        }
    }

    /// 開始監聽回應佇列
    async fn start_response_consumer(
        &self,
        channel: Channel,
        queue_name: String,
    ) -> Result<(), ClientError> {
        let pending_responses = self.pending_responses.clone();

        let consumer = channel
            .basic_consume(
                &queue_name,
                "response_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tokio::spawn(async move {
            debug!("Response consumer started for queue: {}", queue_name);

            Self::handle_responses(consumer, pending_responses).await;
        });

        Ok(())
    }

    /// 處理來自回應佇列的消息
    async fn handle_responses(
        mut consumer: Consumer,
        pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Vec<u8>>>>>,
    ) {
        while let Some(delivery) = consumer.next().await {
            if let Ok(delivery) = delivery {
                if let Some(correlation_id) = delivery.properties.correlation_id() {
                    let correlation_id = correlation_id.to_string();
                    debug!("Received response with correlation_id: {}", correlation_id);

                    let mut pending = pending_responses.lock().await;
                    if let Some(sender) = pending.remove(&correlation_id) {
                        debug!(
                            "Found pending request for correlation_id: {}",
                            correlation_id
                        );
                        let _ = sender.send(delivery.data.clone());
                    } else {
                        debug!(
                            "No pending request found for correlation_id: {}",
                            correlation_id
                        );
                    }
                } else {
                    debug!("Received response without correlation_id");
                }

                if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                    error!("Failed to acknowledge response: {}", e);
                }
            }
        }

        debug!("Response consumer stopped");
    }

    /// 發送請求並等待回應 (RPC模式)
    pub async fn request<T, R>(
        &self,
        exchange: &str,
        routing_key: &str,
        request: &T,
        timeout: Duration,
    ) -> Result<R, ClientError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let correlation_id = Uuid::new_v4().to_string();
        debug!(
            "Sending RPC request with correlation_id: {}",
            correlation_id
        );

        // 獲取通道和回應佇列
        let channel = self.ensure_channel().await?;
        let reply_queue = self.get_reply_queue().await?;

        // 創建消息封裝
        let message = Message::new(routing_key, request);
        let message_data = serde_json::to_vec(&message)?;

        // 創建用於等待回應的通道
        let (sender, receiver) = oneshot::channel();

        // 註冊等待回應
        {
            let mut pending = self.pending_responses.lock().await;
            pending.insert(correlation_id.clone(), sender);
        }

        // 發送請求
        channel
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                &message_data,
                BasicProperties::default()
                    .with_reply_to(reply_queue.into())
                    .with_correlation_id(correlation_id.clone().into()),
            )
            .await?;

        debug!("Request sent, waiting for response...");

        // 等待回應（帶超時）
        let response_data = tokio::time::timeout(timeout, receiver)
            .await
            .map_err(|_| ClientError::Timeout)??;

        debug!("Response received, deserializing...");

        // 解析回應
        let response: Message<R> = serde_json::from_slice(&response_data)?;

        Ok(response.payload)
    }

    /// 發布事件（無需回應）
    pub async fn publish<T>(
        &self,
        exchange: &str,
        routing_key: &str,
        event: &T,
    ) -> Result<(), ClientError>
    where
        T: Serialize,
    {
        debug!(
            "Publishing event to exchange: {}, routing_key: {}",
            exchange, routing_key
        );

        // 獲取通道
        let channel = self.ensure_channel().await?;

        // 創建消息封裝
        let message = Message::new(routing_key, event);
        let message_data = serde_json::to_vec(&message)?;

        // 發布事件
        channel
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                &message_data,
                BasicProperties::default(),
            )
            .await?;

        debug!("Event published");

        Ok(())
    }

    /// 檢查客戶端健康狀態
    pub async fn check_health(&self) -> Result<(), ClientError> {
        // 使用固定超時時間
        let timeout_duration = Duration::from_secs(5);
        let conn =
            match tokio::time::timeout(timeout_duration, self.connection_manager.get_connection())
                .await
            {
                Ok(Ok(conn)) => conn,
                Ok(Err(err)) => return Err(ClientError::Connection(err)),
                Err(_) => {
                    // 改為使用自定義超時錯誤
                    return Err(ClientError::Connection(RabbitMQConnectionError::Timeout));
                }
            };

        debug!("Connection health check passed");

        // 檢查通道
        let _channel = match conn.create_channel().await {
            Ok(channel) => channel,
            Err(err) => return Err(ClientError::Lapin(err)),
        };

        debug!("Channel health check passed");

        Ok(())
    }
}
