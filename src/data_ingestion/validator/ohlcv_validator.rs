use chrono::{DateTime, Utc};
use super::{
    traits::DataValidator,
    error::{DataValidationError, ValidationResult},
};
use crate::domain_types::OHLCVPoint; // Updated import path

pub struct OHLCVValidator {
    // 驗證配置
    min_price: Option<f64>,
    max_price: Option<f64>,
    min_volume: Option<f64>,
    max_volume: Option<f64>,
    min_timestamp: Option<DateTime<Utc>>,
    max_timestamp: Option<DateTime<Utc>>,
    require_positive_prices: bool,
    require_positive_volume: bool,
    check_price_consistency: bool,
    require_chronological_order: bool,
}

impl OHLCVValidator {
    pub fn new() -> Self {
        Self {
            min_price: None,
            max_price: None,
            min_volume: None,
            max_volume: None,
            min_timestamp: None,
            max_timestamp: None,
            require_positive_prices: true,
            require_positive_volume: true,
            check_price_consistency: true,
            require_chronological_order: true,
        }
    }
    
    fn create_range_error(field: &str, value: &str, message: &str) -> DataValidationError {
        DataValidationError::RangeError {
            field: field.to_string(),
            value: value.to_string(),
            message: message.to_string(),
            context: None,
        }
    }
    
    fn create_consistency_error(message: &str) -> DataValidationError {
        DataValidationError::ConsistencyError {
            message: message.to_string(),
            context: None,
        }
    }
    
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
    
    pub fn with_require_positive_prices(mut self, require: bool) -> Self {
        self.require_positive_prices = require;
        self
    }
    
    pub fn with_require_positive_volume(mut self, require: bool) -> Self {
        self.require_positive_volume = require;
        self
    }
    
    pub fn with_check_price_consistency(mut self, check: bool) -> Self {
        self.check_price_consistency = check;
        self
    }
    
    pub fn with_require_chronological_order(mut self, require: bool) -> Self {
        self.require_chronological_order = require;
        self
    }
}

impl DataValidator<OHLCVPoint> for OHLCVValidator {
    fn validate_item(&self, item: &OHLCVPoint) -> ValidationResult<()> {
        // 驗證時間戳
        if let Some(min_time) = self.min_timestamp {
            if item.timestamp < min_time {
                return Err(Self::create_range_error(
                    "timestamp", 
                    &item.timestamp.to_string(), 
                    &format!("時間戳 {} 小於最小允許值 {}", item.timestamp, min_time)
                ));
            }
        }
        
        if let Some(max_time) = self.max_timestamp {
            if item.timestamp > max_time {
                 return Err(Self::create_range_error(
                    "timestamp", 
                    &item.timestamp.to_string(), 
                    &format!("時間戳 {} 大於最大允許值 {}", item.timestamp, max_time)
                ));
            }
        }

        // 驗證價格正負
        if self.require_positive_prices {
            if item.open <= 0.0 {
                return Err(Self::create_range_error("open", &item.open.to_string(), "開盤價必須為正數"));
            }
            if item.high <= 0.0 {
                return Err(Self::create_range_error("high", &item.high.to_string(), "最高價必須為正數"));
            }
            if item.low <= 0.0 {
                return Err(Self::create_range_error("low", &item.low.to_string(), "最低價必須為正數"));
            }
            if item.close <= 0.0 {
                return Err(Self::create_range_error("close", &item.close.to_string(), "收盤價必須為正數"));
            }
        }

        // 驗證價格範圍
        if let Some(min_val) = self.min_price {
            if item.open < min_val {
                return Err(Self::create_range_error("open", &item.open.to_string(), &format!("開盤價 {} 小於設定的最小值 {}", item.open, min_val)));
            }
            if item.high < min_val {
                 return Err(Self::create_range_error("high", &item.high.to_string(), &format!("最高價 {} 小於設定的最小值 {}", item.high, min_val)));
            }
            if item.low < min_val {
                 return Err(Self::create_range_error("low", &item.low.to_string(), &format!("最低價 {} 小於設定的最小值 {}", item.low, min_val)));
            }
            if item.close < min_val {
                 return Err(Self::create_range_error("close", &item.close.to_string(), &format!("收盤價 {} 小於設定的最小值 {}", item.close, min_val)));
            }
        }
        if let Some(max_val) = self.max_price {
            if item.open > max_val {
                return Err(Self::create_range_error("open", &item.open.to_string(), &format!("開盤價 {} 大於設定的最大值 {}", item.open, max_val)));
            }
            if item.high > max_val {
                return Err(Self::create_range_error("high", &item.high.to_string(), &format!("最高價 {} 大於設定的最大值 {}", item.high, max_val)));
            }
            if item.low > max_val {
                return Err(Self::create_range_error("low", &item.low.to_string(), &format!("最低價 {} 大於設定的最大值 {}", item.low, max_val)));
            }
            if item.close > max_val {
                return Err(Self::create_range_error("close", &item.close.to_string(), &format!("收盤價 {} 大於設定的最大值 {}", item.close, max_val)));
            }
        }

        // 驗證成交量正負
        if self.require_positive_volume {
            if item.volume < 0.0 { // Should be <= 0.0 if strictly positive
                return Err(Self::create_range_error("volume", &item.volume.to_string(), "成交量必須為非負數"));
            }
        }

        // 驗證成交量範圍
        if let Some(min_vol) = self.min_volume {
            if item.volume < min_vol {
                return Err(Self::create_range_error("volume", &item.volume.to_string(), &format!("成交量 {} 小於設定的最小值 {}", item.volume, min_vol)));
            }
        }
        if let Some(max_vol) = self.max_volume {
            if item.volume > max_vol {
                return Err(Self::create_range_error("volume", &item.volume.to_string(), &format!("成交量 {} 大於設定的最大值 {}", item.volume, max_vol)));
            }
        }

        // 驗證價格一致性
        if self.check_price_consistency {
            if item.high < item.low {
                return Err(Self::create_consistency_error(&format!("最高價 {} 小於最低價 {}", item.high, item.low)));
            }
            if item.high < item.open {
                return Err(Self::create_consistency_error(&format!("最高價 {} 小於開盤價 {}", item.high, item.open)));
            }
            if item.high < item.close {
                return Err(Self::create_consistency_error(&format!("最高價 {} 小於收盤價 {}", item.high, item.close)));
            }
            if item.low > item.open {
                return Err(Self::create_consistency_error(&format!("最低價 {} 大於開盤價 {}", item.low, item.open)));
            }
            if item.low > item.close {
                return Err(Self::create_consistency_error(&format!("最低價 {} 大於收盤價 {}", item.low, item.close)));
            }
        }

        Ok(())
    }

    fn validate_batch(&self, items: &[OHLCVPoint]) -> ValidationResult<()> {
        if self.require_chronological_order && !items.is_empty() {
            for i in 1..items.len() {
                if items[i].timestamp <= items[i-1].timestamp { // Should be < if strictly increasing
                    return Err(Self::create_time_series_error(&format!(
                        "時間序列未按時間順序排列：索引 {} 的時間戳 ({}) 不大於前一個索引 {} 的時間戳 ({})\n請檢查資料或考慮先進行排序處理。",
                        i, items[i].timestamp, i-1, items[i-1].timestamp
                    )));
                }
            }
        }
        // Call validate_item for each item after chronological check
        for item in items {
            self.validate_item(item)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_types::OHLCVPoint; // Ensure this points to the new location
    use chrono::{TimeZone, Utc, Duration};
    use std::collections::HashMap;

    fn create_test_point(timestamp: DateTime<Utc>, open: f64, high: f64, low: f64, close: f64, volume: f64) -> OHLCVPoint {
        OHLCVPoint {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_ohlcv_validation_valid_point() {
        let validator = OHLCVValidator::new();
        let point = create_test_point(Utc::now(), 100.0, 105.0, 98.0, 102.0, 1000.0);
        assert!(validator.validate_item(&point).is_ok());
    }

    #[test]
    fn test_negative_price_validation() {
        let validator = OHLCVValidator::new().with_require_positive_prices(true);
        let point = create_test_point(Utc::now(), -100.0, 105.0, 98.0, 102.0, 1000.0);
        let result = validator.validate_item(&point);
        assert!(result.is_err());
        if let Err(DataValidationError::RangeError { field, .. }) = result {
            assert_eq!(field, "open");
        } else {
            panic!("Expected RangeError for negative open price");
        }
    }
    
    #[test]
    fn test_zero_price_validation_when_positive_required() {
        let validator = OHLCVValidator::new().with_require_positive_prices(true);
        let point = create_test_point(Utc::now(), 0.0, 105.0, 0.0, 102.0, 1000.0);
        let result = validator.validate_item(&point);
        assert!(result.is_err());
         if let Err(DataValidationError::RangeError { field, .. }) = result {
            assert_eq!(field, "open"); // or low, depending on implementation order
        } else {
            panic!("Expected RangeError for zero open/low price when positive required");
        }
    }

    #[test]
    fn test_price_consistency_high_less_than_low() {
        let validator = OHLCVValidator::new().with_check_price_consistency(true);
        let point = create_test_point(Utc::now(), 100.0, 100.0, 101.0, 100.0, 1000.0);
        let result = validator.validate_item(&point);
        assert!(result.is_err());
        if let Err(DataValidationError::ConsistencyError { .. }) = result {
            // Expected error
        } else {
            panic!("Expected ConsistencyError for high < low");
        }
    }

    #[test]
    fn test_price_range_validation() {
        let validator = OHLCVValidator::new().with_price_range(50.0, 150.0);
        let valid_point = create_test_point(Utc::now(), 100.0, 105.0, 98.0, 102.0, 1000.0);
        assert!(validator.validate_item(&valid_point).is_ok());

        let low_open_point = create_test_point(Utc::now(), 40.0, 105.0, 98.0, 102.0, 1000.0);
        assert!(validator.validate_item(&low_open_point).is_err());
        
        let high_close_point = create_test_point(Utc::now(), 100.0, 160.0, 98.0, 155.0, 1000.0);
         let err = validator.validate_item(&high_close_point).unwrap_err();
        if let DataValidationError::RangeError{field,..} = err {
            assert!(field == "high" || field == "close"); // Order of checks might vary
        } else {
            panic!("Expected RangeError for price out of range.");
        }
    }

    #[test]
    fn test_chronological_order() {
        let validator = OHLCVValidator::new().with_require_chronological_order(true);
        let time1 = Utc::now();
        let time2 = time1 + Duration::minutes(1);
        let time3 = time1 - Duration::minutes(1); // Out of order

        let points_ordered = vec![
            create_test_point(time1, 100.0, 101.0, 99.0, 100.0, 10.0),
            create_test_point(time2, 101.0, 102.0, 100.0, 101.0, 10.0),
        ];
        assert!(validator.validate_batch(&points_ordered).is_ok());

        let points_unordered = vec![
            create_test_point(time1, 100.0, 101.0, 99.0, 100.0, 10.0),
            create_test_point(time3, 99.0, 100.0, 98.0, 99.0, 10.0), // time3 is before time1
        ];
        let result_unordered = validator.validate_batch(&points_unordered);
        assert!(result_unordered.is_err());
        if let Err(DataValidationError::TimeSeriesError { .. }) = result_unordered {
            // Correct error type
        } else {
            panic!("Expected TimeSeriesError for unordered batch");
        }
        
        let points_equal_timestamp = vec![
            create_test_point(time1, 100.0, 101.0, 99.0, 100.0, 10.0),
            create_test_point(time1, 101.0, 102.0, 100.0, 101.0, 10.0), // Same timestamp
        ];
        let result_equal = validator.validate_batch(&points_equal_timestamp);
        assert!(result_equal.is_err()); // Strict chronological order means timestamps must be increasing
         if let Err(DataValidationError::TimeSeriesError { .. }) = result_equal {
            // Correct error type
        } else {
            panic!("Expected TimeSeriesError for equal timestamps when strict order required");
        }
    }
    
    #[test]
    fn test_chronological_order_not_required() {
        let validator = OHLCVValidator::new().with_require_chronological_order(false);
        let time1 = Utc::now();
        let time3 = time1 - Duration::minutes(1);
        let points_unordered = vec![
            create_test_point(time1, 100.0, 101.0, 99.0, 100.0, 10.0),
            create_test_point(time3, 99.0, 100.0, 98.0, 99.0, 10.0),
        ];
        assert!(validator.validate_batch(&points_unordered).is_ok());
    }

    #[test]
    fn test_volume_range_validation() {
        let validator = OHLCVValidator::new().with_volume_range(100.0, 10000.0);
        let valid_point = create_test_point(Utc::now(), 100.0, 105.0, 98.0, 102.0, 1000.0);
        assert!(validator.validate_item(&valid_point).is_ok());

        let low_volume_point = create_test_point(Utc::now(), 100.0, 105.0, 98.0, 102.0, 50.0);
        let result_low = validator.validate_item(&low_volume_point);
        assert!(result_low.is_err());
        if let Err(DataValidationError::RangeError { field, .. }) = result_low {
            assert_eq!(field, "volume");
        } else {
            panic!("Expected RangeError for low volume");
        }

        let high_volume_point = create_test_point(Utc::now(), 100.0, 105.0, 98.0, 102.0, 15000.0);
        let result_high = validator.validate_item(&high_volume_point);
        assert!(result_high.is_err());
        if let Err(DataValidationError::RangeError { field, .. }) = result_high {
            assert_eq!(field, "volume");
        } else {
            panic!("Expected RangeError for high volume");
        }
    }

    #[test]
    fn test_require_positive_volume() {
        let validator = OHLCVValidator::new().with_require_positive_volume(true);
        let point_negative_volume = create_test_point(Utc::now(), 100.0, 105.0, 98.0, 102.0, -100.0);
        let result = validator.validate_item(&point_negative_volume);
        assert!(result.is_err());
        if let Err(DataValidationError::RangeError { field, message, .. }) = result {
            assert_eq!(field, "volume");
            assert!(message.contains("成交量必須為非負數"));
        } else {
            panic!("Expected RangeError for negative volume when positive required");
        }

        // Test with zero volume, should be OK if require_positive_volume is true as 0 is non-negative
        // If strictly positive is desired, the condition in validate_item should be item.volume <= 0.0
        let point_zero_volume = create_test_point(Utc::now(), 100.0, 105.0, 98.0, 102.0, 0.0);
        assert!(validator.validate_item(&point_zero_volume).is_ok()); 
    }
    
    #[test]
    fn test_time_range_validation() {
        let min_time = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        let max_time = Utc.with_ymd_and_hms(2023, 1, 31, 23, 59, 59).unwrap();
        let validator = OHLCVValidator::new().with_time_range(min_time, max_time);

        let valid_time = Utc.with_ymd_and_hms(2023, 1, 15, 0, 0, 0).unwrap();
        let valid_point = create_test_point(valid_time, 100.0, 101.0, 99.0, 100.0, 10.0);
        assert!(validator.validate_item(&valid_point).is_ok());

        let too_early_time = Utc.with_ymd_and_hms(2022, 12, 31, 0, 0, 0).unwrap();
        let early_point = create_test_point(too_early_time, 100.0, 101.0, 99.0, 100.0, 10.0);
        let result_early = validator.validate_item(&early_point);
        assert!(result_early.is_err());
         if let Err(DataValidationError::RangeError { field, .. }) = result_early {
            assert_eq!(field, "timestamp");
        } else {
            panic!("Expected RangeError for early timestamp");
        }

        let too_late_time = Utc.with_ymd_and_hms(2023, 2, 1, 0, 0, 0).unwrap();
        let late_point = create_test_point(too_late_time, 100.0, 101.0, 99.0, 100.0, 10.0);
        let result_late = validator.validate_item(&late_point);
        assert!(result_late.is_err());
        if let Err(DataValidationError::RangeError { field, .. }) = result_late {
            assert_eq!(field, "timestamp");
        } else {
            panic!("Expected RangeError for late timestamp");
        }
    }

    #[test]
    fn test_price_consistency_setting() {
        let validator_check = OHLCVValidator::new().with_check_price_consistency(true);
        let validator_no_check = OHLCVValidator::new().with_check_price_consistency(false);
        
        // High < Low
        let inconsistent_point = create_test_point(Utc::now(), 100.0, 90.0, 95.0, 98.0, 1000.0);
        assert!(validator_check.validate_item(&inconsistent_point).is_err());
        assert!(validator_no_check.validate_item(&inconsistent_point).is_ok()); 
    }

    #[test]
    fn test_positive_price_setting() {
        let validator_positive = OHLCVValidator::new().with_require_positive_prices(true);
        let validator_any_price = OHLCVValidator::new().with_require_positive_prices(false);

        let negative_price_point = create_test_point(Utc::now(), -5.0, 10.0, -6.0, 8.0, 100.0);
        assert!(validator_positive.validate_item(&negative_price_point).is_err());
        assert!(validator_any_price.validate_item(&negative_price_point).is_ok());
    }

    #[test]
    fn test_empty_batch_validation() {
        let validator = OHLCVValidator::new();
        let empty_points: Vec<OHLCVPoint> = Vec::new();
        assert!(validator.validate_batch(&empty_points).is_ok());
    }
} 