use std::error::Error;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::signal;
use tokio::time::sleep;
use std::time::Duration;
use clap::{App, Arg};
use tracing::{info, error, debug, Level};
use tracing_subscriber::FmtSubscriber;

use backtest_server::config::loader::ConfigLoader;
use backtest_server::config::types::{ApplicationConfig, ServerConfig, RabbitMQConfig};
use backtest_server::server::builder::ServerBuilder;
use backtest_server::server::ServerResult;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 解析命令列參數
    let matches = App::new("BacktestServer")
        .version("0.1.0")
        .author("Your Name <your.email@example.com>")
        .about("回測伺服器")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("指定配置文件路徑")
                .takes_value(true)
                .default_value("config/development.toml"),
        )
        .arg(
            Arg::with_name("log-level")
                .long("log-level")
                .value_name("LEVEL")
                .help("設置日誌級別 (trace, debug, info, warn, error)")
                .takes_value(true)
                .default_value("info"),
        )
        .get_matches();

    // 設置日誌級別
    let log_level = match matches.value_of("log-level").unwrap_or("info") {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // 初始化日誌
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // 載入配置
    let config_path = matches.value_of("config").unwrap();
    info!("載入配置文件: {}", config_path);
    
    let config_loader = ConfigLoader::new();
    let config = config_loader.load::<ApplicationConfig>(PathBuf::from(config_path))
        .map_err(|e| {
            error!("載入配置失敗: {}", e);
            e
        })?;
    
    info!("配置載入成功");

    // 建立伺服器
    info!("初始化伺服器...");
    let mut server = ServerBuilder::new()
        .with_server_config(config.server.clone())
        .with_rabbitmq_config(config.rabbitmq.clone())
        .build()
        .await
        .map_err(|e| {
            error!("伺服器初始化失敗: {}", e);
            e
        })?;

    // 啟動伺服器
    server.start().await
        .map_err(|e| {
            error!("伺服器啟動失敗: {}", e);
            e
        })?;

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