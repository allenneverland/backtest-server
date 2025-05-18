use thiserror::Error;
use serde::de::DeserializeOwned;

/// 配置驗證錯誤
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("缺少必要配置項: {0}")]
    MissingField(String),
    
    #[error("無效的配置值: {0}")]
    InvalidValue(String),
    
    #[error("配置範圍錯誤: {field} 的值 {value} 不在範圍 {min}..{max} 內")]
    RangeError {
        field: String,
        value: String,
        min: String,
        max: String,
    },
    
    #[error("依賴錯誤: {dependent} 依賴於 {dependency} 的配置")]
    DependencyError {
        dependent: String,
        dependency: String,
    },
}

/// 配置驗證器trait
pub trait Validator {
    /// 驗證配置
    fn validate(&self) -> Result<(), ValidationError>;
}

/// 驗證配置區段
pub fn validate_config<T>(config: &T) -> Result<(), ValidationError> 
where 
    T: Validator
{
    config.validate()
}

/// 驗證工具函數
pub struct ValidationUtils;

impl ValidationUtils {
    /// 驗證配置值是否在指定範圍內
    pub fn in_range<T>(
        value: T, 
        min: T, 
        max: T, 
        field_name: &str
    ) -> Result<(), ValidationError> 
    where 
        T: PartialOrd + ToString
    {
        if value < min || value > max {
            return Err(ValidationError::RangeError {
                field: field_name.to_string(),
                value: value.to_string(),
                min: min.to_string(),
                max: max.to_string(),
            });
        }
        Ok(())
    }
    
    /// 驗證一個選項是否為某些值中的一個
    pub fn one_of<T>(
        value: &T, 
        options: &[T], 
        field_name: &str
    ) -> Result<(), ValidationError> 
    where 
        T: PartialEq + ToString
    {
        if !options.contains(value) {
            return Err(ValidationError::InvalidValue(format!(
                "{} 的值 {} 不是有效選項: {:?}", 
                field_name, 
                value.to_string(), 
                options.iter().map(ToString::to_string).collect::<Vec<_>>()
            )));
        }
        Ok(())
    }
    
    /// 檢查必要的字串欄位是否有值
    pub fn not_empty(
        value: &str, 
        field_name: &str
    ) -> Result<(), ValidationError> {
        if value.trim().is_empty() {
            return Err(ValidationError::MissingField(field_name.to_string()));
        }
        Ok(())
    }
    
    /// 檢查兩個欄位的依賴關係
    pub fn check_dependency(
        has_dependent: bool,
        has_dependency: bool,
        dependent_name: &str,
        dependency_name: &str
    ) -> Result<(), ValidationError> {
        if has_dependent && !has_dependency {
            return Err(ValidationError::DependencyError {
                dependent: dependent_name.to_string(),
                dependency: dependency_name.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_in_range() {
        // 測試有效範圍
        assert!(ValidationUtils::in_range(5, 1, 10, "test_field").is_ok());
        
        // 測試無效範圍
        let err = ValidationUtils::in_range(15, 1, 10, "test_field").unwrap_err();
        match err {
            ValidationError::RangeError { field, value, min, max } => {
                assert_eq!(field, "test_field");
                assert_eq!(value, "15");
                assert_eq!(min, "1");
                assert_eq!(max, "10");
            },
            _ => panic!("Expected RangeError"),
        }
    }
    
    #[test]
    fn test_one_of() {
        // 測試有效選項
        assert!(ValidationUtils::one_of(&"test", &["test", "sample", "example"], "test_field").is_ok());
        
        // 測試無效選項
        assert!(ValidationUtils::one_of(&"invalid", &["test", "sample", "example"], "test_field").is_err());
    }
    
    #[test]
    fn test_not_empty() {
        // 測試非空字串
        assert!(ValidationUtils::not_empty("test", "test_field").is_ok());
        
        // 測試空字串
        assert!(ValidationUtils::not_empty("", "test_field").is_err());
        assert!(ValidationUtils::not_empty("   ", "test_field").is_err());
    }
    
    #[test]
    fn test_check_dependency() {
        // 正確的依賴關係
        assert!(ValidationUtils::check_dependency(true, true, "dependent", "dependency").is_ok());
        assert!(ValidationUtils::check_dependency(false, true, "dependent", "dependency").is_ok());
        assert!(ValidationUtils::check_dependency(false, false, "dependent", "dependency").is_ok());
        
        // 錯誤的依賴關係
        assert!(ValidationUtils::check_dependency(true, false, "dependent", "dependency").is_err());
    }
} 