use crate::messaging::protocol::Message;
use lapin::{
    options::{
        BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions
    },
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, Consumer, Error as LapinError
};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use std::collections::HashMap;
use serde::{Serialize, de::DeserializeOwned};
use tracing::{debug, error, info};
use uuid::Uuid;
use std::time::Duration;

/// RabbitMQ 客戶端
pub struct RabbitMQClient {
    connection: Arc<Connection>,
    channel: Arc<Channel>,
    reply_queue: String,
    pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Vec<u8>>>>>,
}

impl RabbitMQClient {
    /// 創建新的客戶端
    pub async fn new(amqp_url: &str) -> Result<Self, LapinError> {
        info!("Connecting to RabbitMQ: {}", amqp_url);
        
        let connection = Connection::connect(
            amqp_url,
            ConnectionProperties::default(),
        ).await?;
        
        let channel = connection.create_channel().await?;
        
        // 創建臨時回應佇列
        let queue = channel.queue_declare(
            "",  // 空名稱 = 生成臨時佇列
            QueueDeclareOptions {
                exclusive: true,
                auto_delete: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        ).await?;
        
        let reply_queue = queue.name().to_string();
        debug!("Created reply queue: {}", reply_queue);
        
        let client = Self {
            connection: Arc::new(connection),
            channel: Arc::new(channel),
            reply_queue,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
        };
        
        // 開始監聽回應
        client.start_response_consumer().await?;
        
        info!("RabbitMQ client initialized");
        
        Ok(client)
    }
    
    /// 開始監聽回應佇列
    async fn start_response_consumer(&self) -> Result<(), LapinError> {
        let channel = self.channel.clone();
        let queue_name = self.reply_queue.clone();
        let pending_responses = self.pending_responses.clone();
        
        let consumer = channel.basic_consume(
            &queue_name,
            "response_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        ).await?;
        
        tokio::spawn(async move {
            debug!("Response consumer started for queue: {}", queue_name);
            
            Self::handle_responses(consumer, pending_responses).await;
        });
        
        Ok(())
    }
    
    /// 處理來自回應佇列的消息
    async fn handle_responses(
        mut consumer: Consumer,
        pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Vec<u8>>>>
    ) {
        while let Some(delivery) = consumer.next().await {
            if let Ok(delivery) = delivery {
                if let Some(correlation_id) = delivery.properties.correlation_id() {
                    let correlation_id = correlation_id.to_string();
                    debug!("Received response with correlation_id: {}", correlation_id);
                    
                    let mut pending = pending_responses.lock().await;
                    if let Some(sender) = pending.remove(&correlation_id) {
                        debug!("Found pending request for correlation_id: {}", correlation_id);
                        let _ = sender.send(delivery.data.clone());
                    } else {
                        debug!("No pending request found for correlation_id: {}", correlation_id);
                    }
                } else {
                    debug!("Received response without correlation_id");
                }
                
                if let Err(e) = delivery.ack(BasicConsumeOptions::default()).await {
                    error!("Failed to acknowledge response: {}", e);
                }
            }
        }
        
        debug!("Response consumer stopped");
    }
    
    /// 發送請求並等待回應 (RPC模式)
    pub async fn request<T, R>(&self, exchange: &str, routing_key: &str, request: &T, timeout: Duration) -> Result<R, anyhow::Error> 
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let correlation_id = Uuid::new_v4().to_string();
        debug!("Sending RPC request with correlation_id: {}", correlation_id);
        
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
        self.channel.basic_publish(
            exchange,
            routing_key,
            BasicPublishOptions::default(),
            &message_data,
            BasicProperties::default()
                .with_reply_to(self.reply_queue.clone())
                .with_correlation_id(correlation_id.clone().into()),
        ).await?;
        
        debug!("Request sent, waiting for response...");
        
        // 等待回應（帶超時）
        let response_data = tokio::time::timeout(
            timeout,
            receiver
        ).await??;
        
        debug!("Response received, deserializing...");
        
        // 解析回應
        let response: Message<R> = serde_json::from_slice(&response_data)?;
        
        Ok(response.payload)
    }
    
    /// 發布事件（無需回應）
    pub async fn publish<T>(&self, exchange: &str, routing_key: &str, event: &T) -> Result<(), LapinError> 
    where
        T: Serialize,
    {
        debug!("Publishing event to exchange: {}, routing_key: {}", exchange, routing_key);
        
        // 創建消息封裝
        let message = Message::new(routing_key, event);
        let message_data = serde_json::to_vec(&message).map_err(|e| {
            error!("Failed to serialize event: {}", e);
            LapinError::ProtocolError("Serialization error".into())
        })?;
        
        // 發布事件
        self.channel.basic_publish(
            exchange,
            routing_key,
            BasicPublishOptions::default(),
            &message_data,
            BasicProperties::default(),
        ).await?;
        
        debug!("Event published");
        
        Ok(())
    }
}