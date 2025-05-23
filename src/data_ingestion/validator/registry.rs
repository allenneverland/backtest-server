use super::error::ValidationErrors;
use super::traits::{ValidationConfig, Validator};
use std::collections::HashMap;
use std::sync::Arc;

/// 驗證器類型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidatorType {
    /// OHLCV 數據驗證器
    Ohlcv,
    /// Tick 數據驗證器
    Tick,
    /// 時間序列驗證器
    TimeSeries,
    /// 自定義驗證器
    Custom(String),
}

impl ValidatorType {
    /// 從字串創建驗證器類型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ohlcv" => ValidatorType::Ohlcv,
            "tick" => ValidatorType::Tick,
            "timeseries" | "time_series" => ValidatorType::TimeSeries,
            custom => ValidatorType::Custom(custom.to_string()),
        }
    }
}

/// 驗證器工廠函數類型
pub type ValidatorFactory<T> = Arc<dyn Fn(ValidationConfig) -> Box<dyn Validator<Data = T>> + Send + Sync>;

/// 驗證器註冊表
pub struct ValidatorRegistry<T> {
    validators: HashMap<ValidatorType, ValidatorFactory<T>>,
}

impl<T: 'static> ValidatorRegistry<T> {
    /// 創建新的註冊表
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }

    /// 註冊驗證器工廠
    pub fn register<F>(&mut self, validator_type: ValidatorType, factory: F) -> &mut Self
    where
        F: Fn(ValidationConfig) -> Box<dyn Validator<Data = T>> + Send + Sync + 'static,
    {
        self.validators.insert(validator_type, Arc::new(factory));
        self
    }

    /// 創建驗證器實例
    pub fn create(
        &self,
        validator_type: &ValidatorType,
        config: ValidationConfig,
    ) -> Option<Box<dyn Validator<Data = T>>> {
        self.validators.get(validator_type).map(|factory| factory(config))
    }

    /// 獲取所有註冊的驗證器類型
    pub fn registered_types(&self) -> Vec<ValidatorType> {
        self.validators.keys().cloned().collect()
    }

    /// 檢查是否已註冊特定類型
    pub fn is_registered(&self, validator_type: &ValidatorType) -> bool {
        self.validators.contains_key(validator_type)
    }

    /// 清空註冊表
    pub fn clear(&mut self) {
        self.validators.clear();
    }
}

impl<T: 'static> Default for ValidatorRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// 驗證器鏈
pub struct ValidatorChain<T> {
    validators: Vec<(ValidatorType, Box<dyn Validator<Data = T>>)>,
}

impl<T: Send + Sync + 'static> ValidatorChain<T> {
    /// 創建新的驗證器鏈
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// 添加驗證器
    pub fn add(
        mut self,
        validator_type: ValidatorType,
        validator: Box<dyn Validator<Data = T>>,
    ) -> Self {
        self.validators.push((validator_type, validator));
        self
    }

    /// 從註冊表創建驗證器並添加
    pub fn add_from_registry(
        mut self,
        registry: &ValidatorRegistry<T>,
        validator_type: ValidatorType,
        config: ValidationConfig,
    ) -> Result<Self, String> {
        match registry.create(&validator_type, config) {
            Some(validator) => {
                self.validators.push((validator_type, validator));
                Ok(self)
            }
            None => Err(format!("驗證器類型 {:?} 未註冊", validator_type)),
        }
    }

    /// 執行所有驗證器
    pub fn validate(&self, data: &T) -> Result<(), Vec<(ValidatorType, ValidationErrors)>> {
        let mut all_errors = Vec::new();

        for (validator_type, validator) in &self.validators {
            if !validator.config().enabled {
                continue;
            }

            if let Err(error) = validator.validate_record(data) {
                let mut errors = ValidationErrors::new();
                errors.add(0, error);
                all_errors.push((validator_type.clone(), errors));

                if validator.config().fail_on_error {
                    break;
                }
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }

    /// 批次驗證
    pub fn validate_batch(&self, data: &[T]) -> Result<(), Vec<(ValidatorType, ValidationErrors)>> {
        let mut all_errors = Vec::new();

        for (validator_type, validator) in &self.validators {
            if !validator.config().enabled {
                continue;
            }

            if let Err(errors) = validator.validate_batch(data) {
                all_errors.push((validator_type.clone(), errors));

                if validator.config().fail_on_error {
                    break;
                }
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }

    /// 獲取驗證器數量
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// 檢查是否為空
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }
}

impl<T: Send + Sync + 'static> Default for ValidatorChain<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// 創建預設的 OHLCV 驗證器註冊表
pub fn create_ohlcv_registry() -> ValidatorRegistry<super::ohlcv_validator::OhlcvRecord> {
    use super::ohlcv_validator::OhlcvValidator;
    use super::time_series_validator::TimeSeriesValidator;

    let mut registry = ValidatorRegistry::new();

    // 註冊 OHLCV 驗證器
    registry.register(ValidatorType::Ohlcv, |config| {
        let mut validator = OhlcvValidator::new();
        
        if let Some(max_gap) = config.get_param::<i64>("max_gap_seconds") {
            validator = validator.with_max_gap(max_gap);
        }
        if let Some(min_price) = config.get_param::<f64>("min_price") {
            if let Some(max_price) = config.get_param::<f64>("max_price") {
                validator = validator.with_price_range(min_price, max_price);
            }
        }
        
        Box::new(validator)
    });

    // 註冊時間序列驗證器
    registry.register(ValidatorType::TimeSeries, |config| {
        let mut validator = TimeSeriesValidator::new();
        
        if let Some(allow_duplicates) = config.get_param::<bool>("allow_duplicates") {
            validator = validator.with_allow_duplicates(allow_duplicates);
        }
        if let Some(strict_ascending) = config.get_param::<bool>("strict_ascending") {
            validator = validator.with_strict_ascending(strict_ascending);
        }
        
        Box::new(validator)
    });

    registry
}

/// 創建預設的 Tick 驗證器註冊表
pub fn create_tick_registry() -> ValidatorRegistry<super::tick_validator::TickRecord> {
    use super::tick_validator::TickValidator;
    use super::time_series_validator::TimeSeriesValidator;

    let mut registry = ValidatorRegistry::new();

    // 註冊 Tick 驗證器
    registry.register(ValidatorType::Tick, |config| {
        let mut validator = TickValidator::new();
        
        if let Some(max_gap) = config.get_param::<i64>("max_gap_seconds") {
            validator = validator.with_max_gap(max_gap);
        }
        if let Some(max_spread) = config.get_param::<f64>("max_spread_percent") {
            validator = validator.with_max_spread_percent(max_spread);
        }
        
        Box::new(validator)
    });

    // 註冊時間序列驗證器
    registry.register(ValidatorType::TimeSeries, |config| {
        let mut validator = TimeSeriesValidator::new();
        
        if let Some(allow_duplicates) = config.get_param::<bool>("allow_duplicates") {
            validator = validator.with_allow_duplicates(allow_duplicates);
        }
        
        Box::new(validator)
    });

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ohlcv_validator::OhlcvRecord;
    use chrono::Utc;

    #[test]
    fn test_registry_registration() {
        let mut registry = ValidatorRegistry::<OhlcvRecord>::new();
        
        registry.register(ValidatorType::Custom("test".to_string()), |_config| {
            Box::new(super::super::ohlcv_validator::OhlcvValidator::new())
        });
        
        assert!(registry.is_registered(&ValidatorType::Custom("test".to_string())));
        assert!(!registry.is_registered(&ValidatorType::Ohlcv));
    }

    #[test]
    fn test_validator_chain() {
        let registry = create_ohlcv_registry();
        let chain = ValidatorChain::new()
            .add_from_registry(&registry, ValidatorType::Ohlcv, ValidationConfig::default())
            .unwrap();
        
        assert_eq!(chain.len(), 1);
        
        let record = OhlcvRecord {
            timestamp: Utc::now(),
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 102.0,
            volume: 1000.0,
        };
        
        assert!(chain.validate(&record).is_ok());
    }
}