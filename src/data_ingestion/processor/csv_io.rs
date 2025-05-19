use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use anyhow::{Context, Result};
use chrono::NaiveDateTime;

// Adjusted import path for domain_types
use crate::domain_types::{
    AssetType, DataType, OHLCVPoint, TimeSeries, TickPoint, TradeType, Frequency
};

// 子模塊聲明保持不變，因為 format.rs 和 options.rs 將位於 ./csv_io/ 下
pub mod format;
pub mod options;

pub use format::CSVFormatType;
pub use options::CSVImportOption;

/// CSV文件讀取器配置
#[derive(Debug, Clone)]
pub struct CSVReaderConfig {
    pub has_header: bool,
    pub delimiter: u8,
    pub date_format: String,
    pub timestamp_column: String,
    pub open_column: Option<String>,
    pub high_column: Option<String>,
    pub low_column: Option<String>,
    pub close_column: Option<String>,
    pub volume_column: Option<String>,
    pub amount_column: Option<String>, // 新增欄位
    pub price_column: Option<String>,  // For Tick data
    pub trade_type_column: Option<String>, // For Tick data
    pub columns_mapping: HashMap<String, String>, // For custom column mapping
    pub timezone: String,
    pub frequency: Option<Frequency>,
}

impl Default for CSVReaderConfig {
    fn default() -> Self {
        Self {
            has_header: true,
            delimiter: b',',
            date_format: "%Y-%m-%d %H:%M:%S".to_string(),
            timestamp_column: "ts".to_string(), // Default timestamp column name
            open_column: Some("Open".to_string()),
            high_column: Some("High".to_string()),
            low_column: Some("Low".to_string()),
            close_column: Some("Close".to_string()),
            volume_column: Some("Volume".to_string()),
            amount_column: Some("Amount".to_string()), // Default amount column name
            price_column: None, // Specific to Tick data, default to None
            trade_type_column: None, // Specific to Tick data, default to None
            columns_mapping: HashMap::new(),
            timezone: "UTC".to_string(),
            frequency: None,
        }
    }
}

impl CSVReaderConfig {
    /// 設置自定義欄位映射 (source_column 是CSV文件中的列名，target_field 是映射的字段名)
    pub fn with_column_mapping(mut self, source_column: &str, target_field: &str) -> Self {
        self.columns_mapping.insert(target_field.to_string(), source_column.to_string());
        self
    }
    
    /// 設置數據頻率
    pub fn with_frequency(mut self, frequency: Frequency) -> Self {
        self.frequency = Some(frequency);
        self
    }
    
    /// 根據目標字段名查找對應的CSV欄位名
    pub fn find_mapped_column(&self, target_field: &str) -> Option<String> {
        self.columns_mapping.get(target_field).cloned()
    }
    
    pub fn with_date_format(mut self, format: &str) -> Self {
        self.date_format = format.to_string();
        self
    }
    
    pub fn with_timezone(mut self, timezone: &str) -> Self {
        self.timezone = timezone.to_string();
        self
    }
}

/// CSV數據導入器
pub struct CSVImporter;

impl CSVImporter {
    /// 從CSV文件導入OHLCV數據
    pub fn import_ohlcv(
        file_path: impl AsRef<Path>,
        symbol: &str,
        asset_type: AssetType,
        config: &CSVReaderConfig,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        let file = File::open(file_path.as_ref()).with_context(|| format!("無法打開CSV文件: {:?}", file_path.as_ref()))?;
        let reader = BufReader::new(file);
        
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(config.has_header)
            .delimiter(config.delimiter)
            .from_reader(reader);
            
        // 根據文件名或配置推斷頻率
        let frequency = Self::infer_frequency_from_path(file_path.as_ref())
            .or_else(|| config.frequency.clone())
            .unwrap_or(Frequency::Minute(1)); // 默認為分鐘頻率
            
        let mut time_series = TimeSeries::new(
            symbol.to_string(),
            asset_type,
            DataType::OHLCV,
            Some(frequency.clone()), // 設置頻率
            config.timezone.clone(),
        );
        
        // 用於設置每個數據點的頻率
        let frequency_str = format!("{:?}", frequency);
        
        let headers = csv_reader.headers()?.clone();
        
        for result in csv_reader.records() {
            let record = result.context("讀取CSV記錄失敗")?;
            
            let timestamp_col_name = config.find_mapped_column("timestamp").unwrap_or_else(|| config.timestamp_column.clone());
            let timestamp_idx = Self::find_column_index_by_headers(&headers, &timestamp_col_name)?;
            let timestamp_str = record.get(timestamp_idx).context("找不到時間戳欄位")?;
            let timestamp = NaiveDateTime::parse_from_str(timestamp_str, &config.date_format)
                .with_context(|| format!("解析時間戳 '{}' 使用格式 '{}' 失敗", timestamp_str, config.date_format))?
                .and_utc(); // Assuming UTC, will need to handle timezone properly later
            
            let open = Self::parse_ohlcv_field(&record, &headers, &config.open_column, "open", 0.0, config)?;
            let high = Self::parse_ohlcv_field(&record, &headers, &config.high_column, "high", 0.0, config)?;
            let low = Self::parse_ohlcv_field(&record, &headers, &config.low_column, "low", 0.0, config)?;
            let close = Self::parse_ohlcv_field(&record, &headers, &config.close_column, "close", 0.0, config)?;
            let volume = Self::parse_ohlcv_field(&record, &headers, &config.volume_column, "volume", 0.0, config)?;
            
            let mut metadata = HashMap::new();
            metadata.insert("frequency".to_string(), frequency_str.clone());
            
            let mut point = OHLCVPoint {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                metadata,
            };
            
            if let Some(amount_col_name_opt) = &config.amount_column {
                 let amount_col_name = config.find_mapped_column("amount").unwrap_or_else(|| amount_col_name_opt.clone());
                if let Ok(amount_val) = Self::parse_optional_field(&record, &headers, &Some(amount_col_name), 0.0) {
                    point.metadata.insert("amount".to_string(), amount_val.to_string());
                }
            }
            
            if config.volume_column.is_none() {
                point.volume = 0.0;
            }
            
            time_series.add_point(point);
        }
        
        Ok(time_series)
    }

    fn parse_ohlcv_field(
        record: &csv::StringRecord,
        headers: &csv::StringRecord,
        config_column: &Option<String>,
        default_field_name: &str,
        default_value: f64,
        config: &CSVReaderConfig,
    ) -> Result<f64> {
        let column_name = config.find_mapped_column(default_field_name)
            .or_else(|| config_column.clone())
            .unwrap_or_else(|| default_field_name.to_string()); // Fallback to common names if not in config
        Self::parse_optional_field(record, headers, &Some(column_name), default_value)
    }
    
    fn parse_optional_field(
        record: &csv::StringRecord,
        headers: &csv::StringRecord,
        column_name_opt: &Option<String>,
        default_value: f64,
    ) -> Result<f64> {
        match column_name_opt {
            Some(column_name) => {
                let idx = Self::find_column_index_by_headers(headers, column_name)?;
                let val_str = record.get(idx).with_context(|| format!("欄位 '{}' 不存在於記錄中", column_name))?;
                val_str.trim().parse::<f64>().with_context(|| format!("解析欄位 '{}' 的值 '{}' 為 f64 失敗", column_name, val_str))
            }
            None => Ok(default_value),
        }
    }
    
    pub fn import_ohlcv_with_option(
        file_path: impl AsRef<Path>,
        symbol: &str,
        asset_type: AssetType,
        option: CSVImportOption,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        let config = option.to_reader_config();
        Self::import_ohlcv(file_path, symbol, asset_type, &config)
    }

    pub fn import_ohlcv_dir(
        dir_path: impl AsRef<Path>,
        asset_type: AssetType,
        config: &CSVReaderConfig,
    ) -> Result<Vec<TimeSeries<OHLCVPoint>>> {
        let mut all_series = Vec::new();
        for entry in std::fs::read_dir(dir_path.as_ref()).with_context(|| format!("無法讀取目錄: {:?}", dir_path.as_ref()))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "csv") {
                let symbol = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                match Self::import_ohlcv(&path, &symbol, asset_type, config) {
                    Ok(ts) => all_series.push(ts),
                    Err(e) => eprintln!("導入文件 {} 失敗: {}", path.display(), e), // Log error and continue
                }
            }
        }
        Ok(all_series)
    }
    
    pub fn import_ohlcv_dir_with_option(
        dir_path: impl AsRef<Path>,
        asset_type: AssetType,
        option: CSVImportOption,
    ) -> Result<Vec<TimeSeries<OHLCVPoint>>> {
        let config = option.to_reader_config();
        Self::import_ohlcv_dir(dir_path, asset_type, &config)
    }
    
    pub fn import_tick(
        file_path: impl AsRef<Path>,
        symbol: &str,
        asset_type: AssetType,
        config: &CSVReaderConfig,
    ) -> Result<TimeSeries<TickPoint>> {
        let file = File::open(file_path.as_ref()).with_context(|| format!("無法打開CSV文件: {:?}", file_path.as_ref()))?;
        let reader = BufReader::new(file);
        
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(config.has_header)
            .delimiter(config.delimiter)
            .from_reader(reader);
            
        let mut time_series = TimeSeries::new(
            symbol.to_string(),
            asset_type,
            DataType::Tick,
            None,
            config.timezone.clone(),
        );
        
        let headers = csv_reader.headers()?.clone();
        
        for result in csv_reader.records() {
            let record = result.context("讀取CSV記錄失敗")?;
            
            let timestamp_col_name = config.find_mapped_column("timestamp").unwrap_or_else(|| config.timestamp_column.clone());
            let timestamp_idx = Self::find_column_index_by_headers(&headers, &timestamp_col_name)?;
            let timestamp_str = record.get(timestamp_idx).context("找不到時間戳欄位")?;
            let timestamp = NaiveDateTime::parse_from_str(timestamp_str, &config.date_format)
                .with_context(|| format!("解析時間戳 '{}' 使用格式 '{}' 失敗", timestamp_str, config.date_format))?
                .and_utc(); 
            
            let price = Self::parse_tick_field(&record, &headers, &config.price_column, "price", 0.0, config)?;
            let volume = Self::parse_tick_field(&record, &headers, &config.volume_column, "volume", 0.0, config)?; // Reuse volume column for tick
            
            // TradeType parsing (example, needs robust implementation)
            let trade_type_str = match &config.trade_type_column {
                Some(col) => {
                    let tt_col_name = config.find_mapped_column("tradetype").unwrap_or_else(|| col.clone());
                    let idx = Self::find_column_index_by_headers(&headers, &tt_col_name)?;
                    record.get(idx).unwrap_or("Unknown").to_lowercase()
                }
                None => "unknown".to_string()
            };
            let trade_type = match trade_type_str.as_str() {
                "buy" | "b" | "1" => TradeType::Buy,
                "sell" | "s" | "2" => TradeType::Sell,
                "neutral" | "n" | "0" => TradeType::Neutral,
                "cross" | "c" | "3" => TradeType::Cross,
                _ => TradeType::Unknown,
            };
            
            // For TickPoint, other fields like bid/ask prices/volumes are set to default 0.0
            // This basic importer doesn't assume их наличие в простом CSV.
            let point = TickPoint {
                timestamp,
                price,
                volume,
                trade_type,
                bid_price_1: 0.0, bid_price_2: 0.0, bid_price_3: 0.0, bid_price_4: 0.0, bid_price_5: 0.0,
                bid_volume_1: 0.0, bid_volume_2: 0.0, bid_volume_3: 0.0, bid_volume_4: 0.0, bid_volume_5: 0.0,
                ask_price_1: 0.0, ask_price_2: 0.0, ask_price_3: 0.0, ask_price_4: 0.0, ask_price_5: 0.0,
                ask_volume_1: 0.0, ask_volume_2: 0.0, ask_volume_3: 0.0, ask_volume_4: 0.0, ask_volume_5: 0.0,
                metadata: HashMap::new(),
            };
            time_series.add_point(point);
        }
        Ok(time_series)
    }

    fn parse_tick_field(
        record: &csv::StringRecord,
        headers: &csv::StringRecord,
        config_column: &Option<String>,
        default_field_name: &str,
        default_value: f64,
        config: &CSVReaderConfig,
    ) -> Result<f64> {
         let column_name = config.find_mapped_column(default_field_name)
            .or_else(|| config_column.clone())
            .unwrap_or_else(|| default_field_name.to_string());
        Self::parse_optional_field(record, headers, &Some(column_name), default_value)
    }

    pub fn import_tick_with_option(
        file_path: impl AsRef<Path>,
        symbol: &str,
        asset_type: AssetType,
        option: CSVImportOption,
    ) -> Result<TimeSeries<TickPoint>> {
        let config = option.to_reader_config();
        Self::import_tick(file_path, symbol, asset_type, &config)
    }

    fn find_column_index_by_headers(headers: &csv::StringRecord, column_name: &str) -> Result<usize> {
        headers.iter().position(|h| h.trim().to_lowercase() == column_name.trim().to_lowercase())
            .with_context(|| format!("在CSV表頭中找不到欄位: '{}'", column_name))
    }

    // 根據文件路徑推斷頻率
    fn infer_frequency_from_path(path: &Path) -> Option<Frequency> {
        let file_name = path.file_name()?.to_str()?;
        
        if file_name.contains("1m") || file_name.contains("minute") || file_name.contains("min") {
            Some(Frequency::Minute(1))
        } else if file_name.contains("5m") {
            Some(Frequency::Minute(5))
        } else if file_name.contains("15m") {
            Some(Frequency::Minute(15))
        } else if file_name.contains("30m") {
            Some(Frequency::Minute(30))
        } else if file_name.contains("1h") || file_name.contains("hour") {
            Some(Frequency::Hour(1))
        } else if file_name.contains("4h") {
            Some(Frequency::Hour(4))
        } else if file_name.contains("1d") || file_name.contains("day") {
            Some(Frequency::Day)
        } else if file_name.contains("1w") || file_name.contains("week") {
            Some(Frequency::Week)
        } else if file_name.contains("1M") || file_name.contains("month") {
            Some(Frequency::Month)
        } else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    // Assuming domain_types is correctly set up in lib.rs or accessible via crate root
    use crate::domain_types::{AssetType, TradeType};
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn create_test_ohlcv_csv(content: &str) -> Result<NamedTempFile> {
        let temp_file = NamedTempFile::new()?;
        write!(temp_file.as_file(), "{}", content)?;
        Ok(temp_file)
    }
    
    #[test]
    fn test_import_ohlcv_simple() -> Result<()> {
        let csv_content = "ts,Open,High,Low,Close,Volume\n\
                           2023-01-01 00:00:00,100.0,105.0,99.0,102.0,1000.0\n\
                           2023-01-01 00:01:00,102.0,103.0,101.0,102.5,1200.0";
        let temp_file = create_test_ohlcv_csv(csv_content)?;
        let config = CSVReaderConfig::default();
        let ts = CSVImporter::import_ohlcv(temp_file.path(), "TEST", AssetType::Stock, &config)?;

        assert_eq!(ts.len(), 2);
        assert_eq!(ts.data[0].open, 100.0);
        assert_eq!(ts.data[1].close, 102.5);
        Ok(())
    }

    #[test]
    fn test_import_ohlcv_with_custom_mapping_and_date_format() -> Result<()> {
        let csv_content = "datetime,o,h,l,c,v\n\
                           01/01/2023 10:00,10,15,9,12,100\n\
                           01/01/2023 10:05,12,18,11,15,150";
        let temp_file = create_test_ohlcv_csv(csv_content)?;
        let config = CSVReaderConfig::default()
            .with_date_format("%d/%m/%Y %H:%M")
            .with_column_mapping("datetime", "timestamp")
            .with_column_mapping("o", "open")
            .with_column_mapping("h", "high")
            .with_column_mapping("l", "low")
            .with_column_mapping("c", "close")
            .with_column_mapping("v", "volume");
            
        let ts = CSVImporter::import_ohlcv(temp_file.path(), "TESTMAP", AssetType::Stock, &config)?;
        assert_eq!(ts.len(), 2);
        assert_eq!(ts.data[0].open, 10.0);
        assert_eq!(ts.data[1].volume, 150.0);
        // Further checks for timestamp parsing would be good here
        Ok(())
    }
    
    #[test]
    fn test_import_tick_simple() -> Result<()> {
        let csv_content = "ts,Price,Volume,Side\n\
                           2023-01-01 00:00:00.100,100.0,10,B\n\
                           2023-01-01 00:00:00.200,100.1,5,S";
        let temp_file = create_test_ohlcv_csv(csv_content)?; // Reusing ohlcv creator for simplicity
        let config = CSVReaderConfig {
            date_format: "%Y-%m-%d %H:%M:%S%.3f".to_string(), // Note: %.3f for milliseconds
            price_column: Some("Price".to_string()),
            trade_type_column: Some("Side".to_string()),
            ..CSVReaderConfig::default()
        };
        let ts = CSVImporter::import_tick(temp_file.path(), "TICKTEST", AssetType::Crypto, &config)?;

        assert_eq!(ts.len(), 2);
        assert_eq!(ts.data[0].price, 100.0);
        assert_eq!(ts.data[0].trade_type, TradeType::Buy);
        assert_eq!(ts.data[1].price, 100.1);
        assert_eq!(ts.data[1].trade_type, TradeType::Sell);
        Ok(())
    }


    #[test]
    fn test_column_not_found_error() {
        let csv_content = "date,val\n2023-01-01,10";
        let temp_file = create_test_ohlcv_csv(csv_content).unwrap();
        let config = CSVReaderConfig {
            timestamp_column: "nonexistent_ts".to_string(), // This column does not exist
            ..CSVReaderConfig::default()
        };
        let result = CSVImporter::import_ohlcv(temp_file.path(), "ERROR", AssetType::Stock, &config);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("在CSV表頭中找不到欄位: 'nonexistent_ts'"));
        }
    }
} 