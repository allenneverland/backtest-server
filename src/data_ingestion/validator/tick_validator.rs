use super::error::{ValidationError, ValidationResult};
use super::traits::{ComposableValidator, ValidationConfig, ValidationRule, Validator};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tick 數據記錄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickRecord {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
    pub volume: f64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub bid_volume: Option<f64>,
    pub ask_volume: Option<f64>,
}

/// Tick 驗證器
pub struct TickValidator {
    config: ValidationConfig,
    /// 最大允許的時間間隔（秒）
    max_gap_seconds: i64,
    /// 最小價格
    min_price: f64,
    /// 最大價格
    max_price: f64,
    /// 最小成交量
    min_volume: f64,
    /// 最大成交量
    max_volume: f64,
    /// 最大買賣價差（百分比）
    max_spread_percent: f64,
}

impl TickValidator {
    /// 創建新的 Tick 驗證器
    pub fn new() -> Self {
        let mut config = ValidationConfig::default();
        config = config
            .with_param("max_gap_seconds", serde_json::json!(60)) // 預設1分鐘
            .with_param("min_price", serde_json::json!(0.0))
            .with_param("max_price", serde_json::json!(1_000_000.0))
            .with_param("min_volume", serde_json::json!(0.0))
            .with_param("max_volume", serde_json::json!(1e15))
            .with_param("max_spread_percent", serde_json::json!(5.0)); // 5%

        Self {
            config,
            max_gap_seconds: 60,
            min_price: 0.0,
            max_price: 1_000_000.0,
            min_volume: 0.0,
            max_volume: 1e15,
            max_spread_percent: 5.0,
        }
    }

    /// 設置最大時間間隔
    pub fn with_max_gap(mut self, seconds: i64) -> Self {
        self.max_gap_seconds = seconds;
        self.config = self.config.with_param("max_gap_seconds", serde_json::json!(seconds));
        self
    }

    /// 設置價格範圍
    pub fn with_price_range(mut self, min: f64, max: f64) -> Self {
        self.min_price = min;
        self.max_price = max;
        self.config = self.config
            .with_param("min_price", serde_json::json!(min))
            .with_param("max_price", serde_json::json!(max));
        self
    }

    /// 設置成交量範圍
    pub fn with_volume_range(mut self, min: f64, max: f64) -> Self {
        self.min_volume = min;
        self.max_volume = max;
        self.config = self.config
            .with_param("min_volume", serde_json::json!(min))
            .with_param("max_volume", serde_json::json!(max));
        self
    }

    /// 設置最大買賣價差
    pub fn with_max_spread_percent(mut self, percent: f64) -> Self {
        self.max_spread_percent = percent;
        self.config = self.config.with_param("max_spread_percent", serde_json::json!(percent));
        self
    }

    /// 驗證價格
    fn validate_price(&self, record: &TickRecord) -> ValidationResult<()> {
        // 驗證成交價
        if record.price < self.min_price || record.price > self.max_price {
            return Err(ValidationError::OutOfRange {
                field: "價格".to_string(),
                value: record.price.to_string(),
                min: self.min_price.to_string(),
                max: self.max_price.to_string(),
            });
        }

        // 檢查是否為有效數值
        if record.price.is_nan() || record.price.is_infinite() {
            return Err(ValidationError::InvalidValue {
                field: "價格".to_string(),
                value: record.price.to_string(),
                reason: "無效的數值（NaN 或無限大）".to_string(),
            });
        }

        // 檢查負值
        if record.price <= 0.0 {
            return Err(ValidationError::InvalidValue {
                field: "價格".to_string(),
                value: record.price.to_string(),
                reason: "價格必須為正數".to_string(),
            });
        }

        Ok(())
    }

    /// 驗證買賣價
    fn validate_bid_ask(&self, record: &TickRecord) -> ValidationResult<()> {
        if let (Some(bid), Some(ask)) = (record.bid, record.ask) {
            // 檢查買價和賣價的有效性
            for (name, price) in [("買價", bid), ("賣價", ask)] {
                if price < self.min_price || price > self.max_price {
                    return Err(ValidationError::OutOfRange {
                        field: name.to_string(),
                        value: price.to_string(),
                        min: self.min_price.to_string(),
                        max: self.max_price.to_string(),
                    });
                }

                if price.is_nan() || price.is_infinite() {
                    return Err(ValidationError::InvalidValue {
                        field: name.to_string(),
                        value: price.to_string(),
                        reason: "無效的數值（NaN 或無限大）".to_string(),
                    });
                }

                if price <= 0.0 {
                    return Err(ValidationError::InvalidValue {
                        field: name.to_string(),
                        value: price.to_string(),
                        reason: "價格必須為正數".to_string(),
                    });
                }
            }

            // 檢查買賣價關係
            if bid >= ask {
                return Err(ValidationError::InconsistentValue {
                    description: format!("買價 ({}) 必須小於賣價 ({})", bid, ask),
                });
            }

            // 檢查買賣價差
            let spread_percent = ((ask - bid) / bid) * 100.0;
            if spread_percent > self.max_spread_percent {
                return Err(ValidationError::InvalidValue {
                    field: "買賣價差".to_string(),
                    value: format!("{:.2}%", spread_percent),
                    reason: format!("價差超過最大允許值 {}%", self.max_spread_percent),
                });
            }

            // 檢查成交價是否在買賣價之間
            if record.price < bid || record.price > ask {
                return Err(ValidationError::InconsistentValue {
                    description: format!(
                        "成交價 ({}) 應該在買價 ({}) 和賣價 ({}) 之間",
                        record.price, bid, ask
                    ),
                });
            }
        }

        Ok(())
    }

    /// 驗證成交量
    fn validate_volume(&self, record: &TickRecord) -> ValidationResult<()> {
        // 驗證主成交量
        if record.volume < self.min_volume || record.volume > self.max_volume {
            return Err(ValidationError::OutOfRange {
                field: "成交量".to_string(),
                value: record.volume.to_string(),
                min: self.min_volume.to_string(),
                max: self.max_volume.to_string(),
            });
        }

        if record.volume.is_nan() || record.volume.is_infinite() {
            return Err(ValidationError::InvalidValue {
                field: "成交量".to_string(),
                value: record.volume.to_string(),
                reason: "無效的數值（NaN 或無限大）".to_string(),
            });
        }

        if record.volume < 0.0 {
            return Err(ValidationError::InvalidValue {
                field: "成交量".to_string(),
                value: record.volume.to_string(),
                reason: "成交量不能為負數".to_string(),
            });
        }

        // 驗證買賣成交量（如果存在）
        for (name, volume_opt) in [
            ("買方成交量", record.bid_volume),
            ("賣方成交量", record.ask_volume),
        ] {
            if let Some(volume) = volume_opt {
                if volume < 0.0 {
                    return Err(ValidationError::InvalidValue {
                        field: name.to_string(),
                        value: volume.to_string(),
                        reason: "成交量不能為負數".to_string(),
                    });
                }

                if volume.is_nan() || volume.is_infinite() {
                    return Err(ValidationError::InvalidValue {
                        field: name.to_string(),
                        value: volume.to_string(),
                        reason: "無效的數值（NaN 或無限大）".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 驗證時間序列
    pub fn validate_time_series(&self, records: &[TickRecord]) -> ValidationResult<()> {
        if records.len() < 2 {
            return Ok(());
        }

        let mut prev_timestamp = records[0].timestamp;
        
        for (_i, record) in records.iter().enumerate().skip(1) {
            // 檢查時間順序
            if record.timestamp <= prev_timestamp {
                return Err(ValidationError::OutOfOrder {
                    previous: prev_timestamp.to_rfc3339(),
                    current: record.timestamp.to_rfc3339(),
                });
            }

            // 檢查時間間隔
            let gap = (record.timestamp - prev_timestamp).num_seconds();
            if gap > self.max_gap_seconds {
                return Err(ValidationError::LargeGap {
                    gap_seconds: gap,
                    max_gap_seconds: self.max_gap_seconds,
                });
            }

            prev_timestamp = record.timestamp;
        }

        Ok(())
    }
}

impl Default for TickValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for TickValidator {
    type Data = TickRecord;

    fn name(&self) -> &str {
        "TickValidator"
    }

    fn validate_record(&self, record: &Self::Data) -> ValidationResult<()> {
        // 驗證價格
        self.validate_price(record)?;
        
        // 驗證買賣價
        self.validate_bid_ask(record)?;
        
        // 驗證成交量
        self.validate_volume(record)?;

        Ok(())
    }

    fn config(&self) -> &ValidationConfig {
        &self.config
    }
}

impl ComposableValidator for TickValidator {}

/// 創建預設的 Tick 驗證規則
pub fn create_tick_rules() -> Vec<ValidationRule<TickRecord>> {
    vec![
        // 檢查價格合理性
        ValidationRule::new(
            "reasonable_price_change",
            |record: &TickRecord| {
                // 如果有買賣價，價格變動應該合理
                if let (Some(bid), Some(ask)) = (record.bid, record.ask) {
                    let mid_price = (bid + ask) / 2.0;
                    let deviation = ((record.price - mid_price) / mid_price).abs();
                    deviation < 0.1 // 10% 偏差
                } else {
                    true
                }
            },
            "成交價偏離中間價過大",
        ),
        // 檢查成交量與價格的關係
        ValidationRule::new(
            "volume_price_consistency",
            |record: &TickRecord| {
                // 有成交量時必須有有效價格
                if record.volume > 0.0 {
                    record.price > 0.0
                } else {
                    true
                }
            },
            "有成交量時必須有有效價格",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tick(
        timestamp: DateTime<Utc>,
        price: f64,
        volume: f64,
        bid: Option<f64>,
        ask: Option<f64>,
    ) -> TickRecord {
        TickRecord {
            timestamp,
            price,
            volume,
            bid,
            ask,
            bid_volume: None,
            ask_volume: None,
        }
    }

    #[test]
    fn test_valid_tick() {
        let validator = TickValidator::new();
        let record = create_test_tick(
            Utc::now(),
            100.0,
            1000.0,
            Some(99.5),
            Some(100.5),
        );
        
        assert!(validator.validate_record(&record).is_ok());
    }

    #[test]
    fn test_invalid_bid_ask() {
        let validator = TickValidator::new();
        let record = create_test_tick(
            Utc::now(),
            100.0,
            1000.0,
            Some(101.0),  // bid > ask
            Some(99.0),
        );
        
        assert!(validator.validate_record(&record).is_err());
    }

    #[test]
    fn test_price_outside_spread() {
        let validator = TickValidator::new();
        let record = create_test_tick(
            Utc::now(),
            105.0,  // price outside bid-ask
            1000.0,
            Some(99.0),
            Some(101.0),
        );
        
        assert!(validator.validate_record(&record).is_err());
    }

    #[test]
    fn test_large_spread() {
        let validator = TickValidator::new().with_max_spread_percent(2.0);
        let record = create_test_tick(
            Utc::now(),
            100.0,
            1000.0,
            Some(95.0),   // 5.26% spread
            Some(100.0),
        );
        
        assert!(validator.validate_record(&record).is_err());
    }
}