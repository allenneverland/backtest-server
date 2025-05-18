use ndarray::{Array1, Array2};

use super::{AssetType, DataType, OHLCVPoint, TickPoint, TimeSeries};

/// 用於高效數值計算的通用時間序列矩陣
/// 以列式儲存數據，便於批量計算
#[derive(Debug, Clone)]
pub struct DataMatrix {
    pub symbol: String,
    pub asset_type: AssetType,
    pub data_type: DataType,
    pub timestamps: Array1<i64>,       // 時間戳(Unix時間戳)
    pub data: Array2<f64>,             // 列式數據矩陣 [rows=時間點, cols=數值]
    pub columns: Vec<String>,          // 列名稱
}

impl DataMatrix {
    /// 創建新的數據矩陣
    pub fn new(
        symbol: String,
        asset_type: AssetType,
        data_type: DataType,
        columns: Vec<String>,
        capacity: usize
    ) -> Self {
        Self {
            symbol,
            asset_type,
            data_type,
            timestamps: Array1::zeros(capacity),
            data: Array2::zeros((capacity, columns.len())),
            columns,
        }
    }
    
    /// 從 OHLCV 時間序列轉換
    pub fn from_ohlcv_time_series(ts: &TimeSeries<OHLCVPoint>) -> Self {
        let len = ts.len();
        let columns = vec!["open".to_string(), "high".to_string(), "low".to_string(), 
                          "close".to_string(), "volume".to_string()];
        let mut matrix = Self::new(
            ts.symbol.clone(),
            ts.asset_type,
            ts.data_type,
            columns,
            len
        );
        
        for (i, point) in ts.data.iter().enumerate() {
            matrix.timestamps[i] = point.timestamp.timestamp();
            matrix.data[[i, 0]] = point.open;
            matrix.data[[i, 1]] = point.high;
            matrix.data[[i, 2]] = point.low;
            matrix.data[[i, 3]] = point.close;
            matrix.data[[i, 4]] = point.volume;
        }
        
        matrix
    }
    
    /// 從 Tick 時間序列轉換
    pub fn from_tick_time_series(ts: &TimeSeries<TickPoint>) -> Self {
        let len = ts.len();
        let columns = vec![
            "price".to_string(), 
            "volume".to_string(),
            "trade_type".to_string(),
            "bid_price_1".to_string(), 
            "bid_price_2".to_string(),
            "bid_price_3".to_string(),
            "bid_price_4".to_string(),
            "bid_price_5".to_string(),
            "bid_volume_1".to_string(),
            "bid_volume_2".to_string(),
            "bid_volume_3".to_string(),
            "bid_volume_4".to_string(),
            "bid_volume_5".to_string(),
            "ask_price_1".to_string(),
            "ask_price_2".to_string(),
            "ask_price_3".to_string(),
            "ask_price_4".to_string(),
            "ask_price_5".to_string(),
            "ask_volume_1".to_string(),
            "ask_volume_2".to_string(),
            "ask_volume_3".to_string(),
            "ask_volume_4".to_string(),
            "ask_volume_5".to_string(),
        ];
        
        let mut matrix = Self::new(
            ts.symbol.clone(),
            ts.asset_type,
            ts.data_type,
            columns,
            len
        );
        
        for (i, point) in ts.data.iter().enumerate() {
            matrix.timestamps[i] = point.timestamp.timestamp();
            matrix.data[[i, 0]] = point.price;
            matrix.data[[i, 1]] = point.volume;
            matrix.data[[i, 2]] = f64::from(point.trade_type); // Assuming TradeType can be converted to f64
            matrix.data[[i, 3]] = point.bid_price_1;
            matrix.data[[i, 4]] = point.bid_price_2;
            matrix.data[[i, 5]] = point.bid_price_3;
            matrix.data[[i, 6]] = point.bid_price_4;
            matrix.data[[i, 7]] = point.bid_price_5;
            matrix.data[[i, 8]] = point.bid_volume_1;
            matrix.data[[i, 9]] = point.bid_volume_2;
            matrix.data[[i, 10]] = point.bid_volume_3;
            matrix.data[[i, 11]] = point.bid_volume_4;
            matrix.data[[i, 12]] = point.bid_volume_5;
            matrix.data[[i, 13]] = point.ask_price_1;
            matrix.data[[i, 14]] = point.ask_price_2;
            matrix.data[[i, 15]] = point.ask_price_3;
            matrix.data[[i, 16]] = point.ask_price_4;
            matrix.data[[i, 17]] = point.ask_price_5;
            matrix.data[[i, 18]] = point.ask_volume_1;
            matrix.data[[i, 19]] = point.ask_volume_2;
            matrix.data[[i, 20]] = point.ask_volume_3;
            matrix.data[[i, 21]] = point.ask_volume_4;
            matrix.data[[i, 22]] = point.ask_volume_5;
        }
        
        matrix
    }
    
    /// 獲取特定列的數據
    pub fn column(&self, name: &str) -> Option<Array1<f64>> {
        let index = self.columns.iter().position(|col| col == name)?;
        Some(self.data.column(index).to_owned())
    }
    
    /// 獲取時間範圍內的子集
    pub fn slice(&self, start_time: i64, end_time: i64) -> Self {
        let indices: Vec<usize> = self.timestamps
            .iter()
            .enumerate()
            .filter(|(_, &t)| t >= start_time && t <= end_time)
            .map(|(i, _)| i)
            .collect();
        
        let len = indices.len();
        let mut new_timestamps = Array1::zeros(len);
        let mut new_data = Array2::zeros((len, self.columns.len()));
        
        for (new_i, &old_i) in indices.iter().enumerate() {
            new_timestamps[new_i] = self.timestamps[old_i];
            for col in 0..self.columns.len() {
                new_data[[new_i, col]] = self.data[[old_i, col]];
            }
        }
        
        Self {
            symbol: self.symbol.clone(),
            asset_type: self.asset_type,
            data_type: self.data_type,
            timestamps: new_timestamps,
            data: new_data,
            columns: self.columns.clone(),
        }
    }
} 

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_types::data_point::TradeType; // Adjusted import path
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;

    fn create_test_ohlcv_timeseries() -> TimeSeries<OHLCVPoint> {
        let mut ts = TimeSeries::<OHLCVPoint>::new(
            "AAPL".to_string(),
            AssetType::Stock,
            DataType::OHLCV,
            None, // 添加頻率參數
            "UTC".to_string(),
        );

        let time1 = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let time2 = Utc.with_ymd_and_hms(2023, 1, 2, 0, 0, 0).unwrap();
        let time3 = Utc.with_ymd_and_hms(2023, 1, 3, 0, 0, 0).unwrap();

        let point1 = OHLCVPoint {
            timestamp: time1,
            open: 100.0,
            high: 105.0,
            low: 98.0,
            close: 103.0,
            volume: 1000.0,
            metadata: HashMap::new(),
        };
        
        let point2 = OHLCVPoint {
            timestamp: time2,
            open: 103.0,
            high: 108.0,
            low: 102.0,
            close: 107.0,
            volume: 1200.0,
            metadata: HashMap::new(),
        };
        
        let point3 = OHLCVPoint {
            timestamp: time3,
            open: 107.0,
            high: 110.0,
            low: 105.0,
            close: 109.0,
            volume: 1500.0,
            metadata: HashMap::new(),
        };

        ts.data.push(point1);
        ts.data.push(point2);
        ts.data.push(point3);
        
        ts.start_time = Some(time1); // Assuming start_time is Option<DateTime<Utc>>
        ts.end_time = Some(time3);   // Assuming end_time is Option<DateTime<Utc>>

        ts
    }

    fn create_test_tick_timeseries() -> TimeSeries<TickPoint> {
        let mut ts = TimeSeries::<TickPoint>::new(
            "AAPL".to_string(),
            AssetType::Stock,
            DataType::Tick,
            None, // 新增頻率參數
            "UTC".to_string(),
        );

        let time1 = Utc.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();
        let time2 = Utc.with_ymd_and_hms(2023, 1, 1, 10, 0, 1).unwrap();

        let point1 = TickPoint {
            timestamp: time1,
            price: 100.0,
            volume: 10.0,
            trade_type: TradeType::Buy,
            bid_price_1: 99.9,
            bid_price_2: 99.8,
            bid_price_3: 99.7,
            bid_price_4: 99.6,
            bid_price_5: 99.5,
            bid_volume_1: 10.0,
            bid_volume_2: 20.0,
            bid_volume_3: 30.0,
            bid_volume_4: 40.0,
            bid_volume_5: 50.0,
            ask_price_1: 100.1,
            ask_price_2: 100.2,
            ask_price_3: 100.3,
            ask_price_4: 100.4,
            ask_price_5: 100.5,
            ask_volume_1: 15.0,
            ask_volume_2: 25.0,
            ask_volume_3: 35.0,
            ask_volume_4: 45.0,
            ask_volume_5: 55.0,
            metadata: HashMap::new(),
        };
        
        let point2 = TickPoint {
            timestamp: time2,
            price: 100.5,
            volume: 5.0,
            trade_type: TradeType::Sell,
            bid_price_1: 100.4,
            bid_price_2: 100.3,
            bid_price_3: 100.2,
            bid_price_4: 100.1,
            bid_price_5: 100.0,
            bid_volume_1: 12.0,
            bid_volume_2: 22.0,
            bid_volume_3: 32.0,
            bid_volume_4: 42.0,
            bid_volume_5: 52.0,
            ask_price_1: 100.6,
            ask_price_2: 100.7,
            ask_price_3: 100.8,
            ask_price_4: 100.9,
            ask_price_5: 101.0,
            ask_volume_1: 18.0,
            ask_volume_2: 28.0,
            ask_volume_3: 38.0,
            ask_volume_4: 48.0,
            ask_volume_5: 58.0,
            metadata: HashMap::new(),
        };

        ts.data.push(point1);
        ts.data.push(point2);
        
        ts.start_time = Some(time1); // Assuming start_time is Option<DateTime<Utc>>
        ts.end_time = Some(time2);   // Assuming end_time is Option<DateTime<Utc>>
        ts
    }

    #[test]
    fn test_data_matrix_creation() {
        let matrix = DataMatrix::new(
            "TEST".to_string(),
            AssetType::Stock,
            DataType::OHLCV,
            vec!["open".to_string(), "close".to_string()],
            10
        );
        assert_eq!(matrix.symbol, "TEST");
        assert_eq!(matrix.timestamps.len(), 10);
        assert_eq!(matrix.data.shape(), &[10, 2]);
    }

    #[test]
    fn test_from_ohlcv_time_series() {
        let ohlcv_ts = create_test_ohlcv_timeseries();
        let matrix = DataMatrix::from_ohlcv_time_series(&ohlcv_ts);

        assert_eq!(matrix.symbol, "AAPL");
        assert_eq!(matrix.asset_type, AssetType::Stock);
        assert_eq!(matrix.data_type, DataType::OHLCV);
        assert_eq!(matrix.timestamps.len(), 3);
        assert_eq!(matrix.data.shape(), &[3, 5]); // open, high, low, close, volume

        // Check first data point
        assert_eq!(matrix.timestamps[0], ohlcv_ts.data[0].timestamp.timestamp());
        assert_eq!(matrix.data[[0, 0]], ohlcv_ts.data[0].open);
        assert_eq!(matrix.data[[0, 1]], ohlcv_ts.data[0].high);
        assert_eq!(matrix.data[[0, 2]], ohlcv_ts.data[0].low);
        assert_eq!(matrix.data[[0, 3]], ohlcv_ts.data[0].close);
        assert_eq!(matrix.data[[0, 4]], ohlcv_ts.data[0].volume);
    }

    #[test]
    fn test_from_tick_time_series() {
        let tick_ts = create_test_tick_timeseries();
        let matrix = DataMatrix::from_tick_time_series(&tick_ts);

        assert_eq!(matrix.symbol, "AAPL");
        assert_eq!(matrix.asset_type, AssetType::Stock);
        assert_eq!(matrix.data_type, DataType::Tick);
        assert_eq!(matrix.timestamps.len(), 2);
        // price, volume, trade_type, bid_px_1-5, bid_vol_1-5, ask_px_1-5, ask_vol_1-5 = 1+1+1+5+5+5+5 = 23 columns
        assert_eq!(matrix.data.shape(), &[2, 23]); 

        assert_eq!(matrix.timestamps[0], tick_ts.data[0].timestamp.timestamp());
        assert_eq!(matrix.data[[0, 0]], tick_ts.data[0].price);
        assert_eq!(matrix.data[[0, 1]], tick_ts.data[0].volume);
    }

    #[test]
    fn test_column_access() {
        let ohlcv_ts = create_test_ohlcv_timeseries();
        let matrix = DataMatrix::from_ohlcv_time_series(&ohlcv_ts);

        let close_prices = matrix.column("close").unwrap();
        assert_eq!(close_prices.len(), 3);
        assert_eq!(close_prices[0], 103.0);
        assert_eq!(close_prices[1], 107.0);
        assert_eq!(close_prices[2], 109.0);

        assert!(matrix.column("non_existent_column").is_none());
    }

    #[test]
    fn test_time_slice() {
        let ohlcv_ts = create_test_ohlcv_timeseries();
        let matrix = DataMatrix::from_ohlcv_time_series(&ohlcv_ts);
        
        let time1_ts = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap().timestamp();
        let time2_ts = Utc.with_ymd_and_hms(2023, 1, 2, 0, 0, 0).unwrap().timestamp();
        let time3_ts = Utc.with_ymd_and_hms(2023, 1, 3, 0, 0, 0).unwrap().timestamp();

        // Slice for the first two points
        let sliced_matrix1 = matrix.slice(time1_ts, time2_ts);
        assert_eq!(sliced_matrix1.timestamps.len(), 2);
        assert_eq!(sliced_matrix1.data.shape(), &[2, 5]);
        assert_eq!(sliced_matrix1.timestamps[0], time1_ts);
        assert_eq!(sliced_matrix1.timestamps[1], time2_ts);
        assert_eq!(sliced_matrix1.data[[0,3]], 103.0); // close of point1
        assert_eq!(sliced_matrix1.data[[1,3]], 107.0); // close of point2

        // Slice for the last point
        let sliced_matrix2 = matrix.slice(time3_ts, time3_ts);
        assert_eq!(sliced_matrix2.timestamps.len(), 1);
        assert_eq!(sliced_matrix2.data.shape(), &[1, 5]);
        assert_eq!(sliced_matrix2.timestamps[0], time3_ts);
        assert_eq!(sliced_matrix2.data[[0,3]], 109.0); // close of point3
        
        // Slice for a range that includes no points
        let empty_slice = matrix.slice(0, time1_ts -1);
        assert_eq!(empty_slice.timestamps.len(), 0);
        assert_eq!(empty_slice.data.shape(), &[0, 5]);
    }
    
    #[test]
    fn test_empty_matrix() {
        let matrix = DataMatrix::new(
            "EMPTY".to_string(),
            AssetType::Stock,
            DataType::OHLCV,
            vec!["price".to_string()],
            0
        );
        assert_eq!(matrix.timestamps.len(), 0);
        assert_eq!(matrix.data.shape(), &[0, 1]);

        let sliced = matrix.slice(0, 100);
        assert_eq!(sliced.timestamps.len(), 0);
    }
} 