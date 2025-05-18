use serde::{Deserialize, Serialize};
use std::fmt;

/// CSV數據格式枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum CSVFormat {
    /// 台灣股票格式
    TwStock,
    /// 台灣期貨格式
    TwFuture,
}

impl fmt::Display for CSVFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CSVFormat::TwStock => write!(f, "TwStock"),
            CSVFormat::TwFuture => write!(f, "TwFuture"),
        }
    }
}

impl CSVFormat {
    /// 从字符串解析CSV格式
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TWSTOCK" => Some(CSVFormat::TwStock),
            "TWFUTURE" => Some(CSVFormat::TwFuture),
            _ => None,
        }
    }
} 