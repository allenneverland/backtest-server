//! 市場時間序列數據

use polars::prelude::*;
use super::types::{Column, Frequency};
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
            .field("lazy_frame", &"<LazyFrame>")  // 只顯示一個佔位符
            .finish()
    }
}

impl MarketSeries {
    /// 創建新的時間序列
    pub fn new(df: DataFrame, instrument_id: String, frequency: Frequency) -> PolarsResult<Self> {
        // 驗證數據框是否包含必要的列
        if !df.schema().contains(Column::TIME) {
            return Err(PolarsError::ComputeError(
                "DataFrame must contain 'time' column".into()
            ));
        }
        
        Ok(Self {
            lazy_frame: df.lazy(),
            instrument_id,
            frequency,
        })
    }
    
    /// 從另一個頻率重採樣
    pub fn resample(&self, target_frequency: Frequency) -> PolarsResult<Self> {
        // 使用當前 Polars 版本支持的 groupby_dynamic 參數格式
        let idx = Column::TIME.to_string();
        
        let resampled = self.lazy_frame
            .clone()
            .group_by_dynamic(
                col(&idx),
                [col(&idx)],
                DynamicGroupOptions {
                    label: Label::Left,  //（例如：10:00的K線表示10:00-10:59的數據）
                    index_column: idx.as_str().into(),  // 轉換為 PlSmallStr
                    every: target_frequency.to_duration(),
                    period: target_frequency.to_duration(),
                    offset: Duration::new(0),  // 使用 Duration::new 方法
                    include_boundaries: false,
                    closed_window: ClosedWindow::Left,
                    start_by: StartBy::WindowBound,
                }
            )
            .agg([
                col(Column::OPEN).first().alias(Column::OPEN),
                col(Column::HIGH).max().alias(Column::HIGH),
                col(Column::LOW).min().alias(Column::LOW),
                col(Column::CLOSE).last().alias(Column::CLOSE),
                col(Column::VOLUME).sum().alias(Column::VOLUME),
            ]);
        
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
    
    /// 過濾特定時間範圍的數據
    pub fn filter_date_range(&self, start_time: i64, end_time: i64) -> PolarsResult<Self> {
        // 使用 Polars 的表達式 API 進行時間過濾
        let filtered = self.lazy_frame.clone()
            .filter(
                col(Column::TIME).gt_eq(lit(start_time))
                .and(col(Column::TIME).lt_eq(lit(end_time)))
            );
        
        Ok(Self {
            lazy_frame: filtered,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    /// 選擇特定列並返回新的 LazyFrame
    pub fn select_columns(&self, columns: &[&str]) -> PolarsResult<Self> {
        let selected = self.lazy_frame.clone()
            .select(columns.iter().map(|c| col(*c)).collect::<Vec<_>>());
        
        Ok(Self {
            lazy_frame: selected,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    /// 合併與另一個時間序列（横向合併）
    pub fn join(&self, other: &Self, on: &str, how: JoinType) -> PolarsResult<Self> {
        let joined = self.lazy_frame.clone()
            .join(
                other.lazy_frame.clone(),
                [col(on)],
                [col(on)],
                how.into()
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
        let sorted = self.lazy_frame.clone()
            .sort(
                [Column::TIME],
                SortMultipleOptions {
                    descending: vec![descending],
                    nulls_last: vec![true], // 使用 vec 而不是單個 bool
                    maintain_order: false,
                    multithreaded: true,
                    limit: None,
                }
            );
        
        Ok(Self {
            lazy_frame: sorted,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    /// 獲取數據的時間範圍
    pub fn time_range(&self) -> PolarsResult<(i64, i64)> {
        let df = self.collect()?;
        let time_col = df.column(Column::TIME)?;
        
        // 使用 Series 的 min_max 方法
        let (min_time, max_time) = time_col.i64()?
            .min_max()
            .unwrap_or((0, 0));
        
        Ok((min_time, max_time))
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