// resampling.rs
use polars::prelude::*;
use super::types::{Column, Frequency};

/// 提供重採樣核心功能的結構
pub struct Resampler;

impl Resampler {
    /// 對 LazyFrame 進行重採樣
    pub fn resample_lazy(lf: LazyFrame, target_frequency: Frequency) -> LazyFrame {
        let window_size = target_frequency.to_duration();
        
        lf.group_by_dynamic(
            col(Column::TIME),
            [col(Column::TIME)],
            DynamicGroupOptions {
                label: Label::Left,
                start_by: StartBy::WindowBound,
                index_column: Column::TIME.into(),
                every: window_size,
                period: window_size,
                offset: Duration::new(0),
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
        ])
    }
    
    /// 對 DataFrame 進行重採樣
    pub fn resample_df(df: &DataFrame, target_frequency: Frequency) -> PolarsResult<DataFrame> {
        // 先檢查是否為 OHLCV 格式
        if !Self::is_ohlcv(df) {
            return Err(PolarsError::ComputeError(
                "DataFrame is not in OHLCV format".into()
            ));
        }
        
        // 重用惰性版本，然後立即執行
        Self::resample_lazy(df.clone().lazy(), target_frequency).collect()
    }
    
    /// 檢查是否為 OHLCV 數據框架
    pub fn is_ohlcv(df: &DataFrame) -> bool {
        let required_columns = [
            Column::TIME, 
            Column::OPEN,
            Column::HIGH, 
            Column::LOW, 
            Column::CLOSE, 
            Column::VOLUME
        ];
        
        required_columns.iter().all(|&col| df.schema().contains(col))
    }
}