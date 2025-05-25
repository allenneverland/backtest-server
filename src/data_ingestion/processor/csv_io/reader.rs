//! CSV 檔案讀取器

use super::error::{CsvError, CsvResult};
use polars::prelude::*;
use std::path::Path;

/// CSV 讀取器配置
#[derive(Debug, Clone)]
pub struct CsvReaderConfig {
    /// 是否有標題行
    pub has_header: bool,
    /// 分隔符
    pub separator: u8,
    /// 要跳過的行數
    pub skip_rows: usize,
    /// 要讀取的行數（None 表示全部）
    pub n_rows: Option<usize>,
    /// 日期時間格式
    pub datetime_format: Option<String>,
    /// 編碼
    pub encoding: CsvEncoding,
    /// 是否推斷模式類型
    pub infer_schema_length: Option<usize>,
}

impl Default for CsvReaderConfig {
    fn default() -> Self {
        Self {
            has_header: true,
            separator: b',',
            skip_rows: 0,
            n_rows: None,
            datetime_format: None,
            encoding: CsvEncoding::Utf8,
            infer_schema_length: Some(1000),
        }
    }
}

/// CSV 編碼類型
#[derive(Debug, Clone, Copy)]
pub enum CsvEncoding {
    Utf8,
    Utf8Lossy,
}

/// CSV 檔案讀取器
pub struct CsvReader {
    config: CsvReaderConfig,
}

impl CsvReader {
    /// 創建新的 CSV 讀取器
    pub fn new(config: CsvReaderConfig) -> Self {
        Self { config }
    }

    /// 使用預設配置創建 CSV 讀取器
    pub fn default() -> Self {
        Self::new(CsvReaderConfig::default())
    }

    /// 設定分隔符
    pub fn with_separator(mut self, separator: u8) -> Self {
        self.config.separator = separator;
        self
    }

    /// 設定是否有標題行
    pub fn with_header(mut self, has_header: bool) -> Self {
        self.config.has_header = has_header;
        self
    }

    /// 設定要跳過的行數
    pub fn with_skip_rows(mut self, skip_rows: usize) -> Self {
        self.config.skip_rows = skip_rows;
        self
    }

    /// 設定要讀取的行數
    pub fn with_n_rows(mut self, n_rows: Option<usize>) -> Self {
        self.config.n_rows = n_rows;
        self
    }

    /// 設定日期時間格式
    pub fn with_datetime_format(mut self, format: String) -> Self {
        self.config.datetime_format = Some(format);
        self
    }

    /// 從檔案路徑讀取 CSV
    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> CsvResult<DataFrame> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(CsvError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("檔案不存在: {}", path.display()),
            )));
        }

        let reader = CsvReadOptions::default()
            .with_has_header(self.config.has_header)
            .with_parse_options(CsvParseOptions::default().with_separator(self.config.separator))
            .with_skip_rows(self.config.skip_rows)
            .with_n_rows(self.config.n_rows)
            .with_infer_schema_length(self.config.infer_schema_length);

        let df = reader
            .try_into_reader_with_file_path(Some(path.into()))?
            .finish()?;

        Ok(df)
    }

    /// 從字節數組讀取 CSV
    pub fn read_bytes(&self, data: &[u8]) -> CsvResult<DataFrame> {
        use std::io::Cursor;

        let cursor = Cursor::new(data);
        let df = CsvReadOptions::default()
            .with_has_header(self.config.has_header)
            .with_parse_options(CsvParseOptions::default().with_separator(self.config.separator))
            .with_skip_rows(self.config.skip_rows)
            .with_n_rows(self.config.n_rows)
            .with_infer_schema_length(self.config.infer_schema_length)
            .into_reader_with_file_handle(cursor)
            .finish()?;

        Ok(df)
    }

    /// 從字串讀取 CSV
    pub fn read_string(&self, data: &str) -> CsvResult<DataFrame> {
        self.read_bytes(data.as_bytes())
    }

    /// 預覽檔案（讀取前 N 行）
    pub fn preview_file<P: AsRef<Path>>(&self, path: P, n_rows: usize) -> CsvResult<DataFrame> {
        let mut reader = self.clone();
        reader.config.n_rows = Some(n_rows);
        reader.read_file(path)
    }
}

impl Clone for CsvReader {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_reader_config_default() {
        let config = CsvReaderConfig::default();
        assert_eq!(config.has_header, true);
        assert_eq!(config.separator, b',');
        assert_eq!(config.skip_rows, 0);
        assert_eq!(config.n_rows, None);
        assert_eq!(config.datetime_format, None);
        assert_eq!(config.infer_schema_length, Some(1000));
    }

    #[test]
    fn test_csv_reader_builder() {
        let reader = CsvReader::default()
            .with_separator(b';')
            .with_header(false)
            .with_skip_rows(2)
            .with_n_rows(Some(100))
            .with_datetime_format("%Y-%m-%d".to_string());

        assert_eq!(reader.config.separator, b';');
        assert_eq!(reader.config.has_header, false);
        assert_eq!(reader.config.skip_rows, 2);
        assert_eq!(reader.config.n_rows, Some(100));
        assert_eq!(reader.config.datetime_format, Some("%Y-%m-%d".to_string()));
    }

    #[test]
    fn test_read_csv_string() {
        let csv_data = r#"time,open,high,low,close,volume
2024-01-01 09:00:00,100.0,105.0,99.0,103.0,1000000
2024-01-01 10:00:00,103.0,108.0,102.0,107.0,1200000
2024-01-01 11:00:00,107.0,110.0,106.0,109.0,1500000"#;

        let reader = CsvReader::default();
        let result = reader.read_string(csv_data);

        assert!(result.is_ok());
        let df = result.unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.width(), 6);
        assert!(df.column("time").is_ok());
        assert!(df.column("open").is_ok());
        assert!(df.column("high").is_ok());
        assert!(df.column("low").is_ok());
        assert!(df.column("close").is_ok());
        assert!(df.column("volume").is_ok());
    }

    #[test]
    fn test_read_csv_with_custom_separator() {
        let csv_data = r#"time;price;volume
2024-01-01 09:00:00;100.0;1000
2024-01-01 09:00:01;100.5;500
2024-01-01 09:00:02;101.0;750"#;

        let reader = CsvReader::default().with_separator(b';');
        let result = reader.read_string(csv_data);

        assert!(result.is_ok());
        let df = result.unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.width(), 3);
    }

    #[test]
    fn test_read_csv_without_header() {
        let csv_data = r#"2024-01-01,100.0,1000
2024-01-02,101.0,1100
2024-01-03,102.0,1200"#;

        let reader = CsvReader::default().with_header(false);
        let result = reader.read_string(csv_data);

        assert!(result.is_ok());
        let df = result.unwrap();
        assert_eq!(df.height(), 3);
        // 沒有標題時，Polars 會自動生成列名
        assert_eq!(df.width(), 3);
    }
}
