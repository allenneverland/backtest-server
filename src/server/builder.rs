use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

use crate::config::types::{ServerConfig, RabbitMQConfig};
use crate::messaging::rabbitmq::connection::{RabbitMQConnectionManager, RabbitMQConnectionConfig};
use crate::messaging::rabbitmq::broker::{RabbitMQBroker, MessageHandler};
use crate::server::{ServerState, ServerError, ServerResult};

/// 伺服器實例
pub struct Server {
    /// 伺服器狀態
    state: Arc<RwLock<ServerState>>,
    /// RabbitMQ 連接管理器
    rabbitmq_manager: RabbitMQConnectionManager,
    /// RabbitMQ 代理
    rabbitmq_broker: Arc<RabbitMQBroker>,
    /// 關閉通道發送端
    shutdown_tx: mpsc::Sender<()>,
    /// 關閉通道接收端
    shutdown_rx: mpsc::Receiver<()>,
}

impl Server {
    /// 啟動伺服器
    pub async fn start(&mut self) -> ServerResult<()> {
        info!("啟動伺服器...");
        
        // 更新狀態為初始化中
        {
            let mut state = self.state.write().await;
            *state = ServerState::Initializing;
        }
        
        // 初始化 RabbitMQ 代理
        self.rabbitmq_broker.initialize().await
            .map_err(|e| ServerError::Initialization(format!("無法初始化RabbitMQ代理: {}", e)))?;
        
        // 更新狀態為運行中
        {
            let mut state = self.state.write().await;
            *state = ServerState::Running;
        }
        
        info!("伺服器已啟動");
        
        Ok(())
    }
    
    /// 檢查伺服器健康狀態
    pub async fn check_health(&self) -> ServerResult<()> {
        debug!("檢查伺服器健康狀態");
        
        // 檢查狀態
        let state = self.state.read().await;
        if *state != ServerState::Running {
            return Err(ServerError::Runtime(format!("伺服器狀態不是運行中: {:?}", *state)));
        }
        
        // 檢查 RabbitMQ 連接
        self.rabbitmq_manager.check_health().await?;
        
        Ok(())
    }
    
    /// 優雅關閉伺服器
    pub async fn shutdown(&mut self) -> ServerResult<()> {
        info!("正在關閉伺服器...");
        
        // 更新狀態為關閉中
        {
            let mut state = self.state.write().await;
            *state = ServerState::ShuttingDown;
        }
        
        // 發送關閉訊號
        if let Err(e) = self.shutdown_tx.send(()).await {
            error!("發送關閉訊號失敗: {}", e);
            return Err(ServerError::Shutdown(format!("發送關閉訊號失敗: {}", e)));
        }
        
        // 等待所有正在處理的請求完成
        self.wait_for_requests_completion().await?;
        
        // 更新狀態為已停止
        {
            let mut state = self.state.write().await;
            *state = ServerState::Stopped;
        }
        
        info!("伺服器已關閉");
        
        Ok(())
    }
    
    /// 等待所有請求完成處理
    async fn wait_for_requests_completion(&self) -> ServerResult<()> {
        info!("等待所有正在處理的請求完成...");
        
        // 設置超時時間
        let max_wait_time = tokio::time::Duration::from_secs(30);
        let start = tokio::time::Instant::now();
        
        // 模擬等待，實際實現中應該檢查活躍的連接數和請求數
        // 這裡只是簡單地等待一小段時間
        while start.elapsed() < max_wait_time {
            // 在實際實現中，可以檢查是否還有活躍的連接和請求
            // 如果沒有，則提前退出
            
            // 這裡實作簡單暫停，每秒檢查一次
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            debug!("等待請求完成中... 已等待 {:?}", start.elapsed());
            
            // TODO: 實現實際的請求完成檢查邏輯
            // 如果沒有活躍請求了，可以提前退出
            // if active_requests == 0 { break; }
        }
        
        if start.elapsed() >= max_wait_time {
            warn!("等待請求完成超時，強制關閉");
        } else {
            info!("所有請求已完成處理");
        }
        
        Ok(())
    }
    
    /// 註冊消息處理器
    pub async fn register_message_handler(
        &self, 
        queue_name: &str, 
        routing_key: &str, 
        handler: Arc<dyn MessageHandler>
    ) -> ServerResult<()> {
        debug!("註冊消息處理器: 佇列={}, 路由鍵={}", queue_name, routing_key);
        
        self.rabbitmq_broker.register_handler(queue_name, routing_key, handler).await
            .map_err(ServerError::Broker)?;
        
        Ok(())
    }
    
    /// 啟動消息監聽
    pub async fn start_consuming(
        &self,
        queue_name: &str,
        exchange_name: &str,
        routing_key: &str
    ) -> ServerResult<()> {
        debug!("啟動消息監聽: 佇列={}, 交換機={}, 路由鍵={}", queue_name, exchange_name, routing_key);
        
        self.rabbitmq_broker.start_consuming(queue_name, exchange_name, routing_key).await
            .map_err(ServerError::Broker)?;
        
        Ok(())
    }
    
    /// 獲取伺服器狀態
    pub async fn state(&self) -> ServerState {
        *self.state.read().await
    }
    
    /// 獲取 RabbitMQ 代理
    pub fn rabbitmq_broker(&self) -> Arc<RabbitMQBroker> {
        self.rabbitmq_broker.clone()
    }
}

/// 伺服器構建器
pub struct ServerBuilder {
    server_config: Option<ServerConfig>,
    rabbitmq_config: Option<RabbitMQConfig>,
}

impl ServerBuilder {
    /// 創建新的伺服器構建器
    pub fn new() -> Self {
        Self {
            server_config: None,
            rabbitmq_config: None,
        }
    }
    
    /// 設置伺服器配置
    pub fn with_server_config(mut self, config: ServerConfig) -> Self {
        self.server_config = Some(config);
        self
    }
    
    /// 設置 RabbitMQ 配置
    pub fn with_rabbitmq_config(mut self, config: RabbitMQConfig) -> Self {
        self.rabbitmq_config = Some(config);
        self
    }
    
    /// 構建並返回伺服器實例
    pub async fn build(self) -> ServerResult<Server> {
        info!("構建伺服器實例");
        
        // 驗證配置
        let server_config = self.server_config
            .ok_or_else(|| ServerError::Config("未提供伺服器配置".to_string()))?;
        
        let rabbitmq_config = self.rabbitmq_config
            .ok_or_else(|| ServerError::Config("未提供RabbitMQ配置".to_string()))?;
        
        // 創建 RabbitMQ 連接管理器
        let rabbitmq_manager = RabbitMQConnectionManager::from_config(&rabbitmq_config)
            .await
            .map_err(ServerError::RabbitMQConnection)?;
        
        // 創建 RabbitMQ 代理
        let rabbitmq_broker = Arc::new(RabbitMQBroker::new(rabbitmq_manager.clone()));
        
        // 創建關閉通道
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        // 創建伺服器實例
        let server = Server {
            state: Arc::new(RwLock::new(ServerState::Initializing)),
            rabbitmq_manager,
            rabbitmq_broker,
            shutdown_tx,
            shutdown_rx,
        };
        
        info!("伺服器實例構建完成");
        
        Ok(server)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}