use super::error::{ValidationError, ValidationErrors, ValidationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 驗證器特徵
pub trait Validator: Send + Sync {
    /// 驗證的數據類型
    type Data;

    /// 驗證器名稱
    fn name(&self) -> &str;

    /// 驗證單筆數據
    fn validate_record(&self, data: &Self::Data) -> ValidationResult<()>;

    /// 批次驗證數據
    fn validate_batch(&self, data: &[Self::Data]) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        for (index, record) in data.iter().enumerate() {
            if let Err(e) = self.validate_record(record) {
                errors.add(index, e);
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    /// 獲取驗證配置
    fn config(&self) -> &ValidationConfig;
}

/// 可組合的驗證器
pub trait ComposableValidator: Validator {
    /// 與其他驗證器組合
    fn and<V>(self, other: V) -> CompositeValidator<Self::Data>
    where
        Self: Sized + 'static,
        V: Validator<Data = Self::Data> + 'static,
    {
        CompositeValidator::new(vec![Box::new(self), Box::new(other)])
    }
}

/// 驗證配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// 是否啟用驗證
    pub enabled: bool,
    /// 是否在錯誤時停止處理
    pub fail_on_error: bool,
    /// 最大錯誤數量（超過則停止）
    pub max_errors: Option<usize>,
    /// 自定義參數
    pub params: HashMap<String, serde_json::Value>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fail_on_error: false,
            max_errors: Some(1000),
            params: HashMap::new(),
        }
    }
}

impl ValidationConfig {
    /// 創建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 設置是否啟用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 設置是否在錯誤時停止
    pub fn with_fail_on_error(mut self, fail_on_error: bool) -> Self {
        self.fail_on_error = fail_on_error;
        self
    }

    /// 設置最大錯誤數
    pub fn with_max_errors(mut self, max_errors: Option<usize>) -> Self {
        self.max_errors = max_errors;
        self
    }

    /// 添加自定義參數
    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// 獲取參數
    pub fn get_param<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.params
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// 組合驗證器
pub struct CompositeValidator<T> {
    validators: Vec<Box<dyn Validator<Data = T>>>,
    config: ValidationConfig,
}

impl<T> CompositeValidator<T> {
    /// 創建新的組合驗證器
    pub fn new(validators: Vec<Box<dyn Validator<Data = T>>>) -> Self {
        Self {
            validators,
            config: ValidationConfig::default(),
        }
    }

    /// 添加驗證器
    pub fn add_validator(mut self, validator: Box<dyn Validator<Data = T>>) -> Self {
        self.validators.push(validator);
        self
    }

    /// 設置配置
    pub fn with_config(mut self, config: ValidationConfig) -> Self {
        self.config = config;
        self
    }
}

impl<T: Send + Sync> Validator for CompositeValidator<T> {
    type Data = T;

    fn name(&self) -> &str {
        "CompositeValidator"
    }

    fn validate_record(&self, data: &Self::Data) -> ValidationResult<()> {
        for validator in &self.validators {
            if validator.config().enabled {
                validator.validate_record(data)?;
            }
        }
        Ok(())
    }

    fn validate_batch(&self, data: &[Self::Data]) -> Result<(), ValidationErrors> {
        let mut all_errors = ValidationErrors::new();

        for validator in &self.validators {
            if !validator.config().enabled {
                continue;
            }

            if let Err(errors) = validator.validate_batch(data) {
                all_errors.merge(errors);

                // 檢查是否超過最大錯誤數
                if let Some(max) = self.config.max_errors {
                    if all_errors.error_count() >= max {
                        break;
                    }
                }
            }
        }

        if all_errors.has_errors() {
            Err(all_errors)
        } else {
            Ok(())
        }
    }

    fn config(&self) -> &ValidationConfig {
        &self.config
    }
}

/// 驗證規則構建器
pub struct ValidationRule<T> {
    name: String,
    predicate: Box<dyn Fn(&T) -> bool + Send + Sync>,
    error_message: String,
}

impl<T> ValidationRule<T> {
    /// 創建新的驗證規則
    pub fn new(
        name: impl Into<String>,
        predicate: impl Fn(&T) -> bool + Send + Sync + 'static,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            predicate: Box::new(predicate),
            error_message: error_message.into(),
        }
    }

    /// 驗證數據
    pub fn validate(&self, data: &T) -> ValidationResult<()> {
        if (self.predicate)(data) {
            Ok(())
        } else {
            Err(ValidationError::CustomRuleFailed {
                rule: self.name.clone(),
                message: self.error_message.clone(),
            })
        }
    }
}
