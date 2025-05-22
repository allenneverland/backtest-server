//! 金融時間序列數據

use super::resampler::Resampler;
use super::types::{ColumnName, DataFormat, FrequencyMarker, Frequency};
use crate::utils::time_utils::{
    datetime_range_to_timestamp_range, timestamp_range_to_datetime_range,
};
use chrono::{DateTime, Utc};
use polars::prelude::*;
use std::fmt;
use std::marker::PhantomData;

/// 金融時間序列
///
/// 泛型時間序列結構，支持不同頻率和數據格式的組合
/// F: 頻率標記類型 (Day, Minute, Hour 等)
/// D: 數據格式類型 (OhlcvFormat, TickFormat 等)
#[derive(Clone)]
pub struct FinancialSeries<F: FrequencyMarker, D: DataFormat> {
    lazy_frame: LazyFrame,
    instrument_id: String,
    _frequency: PhantomData<F>,
    _format: PhantomData<D>,
}

// 手動實現 Debug
impl<F: FrequencyMarker, D: DataFormat> fmt::Debug for FinancialSeries<F, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FinancialSeries")
            .field("instrument_id", &self.instrument_id)
            .field("frequency", &F::name())
            .field("format", &D::format_name())
            .field("lazy_frame", &"<LazyFrame>")
            .finish()
    }
}

impl<F: FrequencyMarker, D: DataFormat> FinancialSeries<F, D> {
    /// 創建新的金融時間序列
    pub fn new(df: DataFrame, instrument_id: String) -> PolarsResult<Self> {
        // 驗證 DataFrame 是否符合格式要求
        D::validate_dataframe(&df)?;

        Ok(Self {
            lazy_frame: df.lazy(),
            instrument_id,
            _frequency: PhantomData,
            _format: PhantomData,
        })
    }

    /// 從 LazyFrame 創建 FinancialSeries
    pub fn from_lazy(lazy_frame: LazyFrame, instrument_id: String) -> Self {
        Self {
            lazy_frame,
            instrument_id,
            _frequency: PhantomData,
            _format: PhantomData,
        }
    }

    /// 從 DataFrame 創建，不進行格式驗證（不安全但高效）
    pub fn from_dataframe_unchecked(df: DataFrame, instrument_id: String) -> Self {
        Self {
            lazy_frame: df.lazy(),
            instrument_id,
            _frequency: PhantomData,
            _format: PhantomData,
        }
    }

    /// 執行計算並返回 DataFrame（終止操作）
    pub fn collect(self) -> PolarsResult<DataFrame> {
        self.lazy_frame.collect()
    }

    /// 獲取 LazyFrame 引用
    pub fn lazy_frame(&self) -> &LazyFrame {
        &self.lazy_frame
    }

    /// 獲取頻率
    pub fn frequency(&self) -> Frequency {
        F::to_frequency()
    }

    /// 獲取標的物 ID
    pub fn instrument_id(&self) -> &str {
        &self.instrument_id
    }

    /// 過濾特定時間範圍的數據（使用 i64 時間戳）- 可鏈接
    pub fn filter_date_range(self, start_time: i64, end_time: i64) -> Self {
        let filtered = self.lazy_frame.filter(
            col(ColumnName::TIME)
                .gt_eq(lit(start_time).cast(DataType::Int64))
                .and(col(ColumnName::TIME).lt_eq(lit(end_time).cast(DataType::Int64))),
        );

        Self {
            lazy_frame: filtered,
            instrument_id: self.instrument_id,
            _frequency: PhantomData,
            _format: PhantomData,
        }
    }
    
    /// 過濾特定時間範圍的數據（使用 DateTime<Utc>）- 可鏈接
    pub fn filter_date_range_datetime(
        self,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
    ) -> Self {
        let (start_ts, end_ts) = datetime_range_to_timestamp_range(start_time, end_time);
        self.filter_date_range(start_ts, end_ts)
    }

    /// 選擇特定列 - 可鏈接
    pub fn select_columns(self, columns: &[&str]) -> Self {
        let selected = self
            .lazy_frame
            .select(columns.iter().map(|c| col(*c)).collect::<Vec<_>>());

        Self {
            lazy_frame: selected,
            instrument_id: self.instrument_id,
            _frequency: PhantomData,
            _format: PhantomData,
        }
    }

    /// 對時間列排序 - 可鏈接
    pub fn sort_by_time(self, descending: bool) -> Self {
        let sorted = self.lazy_frame.sort(
            [ColumnName::TIME],
            SortMultipleOptions {
                descending: vec![descending],
                nulls_last: vec![true],
                maintain_order: false,
                multithreaded: true,
                limit: None,
            },
        );

        Self {
            lazy_frame: sorted,
            instrument_id: self.instrument_id,
            _frequency: PhantomData,
            _format: PhantomData,
        }
    }

    /// 獲取數據的時間範圍（i64 時間戳）
    pub fn time_range(&self) -> PolarsResult<(i64, i64)> {
        let df = self.lazy_frame.clone().collect()?;
        let time_col = df.column(ColumnName::TIME)?;

        let (min_time, max_time) = time_col.i64()?.min_max().unwrap_or((0, 0));

        Ok((min_time, max_time))
    }
    
    /// 獲取數據的時間範圍（DateTime<Utc>）
    pub fn time_range_datetime(&self) -> PolarsResult<(DateTime<Utc>, DateTime<Utc>)> {
        let (start_ts, end_ts) = self.time_range()?;
        Ok(timestamp_range_to_datetime_range(start_ts, end_ts))
    }

    /// 重採樣到不同頻率（需要指定目標頻率類型）
    pub fn resample_to<NewF: FrequencyMarker>(self) -> PolarsResult<FinancialSeries<NewF, D>> {
        let resampled = Resampler::resample_lazy(self.lazy_frame, NewF::to_frequency());

        Ok(FinancialSeries {
            lazy_frame: resampled,
            instrument_id: self.instrument_id,
            _frequency: PhantomData,
            _format: PhantomData,
        })
    }
}

// ========== 類型別名 ==========

use super::types::{OhlcvFormat, TickFormat, Day, Minute, Hour, Tick, Second, FiveMinutes, FifteenMinutes, Week, Month};

/// OHLCV 時間序列類型別名
pub type OhlcvSeries<F> = FinancialSeries<F, OhlcvFormat>;

/// Tick 時間序列類型別名
pub type TickSeries<F> = FinancialSeries<F, TickFormat>;

// 常用頻率組合的便利類型別名

/// 日線 OHLCV 數據
pub type DailyOhlcv = OhlcvSeries<Day>;

/// 分鐘線 OHLCV 數據
pub type MinuteOhlcv = OhlcvSeries<Minute>;

/// 小時線 OHLCV 數據
pub type HourlyOhlcv = OhlcvSeries<Hour>;

/// 5分鐘線 OHLCV 數據
pub type FiveMinuteOhlcv = OhlcvSeries<FiveMinutes>;

/// 15分鐘線 OHLCV 數據
pub type FifteenMinuteOhlcv = OhlcvSeries<FifteenMinutes>;

/// 週線 OHLCV 數據
pub type WeeklyOhlcv = OhlcvSeries<Week>;

/// 月線 OHLCV 數據
pub type MonthlyOhlcv = OhlcvSeries<Month>;

/// Tick 數據（最小頻率）
pub type TickData = TickSeries<Tick>;

/// 秒級 Tick 數據
pub type SecondTick = TickSeries<Second>;

/// 分鐘級 Tick 數據
pub type MinuteTick = TickSeries<Minute>;

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;

    fn create_test_ohlcv_dataframe() -> DataFrame {
        let time = Series::new(ColumnName::TIME.into(), &[1000i64, 2000, 3000, 4000, 5000]);
        let open = Series::new(ColumnName::OPEN.into(), &[100.0, 101.0, 102.0, 103.0, 104.0]);
        let high = Series::new(ColumnName::HIGH.into(), &[105.0, 106.0, 107.0, 108.0, 109.0]);
        let low = Series::new(ColumnName::LOW.into(), &[95.0, 96.0, 97.0, 98.0, 99.0]);
        let close = Series::new(ColumnName::CLOSE.into(), &[102.0, 103.0, 104.0, 105.0, 106.0]);
        let volume = Series::new(ColumnName::VOLUME.into(), &[1000i32, 2000, 3000, 4000, 5000]);

        DataFrame::new(vec![
            time.into(),
            open.into(),
            high.into(),
            low.into(),
            close.into(),
            volume.into(),
        ])
        .unwrap()
    }

    fn create_test_tick_dataframe() -> DataFrame {
        let time = Series::new(ColumnName::TIME.into(), &[1000i64, 1001, 1002, 1003, 1004]);
        let price = Series::new(ColumnName::PRICE.into(), &[100.0, 101.0, 102.0, 103.0, 104.0]);
        let volume = Series::new(ColumnName::VOLUME.into(), &[10i32, 20, 30, 40, 50]);

        DataFrame::new(vec![time.into(), price.into(), volume.into()]).unwrap()
    }

    #[test]
    fn test_daily_ohlcv_creation() {
        let df = create_test_ohlcv_dataframe();
        let daily_ohlcv = DailyOhlcv::new(df, "AAPL".to_string()).unwrap();

        assert_eq!(daily_ohlcv.instrument_id(), "AAPL");
        assert_eq!(daily_ohlcv.frequency(), Frequency::Day);
    }

    #[test]
    fn test_tick_data_creation() {
        let df = create_test_tick_dataframe();
        let tick_data = TickData::new(df, "BTC/USD".to_string()).unwrap();

        assert_eq!(tick_data.instrument_id(), "BTC/USD");
        assert_eq!(tick_data.frequency(), Frequency::Tick);
    }

    // TODO: Fix Polars type inference issue in method chaining
    #[test]
    #[ignore]
    fn test_method_chaining() {
        let df = create_test_ohlcv_dataframe();
        let daily_ohlcv = DailyOhlcv::new(df, "AAPL".to_string()).unwrap();
        
        // 測試過濾
        let filtered = daily_ohlcv.filter_date_range(2000i64, 4000i64);
        let filtered_result = filtered.collect().unwrap();
        assert_eq!(filtered_result.height(), 3); // 2000, 3000, 4000
        
        // 測試排序和列選擇
        let df2 = create_test_ohlcv_dataframe();
        let sorted_selected = DailyOhlcv::new(df2, "AAPL".to_string())
            .unwrap()
            .sort_by_time(false)
            .select_columns(&[ColumnName::TIME, ColumnName::CLOSE]);
        
        let result = sorted_selected.collect().unwrap();
        assert_eq!(result.width(), 2);  // time, close columns
    }

    #[test]
    fn test_time_range() {
        let df = create_test_ohlcv_dataframe();
        let daily_ohlcv = DailyOhlcv::new(df, "AAPL".to_string()).unwrap();

        let (start, end) = daily_ohlcv.time_range().unwrap();
        assert_eq!(start, 1000);
        assert_eq!(end, 5000);
    }

    #[test]
    fn test_format_validation() {
        // 測試缺少必要列的情況
        let incomplete_df = DataFrame::new(vec![
            Series::new(ColumnName::TIME.into(), &[1000i64, 2000]).into(),
            Series::new(ColumnName::OPEN.into(), &[100.0, 101.0]).into(),
        ]).unwrap();

        let result = DailyOhlcv::new(incomplete_df, "AAPL".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_type_aliases() {
        let df = create_test_ohlcv_dataframe();
        
        // 測試不同的類型別名
        let minute_ohlcv = MinuteOhlcv::new(df.clone(), "AAPL".to_string()).unwrap();
        let hourly_ohlcv = HourlyOhlcv::new(df.clone(), "AAPL".to_string()).unwrap();
        let five_min_ohlcv = FiveMinuteOhlcv::new(df, "AAPL".to_string()).unwrap();

        assert_eq!(minute_ohlcv.frequency(), Frequency::Minute);
        assert_eq!(hourly_ohlcv.frequency(), Frequency::Hour);
        assert_eq!(five_min_ohlcv.frequency(), Frequency::FiveMinutes);
    }
}
