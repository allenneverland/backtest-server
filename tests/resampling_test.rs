use crate::data_provider::resampler::{ResamplerConfig, FillStrategy, TimeSeriesResampler};
use crate::domain_types::{
    data_point::{OHLCVPoint, TickPoint},
    frequency::Frequency,
    time_series::TimeSeries,
    AssetType,
    TradeType,
};
use anyhow::Result;
use chrono::{DateTime, Duration, TimeZone, Utc};
use std::collections::HashMap;

// 創建測試用 OHLCV 數據，有連續的 1 分鐘 K 線資料
fn create_minute_ohlcv_data() -> TimeSeries<OHLCVPoint> {
    let mut series = TimeSeries::new_ohlcv(
        "AAPL".to_string(),
        AssetType::Stock,
        Some(Frequency::Minute(1)),
        "UTC".to_string(),
    );
    
    let base_time = Utc.with_ymd_and_hms(2023, 5, 1, 9, 30, 0).unwrap();
    
    // 創建 1 交易日的分鐘資料 (6.5 小時 = 390 分鐘)
    for i in 0..390 {
        let time = base_time + Duration::minutes(i);
        let point = OHLCVPoint {
            timestamp: time,
            open: 150.0 + (i as f64 * 0.01).sin(),
            high: 151.0 + (i as f64 * 0.01).sin(),
            low: 149.0 + (i as f64 * 0.01).sin(),
            close: 150.5 + (i as f64 * 0.01).sin(),
            volume: 1000.0 + (i as f64 * 10.0),
            metadata: HashMap::new(),
        };
        series.add_point(point);
    }
    
    series
}

// 創建測試用 Tick 數據
fn create_tick_data() -> TimeSeries<TickPoint> {
    let mut series = TimeSeries::new_tick(
        "AAPL".to_string(),
        AssetType::Stock,
        "UTC".to_string(),
    );
    
    let base_time = Utc.with_ymd_and_hms(2023, 5, 1, 9, 30, 0).unwrap();
    
    // 創建 1 小時的 Tick 數據 (每秒 1 筆，共 3600 筆)
    for i in 0..3600 {
        let time = base_time + Duration::seconds(i);
        let point = TickPoint {
            timestamp: time,
            price: 150.0 + (i as f64 * 0.001).sin(),
            volume: 10.0,
            trade_type: if i % 2 == 0 { TradeType::Buy } else { TradeType::Sell },
            bid_price_1: 149.9 + (i as f64 * 0.001).sin(),
            bid_price_2: 149.8 + (i as f64 * 0.001).sin(),
            bid_price_3: 149.7 + (i as f64 * 0.001).sin(),
            bid_price_4: 149.6 + (i as f64 * 0.001).sin(),
            bid_price_5: 149.5 + (i as f64 * 0.001).sin(),
            bid_volume_1: 100.0,
            bid_volume_2: 200.0,
            bid_volume_3: 300.0,
            bid_volume_4: 400.0,
            bid_volume_5: 500.0,
            ask_price_1: 150.1 + (i as f64 * 0.001).sin(),
            ask_price_2: 150.2 + (i as f64 * 0.001).sin(),
            ask_price_3: 150.3 + (i as f64 * 0.001).sin(),
            ask_price_4: 150.4 + (i as f64 * 0.001).sin(),
            ask_price_5: 150.5 + (i as f64 * 0.001).sin(),
            ask_volume_1: 100.0,
            ask_volume_2: 200.0,
            ask_volume_3: 300.0,
            ask_volume_4: 400.0,
            ask_volume_5: 500.0,
            metadata: HashMap::new(),
        };
        series.add_point(point);
    }
    
    series
}

#[test]
fn test_resample_1min_to_5min() -> Result<()> {
    let minute_data = create_minute_ohlcv_data();
    
    // 原始 1 分鐘資料應該有 390 筆
    assert_eq!(minute_data.len(), 390);
    
    let config = ResamplerConfig {
        target_frequency: Frequency::Minute(5),
        fill_strategy: Some(FillStrategy::Forward),
        align_timestamp: true,
        params: HashMap::new(),
    };
    
    let resampled = TimeSeriesResampler::resample_ohlcv(&minute_data, config)?;
    
    // 重採樣為 5 分鐘，應該有 390/5 = 78 筆
    assert_eq!(resampled.len(), 78);
    assert_eq!(resampled.frequency, Some(Frequency::Minute(5)));
    
    // 檢查第一筆和最後一筆資料的時間戳是否正確
    let first_timestamp = resampled.data.first().unwrap().timestamp;
    let last_timestamp = resampled.data.last().unwrap().timestamp;
    
    let expected_first = Utc.with_ymd_and_hms(2023, 5, 1, 9, 30, 0).unwrap();
    let expected_last = Utc.with_ymd_and_hms(2023, 5, 1, 16, 0, 0).unwrap();
    
    assert_eq!(first_timestamp, expected_first);
    assert_eq!(last_timestamp, expected_last);
    
    Ok(())
}

#[test]
fn test_resample_1min_to_hourly() -> Result<()> {
    let minute_data = create_minute_ohlcv_data();
    
    let config = ResamplerConfig {
        target_frequency: Frequency::Hour(1),
        fill_strategy: None,
        align_timestamp: true,
        params: HashMap::new(),
    };
    
    let resampled = TimeSeriesResampler::resample_ohlcv(&minute_data, config)?;
    
    // 重採樣為小時，應該有 7 筆 (6.5 小時向上取整)
    assert_eq!(resampled.len(), 7);
    assert_eq!(resampled.frequency, Some(Frequency::Hour(1)));
    
    // 檢查資料內容
    for point in &resampled.data {
        // 小時線的成交量應該是分鐘線的總和 (約每小時 60 * 1000 = 60000 左右)
        assert!(point.volume > 50000.0);
        
        // 檢查 OHLC 關係
        assert!(point.high >= point.open);
        assert!(point.high >= point.close);
        assert!(point.low <= point.open);
        assert!(point.low <= point.close);
    }
    
    Ok(())
}

#[test]
fn test_resample_1min_to_daily() -> Result<()> {
    let minute_data = create_minute_ohlcv_data();
    
    let config = ResamplerConfig {
        target_frequency: Frequency::Day,
        fill_strategy: None,
        align_timestamp: true,
        params: HashMap::new(),
    };
    
    let resampled = TimeSeriesResampler::resample_ohlcv(&minute_data, config)?;
    
    // 重採樣為日線，應該有 1 筆
    assert_eq!(resampled.len(), 1);
    assert_eq!(resampled.frequency, Some(Frequency::Day));
    
    // 檢查日線資料
    let daily_point = &resampled.data[0];
    
    // 日線的成交量應該是所有分鐘線的總和
    assert!(daily_point.volume > 380000.0);
    
    // 檢查時間戳是否正確
    let expected_date = Utc.with_ymd_and_hms(2023, 5, 1, 0, 0, 0).unwrap();
    assert_eq!(daily_point.timestamp.date_naive(), expected_date.date_naive());
    
    Ok(())
}

#[test]
fn test_tick_to_ohlcv_conversion() -> Result<()> {
    let tick_data = create_tick_data();
    
    // 原始 Tick 資料應該有 3600 筆
    assert_eq!(tick_data.len(), 3600);
    
    // 轉換為 1 分鐘 K 線
    let config = ResamplerConfig {
        target_frequency: Frequency::Minute(1),
        fill_strategy: None,
        align_timestamp: true,
        params: HashMap::new(),
    };
    
    let ohlcv_data = TimeSeriesResampler::resample_tick_to_ohlcv(&tick_data, config)?;
    
    // 轉換為 1 分鐘 K 線，應該有 60 筆
    assert_eq!(ohlcv_data.len(), 60);
    assert_eq!(ohlcv_data.frequency, Some(Frequency::Minute(1)));
    
    // 檢查 OHLCV 關係
    for point in &ohlcv_data.data {
        assert!(point.high >= point.open);
        assert!(point.high >= point.close);
        assert!(point.low <= point.open);
        assert!(point.low <= point.close);
        
        // 每分鐘的成交量應該約為 60 * 10 = 600
        assert!(point.volume >= 590.0 && point.volume <= 610.0);
    }
    
    Ok(())
}

#[test]
fn test_fill_strategies() -> Result<()> {
    let mut minute_data = create_minute_ohlcv_data();
    
    // 刪除一些數據點，創造空缺
    minute_data.data.remove(50);
    minute_data.data.remove(100);
    minute_data.data.remove(150);
    minute_data.data.remove(200);
    
    // 使用前值填充策略
    let config_forward = ResamplerConfig {
        target_frequency: Frequency::Minute(15),
        fill_strategy: Some(FillStrategy::Forward),
        align_timestamp: true,
        params: HashMap::new(),
    };
    
    let resampled_forward = TimeSeriesResampler::resample_ohlcv(&minute_data, config_forward)?;
    
    // 使用線性插值填充策略
    let config_linear = ResamplerConfig {
        target_frequency: Frequency::Minute(15),
        fill_strategy: Some(FillStrategy::Linear),
        align_timestamp: true,
        params: HashMap::new(),
    };
    
    let resampled_linear = TimeSeriesResampler::resample_ohlcv(&minute_data, config_linear)?;
    
    // 不使用填充策略
    let config_none = ResamplerConfig {
        target_frequency: Frequency::Minute(15),
        fill_strategy: Some(FillStrategy::None),
        align_timestamp: true,
        params: HashMap::new(),
    };
    
    let resampled_none = TimeSeriesResampler::resample_ohlcv(&minute_data, config_none)?;
    
    // 檢查結果不為空
    assert!(!resampled_forward.is_empty());
    assert!(!resampled_linear.is_empty());
    assert!(!resampled_none.is_empty());
    
    // 所有策略應該產生相同數量的數據點
    assert_eq!(resampled_forward.len(), resampled_linear.len());
    assert_eq!(resampled_forward.len(), resampled_none.len());
    
    Ok(())
}