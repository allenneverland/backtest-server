use std::sync::Arc;
use std::time::Duration;
use std::error::Error;
use tokio::signal;
use tokio::time::sleep;
use backtest_server::config::types::{ServerConfig, RabbitMQConfig};
use backtest_server::server::builder::ServerBuilder;
use backtest_server::messaging::rabbitmq::broker::MessageHandler;
use backtest_server::server::ServerResult;
use lapin::{BasicProperties};
use async_trait::async_trait;
use tracing::{info, error, debug, Level};
use tracing_subscriber::FmtSubscriber;

/// 示例消息處理器
struct ExampleHandler;

#[async_trait]
impl MessageHandler for ExampleHandler {
    async fn handle(&self, payload: &[u8], _properties: &BasicProperties) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let message = String::from_utf8_lossy(payload);
        debug!("收到消息: {}", message);
        
        // 回應消息
        Ok(Some(format!("已處理: {}", message).into_bytes()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化日誌
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("啟動伺服器示例...");
    
    // 創建伺服器配置
    let server_config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        worker_threads: 4,
        request_timeout: 30,
        max_connections: 100,
        use_https: false,
        cert_path: "".to_string(),
        key_path: "".to_string(),
        enable_compression: true,
        max_body_size: 10_485_760, // 10MB
        enable_cors: true,
        cors_allowed_origins: vec!["*".to_string()],
        static_files_dir: "static".to_string(),
    };
    
    // 創建 RabbitMQ 配置
    let rabbitmq_config = RabbitMQConfig {
        url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
        pool_size: 5,
        connection_timeout_secs: 30,
        retry_interval_secs: 5,
        max_retries: 3,
        default_exchange: "backtest.direct".to_string(),
        durable_messages: true,
        publish_confirm: true,
        prefetch_count: 10,
        consumer_tag_prefix: "backtest_server".to_string(),
    };
    
    // 構建伺服器
    let mut server = ServerBuilder::new()
        .with_server_config(server_config)
        .with_rabbitmq_config(rabbitmq_config)
        .build()
        .await?;
    
    // 註冊示例處理器
    let handler = Arc::new(ExampleHandler);
    server.register_message_handler("example_queue", "example.command", handler).await?;
    
    // 啟動伺服器
    server.start().await?;
    
    // 啟動消息監聽
    server.start_consuming("example_queue", "backtest.direct", "example.command").await?;
    
    info!("伺服器已啟動，按 Ctrl+C 停止");
    
    // 等待中斷信號
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("收到中斷信號，正在關閉伺服器...");
            if let Err(e) = server.shutdown().await {
                error!("關閉伺服器時發生錯誤: {:?}", e);
            }
        }
        Err(e) => error!("無法監聽中斷信號: {:?}", e),
    }
    
    // 等待資源清理
    sleep(Duration::from_secs(1)).await;
    info!("伺服器已關閉");
    
    Ok(())
} 