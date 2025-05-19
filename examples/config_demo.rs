use backtest_server::config::{
    Environment, get_config, init_config, ValidationUtils
};
use std::env;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日誌
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    // 從命令行參數獲取環境設置
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // 設置環境變數
        env::set_var("BACKTEST_ENV", &args[1]);
        info!("設置環境為: {}", args[1]);
    }
    
    // 初始化配置
    init_config()?;
    
    // 獲取全局配置
    let config = get_config();
    
    // 輸出當前環境配置信息
    info!("=== 配置信息 ===");
    info!("環境: {:?}", Environment::from_env());
    
    info!("資料庫主機: {}", config.database.host);
    info!("資料庫端口: {}", config.database.port);
    info!("資料庫名稱: {}", config.database.database);
    
    info!("日誌級別: {}", config.log.level);
    info!("日誌格式: {}", config.log.format);
    
    info!("伺服器主機: {}", config.server.host);
    info!("伺服器端口: {}", config.server.port);
    info!("使用HTTPS: {}", config.server.use_https);
    
    info!("策略目錄: {}", config.strategy.directory);
    info!("啟用熱更新: {}", config.strategy.hot_update_enabled);
    
    
    // 簡單驗證示例
    info!("=== 驗證示例 ===");
    match ValidationUtils::in_range(config.server.port, 1, 65535, "server.port") {
        Ok(_) => info!("伺服器端口驗證通過"),
        Err(e) => warn!("伺服器端口驗證失敗: {}", e),
    }
    
    match ValidationUtils::not_empty(&config.database.host, "database.host") {
        Ok(_) => info!("資料庫主機名稱驗證通過"),
        Err(e) => warn!("資料庫主機名稱驗證失敗: {}", e),
    }
    
    Ok(())
} 