use crate::messaging::rabbitmq::connection::RabbitMQConnectionManager;
use crate::messaging::rabbitmq::error::RabbitMQError;
use crate::messaging::protocol::Message;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions,
        QueueDeclareOptions
    },
    types::FieldTable,
    BasicProperties, Channel, Consumer
};
use uuid::Uuid;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, oneshot};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// RPC 客戶端配置
#[derive(Clone, Debug)]
pub struct RpcClientConfig {
    pub exchange: String,
    pub default_timeout_ms: u64,
    pub reply_queue_prefix: String,
}

impl Default for RpcClientConfig {
    fn default() -> Self {
        Self {
            exchange: "backtest.direct".to_string(), 
            default_timeout_ms: 30000,  // 30 秒
            reply_queue_prefix: "rpc.reply".to_string(),
        }
    }
}

/// RPC 服務配置
#[derive(Clone, Debug)]
pub struct RpcServerConfig {
    pub queue_name: String,
    pub exchange: String,
    pub routing_key: String,
    pub prefetch_count: u16,
}

impl Default for RpcServerConfig {
    fn default() -> Self {
        Self {
            queue_name: "".to_string(),
            exchange: "backtest.direct".to_string(),
            routing_key: "".to_string(),
            prefetch_count: 10,
        }
    }
}

/// RPC 處理器特徵
pub trait RpcHandler<R, P>: Send + Sync
where
    R: Serialize + Send + 'static,
    P: DeserializeOwned + Send + 'static,
{
    fn handle(&self, payload: P) -> Result<R, RabbitMQError>;
}

impl<F, R, P> RpcHandler<R, P> for F
where
    F: Fn(P) -> Result<R, RabbitMQError> + Send + Sync,
    R: Serialize + Send + 'static,
    P: DeserializeOwned + Send + 'static,
{
    fn handle(&self, payload: P) -> Result<R, RabbitMQError> {
        self(payload)
    }
}

/// RPC 客戶端
pub struct RpcClient {
    connection_manager: RabbitMQConnectionManager,
    config: RpcClientConfig,
    channel: Arc<Mutex<Option<Channel>>>,
    reply_queue: Arc<Mutex<Option<String>>>,
    callbacks: Arc<Mutex<HashMap<String, oneshot::Sender<Vec<u8>>>>>,
    consumer_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl RpcClient {
    /// 創建新的 RPC 客戶端
    pub fn new(connection_manager: RabbitMQConnectionManager, config: RpcClientConfig) -> Self {
        Self {
            connection_manager,
            config,
            channel: Arc::new(Mutex::new(None)),
            reply_queue: Arc::new(Mutex::new(None)),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
            consumer_task: Arc::new(Mutex::new(None)),
        }
    }
    
    /// 初始化 RPC 客戶端
    pub async fn initialize(&self) -> Result<(), RabbitMQError> {
        debug!("Initializing RPC client");
        
        let conn = self.connection_manager.get_connection().await?;
        let channel = conn.create_channel().await?;
        
        // 創建隨機回覆佇列
        let reply_queue_name = format!("{}.{}", self.config.reply_queue_prefix, Uuid::new_v4());
        let queue = channel.queue_declare(
            &reply_queue_name,
            QueueDeclareOptions {
                exclusive: true,
                auto_delete: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        ).await?;
        
        let reply_queue = queue.name().to_string();
        debug!("Created RPC reply queue: {}", reply_queue);
        
        // 保存通道和回應佇列
        {
            let mut channel_guard = self.channel.lock().await;
            *channel_guard = Some(channel.clone());
            
            let mut reply_queue_guard = self.reply_queue.lock().await;
            *reply_queue_guard = Some(reply_queue.clone());
        }
        
        // 啟動回應消費者
        let callbacks = self.callbacks.clone();
        let consumer = channel.basic_consume(
            &reply_queue,
            "rpc_client_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        ).await?;
        
        let consumer_task = tokio::spawn(async move {
            Self::handle_replies(consumer, callbacks).await;
        });
        
        {
            let mut task_guard = self.consumer_task.lock().await;
            *task_guard = Some(consumer_task);
        }
        
        info!("RPC client initialized with reply queue: {}", reply_queue);
        
        Ok(())
    }
    
    /// 處理回應
    async fn handle_replies(mut consumer: Consumer, callbacks: Arc<Mutex<HashMap<String, oneshot::Sender<Vec<u8>>>>>) {
        debug!("Starting RPC reply handler");
        
        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    // 獲取相關ID
                    if let Some(correlation_id) = delivery.properties.correlation_id() {
                        let correlation_id = correlation_id.to_string();
                        debug!("Received RPC reply with correlation_id: {}", correlation_id);
                        
                        // 查找並觸發回調
                        let mut callbacks_guard = callbacks.lock().await;
                        if let Some(sender) = callbacks_guard.remove(&correlation_id) {
                            if sender.send(delivery.data).is_err() {
                                warn!("Failed to send RPC response to callback: receiver dropped");
                            }
                        } else {
                            warn!("No callback found for correlation_id: {}", correlation_id);
                        }
                    } else {
                        warn!("Received RPC reply without correlation_id");
                    }
                    
                    // 確認消息
                    if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                        error!("Failed to acknowledge RPC reply: {}", e);
                    }
                }
                Err(e) => {
                    error!("Error receiving RPC reply: {}", e);
                }
            }
        }
        
        debug!("RPC reply handler stopped");
    }
    
    /// 發送 RPC 請求
    pub async fn call<T, R>(&self, routing_key: &str, request: &T, timeout_ms: Option<u64>) -> Result<R, RabbitMQError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        // 確保客戶端已初始化
        self.ensure_initialized().await?;
        
        let channel = {
            let channel_guard = self.channel.lock().await;
            match channel_guard.as_ref() {
                Some(channel) => channel.clone(),
                None => return Err(RabbitMQError::Other("RPC client not initialized".into())),
            }
        };
        
        let reply_queue = {
            let reply_queue_guard = self.reply_queue.lock().await;
            match reply_queue_guard.as_ref() {
                Some(queue) => queue.clone(),
                None => return Err(RabbitMQError::Other("RPC reply queue not initialized".into())),
            }
        };
        
        // 創建唯一的相關ID
        let correlation_id = Uuid::new_v4().to_string();
        
        // 創建回調通道
        let (sender, receiver) = oneshot::channel();
        
        // 註冊回調
        {
            let mut callbacks_guard = self.callbacks.lock().await;
            callbacks_guard.insert(correlation_id.clone(), sender);
        }
        
        // 準備消息屬性
        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_reply_to(reply_queue.into())
            .with_correlation_id(correlation_id.clone().into());
        
        // 序列化請求
        let payload = serde_json::to_vec(request)?;
        
        debug!(
            "Sending RPC request to routing_key: {}, correlation_id: {}",
            routing_key, correlation_id
        );
        
        // 發送請求
        channel.basic_publish(
            &self.config.exchange,
            routing_key,
            BasicPublishOptions::default(),
            &payload,
            properties,
        ).await?;
        
        // 等待回應，帶超時
        let timeout_duration = Duration::from_millis(
            timeout_ms.unwrap_or(self.config.default_timeout_ms)
        );
        
        let start_time = Instant::now();
        
        let result = match timeout(timeout_duration, receiver).await {
            Ok(result) => {
                match result {
                    Ok(data) => {
                        let elapsed = start_time.elapsed();
                        debug!(
                            "Received RPC response for correlation_id: {}, elapsed: {:?}",
                            correlation_id, elapsed
                        );
                        
                        // 解析回應
                        match serde_json::from_slice::<R>(&data) {
                            Ok(response) => Ok(response),
                            Err(e) => {
                                error!("Failed to deserialize RPC response: {}", e);
                                Err(RabbitMQError::Serialization(e))
                            }
                        }
                    }
                    Err(_) => {
                        warn!("RPC response channel closed for correlation_id: {}", correlation_id);
                        Err(RabbitMQError::ChannelClosed)
                    }
                }
            }
            Err(_) => {
                // 超時，清理回調
                {
                    let mut callbacks_guard = self.callbacks.lock().await;
                    callbacks_guard.remove(&correlation_id);
                }
                
                warn!(
                    "RPC request timed out after {:?} for correlation_id: {}",
                    timeout_duration, correlation_id
                );
                
                Err(RabbitMQError::Timeout)
            }
        };
        
        result
    }
    
    /// 確保 RPC 客戶端已初始化
    async fn ensure_initialized(&self) -> Result<(), RabbitMQError> {
        let is_initialized = {
            let channel_guard = self.channel.lock().await;
            channel_guard.is_some()
        };
        
        if !is_initialized {
            debug!("RPC client not initialized, initializing now");
            self.initialize().await?;
        }
        
        Ok(())
    }
    
    /// 檢查 RPC 客戶端健康狀態
    pub async fn check_health(&self) -> Result<(), RabbitMQError> {
        self.ensure_initialized().await?;
        
        let channel = {
            let channel_guard = self.channel.lock().await;
            match channel_guard.as_ref() {
                Some(channel) => channel.clone(),
                None => return Err(RabbitMQError::Other("RPC client not initialized".into())),
            }
        };
        
        // 嘗試獲取通道狀態以檢查健康狀態
        let _ = channel.status().await?;
        
        Ok(())
    }
}

/// RPC 服務
pub struct RpcServer<R, P>
where
    R: Serialize + Send + 'static,
    P: DeserializeOwned + Send + 'static,
{
    connection_manager: RabbitMQConnectionManager,
    config: RpcServerConfig,
    handler: Arc<dyn RpcHandler<R, P>>,
    consumer_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl<R, P> RpcServer<R, P>
where
    R: Serialize + Send + 'static,
    P: DeserializeOwned + Send + 'static,
{
    /// 創建新的 RPC 服務
    pub fn new(
        connection_manager: RabbitMQConnectionManager,
        config: RpcServerConfig,
        handler: impl RpcHandler<R, P> + 'static,
    ) -> Self {
        Self {
            connection_manager,
            config,
            handler: Arc::new(handler),
            consumer_task: Arc::new(Mutex::new(None)),
        }
    }
    
    /// 啟動 RPC 服務
    pub async fn start(&self) -> Result<(), RabbitMQError> {
        debug!("Starting RPC server for queue: {}", self.config.queue_name);
        
        // 檢查是否已經啟動
        {
            let task_guard = self.consumer_task.lock().await;
            if task_guard.is_some() {
                warn!("RPC server already running");
                return Ok(());
            }
        }
        
        let conn = self.connection_manager.get_connection().await?;
        let channel = conn.create_channel().await?;
        
        // 設置預取數量
        channel.basic_qos(self.config.prefetch_count, lapin::options::BasicQosOptions::default()).await?;
        
        // 宣告佇列
        channel.queue_declare(
            &self.config.queue_name,
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        ).await?;
        
        // 如果指定了交換機和路由鍵，綁定佇列
        if !self.config.exchange.is_empty() && !self.config.routing_key.is_empty() {
            debug!(
                "Binding queue {} to exchange {} with routing key {}",
                self.config.queue_name, self.config.exchange, self.config.routing_key
            );
            
            channel.queue_bind(
                &self.config.queue_name,
                &self.config.exchange,
                &self.config.routing_key,
                lapin::options::QueueBindOptions::default(),
                FieldTable::default(),
            ).await?;
        }
        
        // 創建消費者
        let consumer = channel.basic_consume(
            &self.config.queue_name,
            &format!("rpc_server_{}", Uuid::new_v4()),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        ).await?;
        
        let handler = self.handler.clone();
        let task = tokio::spawn(async move {
            Self::handle_requests(consumer, channel, handler).await;
        });
        
        // 保存任務
        {
            let mut task_guard = self.consumer_task.lock().await;
            *task_guard = Some(task);
        }
        
        info!("RPC server started for queue: {}", self.config.queue_name);
        
        Ok(())
    }
    
    /// 處理請求
    async fn handle_requests(
        mut consumer: Consumer,
        channel: Channel,
        handler: Arc<dyn RpcHandler<R, P>>,
    ) {
        debug!("RPC request handler started");
        
        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    let delivery_tag = delivery.delivery_tag;
                    
                    // 檢查是否有回覆佇列和相關ID
                    if let (Some(reply_to), Some(correlation_id)) = (
                        delivery.properties.reply_to(),
                        delivery.properties.correlation_id()
                    ) {
                        let reply_to = reply_to.to_string();
                        let correlation_id = correlation_id.to_string();
                        
                        debug!(
                            "Received RPC request with correlation_id: {}, reply_to: {}",
                            correlation_id, reply_to
                        );
                        
                        // 解析請求載荷
                        match serde_json::from_slice::<P>(&delivery.data) {
                            Ok(payload) => {
                                // 處理請求
                                match handler.handle(payload) {
                                    Ok(response) => {
                                        // 序列化回應
                                        match serde_json::to_vec(&response) {
                                            Ok(response_data) => {
                                                // 發送回應
                                                let properties = BasicProperties::default()
                                                    .with_correlation_id(correlation_id.into())
                                                    .with_content_type("application/json".into());
                                                
                                                if let Err(e) = channel.basic_publish(
                                                    "",  // 使用預設交換機發送到回覆佇列
                                                    &reply_to,
                                                    BasicPublishOptions::default(),
                                                    &response_data,
                                                    properties,
                                                ).await {
                                                    error!("Failed to send RPC response: {}", e);
                                                } else {
                                                    debug!("Sent RPC response to: {}", reply_to);
                                                }
                                            }
                                            Err(e) => {
                                                error!("Failed to serialize RPC response: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("RPC handler error: {}", e);
                                        
                                        // 可以選擇發送錯誤回應
                                        // 這裡簡化處理，實際應用中應該有標準化的錯誤回應格式
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to deserialize RPC request: {}", e);
                            }
                        }
                    } else {
                        warn!("Received RPC request without reply_to or correlation_id");
                    }
                    
                    // 確認消息
                    if let Err(e) = channel.basic_ack(delivery_tag, BasicAckOptions::default()).await {
                        error!("Failed to acknowledge RPC request: {}", e);
                    }
                }
                Err(e) => {
                    error!("Error receiving RPC request: {}", e);
                }
            }
        }
        
        debug!("RPC request handler stopped");
    }
    
    /// 停止 RPC 服務
    pub async fn stop(&self) {
        debug!("Stopping RPC server for queue: {}", self.config.queue_name);
        
        let task = {
            let mut task_guard = self.consumer_task.lock().await;
            task_guard.take()
        };
        
        if let Some(task) = task {
            task.abort();
            info!("RPC server stopped for queue: {}", self.config.queue_name);
        } else {
            debug!("RPC server was not running");
        }
    }
    
    /// 檢查 RPC 服務健康狀態
    pub async fn check_health(&self) -> Result<(), RabbitMQError> {
        let is_running = {
            let task_guard = self.consumer_task.lock().await;
            task_guard.is_some() && !task_guard.as_ref().unwrap().is_finished()
        };
        
        if is_running {
            // 還需要檢查連接
            let conn = self.connection_manager.get_connection().await?;
            let channel = conn.create_channel().await?;
            let _ = channel.status().await?;
            
            Ok(())
        } else {
            Err(RabbitMQError::Other("RPC server is not running".into()))
        }
    }
} 