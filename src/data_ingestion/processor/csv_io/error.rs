//! CSV 處理錯誤定義

use thiserror::Error;

/// CSV 處理錯誤類型
#[derive(Error, Debug)]
pub enum CsvError {
    #[error("檔案讀取錯誤: {0}")]
    IoError(#[from] std::io::Error),

    #[error("CSV 解析錯誤: {0}")]
    ParseError(String),

    #[error("缺少必要欄位: {0}")]
    MissingColumn(String),

    #[error("無效的欄位格式: 欄位 {column}, 值 {value}, 原因: {reason}")]
    InvalidFormat {
        column: String,
        value: String,
        reason: String,
    },

    #[error("時間戳解析錯誤: {0}")]
    TimestampParseError(String),

    #[error("數值解析錯誤: {0}")]
    NumberParseError(String),

    #[error("不支援的檔案格式: {0}")]
    UnsupportedFormat(String),

    #[error("Polars 錯誤: {0}")]
    PolarsError(#[from] polars::error::PolarsError),
}

/// CSV 處理結果類型
pub type CsvResult<T> = Result<T, CsvError>;
