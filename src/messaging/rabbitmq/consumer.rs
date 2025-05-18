use crate::messaging::rabbitmq::connection::RabbitMQConnectionManager;
use crate::messaging::rabbitmq::error::RabbitMQError;
use crate::messaging::protocol::Message;
use async_trait::async_trait;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, QueueBindOptions,
        QueueDeclareOptions, ExchangeDeclareOptions
    },
    types::FieldTable,
    Consumer, Channel, BasicProperties, ExchangeKind
};
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// 消息處理器特徵
#[async_trait]
pub trait MessageConsumerHandler<T: DeserializeOwned + Send + 'static>: Send + Sync {
    async fn handle(&self, message: Message<T>, delivery_tag: u64) -> Result<(), RabbitMQError>;
}

/// 消息消費者配置
#[derive(Clone, Debug)]
pub struct ConsumerConfig {
    pub queue_name: String,
    pub exchange_name: String, 
    pub exchange_type: ExchangeKind,
    pub routing_key: String,
    pub consumer_tag: Option<String>,
    pub prefetch_count: u16,
    pub auto_ack: bool,
    pub queue_durable: bool,
    pub exchange_durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            queue_name: String::new(),
            exchange_name: String::new(),
            exchange_type: ExchangeKind::Direct,
            routing_key: String::new(),
            consumer_tag: None,
            prefetch_count: 10,
            auto_ack: false,
            queue_durable: true,
            exchange_durable: true,
            exclusive: false,
            auto_delete: false,
        }
    }
}

/// 消息消費者
pub struct RabbitMQConsumer<T: DeserializeOwned + Send + 'static> {
    connection_manager: RabbitMQConnectionManager,
    config: ConsumerConfig,
    handler: Arc<dyn MessageConsumerHandler<T>>,
    running_task: Option<JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl<T: DeserializeOwned + Send + 'static> RabbitMQConsumer<T> {
    /// 創建新的消費者
    pub fn new(
        connection_manager: RabbitMQConnectionManager,
        config: ConsumerConfig,
        handler: Arc<dyn MessageConsumerHandler<T>>,
    ) -> Self {
        Self {
            connection_manager,
            config,
            handler,
            running_task: None,
            shutdown_tx: None,
        }
    }
    
    /// 初始化消費者
    pub async fn initialize(&mut self) -> Result<(), RabbitMQError> {
        let conn = self.connection_manager.get_connection().await?;
        let channel = conn.create_channel().await?;
        
        // 設置預取數量
        channel.basic_qos(self.config.prefetch_count, lapin::options::BasicQosOptions::default()).await?;
        
        // 宣告交換機
        if !self.config.exchange_name.is_empty() {
            debug!("Declaring exchange: {}", self.config.exchange_name);
            
            channel.exchange_declare(
                &self.config.exchange_name,
                self.config.exchange_type.clone(),
                ExchangeDeclareOptions {
                    durable: self.config.exchange_durable,
                    ..ExchangeDeclareOptions::default()
                },
                FieldTable::default(),
            ).await?;
        }
        
        // 宣告佇列
        debug!("Declaring queue: {}", self.config.queue_name);
        
        channel.queue_declare(
            &self.config.queue_name,
            QueueDeclareOptions {
                durable: self.config.queue_durable,
                exclusive: self.config.exclusive,
                auto_delete: self.config.auto_delete,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        ).await?;
        
        // 如果有交換機和路由鍵，綁定佇列
        if !self.config.exchange_name.is_empty() && !self.config.routing_key.is_empty() {
            debug!(
                "Binding queue {} to exchange {} with routing key {}",
                self.config.queue_name, self.config.exchange_name, self.config.routing_key
            );
            
            channel.queue_bind(
                &self.config.queue_name,
                &self.config.exchange_name,
                &self.config.routing_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            ).await?;
        }
        
        info!("Consumer initialized for queue: {}", self.config.queue_name);
        
        Ok(())
    }
    
    /// 開始消費消息
    pub async fn start(&mut self) -> Result<(), RabbitMQError> {
        if self.running_task.is_some() {
            warn!("Consumer is already running");
            return Ok(());
        }
        
        let conn = self.connection_manager.get_connection().await?;
        let channel = conn.create_channel().await?;
        
        // 設置預取數量
        channel.basic_qos(self.config.prefetch_count, lapin::options::BasicQosOptions::default()).await?;
        
        let consumer_tag = self.config.consumer_tag.clone()
            .unwrap_or_else(|| format!("consumer-{}", uuid::Uuid::new_v4()));
        
        debug!("Starting consumer with tag: {}", consumer_tag);
        
        let consumer = channel.basic_consume(
            &self.config.queue_name,
            &consumer_tag,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        ).await?;
        
        let handler = self.handler.clone();
        let auto_ack = self.config.auto_ack;
        let queue_name = self.config.queue_name.clone();
        
        // 創建用於關閉消費者的通道
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        // 啟動消費者任務
        let task = tokio::spawn(async move {
            info!("Consumer started for queue: {}", queue_name);
            
            Self::consume_messages(consumer, handler, auto_ack, channel, &mut shutdown_rx).await;
            
            info!("Consumer stopped for queue: {}", queue_name);
        });
        
        self.running_task = Some(task);
        
        Ok(())
    }
    
    /// 處理消息消費邏輯
    async fn consume_messages(
        mut consumer: Consumer,
        handler: Arc<dyn MessageConsumerHandler<T>>,
        auto_ack: bool,
        channel: Channel,
        shutdown_rx: &mut mpsc::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                // 檢查是否收到關閉信號
                _ = shutdown_rx.recv() => {
                    debug!("Received shutdown signal");
                    break;
                }
                
                // 等待下一條消息
                delivery_result = consumer.next() => {
                    match delivery_result {
                        Some(Ok(delivery)) => {
                            let delivery_tag = delivery.delivery_tag;
                            
                            debug!("Received message with delivery_tag: {}", delivery_tag);
                            
                            match serde_json::from_slice::<Message<T>>(&delivery.data) {
                                Ok(message) => {
                                    // 自動確認模式
                                    if auto_ack {
                                        if let Err(e) = handler.handle(message, delivery_tag).await {
                                            error!("Error handling message: {}", e);
                                        }
                                    } else {
                                        // 手動確認模式
                                        match handler.handle(message, delivery_tag).await {
                                            Ok(_) => {
                                                debug!("Acknowledging message: {}", delivery_tag);
                                                if let Err(e) = channel.basic_ack(delivery_tag, BasicAckOptions::default()).await {
                                                    error!("Failed to acknowledge message: {}", e);
                                                }
                                            }
                                            Err(e) => {
                                                error!("Error handling message: {}", e);
                                                // 在錯誤處理中可以實現重試邏輯或拒絕消息
                                                // 這裡簡單地確認消息，以避免無限重試
                                                if let Err(e) = channel.basic_ack(delivery_tag, BasicAckOptions::default()).await {
                                                    error!("Failed to acknowledge message after error: {}", e);
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to deserialize message: {}", e);
                                    // 確認無效消息，以避免無限重試
                                    if let Err(e) = channel.basic_ack(delivery_tag, BasicAckOptions::default()).await {
                                        error!("Failed to acknowledge invalid message: {}", e);
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("Error receiving message: {}", e);
                        }
                        None => {
                            debug!("Consumer channel closed");
                            break;
                        }
                    }
                }
            }
        }
    }
    
    /// 停止消費者
    pub async fn stop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            debug!("Sending shutdown signal to consumer");
            
            let _ = shutdown_tx.send(()).await;
            
            if let Some(task) = self.running_task.take() {
                debug!("Waiting for consumer task to complete");
                
                if let Err(e) = task.await {
                    error!("Error waiting for consumer task: {}", e);
                }
            }
            
            info!("Consumer stopped");
        }
    }
    
    /// 檢查消費者健康狀態
    pub async fn check_health(&self) -> Result<(), RabbitMQError> {
        match &self.running_task {
            Some(task) if !task.is_finished() => Ok(()),
            Some(_) => Err(RabbitMQError::Other("Consumer task has finished unexpectedly".into())),
            None => Err(RabbitMQError::Other("Consumer is not running".into())),
        }
    }
} 