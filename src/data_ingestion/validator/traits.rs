use super::error::{ValidationResult, DataValidationError};

/// 數據驗證器特徵
pub trait DataValidator<T> {
    /// 驗證單個數據項
    fn validate_item(&self, item: &T) -> ValidationResult<()>;
    
    /// 批量驗證多個數據項
    fn validate_batch(&self, items: &[T]) -> ValidationResult<()> {
        for item in items {
            self.validate_item(item)?;
        }
        Ok(())
    }
    
    /// 驗證並返回有效數據（過濾無效數據）
    fn validate_and_filter(&self, items: Vec<T>) -> (Vec<T>, Vec<(T, DataValidationError)>) {
        let mut valid_items = Vec::new();
        let mut invalid_items = Vec::new();
        
        for item in items {
            match self.validate_item(&item) {
                Ok(_) => valid_items.push(item),
                Err(e) => invalid_items.push((item, e)),
            }
        }
        
        (valid_items, invalid_items)
    }
} 