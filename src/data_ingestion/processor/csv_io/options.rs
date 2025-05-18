use std::collections::HashMap;
use crate::data_ingestion::processor::csv_io::{CSVReaderConfig, format::CSVFormatType};
use crate::domain_types::csv_format::CSVFormat;

/// CSV導入選項
#[derive(Debug, Clone)]
pub enum CSVImportOption {
    /// 使用預定義格式
    PredefinedFormat(CSVFormat),
    /// 使用自定義映射
    CustomMapping(HashMap<String, String>),
    /// 直接使用配置對象
    CustomConfig(CSVReaderConfig),
}

impl CSVImportOption {
    /// 獲取對應的CSV閱讀器配置
    pub fn to_reader_config(&self) -> CSVReaderConfig {
        match self {
            CSVImportOption::PredefinedFormat(format) => {
                CSVFormatType::from_format(*format).get_config()
            },
            CSVImportOption::CustomMapping(mapping) => {
                let mut cfg = CSVReaderConfig::default();
                cfg.columns_mapping = mapping.clone();
                
                // 從映射中設置必要的列
                for (src_col, target_field) in mapping {
                    match target_field.as_str() {
                        "timestamp" => cfg.timestamp_column = src_col.clone(),
                        "open" => cfg.open_column = Some(src_col.clone()),
                        "high" => cfg.high_column = Some(src_col.clone()),
                        "low" => cfg.low_column = Some(src_col.clone()),
                        "close" => cfg.close_column = Some(src_col.clone()),
                        "volume" => cfg.volume_column = Some(src_col.clone()),
                        "amount" => cfg.amount_column = Some(src_col.clone()),
                        _ => {} // 其他映射保留在columns_mapping中
                    }
                }
                
                cfg
            },
            CSVImportOption::CustomConfig(config) => config.clone(),
        }
    }
    
    /// 創建台灣股票格式的導入選項
    pub fn tw_stock() -> Self {
        CSVImportOption::PredefinedFormat(CSVFormat::TwStock)
    }
    
    /// 創建台灣期貨格式的導入選項
    pub fn tw_future() -> Self {
        CSVImportOption::PredefinedFormat(CSVFormat::TwFuture)
    }
    
    /// 從映射創建自定義導入選項
    pub fn from_mapping(mapping: HashMap<String, String>) -> Self {
        CSVImportOption::CustomMapping(mapping)
    }
    
    /// 從配置創建自定義導入選項
    pub fn from_config(config: CSVReaderConfig) -> Self {
        CSVImportOption::CustomConfig(config)
    }
} 