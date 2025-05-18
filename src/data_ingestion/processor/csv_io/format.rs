use super::CSVReaderConfig;
use crate::domain_types::csv_format::CSVFormat;

/// 預定義的CSV格式類型
#[derive(Debug, Clone)]
pub enum CSVFormatType {
    /// 預定義格式
    Predefined(CSVFormat),
    /// 自定義格式
    Custom(CSVReaderConfig),
}

impl CSVFormatType {
    /// 獲取對應的配置
    pub fn get_config(&self) -> CSVReaderConfig {
        match self {
            CSVFormatType::Predefined(format) => match format {
                CSVFormat::TwStock => {
                    CSVReaderConfig {
                        has_header: true,
                        date_format: "%Y-%m-%d %H:%M:%S".to_string(),
                        timestamp_column: "ts".to_string(),
                        open_column: Some("Open".to_string()),
                        high_column: Some("High".to_string()),
                        low_column: Some("Low".to_string()),
                        close_column: Some("Close".to_string()),
                        volume_column: Some("Volume".to_string()),
                        amount_column: Some("Amount".to_string()),
                        timezone: "Asia/Taipei".to_string(),
                        ..Default::default()
                    }
                },
                CSVFormat::TwFuture => {
                    CSVReaderConfig {
                        has_header: true,
                        date_format: "%Y-%m-%d %H:%M:%S".to_string(),
                        timestamp_column: "ts".to_string(),
                        open_column: Some("Open".to_string()),
                        high_column: Some("High".to_string()),
                        low_column: Some("Low".to_string()),
                        close_column: Some("Close".to_string()),
                        volume_column: Some("Volume".to_string()),
                        amount_column: Some("Amount".to_string()),
                        timezone: "Asia/Taipei".to_string(),
                        ..Default::default()
                    }
                },
            },
            CSVFormatType::Custom(config) => config.clone(),
        }
    }
    
    /// 從CSVFormat創建CSVFormatType
    pub fn from_format(format: CSVFormat) -> Self {
        CSVFormatType::Predefined(format)
    }
} 