use anyhow::Result;
use chrono::{Duration, Utc};
use tokio;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use backtest_server::{
    config::{self, Environment, get_config, init_config},
    data_provider::{DataLoader, loader::{DatabaseDataLoader, CachedDataLoader}},
    domain_types::{AssetType, Frequency, DataType},
    storage::database,
};
use std::sync::Arc;

// 配置並加載數據的示例
async fn load_data_with_config() -> Result<()> {
    // 獲取配置
    let config = get_config();
    
    // 建立資料庫連接
    info!("連接到資料庫 {}:{}...", config.database.host, config.database.port);
    let db_pool = database::get_db_pool(false).await?;
    
    // 建立數據加載器
    info!("創建數據加載器...");
    let db_loader = DatabaseDataLoader::new().await?;
    
    // 包裝為快取加載器 (增強性能)
    let cached_loader = Arc::new(CachedDataLoader::new(Arc::new(db_loader)));
    
    // 設置時間範圍（過去 30 天）
    let end_time = Utc::now();
    let start_time = end_time - Duration::days(30);
    
    // 示例: 加載某個股票的日線數據
    let instrument_id = 1; // 假設 ID 為 1
    let symbol = "AAPL";   // 假設代碼為 AAPL
    
    info!("查詢 {} 的數據時間範圍...", symbol);
    if let Some(range) = cached_loader.get_data_time_range(instrument_id, DataType::OHLCV).await? {
        info!("數據可用時間範圍: {} 至 {}", range.start, range.end);
    } else {
        warn!("找不到 {} 的數據時間範圍", symbol);
        return Ok(());
    }
    
    info!("加載 {} 的日線數據...", symbol);
    let daily_data = cached_loader.load_daily_ohlcv(
        instrument_id,
        symbol,
        AssetType::Stock,
        start_time,
        end_time,
    ).await?;
    
    info!(
        "成功加載 {} 的日線數據: {} 筆記錄 (從 {} 到 {})",
        symbol,
        daily_data.len(),
        daily_data.start_time.map_or("N/A".to_string(), |t| t.to_string()),
        daily_data.end_time.map_or("N/A".to_string(), |t| t.to_string())
    );
    
    // 如果有數據，顯示前幾個數據點
    if !daily_data.is_empty() {
        info!("顯示前 5 筆日線數據:");
        for (i, point) in daily_data.data.iter().take(5).enumerate() {
            info!(
                "[{}] 日期: {}, 開盤: {:.2}, 最高: {:.2}, 最低: {:.2}, 收盤: {:.2}, 成交量: {:.0}",
                i + 1,
                point.timestamp.format("%Y-%m-%d"),
                point.open,
                point.high,
                point.low,
                point.close,
                point.volume
            );
        }
        
        // 顯示平均成交量
        let avg_volume = daily_data.data.iter().map(|p| p.volume).sum::<f64>() / daily_data.len() as f64;
        info!("平均日成交量: {:.0}", avg_volume);
    }
    
    // 加載分鐘級數據 (示範不同頻率)
    info!("加載 {} 的分鐘線數據...", symbol);
    let minute_data = cached_loader.load_ohlcv(
        instrument_id,
        symbol,
        AssetType::Stock,
        Frequency::Minute(1),
        start_time,
        end_time,
        Some(100), // 限制最多 100 筆記錄
    ).await?;
    
    info!(
        "成功加載 {} 的分鐘線數據: {} 筆記錄",
        symbol,
        minute_data.len()
    );
    
    Ok(())
}

// 主函數
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    // 顯示程式資訊
    info!("=== 數據提供模組與配置示例 ===");
    
    // 初始化配置 (從預設位置或環境變數)
    info!("初始化配置...");
    init_config()?;
    
    let config = get_config();
    info!("當前環境: {:?}", Environment::from_env());
    info!("資料庫設定: {}:{}/{}", config.database.host, config.database.port, config.database.database);
    
    // 執行加載數據示例
    match load_data_with_config().await {
        Ok(_) => info!("示例執行成功!"),
        Err(e) => warn!("示例執行失敗: {}", e),
    }
    
    Ok(())
}