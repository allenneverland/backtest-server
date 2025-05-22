//! Domain Types Module 使用範例
//!
//! 此範例展示如何使用 domain_types 模組中的核心功能：
//! 1. 創建不同頻率和格式的時間序列
//! 2. 使用技術指標擴展
//! 3. 處理金融工具和重新採樣
//! 4. 使用 Polars 進行高效數據處理

use backtest_server::domain_types::{
    // 泛型時間序列和類型別名
    DailyOhlcv, MinuteOhlcv, TickData,
    
    // 頻率標記類型和 traits
    Day, Minute, Tick, FrequencyMarker,
    
    // 數據格式類型和 traits
    OhlcvFormat, TickFormat, DataFormat,
    
    // 基礎類型
    AssetType, ColumnName, Frequency, Direction, OrderType,
    
    // 功能擴展
    IndicatorsExt, Instrument,
};

use polars::prelude::*;

fn main() -> PolarsResult<()> {
    println!("=== Domain Types Module 使用範例 ===\n");

    // 1. 創建範例 OHLCV 數據
    demo_ohlcv_series()?;
    
    // 2. 創建範例 Tick 數據
    demo_tick_series()?;
    
    // 3. 展示技術指標功能
    demo_technical_indicators()?;
    
    // 4. 展示金融工具創建
    demo_instrument_creation();
    
    // 5. 展示類型系統的編譯時安全性
    demo_type_safety();

    Ok(())
}

/// 演示 OHLCV 時間序列的創建和基本操作
fn demo_ohlcv_series() -> PolarsResult<()> {
    println!("1. OHLCV 時間序列範例");
    println!("===================");

    // 創建範例 OHLCV 數據
    let ohlcv_data = df! [
        ColumnName::TIME => [
            1640995200000i64, // 2022-01-01 00:00:00
            1640995260000i64, // 2022-01-01 00:01:00  
            1640995320000i64, // 2022-01-01 00:02:00
            1640995380000i64, // 2022-01-01 00:03:00
            1640995440000i64, // 2022-01-01 00:04:00
        ],
        ColumnName::OPEN => [100.0, 101.5, 102.0, 101.8, 103.2],
        ColumnName::HIGH => [101.8, 103.0, 103.5, 104.2, 105.1],
        ColumnName::LOW => [99.5, 101.0, 101.5, 101.2, 102.8],
        ColumnName::CLOSE => [101.5, 102.0, 101.8, 103.2, 104.5],
        ColumnName::VOLUME => [10000.0, 15000.0, 12000.0, 18000.0, 20000.0],
    ]?;

    // 創建分鐘級 OHLCV 序列
    let minute_ohlcv = MinuteOhlcv::new(ohlcv_data.clone(), "AAPL".to_string())?;
    println!("分鐘級 OHLCV 序列: {:?}", minute_ohlcv);

    // 創建日級 OHLCV 序列  
    let daily_ohlcv = DailyOhlcv::new(ohlcv_data.clone(), "AAPL".to_string())?;
    println!("日級 OHLCV 序列: {:?}", daily_ohlcv);

    // 展示數據操作
    let collected = minute_ohlcv.collect()?;
    println!("收集的數據形狀: {:?}", collected.shape());
    
    // 為了獲取時間範圍，需要重新創建序列 (因為 collect 消費了原來的序列)
    let minute_ohlcv_for_time = MinuteOhlcv::new(ohlcv_data.clone(), "AAPL".to_string())?;
    let time_range = minute_ohlcv_for_time.time_range()?;
    println!("時間範圍 (毫秒): {:?}", time_range);

    println!();
    Ok(())
}

/// 演示 Tick 數據時間序列的創建和操作
fn demo_tick_series() -> PolarsResult<()> {
    println!("2. Tick 數據時間序列範例");
    println!("======================");

    // 創建範例 Tick 數據
    let tick_data = df! [
        ColumnName::TIME => [
            1640995200000i64,
            1640995200100i64, // 100ms 後
            1640995200200i64, // 200ms 後
            1640995200300i64, // 300ms 後
        ],
        ColumnName::PRICE => [100.5, 100.6, 100.4, 100.7],
        ColumnName::VOLUME => [1000.0, 1500.0, 800.0, 2000.0],
    ]?;

    // 創建 Tick 數據序列
    let tick_series = TickData::new(tick_data, "EURUSD".to_string())?;
    println!("Tick 數據序列: {:?}", tick_series);

    // 展示數據收集
    let collected_ticks = tick_series.collect()?;
    println!("Tick 數據形狀: {:?}", collected_ticks.shape());
    println!("Tick 數據預覽:");
    println!("{}", collected_ticks.head(Some(3)));

    println!();
    Ok(())
}

/// 演示技術指標功能
fn demo_technical_indicators() -> PolarsResult<()> {
    println!("3. 技術指標範例");
    println!("===============");

    // 創建更多的 OHLCV 數據用於技術指標計算
    let extended_data = df! [
        ColumnName::TIME => (0..20).map(|i| 1640995200000i64 + i * 60000).collect::<Vec<_>>(),
        ColumnName::OPEN => (0..20).map(|i| 100.0 + (i as f64) * 0.5).collect::<Vec<_>>(),
        ColumnName::HIGH => (0..20).map(|i| 101.0 + (i as f64) * 0.5).collect::<Vec<_>>(),
        ColumnName::LOW => (0..20).map(|i| 99.0 + (i as f64) * 0.5).collect::<Vec<_>>(),
        ColumnName::CLOSE => (0..20).map(|i| 100.5 + (i as f64) * 0.5 + ((i % 3) as f64 - 1.0) * 0.2).collect::<Vec<_>>(),
        ColumnName::VOLUME => (0..20).map(|i| 10000.0 + (i as f64) * 1000.0).collect::<Vec<_>>(),
    ]?;

    let ohlcv_series = MinuteOhlcv::new(extended_data, "GOOGL".to_string())?;
    
    // 計算移動平均線
    let with_sma = ohlcv_series
        .collect()?
        .sma(ColumnName::CLOSE, 5, Some("sma_5"))?;
    
    println!("添加 5 期移動平均線後的數據形狀: {:?}", with_sma.shape());
    println!("帶 SMA 的數據預覽:");
    println!("{}", with_sma.head(Some(10)));

    println!();
    Ok(())
}

/// 演示金融工具的創建
fn demo_instrument_creation() {
    println!("4. 金融工具創建範例");
    println!("==================");

    // 創建股票工具
    let stock = Instrument::new(
        "AAPL".to_string(),
        "AAPL".to_string(),
        "NASDAQ".to_string(),
        AssetType::Stock,
    );
    println!("股票工具: {:?}", stock);

    // 創建外匯工具
    let forex = Instrument::new(
        "EURUSD".to_string(),
        "EURUSD".to_string(),
        "FX".to_string(),
        AssetType::Forex,
    );
    println!("外匯工具: {:?}", forex);

    // 創建加密貨幣工具
    let crypto = Instrument::new(
        "BTCUSD".to_string(),
        "BTCUSD".to_string(),
        "BINANCE".to_string(),
        AssetType::Crypto,
    );
    println!("加密貨幣工具: {:?}", crypto);

    println!();
}

/// 演示類型系統的編譯時安全性
fn demo_type_safety() {
    println!("5. 類型安全性範例");
    println!("================");

    // 展示不同頻率標記的使用
    println!("支持的頻率類型:");
    println!("- Day: {}", Day::name());
    println!("- Minute: {}", Minute::name());  
    println!("- Tick: {}", Tick::name());

    // 展示不同數據格式的使用
    println!("\n支持的數據格式:");
    println!("- OHLCV: {}", OhlcvFormat::format_name());
    println!("- Tick: {}", TickFormat::format_name());

    // 展示基礎枚舉類型
    println!("\n資產類型範例:");
    let asset_types = [
        AssetType::Stock,
        AssetType::Future,
        AssetType::Option,
        AssetType::Forex,
        AssetType::Crypto,
    ];
    for asset_type in &asset_types {
        println!("- {}", asset_type);
    }

    println!("\n交易方向範例:");
    println!("- Long: {}", Direction::Long);
    println!("- Short: {}", Direction::Short);

    println!("\n訂單類型範例:");
    println!("- Market: {}", OrderType::Market);
    println!("- Limit: {}", OrderType::Limit);
    println!("- Stop: {}", OrderType::Stop);

    println!("\n頻率轉換範例:");
    let freq = Frequency::Minute;
    println!("- {:?} 轉為 Polars 持續時間字串: {}", freq, freq.to_polars_duration_string());
    println!("- {:?} 轉為標準持續時間: {:?}", freq, freq.to_std_duration());

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ohlcv_creation() -> PolarsResult<()> {
        let data = df! [
            ColumnName::TIME => [1640995200000i64],
            ColumnName::OPEN => [100.0],
            ColumnName::HIGH => [101.0],
            ColumnName::LOW => [99.0],
            ColumnName::CLOSE => [100.5],
            ColumnName::VOLUME => [10000.0],
        ]?;

        let series = MinuteOhlcv::new(data, "TEST".to_string())?;
        assert_eq!(series.instrument_id(), "TEST");
        Ok(())
    }

    #[test]
    fn test_tick_creation() -> PolarsResult<()> {
        let data = df! [
            ColumnName::TIME => [1640995200000i64],
            ColumnName::PRICE => [100.5],
            ColumnName::VOLUME => [1000.0],
        ]?;

        let series = TickData::new(data, "TEST".to_string())?;
        assert_eq!(series.instrument_id(), "TEST");
        Ok(())
    }

    #[test]
    fn test_instrument_creation() {
        let instrument = Instrument::new(
            "TEST".to_string(),
            "TEST".to_string(),
            "TEST_EXCHANGE".to_string(),
            AssetType::Stock,
        );

        assert_eq!(instrument.symbol, "TEST");
        assert_eq!(instrument.asset_type, AssetType::Stock);
    }
}