use crate::messaging::protocol::Message;
use crate::messaging::rabbitmq::connection::RabbitMQPool;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, 
        ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions
    },
    types::FieldTable,
    BasicProperties, Channel, Consumer, Error as LapinError
};
use std::sync::Arc;
use tracing::{debug, error, info};
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use serde::{Serialize, de::DeserializeOwned};
use async_trait::async_trait;

/// 消息處理器特徵
#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(&self, payload: &[u8], properties: &BasicProperties) -> Result<Option<Vec<u8>>, anyhow::Error>;
}

/// RabbitMQ 代理
pub struct RabbitMQBroker {
    pool: RabbitMQPool,
    handlers: Arc<RwLock<HashMap<String, Arc<dyn MessageHandler>>>>,
}

impl RabbitMQBroker {
    /// 創建新的消息代理
    pub fn new(pool: RabbitMQPool) -> Self {
        Self {
            pool,
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 初始化必要的交換機和佇列
    pub async fn initialize(&self) -> Result<(), LapinError> {
        debug!("Initializing RabbitMQ broker");
        
        let conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get connection: {}", e);
            LapinError::ChannelError("Failed to get connection".into())
        })?;
        
        let channel = conn.create_channel().await?;
        
        // 宣告交換機
        for exchange_name in &["backtest.direct", "backtest.topic", "backtest.events"] {
            let exchange_type = if exchange_name.contains("direct") {
                lapin::ExchangeKind::Direct
            } else {
                lapin::ExchangeKind::Topic
            };
            
            debug!("Declaring exchange: {}", exchange_name);
            
            channel.exchange_declare(
                exchange_name,
                exchange_type,
                ExchangeDeclareOptions {
                    durable: true,
                    ..ExchangeDeclareOptions::default()
                },
                FieldTable::default(),
            ).await?;
        }
        
        info!("RabbitMQ broker initialized");
        
        Ok(())
    }
    
    /// 註冊消息處理器
    pub async fn register_handler(&self, queue_name: &str, routing_key: &str, handler: Arc<dyn MessageHandler>) -> Result<(), LapinError> {
        debug!("Registering handler for queue: {}, routing_key: {}", queue_name, routing_key);
        
        // 存儲處理器
        let mut handlers = self.handlers.write().await;
        handlers.insert(routing_key.to_string(), handler);
        
        info!("Handler registered for queue: {}, routing_key: {}", queue_name, routing_key);
        
        Ok(())
    }
    
    /// 啟動消息監聽
    pub async fn start_consuming(&self, queue_name: &str, exchange_name: &str, routing_key: &str) -> Result<(), LapinError> {
        debug!("Starting consumer for queue: {}, routing_key: {}", queue_name, routing_key);
        
        let conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get connection: {}", e);
            LapinError::ChannelError("Failed to get connection".into())
        })?;
        
        let channel = conn.create_channel().await?;
        
        // 宣告佇列
        channel.queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        ).await?;
        
        // 綁定佇列到交換機
        channel.queue_bind(
            queue_name,
            exchange_name,
            routing_key,
            QueueBindOptions::default(),
            FieldTable::default(),
        ).await?;
        
        // 創建消費者
        let consumer = channel.basic_consume(
            queue_name,
            &format!("consumer-{}", routing_key),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        ).await?;
        
        self.handle_consumer(consumer, channel, routing_key.to_string()).await;
        
        info!("Consumer started for queue: {}, routing_key: {}", queue_name, routing_key);
        
        Ok(())
    }
    
    /// 處理消費者
    async fn handle_consumer(&self, mut consumer: Consumer, channel: Channel, routing_key: String) {
        let handlers = self.handlers.clone();
        
        tokio::spawn(async move {
            debug!("Consumer handler started for routing_key: {}", routing_key);
            
            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        let delivery_tag = delivery.delivery_tag;
                        let routing_key = delivery.routing_key.to_string();
                        let reply_to = delivery.properties.reply_to().cloned();
                        let correlation_id = delivery.properties.correlation_id().cloned();
                        
                        debug!("Received message with routing_key: {}", routing_key);
                        
                        // 尋找處理器
                        let handler = {
                            let handlers = handlers.read().await;
                            handlers.get(&routing_key).cloned()
                        };
                        
                        if let Some(handler) = handler {
                            match handler.handle(&delivery.data, &delivery.properties).await {
                                Ok(Some(response_data)) => {
                                    // 如果有回應且有回應佇列，發送回應
                                    if let Some(reply_to) = reply_to {
                                        if let Some(correlation_id) = correlation_id.clone() {
                                            debug!("Sending response to {}", reply_to);
                                            
                                            if let Err(err) = channel.basic_publish(
                                                "",  // 預設交換機
                                                &reply_to,
                                                BasicPublishOptions::default(),
                                                &response_data,
                                                BasicProperties::default()
                                                    .with_correlation_id(correlation_id),
                                            ).await {
                                                error!("Failed to send response: {}", err);
                                            }
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // 無需回應
                                    debug!("Handler completed with no response");
                                }
                                Err(err) => {
                                    error!("Error handling message: {}", err);
                                    
                                    // 可以選擇發送錯誤回應
                                    if let Some(reply_to) = reply_to {
                                        if let Some(correlation_id) = correlation_id.clone() {
                                            // 構建錯誤回應...
                                        }
                                    }
                                }
                            }
                        } else {
                            error!("No handler found for routing_key: {}", routing_key);
                        }
                        
                        // 確認消息處理完成
                        if let Err(err) = channel.basic_ack(delivery_tag, BasicAckOptions::default()).await {
                            error!("Failed to acknowledge message: {}", err);
                        }
                    }
                    Err(err) => {
                        error!("Error receiving message: {}", err);
                    }
                }
            }
            
            debug!("Consumer handler stopped for routing_key: {}", routing_key);
        });
    }
    
    /// 發布消息
    pub async fn publish<T: Serialize>(&self, exchange: &str, routing_key: &str, payload: &T) -> Result<(), LapinError> {
        let conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get connection: {}", e);
            LapinError::ChannelError("Failed to get connection".into())
        })?;
        
        let channel = conn.create_channel().await?;
        
        let payload_data = serde_json::to_vec(payload).map_err(|e| {
            error!("Failed to serialize payload: {}", e);
            LapinError::ProtocolError("Serialization error".into())
        })?;
        
        debug!("Publishing message to exchange: {}, routing_key: {}", exchange, routing_key);
        
        channel.basic_publish(
            exchange,
            routing_key,
            BasicPublishOptions::default(),
            &payload_data,
            BasicProperties::default(),
        ).await?;
        
        Ok(())
    }
}