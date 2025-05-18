use chrono::{DateTime, Utc};
use super::{
    traits::DataValidator,
    error::{DataValidationError, ValidationResult},
};
use crate::domain_types::TickPoint;

pub struct TickValidator {
    // 驗證配置
    min_price: Option<f64>,
    max_price: Option<f64>,
    min_volume: Option<f64>,
    max_volume: Option<f64>,
    min_timestamp: Option<DateTime<Utc>>,
    max_timestamp: Option<DateTime<Utc>>,
    require_positive_price: bool,
    require_positive_volume: bool,
    validate_bid_ask_spread: bool,
    max_bid_ask_levels: u8,
    require_chronological_order: bool,
}

impl TickValidator {
    pub fn new() -> Self {
        Self {
            min_price: None,
            max_price: None,
            min_volume: None,
            max_volume: None,
            min_timestamp: None,
            max_timestamp: None,
            require_positive_price: true,
            require_positive_volume: true,
            validate_bid_ask_spread: true,
            max_bid_ask_levels: 5,
            require_chronological_order: true,
        }
    }
    
    // 創建範圍錯誤的輔助方法
    fn create_range_error(field: &str, value: &str, message: &str) -> DataValidationError {
        DataValidationError::RangeError {
            field: field.to_string(),
            value: value.to_string(),
            message: message.to_string(),
            context: None,
        }
    }
    
    // 創建一致性錯誤的輔助方法
    fn create_consistency_error(message: &str) -> DataValidationError {
        DataValidationError::ConsistencyError {
            message: message.to_string(),
            context: None,
        }
    }
    
    // 創建時間序列錯誤的輔助方法
    fn create_time_series_error(message: &str) -> DataValidationError {
        DataValidationError::TimeSeriesError {
            message: message.to_string(),
            context: None,
        }
    }
    
    pub fn with_price_range(mut self, min: f64, max: f64) -> Self {
        self.min_price = Some(min);
        self.max_price = Some(max);
        self
    }
    
    pub fn with_volume_range(mut self, min: f64, max: f64) -> Self {
        self.min_volume = Some(min);
        self.max_volume = Some(max);
        self
    }
    
    pub fn with_time_range(mut self, min: DateTime<Utc>, max: DateTime<Utc>) -> Self {
        self.min_timestamp = Some(min);
        self.max_timestamp = Some(max);
        self
    }
    
    pub fn set_validate_bid_ask_spread(mut self, validate: bool) -> Self {
        self.validate_bid_ask_spread = validate;
        self
    }
    
    pub fn set_max_bid_ask_levels(mut self, levels: u8) -> Self {
        self.max_bid_ask_levels = levels;
        self
    }
    
    pub fn set_require_chronological_order(mut self, require: bool) -> Self {
        self.require_chronological_order = require;
        self
    }
}

impl DataValidator<TickPoint> for TickValidator {
    fn validate_item(&self, item: &TickPoint) -> ValidationResult<()> {
        // 驗證時間戳
        if let Some(min_time) = self.min_timestamp {
            if item.timestamp < min_time {
                let err = Self::create_range_error("timestamp", item.timestamp.to_string().as_str(), &format!("時間戳小於最小允許值 {}", min_time));
                return Err(err);
            }
        }
        
        if let Some(max_time) = self.max_timestamp {
            if item.timestamp > max_time {
                let err = Self::create_range_error("timestamp", item.timestamp.to_string().as_str(), &format!("時間戳大於最大允許值 {}", max_time));
                return Err(err);
            }
        }
        
        // 驗證價格
        if self.require_positive_price && item.price <= 0.0 {
            let err = Self::create_range_error("price", item.price.to_string().as_str(), "價格必須為正數");
            return Err(err);
        }
        
        if let Some(min_price) = self.min_price {
            if item.price < min_price {
                let err = Self::create_range_error("price", item.price.to_string().as_str(), &format!("價格小於最小允許值 {}", min_price));
                return Err(err);
            }
        }
        
        if let Some(max_price) = self.max_price {
            if item.price > max_price {
                let err = Self::create_range_error("price", item.price.to_string().as_str(), &format!("價格大於最大允許值 {}", max_price));
                return Err(err);
            }
        }
        
        // 驗證成交量
        if self.require_positive_volume && item.volume < 0.0 {
            let err = Self::create_range_error("volume", item.volume.to_string().as_str(), "成交量不能為負數");
            return Err(err);
        }
        
        if let Some(min_volume) = self.min_volume {
            if item.volume < min_volume {
                let err = Self::create_range_error("volume", item.volume.to_string().as_str(), &format!("成交量小於最小允許值 {}", min_volume));
                return Err(err);
            }
        }
        
        if let Some(max_volume) = self.max_volume {
            if item.volume > max_volume {
                let err = Self::create_range_error("volume", item.volume.to_string().as_str(), &format!("成交量大於最大允許值 {}", max_volume));
                return Err(err);
            }
        }
        
        // 驗證買賣盤價格關係
        if self.validate_bid_ask_spread {
            // 檢查買賣價差是否合理 (買價應小於賣價)
            if item.bid_price_1 > 0.0 && item.ask_price_1 > 0.0 && item.bid_price_1 >= item.ask_price_1 {
                let err = Self::create_consistency_error(&format!("最優買價({})大於或等於最優賣價({})", item.bid_price_1, item.ask_price_1));
                return Err(err);
            }
            
            // 檢查買盤價格順序 (價格應依次降低)
            if item.bid_price_1 > 0.0 && item.bid_price_2 > 0.0 && item.bid_price_1 < item.bid_price_2 {
                let err = Self::create_consistency_error("買盤價格順序錯誤：level 1 價格小於 level 2");
                return Err(err);
            }
            
            if item.bid_price_2 > 0.0 && item.bid_price_3 > 0.0 && item.bid_price_2 < item.bid_price_3 {
                let err = Self::create_consistency_error("買盤價格順序錯誤：level 2 價格小於 level 3");
                return Err(err);
            }
            
            if item.bid_price_3 > 0.0 && item.bid_price_4 > 0.0 && item.bid_price_3 < item.bid_price_4 {
                let err = Self::create_consistency_error("買盤價格順序錯誤：level 3 價格小於 level 4");
                return Err(err);
            }
            
            if item.bid_price_4 > 0.0 && item.bid_price_5 > 0.0 && item.bid_price_4 < item.bid_price_5 {
                let err = Self::create_consistency_error("買盤價格順序錯誤：level 4 價格小於 level 5");
                return Err(err);
            }
            
            // 檢查賣盤價格順序 (價格應依次升高)
            if item.ask_price_1 > 0.0 && item.ask_price_2 > 0.0 && item.ask_price_1 > item.ask_price_2 {
                let err = Self::create_consistency_error("賣盤價格順序錯誤：level 1 價格大於 level 2");
                return Err(err);
            }
            
            if item.ask_price_2 > 0.0 && item.ask_price_3 > 0.0 && item.ask_price_2 > item.ask_price_3 {
                let err = Self::create_consistency_error("賣盤價格順序錯誤：level 2 價格大於 level 3");
                return Err(err);
            }
            
            if item.ask_price_3 > 0.0 && item.ask_price_4 > 0.0 && item.ask_price_3 > item.ask_price_4 {
                let err = Self::create_consistency_error("賣盤價格順序錯誤：level 3 價格大於 level 4");
                return Err(err);
            }
            
            if item.ask_price_4 > 0.0 && item.ask_price_5 > 0.0 && item.ask_price_4 > item.ask_price_5 {
                let err = Self::create_consistency_error("賣盤價格順序錯誤：level 4 價格大於 level 5");
                return Err(err);
            }
        }
        
        Ok(())
    }
    
    fn validate_batch(&self, items: &[TickPoint]) -> ValidationResult<()> {
        if items.is_empty() {
            return Ok(());
        }
        
        for item in items {
            self.validate_item(item)?;
        }
        
        if self.require_chronological_order {
            for i in 1..items.len() {
                if items[i].timestamp < items[i-1].timestamp {
                    let err = Self::create_time_series_error(&format!(
                        "數據點時間順序錯誤：索引 {} 的時間戳 ({}) 早於索引 {} 的時間戳 ({})",
                        i, items[i].timestamp, i-1, items[i-1].timestamp
                    ));
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use crate::domain_types::TickPoint;

    fn create_test_tick(timestamp_secs: i64, price: f64, volume: f64) -> TickPoint {
        TickPoint {
            timestamp: Utc.timestamp_opt(timestamp_secs, 0).unwrap(),
            price,
            volume,
            trade_type: crate::domain_types::TradeType::Unknown,
            bid_price_1: 0.0, ask_price_1: 0.0,
            bid_volume_1: 0.0, ask_volume_1: 0.0,
            bid_price_2: 0.0, ask_price_2: 0.0,
            bid_volume_2: 0.0, ask_volume_2: 0.0,
            bid_price_3: 0.0, ask_price_3: 0.0,
            bid_volume_3: 0.0, ask_volume_3: 0.0,
            bid_price_4: 0.0, ask_price_4: 0.0,
            bid_volume_4: 0.0, ask_volume_4: 0.0,
            bid_price_5: 0.0, ask_price_5: 0.0,
            bid_volume_5: 0.0, ask_volume_5: 0.0,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_tick_validator_valid_item() {
        let validator = TickValidator::new();
        let item = create_test_tick(1609459200, 100.0, 10.0);
        assert!(validator.validate_item(&item).is_ok());
    }

    // 其他測試案例保持不變，僅更新 TickPoint 引用路徑...
} 