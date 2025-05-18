// builder.rs - 伺服器構建器模式實現

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::ServerConfig;
use crate::server::ServerState;
use crate::server::error::{ServerError, ServerResult};

/// 伺服器構建器
/// 
/// 使用建造者模式來創建和配置伺服器實例
pub struct ServerBuilder {
    config: ServerConfig,
    state: ServerState,
}

impl ServerBuilder {
    /// 創建新的伺服器構建器實例
    /// 
    /// 參數:
    ///     config: 伺服器配置
    /// 
    /// 返回:
    ///     ServerBuilder: 新的伺服器構建器實例
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            state: ServerState::Initializing,
        }
    }

    /// 設置伺服器端口
    /// 
    /// 參數:
    ///     port: 伺服器監聽端口
    /// 
    /// 返回:
    ///     Self: 構建器實例，支持方法鏈
    pub fn with_port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// 設置伺服器主機名
    /// 
    /// 參數:
    ///     host: 伺服器主機名
    /// 
    /// 返回:
    ///     Self: 構建器實例，支持方法鏈
    pub fn with_host(mut self, host: String) -> Self {
        self.config.host = host;
        self
    }

    /// 設置伺服器工作線程數
    /// 
    /// 參數:
    ///     worker_threads: 工作線程數量
    /// 
    /// 返回:
    ///     Self: 構建器實例，支持方法鏈
    pub fn with_worker_threads(mut self, worker_threads: usize) -> Self {
        self.config.worker_threads = worker_threads;
        self
    }

    /// 構建伺服器實例
    /// 
    /// 返回:
    ///     ServerResult<Server>: 成功構建的伺服器實例或錯誤
    pub fn build(self) -> ServerResult<Server> {
        if self.config.port == 0 {
            return Err(ServerError::InvalidConfiguration("Port cannot be zero".to_string()));
        }

        Ok(Server {
            config: self.config,
            state: Arc::new(RwLock::new(self.state)),
        })
    }
}

/// 伺服器實例
pub struct Server {
    config: ServerConfig,
    state: Arc<RwLock<ServerState>>,
}

impl Server {
    /// 創建新的伺服器實例
    /// 
    /// 參數:
    ///     config: 伺服器配置
    /// 
    /// 返回:
    ///     ServerBuilder: 伺服器構建器實例
    pub fn builder(config: ServerConfig) -> ServerBuilder {
        ServerBuilder::new(config)
    }

    /// 啟動伺服器
    /// 
    /// 參數: 無
    /// 
    /// 返回:
    ///     ServerResult<()>: 啟動結果
    pub async fn start(&self) -> ServerResult<()> {
        // 更新伺服器狀態為運行中
        let mut state = self.state.write().await;
        *state = ServerState::Running;
        drop(state);

        // 記錄伺服器啟動信息
        println!("Server started on {}:{}", self.config.host, self.config.port);
        
        // 在實際項目中，這裡需要實現伺服器監聽邏輯
        // 為了簡單演示，這裡只返回成功
        Ok(())
    }

    /// 停止伺服器
    /// 
    /// 參數: 無
    /// 
    /// 返回:
    ///     ServerResult<()>: 停止結果
    pub async fn stop(&self) -> ServerResult<()> {
        // 更新伺服器狀態為關閉中
        let mut state = self.state.write().await;
        *state = ServerState::ShuttingDown;
        
        // 執行實際關閉邏輯 (在此僅為示例)
        println!("Server shutting down...");
        
        // 關閉完成，更新狀態
        *state = ServerState::Stopped;
        
        Ok(())
    }

    /// 獲取伺服器當前狀態
    /// 
    /// 返回:
    ///     ServerResult<ServerState>: 伺服器當前狀態
    pub async fn get_state(&self) -> ServerResult<ServerState> {
        let state = self.state.read().await;
        Ok(*state)
    }

    /// 獲取伺服器配置
    /// 
    /// 返回:
    ///     &ServerConfig: 伺服器配置引用
    pub fn get_config(&self) -> &ServerConfig {
        &self.config
    }
} 