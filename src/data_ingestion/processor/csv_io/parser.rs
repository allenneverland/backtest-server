//! CSV 資料解析器

use super::error::{CsvError, CsvResult};
use crate::domain_types::{
    ColumnName, FinancialSeries, OhlcvSeries, TickSeries,
    FrequencyMarker,
};
use polars::prelude::*;

/// CSV 解析器
pub struct CsvParser;

impl CsvParser {
    /// 解析 OHLCV 資料
    /// 
    /// 將 DataFrame 轉換為指定頻率的 OHLCV 時間序列
    pub fn parse_ohlcv<F: FrequencyMarker>(
        df: DataFrame,
        instrument_id: String,
        time_column: &str,
        time_format: Option<&str>,
    ) -> CsvResult<OhlcvSeries<F>> {
        // 確保必要的欄位存在
        let required_columns = [
            time_column,
            ColumnName::OPEN,
            ColumnName::HIGH,
            ColumnName::LOW,
            ColumnName::CLOSE,
            ColumnName::VOLUME,
        ];
        
        for col in &required_columns {
            if df.column(col).is_err() {
                return Err(CsvError::MissingColumn(col.to_string()));
            }
        }
        
        // 轉換時間欄位為 i64 時間戳（毫秒）
        let df = Self::convert_time_column(df, time_column, time_format)?;
        
        // 重新排列欄位順序並重命名
        let df = df.lazy()
            .select([
                col(time_column).alias(ColumnName::TIME),
                col(ColumnName::OPEN),
                col(ColumnName::HIGH),
                col(ColumnName::LOW),
                col(ColumnName::CLOSE),
                col(ColumnName::VOLUME),
            ])
            .collect()?;
        
        // 確保數值欄位的類型正確
        let df = df.lazy()
            .with_column(col(ColumnName::OPEN).cast(DataType::Float64))
            .with_column(col(ColumnName::HIGH).cast(DataType::Float64))
            .with_column(col(ColumnName::LOW).cast(DataType::Float64))
            .with_column(col(ColumnName::CLOSE).cast(DataType::Float64))
            .with_column(col(ColumnName::VOLUME).cast(DataType::Float64))
            .collect()?;
        
        // 按時間排序
        let df = df.sort([ColumnName::TIME], SortMultipleOptions::default())?;
        
        Ok(FinancialSeries::new(df, instrument_id)?)
    }
    
    /// 解析 Tick 資料
    /// 
    /// 將 DataFrame 轉換為指定頻率的 Tick 時間序列
    pub fn parse_tick<F: FrequencyMarker>(
        df: DataFrame,
        instrument_id: String,
        time_column: &str,
        time_format: Option<&str>,
    ) -> CsvResult<TickSeries<F>> {
        // 確保必要的欄位存在
        let required_columns = [
            time_column,
            ColumnName::PRICE,
            ColumnName::VOLUME,
        ];
        
        for col in &required_columns {
            if df.column(col).is_err() {
                return Err(CsvError::MissingColumn(col.to_string()));
            }
        }
        
        // 轉換時間欄位
        let df = Self::convert_time_column(df, time_column, time_format)?;
        
        // 選擇並重命名欄位
        let mut select_cols = vec![
            col(time_column).alias(ColumnName::TIME),
            col(ColumnName::PRICE),
            col(ColumnName::VOLUME),
        ];
        
        // 檢查並添加可選欄位
        if df.column(ColumnName::BID).is_ok() {
            select_cols.push(col(ColumnName::BID));
        }
        if df.column(ColumnName::ASK).is_ok() {
            select_cols.push(col(ColumnName::ASK));
        }
        if df.column(ColumnName::BID_VOLUME).is_ok() {
            select_cols.push(col(ColumnName::BID_VOLUME));
        }
        if df.column(ColumnName::ASK_VOLUME).is_ok() {
            select_cols.push(col(ColumnName::ASK_VOLUME));
        }
        
        let df = df.lazy().select(&select_cols).collect()?;
        
        // 確保數值類型正確
        let df = df.lazy()
            .with_column(col(ColumnName::PRICE).cast(DataType::Float64))
            .with_column(col(ColumnName::VOLUME).cast(DataType::Float64))
            .collect()?;
        
        // 按時間排序
        let df = df.sort([ColumnName::TIME], SortMultipleOptions::default())?;
        
        Ok(FinancialSeries::new(df, instrument_id)?)
    }
    
    /// 轉換時間欄位為 i64 時間戳（毫秒）
    fn convert_time_column(
        df: DataFrame,
        time_column: &str,
        time_format: Option<&str>,
    ) -> CsvResult<DataFrame> {
        let time_col = df.column(time_column)
            .map_err(|_| CsvError::MissingColumn(time_column.to_string()))?;
        
        // 檢查時間欄位的數據類型
        match time_col.dtype() {
            DataType::Int64 => {
                // 已經是時間戳格式，直接返回
                Ok(df)
            }
            DataType::Datetime(_, _) => {
                // Polars datetime 類型，轉換為毫秒時間戳
                let df = df.lazy()
                    .with_column(
                        col(time_column)
                            .dt()
                            .timestamp(TimeUnit::Milliseconds)
                            .alias(time_column)
                    )
                    .collect()?;
                Ok(df)
            }
            DataType::String => {
                // 字串格式，需要解析
                Self::parse_string_time_column(df, time_column, time_format)
            }
            _ => {
                Err(CsvError::InvalidFormat {
                    column: time_column.to_string(),
                    value: format!("{:?}", time_col.dtype()),
                    reason: "不支援的時間欄位類型".to_string(),
                })
            }
        }
    }
    
    /// 解析字串格式的時間欄位
    fn parse_string_time_column(
        df: DataFrame,
        time_column: &str,
        time_format: Option<&str>,
    ) -> CsvResult<DataFrame> {
        // 預設時間格式
        let formats = if let Some(fmt) = time_format {
            vec![fmt]
        } else {
            vec![
                "%Y-%m-%d %H:%M:%S",
                "%Y-%m-%d %H:%M:%S%.f",
                "%Y-%m-%dT%H:%M:%S",
                "%Y-%m-%dT%H:%M:%S%.f",
                "%Y-%m-%dT%H:%M:%SZ",
                "%Y-%m-%dT%H:%M:%S%.fZ",
                "%Y/%m/%d %H:%M:%S",
                "%Y/%m/%d %H:%M:%S%.f",
            ]
        };
        
        // 嘗試使用不同的格式解析
        for fmt in formats {
            let result = df.clone().lazy()
                .with_column(
                    col(time_column)
                        .cast(DataType::String)
                        .str()
                        .strptime(
                            DataType::Datetime(TimeUnit::Milliseconds, None),
                            StrptimeOptions {
                                format: Some(fmt.into()),
                                strict: false,
                                exact: false,
                                cache: false,
                            },
                            lit("raise"),
                        )
                        .dt()
                        .timestamp(TimeUnit::Milliseconds)
                        .alias(time_column)
                )
                .collect();
            
            if let Ok(parsed_df) = result {
                return Ok(parsed_df);
            }
        }
        
        Err(CsvError::TimestampParseError(
            format!("無法解析時間欄位 '{}' 的格式", time_column)
        ))
    }
    
    /// 自動檢測 CSV 格式類型（OHLCV 或 Tick）
    pub fn detect_format(df: &DataFrame) -> CsvResult<CsvFormat> {
        let columns = df.get_column_names();
        
        // 檢查 OHLCV 格式
        let ohlcv_columns = ["open", "high", "low", "close", "volume"];
        let has_ohlcv = ohlcv_columns.iter().all(|&col| 
            columns.iter().any(|&c| c.to_lowercase() == col)
        );
        
        if has_ohlcv {
            return Ok(CsvFormat::Ohlcv);
        }
        
        // 檢查 Tick 格式
        let tick_columns = ["price", "volume"];
        let has_tick = tick_columns.iter().all(|&col| 
            columns.iter().any(|&c| c.to_lowercase() == col)
        );
        
        if has_tick {
            return Ok(CsvFormat::Tick);
        }
        
        Err(CsvError::UnsupportedFormat(
            "無法自動檢測 CSV 格式，請確保包含 OHLCV 或 Tick 必要欄位".to_string()
        ))
    }
}

/// CSV 格式類型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvFormat {
    Ohlcv,
    Tick,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_types::{Minute, Day};
    
    fn create_test_ohlcv_df() -> DataFrame {
        df![
            "time" => ["2024-01-01 09:00:00", "2024-01-01 10:00:00", "2024-01-01 11:00:00"],
            "open" => [100.0, 103.0, 107.0],
            "high" => [105.0, 108.0, 110.0],
            "low" => [99.0, 102.0, 106.0],
            "close" => [103.0, 107.0, 109.0],
            "volume" => [1000000.0, 1200000.0, 1500000.0],
        ].unwrap()
    }
    
    fn create_test_tick_df() -> DataFrame {
        df![
            "time" => ["2024-01-01 09:00:00", "2024-01-01 09:00:01", "2024-01-01 09:00:02"],
            "price" => [100.0, 100.5, 101.0],
            "volume" => [1000.0, 500.0, 750.0],
            "bid" => [99.9, 100.4, 100.9],
            "ask" => [100.1, 100.6, 101.1],
        ].unwrap()
    }
    
    #[test]
    fn test_detect_ohlcv_format() {
        let df = create_test_ohlcv_df();
        let format = CsvParser::detect_format(&df);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), CsvFormat::Ohlcv);
    }
    
    #[test]
    fn test_detect_tick_format() {
        let df = create_test_tick_df();
        let format = CsvParser::detect_format(&df);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), CsvFormat::Tick);
    }
    
    #[test]
    fn test_detect_invalid_format() {
        let df = df![
            "time" => ["2024-01-01"],
            "value" => [100.0],
        ].unwrap();
        
        let format = CsvParser::detect_format(&df);
        assert!(format.is_err());
    }
    
    #[test]
    fn test_parse_ohlcv() {
        let df = create_test_ohlcv_df();
        let result = CsvParser::parse_ohlcv::<Minute>(
            df, 
            "TEST".to_string(),
            "time",
            Some("%Y-%m-%d %H:%M:%S")
        );
        
        assert!(result.is_ok());
        let series = result.unwrap();
        assert_eq!(series.instrument_id(), "TEST");
        assert_eq!(series.frequency(), crate::domain_types::Frequency::Minute);
        
        // 檢查數據
        let collected = series.collect().unwrap();
        assert_eq!(collected.height(), 3);
        assert!(collected.column(ColumnName::TIME).is_ok());
        assert!(collected.column(ColumnName::OPEN).is_ok());
        assert!(collected.column(ColumnName::HIGH).is_ok());
        assert!(collected.column(ColumnName::LOW).is_ok());
        assert!(collected.column(ColumnName::CLOSE).is_ok());
        assert!(collected.column(ColumnName::VOLUME).is_ok());
    }
    
    #[test]
    fn test_parse_tick() {
        let df = create_test_tick_df();
        let result = CsvParser::parse_tick::<crate::domain_types::Second>(
            df,
            "TEST".to_string(),
            "time",
            Some("%Y-%m-%d %H:%M:%S")
        );
        
        assert!(result.is_ok());
        let series = result.unwrap();
        assert_eq!(series.instrument_id(), "TEST");
        assert_eq!(series.frequency(), crate::domain_types::Frequency::Second);
        
        // 檢查數據
        let collected = series.collect().unwrap();
        assert_eq!(collected.height(), 3);
        assert!(collected.column(ColumnName::TIME).is_ok());
        assert!(collected.column(ColumnName::PRICE).is_ok());
        assert!(collected.column(ColumnName::VOLUME).is_ok());
        assert!(collected.column(ColumnName::BID).is_ok());
        assert!(collected.column(ColumnName::ASK).is_ok());
    }
    
    #[test]
    fn test_parse_missing_required_column() {
        let df = df![
            "time" => ["2024-01-01"],
            "open" => [100.0],
            "high" => [105.0],
            // 缺少 low, close, volume
        ].unwrap();
        
        let result = CsvParser::parse_ohlcv::<Day>(
            df,
            "TEST".to_string(),
            "time",
            None
        );
        
        assert!(result.is_err());
        match result.unwrap_err() {
            CsvError::MissingColumn(col) => {
                assert!(["low", "close", "volume"].contains(&col.as_str()));
            }
            _ => panic!("Expected MissingColumn error"),
        }
    }
    
    #[test]
    fn test_time_column_conversion() {
        // 測試不同的時間格式
        let test_cases = vec![
            ("%Y-%m-%d %H:%M:%S", "2024-01-01 09:00:00"),
            ("%Y-%m-%dT%H:%M:%S", "2024-01-01T09:00:00"),
            ("%Y/%m/%d %H:%M:%S", "2024/01/01 09:00:00"),
        ];
        
        for (format, time_str) in test_cases {
            let df = df![
                "time" => [time_str],
                "price" => [100.0],
                "volume" => [1000.0],
            ].unwrap();
            
            let result = CsvParser::parse_tick::<crate::domain_types::Second>(
                df,
                "TEST".to_string(),
                "time",
                Some(format)
            );
            
            assert!(result.is_ok(), "Failed to parse time format: {}", format);
        }
    }
}