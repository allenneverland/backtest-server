use thiserror::Error;

/// 驗證錯誤類型
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    /// 數值範圍錯誤
    #[error("數值超出有效範圍: {field} = {value}, 預期範圍: {min} 到 {max}")]
    OutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },

    /// 數值不一致錯誤
    #[error("數值不一致: {description}")]
    InconsistentValue { description: String },

    /// 缺失必要欄位
    #[error("缺失必要欄位: {field}")]
    MissingField { field: String },

    /// 無效的時間戳記
    #[error("無效的時間戳記: {timestamp}, 原因: {reason}")]
    InvalidTimestamp { timestamp: String, reason: String },

    /// 重複的數據記錄
    #[error("發現重複記錄: 時間戳記 {timestamp}")]
    DuplicateEntry { timestamp: String },

    /// 時間順序錯誤
    #[error("時間順序錯誤: 前一筆 {previous} > 當前 {current}")]
    OutOfOrder { previous: String, current: String },

    /// 數據間隔過大
    #[error("數據間隔過大: {gap_seconds} 秒 (最大允許: {max_gap_seconds} 秒)")]
    LargeGap {
        gap_seconds: i64,
        max_gap_seconds: i64,
    },

    /// 無效的數值
    #[error("無效的數值: {field} = {value}, 原因: {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },

    /// 數據類型錯誤
    #[error("數據類型錯誤: 預期 {expected}, 實際 {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// 批次驗證錯誤
    #[error("批次驗證失敗: 共 {total} 筆記錄, {error_count} 筆錯誤")]
    BatchValidationFailed { total: usize, error_count: usize },

    /// 自定義驗證規則失敗
    #[error("自定義驗證失敗: {rule} - {message}")]
    CustomRuleFailed { rule: String, message: String },
}

/// 驗證結果類型
pub type ValidationResult<T> = Result<T, ValidationError>;

/// 驗證錯誤集合
#[derive(Debug, Default)]
pub struct ValidationErrors {
    errors: Vec<(usize, ValidationError)>, // (行號, 錯誤)
}

impl ValidationErrors {
    /// 創建新的錯誤集合
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加錯誤
    pub fn add(&mut self, line: usize, error: ValidationError) {
        self.errors.push((line, error));
    }

    /// 檢查是否有錯誤
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 獲取錯誤數量
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 獲取所有錯誤
    pub fn errors(&self) -> &[(usize, ValidationError)] {
        &self.errors
    }

    /// 轉換為迭代器
    pub fn iter(&self) -> impl Iterator<Item = &(usize, ValidationError)> {
        self.errors.iter()
    }

    /// 合併其他錯誤集合
    pub fn merge(&mut self, other: ValidationErrors) {
        self.errors.extend(other.errors);
    }
}

impl IntoIterator for ValidationErrors {
    type Item = (usize, ValidationError);
    type IntoIter = std::vec::IntoIter<(usize, ValidationError)>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}
