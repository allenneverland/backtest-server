//! 市場時間序列數據

use polars::prelude::*;
use polars::lazy::prelude::*;
use super::types::{Column, Frequency};

/// 市場時間序列
/// 
/// 包裝 LazyFrame 以支持惰性計算和流式處理
#[derive(Debug, Clone)]
pub struct MarketSeries {
    lazy_frame: LazyFrame,
    instrument_id: String,
    frequency: Frequency,
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
        let window_size = target_frequency.to_duration();
        
        let resampled = self.lazy_frame
            .clone()
            .groupby_dynamic(
                [col(Column::TIME)],
                DynamicGroupOptions {
                    label: Column::TIME.to_string(),
                    start_by: StartBy::Start,
                    index_column: Column::TIME.to_string(),
                    every: window_size,
                    period: window_size,
                    offset: None,
                    include_boundaries: false,
                    closed_window: ClosedWindow::Left,
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
    
    pub fn instrument_id(&self) -> &str {
        &self.instrument_id
    }
}