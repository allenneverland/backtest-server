use anyhow::Result;
use chrono::{Duration, Utc};
use tokio;

use backtest_server::{
    config,
    data_provider::{DataLoader, loader::DatabaseDataLoader},
    domain_types::{AssetType, Frequency, TimeSeries, data_point::OHLCVPoint},
    storage::database,
};

// 示例: 從數據庫加載不同頻率的市場數據
async fn load_market_data_example() -> Result<()> {
    // 初始化配置和數據庫連接
    config::init_config("config/development.toml")?;
    let db_pool = database::get_db_pool(true).await?;
    
    println!("Database connected successfully!");
    
    // 創建數據加載器
    let data_loader = DatabaseDataLoader::new().await?;
    
    // 設置時間範圍（過去 30 天）
    let end_time = Utc::now();
    let start_time = end_time - Duration::days(30);
    
    // 假設我們有一個 instrument_id 為 1 的股票
    let instrument_id = 1;
    let symbol = "AAPL";
    let asset_type = AssetType::Stock;
    
    println!("Loading minute OHLCV data...");
    
    // 加載分鐘級 OHLCV 數據
    let minute_ohlcv = data_loader.load_ohlcv(
        instrument_id,
        symbol,
        asset_type,
        Frequency::Minute(1),
        start_time,
        end_time,
        Some(100), // 僅加載前 100 條記錄
    ).await?;
    
    print_time_series_info(&minute_ohlcv, "Minute OHLCV");
    
    println!("Loading daily OHLCV data...");
    
    // 加載日級 OHLCV 數據
    let daily_ohlcv = data_loader.load_daily_ohlcv(
        instrument_id,
        symbol,
        asset_type,
        start_time,
        end_time,
    ).await?;
    
    print_time_series_info(&daily_ohlcv, "Daily OHLCV");
    
    println!("Loading tick data...");
    
    // 加載 Tick 數據
    let tick_data = data_loader.load_tick(
        instrument_id,
        symbol,
        asset_type,
        start_time,
        end_time,
        Some(100), // 僅加載前 100 條記錄
    ).await?;
    
    println!(
        "Tick data: symbol={}, length={}, start_time={:?}, end_time={:?}",
        tick_data.symbol,
        tick_data.len(),
        tick_data.start_time,
        tick_data.end_time
    );
    
    println!("Loading technical indicator data...");
    
    // 假設我們有一個 indicator_id 為 1 的技術指標
    let indicator_id = 1;
    
    // 加載技術指標數據
    let technical_indicators = data_loader.load_technical_indicator(
        instrument_id,
        indicator_id,
        start_time,
        end_time,
    ).await?;
    
    println!("Technical indicators: count={}", technical_indicators.len());
    
    println!("Loading fundamental indicator data...");
    
    // 加載基本面指標數據（例如: 財務比率）
    let fundamental_indicators = data_loader.load_fundamental_indicator(
        instrument_id,
        "financial_ratio",
        start_time,
        end_time,
    ).await?;
    
    println!("Fundamental indicators: count={}", fundamental_indicators.len());
    
    println!("Getting data time range...");
    
    // 獲取數據的可用時間範圍
    let time_range = data_loader.get_data_time_range(
        instrument_id,
        backtest_server::domain_types::DataType::OHLCV,
    ).await?;
    
    println!("OHLCV data time range: {:?}", time_range);
    
    Ok(())
}

// 打印時間序列的基本信息
fn print_time_series_info(ts: &TimeSeries<OHLCVPoint>, label: &str) {
    println!(
        "{}: symbol={}, length={}, start_time={:?}, end_time={:?}",
        label,
        ts.symbol,
        ts.len(),
        ts.start_time,
        ts.end_time
    );
    
    // 打印前 5 個數據點
    println!("First 5 data points:");
    for (i, point) in ts.data.iter().take(5).enumerate() {
        println!(
            "  [{}] time={}, open={:.2}, high={:.2}, low={:.2}, close={:.2}, volume={}",
            i,
            point.timestamp,
            point.open,
            point.high,
            point.low,
            point.close,
            point.volume
        );
    }
}

// 主函數
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Data Loader Example ===");
    
    match load_market_data_example().await {
        Ok(_) => println!("Example completed successfully!"),
        Err(e) => println!("Error: {}", e),
    }
    
    Ok(())
}