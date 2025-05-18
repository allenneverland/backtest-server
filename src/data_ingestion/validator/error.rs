use thiserror::Error;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Error)]
pub enum DataValidationError {
    #[error("數據值範圍錯誤: {field} = {value}, {message}")]
    RangeError {
        field: String,
        value: String,
        message: String,
        context: Option<HashMap<String, String>>,
    },
    
    #[error("數據格式錯誤: {field}, {message}")]
    FormatError {
        field: String,
        message: String,
        context: Option<HashMap<String, String>>,
    },
    
    #[error("數據邏輯錯誤: {message}")]
    LogicError {
        message: String,
        context: Option<HashMap<String, String>>,
    },
    
    #[error("缺失必要數據: {field}")]
    MissingData {
        field: String,
        context: Option<HashMap<String, String>>,
    },
    
    #[error("數據一致性錯誤: {message}")]
    ConsistencyError {
        message: String,
        context: Option<HashMap<String, String>>,
    },
    
    #[error("時間序列錯誤: {message}")]
    TimeSeriesError {
        message: String,
        context: Option<HashMap<String, String>>,
    },
    
    #[error("數據重複: {message}")]
    DuplicateDataError {
        message: String,
        timestamp: Option<DateTime<Utc>>,
        context: Option<HashMap<String, String>>,
    },
    
    #[error("系統錯誤: {message}")]
    SystemError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl DataValidationError {
    // 添加上下文信息到錯誤
    pub fn with_context(self, context: HashMap<String, String>) -> Self {
        match self {
            Self::RangeError { field, value, message, .. } => Self::RangeError {
                field,
                value,
                message,
                context: Some(context),
            },
            Self::FormatError { field, message, .. } => Self::FormatError {
                field,
                message,
                context: Some(context),
            },
            Self::LogicError { message, .. } => Self::LogicError {
                message,
                context: Some(context),
            },
            Self::MissingData { field, .. } => Self::MissingData {
                field,
                context: Some(context),
            },
            Self::ConsistencyError { message, .. } => Self::ConsistencyError {
                message,
                context: Some(context),
            },
            Self::TimeSeriesError { message, .. } => Self::TimeSeriesError {
                message,
                context: Some(context),
            },
            Self::DuplicateDataError { message, timestamp, .. } => Self::DuplicateDataError {
                message,
                timestamp,
                context: Some(context),
            },
            Self::SystemError { message, source } => Self::SystemError {
                message,
                source,
            },
        }
    }
    
    // 添加時間戳到重複數據錯誤
    pub fn duplicate_with_timestamp(message: String, timestamp: DateTime<Utc>) -> Self {
        Self::DuplicateDataError {
            message,
            timestamp: Some(timestamp),
            context: None,
        }
    }
    
    // 從其他錯誤轉換
    pub fn from_error<E>(err: E, message: &str) -> Self 
    where 
        E: std::error::Error + Send + Sync + 'static 
    {
        Self::SystemError {
            message: message.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

pub type ValidationResult<T> = Result<T, DataValidationError>; 