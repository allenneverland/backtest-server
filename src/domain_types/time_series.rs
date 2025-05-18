use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Adjusted imports to use crate::domain_types
use crate::domain_types::{AssetType, DataType, Frequency};
use crate::domain_types::data_point::{OHLCVPoint, TickPoint};

/// 時間序列資料結構，支持向量化操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries<T> {
    pub symbol: String,
    pub asset_type: AssetType,
    pub data_type: DataType,
    pub frequency: Option<Frequency>,
    pub start_time: Option<DateTime<Utc>>, // Changed to Option for flexibility
    pub end_time: Option<DateTime<Utc>>,   // Changed to Option for flexibility
    pub timezone: String,
    pub data: Vec<T>,
    pub metadata: HashMap<String, String>,
}

impl<T> TimeSeries<T> {
    /// 創建新的時間序列
    pub fn new(
        symbol: String, 
        asset_type: AssetType, 
        data_type: DataType, 
        frequency: Option<Frequency>,
        timezone: String,
    ) -> Self {
        Self {
            symbol,
            asset_type,
            data_type,
            frequency,
            start_time: None, 
            end_time: None,   
            timezone,
            data: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// 獲取數據點數量
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 檢查是否為空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 更新時間範圍 (內部輔助函數)
    fn update_time_range(&mut self, timestamp: DateTime<Utc>) {
        if self.start_time.is_none() || timestamp < self.start_time.unwrap() {
            self.start_time = Some(timestamp);
        }
        if self.end_time.is_none() || timestamp > self.end_time.unwrap() {
            self.end_time = Some(timestamp);
        }
    }
    
    /// 重新計算時間範圍 (根據現有數據)
    pub fn recalculate_time_range(&mut self) 
    where 
        T: HasTimestamp,
    {
        self.start_time = None;
        self.end_time = None;
        
        // 收集所有時間戳，避免借用衝突
        let timestamps: Vec<DateTime<Utc>> = self.data.iter()
            .map(|point| point.timestamp())
            .collect();
            
        // 使用收集的時間戳更新時間範圍
        for timestamp in timestamps {
            self.update_time_range(timestamp);
        }
    }
}

/// 為時間序列數據點定義時間戳訪問特徵
pub trait HasTimestamp {
    fn timestamp(&self) -> DateTime<Utc>;
}

impl HasTimestamp for OHLCVPoint {
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

impl HasTimestamp for TickPoint {
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

impl<T> TimeSeries<T> 
where
    T: HasTimestamp,
{
    /// 添加單個數據點 (通用實現)
    pub fn add_point(&mut self, point: T) {
        self.update_time_range(point.timestamp());
        self.data.push(point);
    }

    /// 批量添加數據點 (通用實現)
    pub fn add_points(&mut self, points: Vec<T>) {
        for point in points {
            self.add_point(point);
        }
    }
}

/// OHLCV 時間序列的專用實現
impl TimeSeries<OHLCVPoint> {
    /// 創建新的 OHLCV 時間序列
    pub fn new_ohlcv(
        symbol: String, 
        asset_type: AssetType, 
        frequency: Option<Frequency>,  // 可選的頻率參數
        timezone: String,
    ) -> Self {
        let mut ts = Self::new(symbol, asset_type, DataType::OHLCV, frequency, timezone);
        
        // 如果提供了頻率，則將其存儲在元數據中
        if let Some(freq) = frequency {
            ts.metadata.insert("frequency".to_string(), format!("{:?}", freq));
        }
        
        ts
    }

    /// 獲取收盤價數組
    pub fn close_prices(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.close).collect()
    }

    /// 獲取開盤價數組
    pub fn open_prices(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.open).collect()
    }

    /// 獲取最高價數組
    pub fn high_prices(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.high).collect()
    }

    /// 獲取最低價數組
    pub fn low_prices(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.low).collect()
    }

    /// 獲取成交量數組
    pub fn volumes(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.volume).collect()
    }

    /// 獲取時間戳數組
    pub fn timestamps(&self) -> Vec<DateTime<Utc>> {
        self.data.iter().map(|point| point.timestamp).collect()
    }
}

/// Tick 時間序列的專用實現
impl TimeSeries<TickPoint> {
    /// 創建新的 Tick 時間序列
    pub fn new_tick(
        symbol: String, 
        asset_type: AssetType, 
        timezone: String,
    ) -> Self {
        Self::new(symbol, asset_type, DataType::Tick, None, timezone)
    }

    /// 獲取成交價數組
    pub fn prices(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.price).collect()
    }

    /// 獲取成交量數組
    pub fn volumes(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.volume).collect()
    }

    /// 獲取時間戳數組
    pub fn timestamps(&self) -> Vec<DateTime<Utc>> {
        self.data.iter().map(|point| point.timestamp).collect()
    }

    /// 獲取最優買價數組
    pub fn best_bid_prices(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.bid_price_1).collect()
    }

    /// 獲取最優賣價數組
    pub fn best_ask_prices(&self) -> Vec<f64> {
        self.data.iter().map(|point| point.ask_price_1).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_types::TradeType; // 明確導入 TradeType
    use chrono::TimeZone;
    use std::collections::HashMap;

    fn create_test_ohlcv_point(time: DateTime<Utc>, price: f64, volume: f64) -> OHLCVPoint {
        OHLCVPoint {
            timestamp: time,
            open: price,
            high: price + 1.0,
            low: price - 1.0,
            close: price + 0.5,
            volume,
            metadata: HashMap::new(),
        }
    }

    fn create_test_tick_point(time: DateTime<Utc>, price: f64, volume: f64) -> TickPoint {
        TickPoint {
            timestamp: time,
            price,
            volume,
            trade_type: TradeType::Buy, // TradeType is in scope due to `use crate::domain_types::data_point::TradeType`
            bid_price_1: price - 0.1,
            bid_price_2: price - 0.2,
            bid_price_3: price - 0.3,
            bid_price_4: price - 0.4,
            bid_price_5: price - 0.5,
            bid_volume_1: 100.0,
            bid_volume_2: 200.0,
            bid_volume_3: 300.0,
            bid_volume_4: 400.0,
            bid_volume_5: 500.0,
            ask_price_1: price + 0.1,
            ask_price_2: price + 0.2,
            ask_price_3: price + 0.3,
            ask_price_4: price + 0.4,
            ask_price_5: price + 0.5,
            ask_volume_1: 100.0,
            ask_volume_2: 200.0,
            ask_volume_3: 300.0,
            ask_volume_4: 400.0,
            ask_volume_5: 500.0,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_timeseries_creation() {
        let ts = TimeSeries::<OHLCVPoint>::new(
            "AAPL".to_string(),
            AssetType::Stock,
            DataType::OHLCV,
            None,
            "UTC".to_string(),
        );

        assert_eq!(ts.symbol, "AAPL");
        assert_eq!(ts.asset_type, AssetType::Stock);
        assert_eq!(ts.data_type, DataType::OHLCV);
        assert_eq!(ts.timezone, "UTC");
        assert!(ts.is_empty());
        assert_eq!(ts.len(), 0);
        assert!(ts.start_time.is_none());
        assert!(ts.end_time.is_none());
    }

    #[test]
    fn test_timeseries_ohlcv_add_point() {
        let mut ts = TimeSeries::<OHLCVPoint>::new(
            "AAPL".to_string(),
            AssetType::Stock,
            DataType::OHLCV,
            None,
            "UTC".to_string(),
        );

        let time1 = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let point1 = create_test_ohlcv_point(time1, 100.0, 1000.0);
        ts.add_point(point1.clone());

        assert_eq!(ts.len(), 1);
        assert!(!ts.is_empty());
        assert_eq!(ts.start_time, Some(time1));
        assert_eq!(ts.end_time, Some(time1));
        assert_eq!(ts.data[0].close, 100.5);

        let time2 = Utc.with_ymd_and_hms(2022, 12, 31, 0, 0, 0).unwrap();
        let point2 = create_test_ohlcv_point(time2, 95.0, 2000.0);
        ts.add_point(point2.clone());

        assert_eq!(ts.len(), 2);
        assert_eq!(ts.start_time, Some(time2));
        assert_eq!(ts.end_time, Some(time1));

        let time3 = Utc.with_ymd_and_hms(2023, 1, 2, 0, 0, 0).unwrap();
        let point3 = create_test_ohlcv_point(time3, 105.0, 1500.0);
        ts.add_point(point3.clone());

        assert_eq!(ts.len(), 3);
        assert_eq!(ts.start_time, Some(time2)); 
        assert_eq!(ts.end_time, Some(time3));  
    }

    #[test]
    fn test_timeseries_ohlcv_add_points() {
        let mut ts = TimeSeries::<OHLCVPoint>::new(
            "AAPL".to_string(),
            AssetType::Stock,
            DataType::OHLCV,
            None,
            "UTC".to_string(),
        );
        let time1 = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let point1 = create_test_ohlcv_point(time1, 100.0, 1000.0);
        let time2 = Utc.with_ymd_and_hms(2023, 1, 2, 0, 0, 0).unwrap();
        let point2 = create_test_ohlcv_point(time2, 102.0, 1200.0);
        
        ts.add_points(vec![point1.clone(), point2.clone()]);
        assert_eq!(ts.len(), 2);
        assert_eq!(ts.start_time, Some(time1));
        assert_eq!(ts.end_time, Some(time2));
    }

    #[test]
    fn test_timeseries_ohlcv_getter_methods() {
        let mut ts = TimeSeries::<OHLCVPoint>::new("GOOG".to_string(), AssetType::Stock, DataType::OHLCV, None, "UTC".to_string());
        let time1 = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        ts.add_point(create_test_ohlcv_point(time1, 200.0, 2000.0));
        let time2 = Utc.with_ymd_and_hms(2023, 1, 2, 0, 0, 0).unwrap();
        ts.add_point(create_test_ohlcv_point(time2, 205.0, 2200.0));

        assert_eq!(ts.close_prices(), vec![200.5, 205.5]);
        assert_eq!(ts.open_prices(), vec![200.0, 205.0]);
        assert_eq!(ts.high_prices(), vec![201.0, 206.0]);
        assert_eq!(ts.low_prices(), vec![199.0, 204.0]);
        assert_eq!(ts.volumes(), vec![2000.0, 2200.0]);
        assert_eq!(ts.timestamps(), vec![time1, time2]);
    }

    #[test]
    fn test_timeseries_tick_add_point() {
        let mut ts = TimeSeries::<TickPoint>::new("MSFT".to_string(), AssetType::Crypto, DataType::Tick, None, "UTC".to_string());
        let time1 = Utc.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();
        let point1 = create_test_tick_point(time1, 50.0, 50.0);
        ts.add_point(point1.clone());

        assert_eq!(ts.len(), 1);
        assert_eq!(ts.start_time, Some(time1));
        assert_eq!(ts.end_time, Some(time1));
        assert_eq!(ts.data[0].price, 50.0);
    }

    #[test]
    fn test_timeseries_tick_getter_methods() {
        let mut ts = TimeSeries::<TickPoint>::new("TSLA".to_string(), AssetType::Stock, DataType::Tick, None, "UTC".to_string());
        let time1 = Utc.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();
        let point1 = create_test_tick_point(time1, 50.0, 50.0);
        let time2 = Utc.with_ymd_and_hms(2023, 1, 1, 11, 0, 0).unwrap();
        let point2 = create_test_tick_point(time2, 55.0, 55.0);
        ts.add_point(point1);
        ts.add_point(point2);
        
        assert_eq!(ts.prices(), vec![50.0, 55.0]);
        assert_eq!(ts.volumes(), vec![50.0, 55.0]);
        assert_eq!(ts.timestamps(), vec![time1, time2]);
    }

    #[test]
    fn test_timeseries_edge_cases() {
        // 測試空的 TimeSeries
        let empty_ts = TimeSeries::<OHLCVPoint>::new(
            "EMPTY".to_string(), 
            AssetType::Stock, 
            DataType::OHLCV, 
            None,
            "UTC".to_string(),
        );
        assert_eq!(empty_ts.len(), 0);
        assert!(empty_ts.is_empty());
        assert_eq!(empty_ts.start_time, None);
        assert_eq!(empty_ts.end_time, None);
        
        // 測試只有一個點的 TimeSeries
        let mut single_point_ts = TimeSeries::<OHLCVPoint>::new(
            "SINGLE".to_string(), 
            AssetType::Stock, 
            DataType::OHLCV, 
            None,
            "UTC".to_string(),
        );
        let single_time = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        single_point_ts.add_point(create_test_ohlcv_point(single_time, 100.0, 1000.0));
        assert_eq!(single_point_ts.len(), 1);
        assert!(!single_point_ts.is_empty());
        assert_eq!(single_point_ts.start_time, Some(single_time));
        assert_eq!(single_point_ts.end_time, Some(single_time));
    }
} 