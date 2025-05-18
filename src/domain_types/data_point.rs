use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 交易類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeType {
    Buy,        // 買入交易
    Sell,       // 賣出交易
    Neutral,    // 中性交易
    Cross,      // 撮合交易
    Unknown,    // 未知類型
}

impl From<TradeType> for f64 {
    fn from(trade_type: TradeType) -> Self {
        match trade_type {
            TradeType::Buy => 1.0,
            TradeType::Sell => 2.0,
            TradeType::Neutral => 0.0,
            TradeType::Cross => 3.0,
            TradeType::Unknown => -1.0,
        }
    }
}

/// OHLCV 數據點結構
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OHLCVPoint {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub metadata: HashMap<String, String>,
}

/// Tick 數據點結構
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TickPoint {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
    pub volume: f64,
    pub trade_type: TradeType,
    pub bid_price_1: f64,
    pub bid_price_2: f64,
    pub bid_price_3: f64,
    pub bid_price_4: f64,
    pub bid_price_5: f64,
    pub bid_volume_1: f64,
    pub bid_volume_2: f64,
    pub bid_volume_3: f64,
    pub bid_volume_4: f64,
    pub bid_volume_5: f64,
    pub ask_price_1: f64,
    pub ask_price_2: f64,
    pub ask_price_3: f64,
    pub ask_price_4: f64,
    pub ask_price_5: f64,
    pub ask_volume_1: f64,
    pub ask_volume_2: f64,
    pub ask_volume_3: f64,
    pub ask_volume_4: f64,
    pub ask_volume_5: f64,
    pub metadata: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_trade_type_conversion() {
        assert_eq!(f64::from(TradeType::Buy), 1.0);
        assert_eq!(f64::from(TradeType::Sell), 2.0);
        assert_eq!(f64::from(TradeType::Neutral), 0.0);
        assert_eq!(f64::from(TradeType::Cross), 3.0);
        assert_eq!(f64::from(TradeType::Unknown), -1.0);
    }

    #[test]
    fn test_ohlcv_point_creation() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let point = OHLCVPoint {
            timestamp,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 1000.0,
            metadata: HashMap::new(),
        };

        assert_eq!(point.timestamp, timestamp);
        assert_eq!(point.open, 100.0);
        assert_eq!(point.high, 110.0);
        assert_eq!(point.low, 95.0);
        assert_eq!(point.close, 105.0);
        assert_eq!(point.volume, 1000.0);
    }

    #[test]
    fn test_ohlcv_point_with_metadata() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "test".to_string());
        metadata.insert("adjusted".to_string(), "true".to_string());
        
        let point = OHLCVPoint {
            timestamp,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 1000.0,
            metadata: metadata.clone(),
        };

        assert_eq!(point.metadata.len(), 2);
        assert_eq!(point.metadata.get("source"), Some(&"test".to_string()));
        assert_eq!(point.metadata.get("adjusted"), Some(&"true".to_string()));
    }

    #[test]
    fn test_tick_point_creation() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let point = TickPoint {
            timestamp,
            price: 100.0,
            volume: 10.0,
            trade_type: TradeType::Buy,
            bid_price_1: 99.0,
            bid_price_2: 98.0,
            bid_price_3: 97.0,
            bid_price_4: 96.0,
            bid_price_5: 95.0,
            bid_volume_1: 100.0,
            bid_volume_2: 200.0,
            bid_volume_3: 300.0,
            bid_volume_4: 400.0,
            bid_volume_5: 500.0,
            ask_price_1: 101.0,
            ask_price_2: 102.0,
            ask_price_3: 103.0,
            ask_price_4: 104.0,
            ask_price_5: 105.0,
            ask_volume_1: 100.0,
            ask_volume_2: 200.0,
            ask_volume_3: 300.0,
            ask_volume_4: 400.0,
            ask_volume_5: 500.0,
            metadata: HashMap::new(),
        };

        assert_eq!(point.timestamp, timestamp);
        assert_eq!(point.price, 100.0);
        assert_eq!(point.volume, 10.0);
        assert_eq!(point.trade_type, TradeType::Buy);
        assert_eq!(point.bid_price_1, 99.0);
        assert_eq!(point.ask_price_1, 101.0);
    }

    #[test]
    fn test_tick_point_edge_cases() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let point = TickPoint {
            timestamp,
            price: 100.0,
            volume: 0.0,  
            trade_type: TradeType::Neutral,
            bid_price_1: 99.0,
            bid_price_2: 98.0,
            bid_price_3: 97.0,
            bid_price_4: 96.0,
            bid_price_5: 95.0,
            bid_volume_1: 0.0,  
            bid_volume_2: 0.0,
            bid_volume_3: 0.0,
            bid_volume_4: 0.0,
            bid_volume_5: 0.0,
            ask_price_1: 101.0,
            ask_price_2: 102.0,
            ask_price_3: 103.0,
            ask_price_4: 104.0,
            ask_price_5: 105.0,
            ask_volume_1: 0.0,  
            ask_volume_2: 0.0,
            ask_volume_3: 0.0,
            ask_volume_4: 0.0,
            ask_volume_5: 0.0,
            metadata: HashMap::new(),
        };

        assert_eq!(point.volume, 0.0);
        assert_eq!(point.bid_volume_1, 0.0);
        assert_eq!(point.ask_volume_1, 0.0);
        assert_eq!(point.trade_type, TradeType::Neutral);
    }
} 