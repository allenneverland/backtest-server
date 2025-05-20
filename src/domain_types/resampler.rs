// resampling.rs
use polars::prelude::*;
use super::types::{ColumnName, Frequency};

/// 提供重採樣核心功能的結構
pub struct Resampler;

impl Resampler {
    /// 對 LazyFrame 進行重採樣
    pub fn resample_lazy(lf: LazyFrame, target_frequency: Frequency) -> LazyFrame {
        let window_size = target_frequency.to_duration();
        
        lf.group_by_dynamic(
            col(ColumnName::TIME),
            [col(ColumnName::TIME)],
            DynamicGroupOptions {
                label: Label::Left,
                start_by: StartBy::WindowBound,
                index_column: ColumnName::TIME.into(),
                every: window_size,
                period: window_size,
                offset: Duration::new(0),
                include_boundaries: false,
                closed_window: ClosedWindow::Left,
            }
        )
        .agg([
            col(ColumnName::OPEN).first().alias(ColumnName::OPEN),
            col(ColumnName::HIGH).max().alias(ColumnName::HIGH),
            col(ColumnName::LOW).min().alias(ColumnName::LOW),
            col(ColumnName::CLOSE).last().alias(ColumnName::CLOSE),
            col(ColumnName::VOLUME).sum().alias(ColumnName::VOLUME),
        ])
    }
    
    /// 對 DataFrame 進行重採樣
    pub fn resample_df(df: &DataFrame, target_frequency: Frequency) -> PolarsResult<DataFrame> {
        // 重用惰性版本，然後立即執行
        Self::resample_lazy(df.clone().lazy(), target_frequency).collect()
    }
}