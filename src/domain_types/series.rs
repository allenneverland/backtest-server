//! 市場時間序列數據

use super::resampler::Resampler;
use super::types::{ColumnName, Frequency};
use crate::utils::time_utils::{
    datetime_range_to_timestamp_range, timestamp_range_to_datetime_range,
};
use chrono::{DateTime, Utc};
use polars::prelude::*;
use std::fmt;

/// 市場時間序列
///
/// 包裝 LazyFrame 以支持惰性計算和流式處理
#[derive(Clone)]
pub struct MarketSeries {
    lazy_frame: LazyFrame,
    instrument_id: String,
    frequency: Frequency,
}

// 手動實現 Debug，不嘗試打印 LazyFrame
impl fmt::Debug for MarketSeries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarketSeries")
            .field("instrument_id", &self.instrument_id)
            .field("frequency", &self.frequency)
            .field("lazy_frame", &"<LazyFrame>") // 只顯示一個佔位符
            .finish()
    }
}

impl MarketSeries {
    /// 創建新的時間序列
    pub fn new(df: DataFrame, instrument_id: String, frequency: Frequency) -> PolarsResult<Self> {
        // 驗證數據框是否包含必要的列
        if !df.schema().contains(ColumnName::TIME) {
            return Err(PolarsError::ComputeError(
                "DataFrame must contain 'time' column".into(),
            ));
        }

        Ok(Self {
            lazy_frame: df.lazy(),
            instrument_id,
            frequency,
        })
    }

    /// 從 LazyFrame 創建 MarketSeries
    pub fn from_lazy(
        lazy_frame: LazyFrame,
        instrument_id: String,
        frequency: Frequency,
    ) -> PolarsResult<Self> {
        // TODO: 驗證 LazyFrame 包含必要的列（如果需要）

        Ok(Self {
            lazy_frame,
            instrument_id,
            frequency,
        })
    }

    /// 從另一個頻率重採樣
    pub fn resample(&self, target_frequency: Frequency) -> PolarsResult<Self> {
        // 使用共用的重採樣邏輯
        let resampled = Resampler::resample_lazy(self.lazy_frame.clone(), target_frequency);

        Ok(Self {
            lazy_frame: resampled,
            instrument_id: self.instrument_id.clone(),
            frequency: target_frequency,
        })
    }

    /// 執行計算並返回 DataFrame
    pub fn collect(&self) -> PolarsResult<DataFrame> {
        self.lazy_frame.clone().collect()
    }

    /// 獲取 LazyFrame 引用
    pub fn lazy_frame(&self) -> &LazyFrame {
        &self.lazy_frame
    }

    /// 獲取時間序列屬性
    pub fn frequency(&self) -> Frequency {
        self.frequency
    }

    /// 獲取標的物 ID
    pub fn instrument_id(&self) -> &str {
        &self.instrument_id
    }

    /// 過濾特定時間範圍的數據（使用 i64 時間戳）
    pub fn filter_date_range(&self, start_time: i64, end_time: i64) -> PolarsResult<Self> {
        // 使用 Polars 的表達式 API 進行時間過濾
        let filtered = self.lazy_frame.clone().filter(
            col(ColumnName::TIME)
                .gt_eq(lit(start_time))
                .and(col(ColumnName::TIME).lt_eq(lit(end_time))),
        );

        Ok(Self {
            lazy_frame: filtered,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    /// 過濾特定時間範圍的數據（使用 DateTime<Utc>）
    /// 
    /// 這個方法提供了一個更友好的接口，接受 DateTime<Utc> 類型的參數，
    /// 並在內部轉換為計算核心層需要的 i64 時間戳。
    pub fn filter_date_range_datetime(
        &self,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
    ) -> PolarsResult<Self> {
        let (start_ts, end_ts) = datetime_range_to_timestamp_range(start_time, end_time);
        self.filter_date_range(start_ts, end_ts)
    }

    /// 選擇特定列並返回新的 LazyFrame
    pub fn select_columns(&self, columns: &[&str]) -> PolarsResult<Self> {
        let selected = self
            .lazy_frame
            .clone()
            .select(columns.iter().map(|c| col(*c)).collect::<Vec<_>>());

        Ok(Self {
            lazy_frame: selected,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }

    /// 合併與另一個時間序列（横向合併）
    pub fn join(&self, other: &Self, on: &str, how: JoinType) -> PolarsResult<Self> {
        let joined = self.lazy_frame.clone().join(
            other.lazy_frame.clone(),
            [col(on)],
            [col(on)],
            how.into(),
        );

        Ok(Self {
            lazy_frame: joined,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }

    /// 對時間列排序
    pub fn sort_by_time(&self, descending: bool) -> PolarsResult<Self> {
        // 使用正確的排序方法参數
        let sorted = self.lazy_frame.clone().sort(
            [ColumnName::TIME],
            SortMultipleOptions {
                descending: vec![descending],
                nulls_last: vec![true], // 使用 vec 而不是單個 bool
                maintain_order: false,
                multithreaded: true,
                limit: None,
            },
        );

        Ok(Self {
            lazy_frame: sorted,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }

    /// 獲取數據的時間範圍（i64 時間戳）
    pub fn time_range(&self) -> PolarsResult<(i64, i64)> {
        let df = self.collect()?;
        let time_col = df.column(ColumnName::TIME)?;

        // 使用 Series 的 min_max 方法
        let (min_time, max_time) = time_col.i64()?.min_max().unwrap_or((0, 0));

        Ok((min_time, max_time))
    }
    
    /// 獲取數據的時間範圍（DateTime<Utc>）
    /// 
    /// 返回時間範圍的開始和結束時間，以 DateTime<Utc> 格式表示
    pub fn time_range_datetime(&self) -> PolarsResult<(DateTime<Utc>, DateTime<Utc>)> {
        let (start_ts, end_ts) = self.time_range()?;
        Ok(timestamp_range_to_datetime_range(start_ts, end_ts))
    }

    /// 轉換為另一種頻率（重採樣）
    pub fn to_frequency(&self, target_frequency: Frequency) -> PolarsResult<Self> {
        // 如果目標頻率與當前頻率相同，直接返回
        if self.frequency == target_frequency {
            return Ok(self.clone());
        }

        self.resample(target_frequency)
    }
}
