use super::error::{ValidationError, ValidationResult};
use super::traits::{ComposableValidator, ValidationConfig, ValidationRule, Validator};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OHLCV 數據記錄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvRecord {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// OHLCV 驗證器
pub struct OhlcvValidator {
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
}

impl OhlcvValidator {
    /// 創建新的 OHLCV 驗證器
    pub fn new() -> Self {
        let mut config = ValidationConfig::default();
        config = config
            .with_param("max_gap_seconds", serde_json::json!(3600)) // 預設1小時
            .with_param("min_price", serde_json::json!(0.0))
            .with_param("max_price", serde_json::json!(1_000_000.0))
            .with_param("min_volume", serde_json::json!(0.0))
            .with_param("max_volume", serde_json::json!(1e15));

        Self {
            config,
            max_gap_seconds: 3600,
            min_price: 0.0,
            max_price: 1_000_000.0,
            min_volume: 0.0,
            max_volume: 1e15,
        }
    }

    /// 設置最大時間間隔
    pub fn with_max_gap(mut self, seconds: i64) -> Self {
        self.max_gap_seconds = seconds;
        self.config = self
            .config
            .with_param("max_gap_seconds", serde_json::json!(seconds));
        self
    }

    /// 設置價格範圍
    pub fn with_price_range(mut self, min: f64, max: f64) -> Self {
        self.min_price = min;
        self.max_price = max;
        self.config = self
            .config
            .with_param("min_price", serde_json::json!(min))
            .with_param("max_price", serde_json::json!(max));
        self
    }

    /// 設置成交量範圍
    pub fn with_volume_range(mut self, min: f64, max: f64) -> Self {
        self.min_volume = min;
        self.max_volume = max;
        self.config = self
            .config
            .with_param("min_volume", serde_json::json!(min))
            .with_param("max_volume", serde_json::json!(max));
        self
    }

    /// 驗證價格一致性
    fn validate_price_consistency(&self, record: &OhlcvRecord) -> ValidationResult<()> {
        // 檢查 high >= low
        if record.high < record.low {
            return Err(ValidationError::InconsistentValue {
                description: format!("最高價 ({}) 低於最低價 ({})", record.high, record.low),
            });
        }

        // 檢查 high >= open, close
        if record.high < record.open || record.high < record.close {
            return Err(ValidationError::InconsistentValue {
                description: format!(
                    "最高價 ({}) 必須大於等於開盤價 ({}) 和收盤價 ({})",
                    record.high, record.open, record.close
                ),
            });
        }

        // 檢查 low <= open, close
        if record.low > record.open || record.low > record.close {
            return Err(ValidationError::InconsistentValue {
                description: format!(
                    "最低價 ({}) 必須小於等於開盤價 ({}) 和收盤價 ({})",
                    record.low, record.open, record.close
                ),
            });
        }

        Ok(())
    }

    /// 驗證價格範圍
    fn validate_price_range(&self, record: &OhlcvRecord) -> ValidationResult<()> {
        let prices = vec![
            ("開盤價", record.open),
            ("最高價", record.high),
            ("最低價", record.low),
            ("收盤價", record.close),
        ];

        for (name, price) in prices {
            if price < self.min_price || price > self.max_price {
                return Err(ValidationError::OutOfRange {
                    field: name.to_string(),
                    value: price.to_string(),
                    min: self.min_price.to_string(),
                    max: self.max_price.to_string(),
                });
            }

            // 檢查是否為有效數值
            if price.is_nan() || price.is_infinite() {
                return Err(ValidationError::InvalidValue {
                    field: name.to_string(),
                    value: price.to_string(),
                    reason: "無效的數值（NaN 或無限大）".to_string(),
                });
            }
        }

        Ok(())
    }

    /// 驗證成交量
    fn validate_volume(&self, record: &OhlcvRecord) -> ValidationResult<()> {
        if record.volume < self.min_volume || record.volume > self.max_volume {
            return Err(ValidationError::OutOfRange {
                field: "成交量".to_string(),
                value: record.volume.to_string(),
                min: self.min_volume.to_string(),
                max: self.max_volume.to_string(),
            });
        }

        // 檢查是否為有效數值
        if record.volume.is_nan() || record.volume.is_infinite() {
            return Err(ValidationError::InvalidValue {
                field: "成交量".to_string(),
                value: record.volume.to_string(),
                reason: "無效的數值（NaN 或無限大）".to_string(),
            });
        }

        // 檢查負值
        if record.volume < 0.0 {
            return Err(ValidationError::InvalidValue {
                field: "成交量".to_string(),
                value: record.volume.to_string(),
                reason: "成交量不能為負數".to_string(),
            });
        }

        Ok(())
    }

    /// 驗證時間序列
    pub fn validate_time_series(&self, records: &[OhlcvRecord]) -> ValidationResult<()> {
        if records.len() < 2 {
            return Ok(());
        }

        for i in 1..records.len() {
            let prev = &records[i - 1];
            let curr = &records[i];

            // 檢查時間順序
            if curr.timestamp <= prev.timestamp {
                return Err(ValidationError::OutOfOrder {
                    previous: prev.timestamp.to_rfc3339(),
                    current: curr.timestamp.to_rfc3339(),
                });
            }

            // 檢查時間間隔
            let gap = (curr.timestamp - prev.timestamp).num_seconds();
            if gap > self.max_gap_seconds {
                return Err(ValidationError::LargeGap {
                    gap_seconds: gap,
                    max_gap_seconds: self.max_gap_seconds,
                });
            }
        }

        Ok(())
    }
}

impl Default for OhlcvValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for OhlcvValidator {
    type Data = OhlcvRecord;

    fn name(&self) -> &str {
        "OhlcvValidator"
    }

    fn validate_record(&self, record: &Self::Data) -> ValidationResult<()> {
        // 驗證價格一致性
        self.validate_price_consistency(record)?;

        // 驗證價格範圍
        self.validate_price_range(record)?;

        // 驗證成交量
        self.validate_volume(record)?;

        Ok(())
    }

    fn config(&self) -> &ValidationConfig {
        &self.config
    }
}

impl ComposableValidator for OhlcvValidator {}

/// 創建預設的 OHLCV 驗證規則
pub fn create_ohlcv_rules() -> Vec<ValidationRule<OhlcvRecord>> {
    vec![
        // 檢查價格為正數
        ValidationRule::new(
            "positive_prices",
            |record: &OhlcvRecord| {
                record.open > 0.0 && record.high > 0.0 && record.low > 0.0 && record.close > 0.0
            },
            "所有價格必須為正數",
        ),
        // 檢查成交量合理性（非零時價格應有變動）
        ValidationRule::new(
            "volume_price_consistency",
            |record: &OhlcvRecord| {
                if record.volume > 0.0 {
                    // 有成交量時，通常價格會有變動
                    record.high != record.low || record.open != record.close
                } else {
                    true // 無成交量時不檢查
                }
            },
            "有成交量時價格應有變動",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_record(
        timestamp: DateTime<Utc>,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> OhlcvRecord {
        OhlcvRecord {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    #[test]
    fn test_valid_record() {
        let validator = OhlcvValidator::new();
        let record = create_test_record(Utc::now(), 100.0, 105.0, 99.0, 102.0, 1000.0);

        assert!(validator.validate_record(&record).is_ok());
    }

    #[test]
    fn test_invalid_high_low() {
        let validator = OhlcvValidator::new();
        let record = create_test_record(
            Utc::now(),
            100.0,
            99.0, // high < low
            105.0,
            102.0,
            1000.0,
        );

        assert!(validator.validate_record(&record).is_err());
    }

    #[test]
    fn test_negative_volume() {
        let validator = OhlcvValidator::new();
        let record = create_test_record(
            Utc::now(),
            100.0,
            105.0,
            99.0,
            102.0,
            -100.0, // negative volume
        );

        assert!(validator.validate_record(&record).is_err());
    }

    #[test]
    fn test_time_series_validation() {
        let validator = OhlcvValidator::new().with_max_gap(300); // 5 minutes
        let now = Utc::now();

        let records = vec![
            create_test_record(now, 100.0, 105.0, 99.0, 102.0, 1000.0),
            create_test_record(
                now + Duration::minutes(1),
                102.0,
                103.0,
                101.0,
                102.5,
                500.0,
            ),
            create_test_record(
                now + Duration::minutes(10),
                102.5,
                104.0,
                102.0,
                103.0,
                800.0,
            ), // gap too large
        ];

        assert!(validator.validate_time_series(&records).is_err());
    }
}
