//! 基於 Polars 的市場數據框架

use polars::prelude::*;
use super::types::{Column, Frequency};
use super::series::MarketSeries;

/// 市場數據框架 trait，定義市場數據框架的基本功能
pub trait MarketFrameExt {
    /// 檢查是否為 OHLCV 數據框架
    fn is_ohlcv(&self) -> bool;
    
    /// 檢查是否為 Tick 數據框架
    fn is_tick(&self) -> bool;
    
    /// 獲取時間列
    fn time_series(&self) -> PolarsResult<&Series>;
    
    /// 獲取開盤價列
    fn open_series(&self) -> PolarsResult<&Series>;
    
    /// 獲取最高價列
    fn high_series(&self) -> PolarsResult<&Series>;
    
    /// 獲取最低價列
    fn low_series(&self) -> PolarsResult<&Series>;
    
    /// 獲取收盤價列
    fn close_series(&self) -> PolarsResult<&Series>;
    
    /// 獲取成交量列
    fn volume_series(&self) -> PolarsResult<&Series>;
    
    /// 轉換為 MarketSeries
    fn to_market_series(&self, instrument_id: &str, frequency: Frequency) -> PolarsResult<MarketSeries>;
    
    /// 基本重採樣功能
    fn resample(&self, frequency: Frequency) -> PolarsResult<DataFrame>;
}

impl MarketFrameExt for DataFrame {
    fn is_ohlcv(&self) -> bool {
        let required_columns = [
            Column::TIME, 
            Column::OPEN,
            Column::HIGH, 
            Column::LOW, 
            Column::CLOSE, 
            Column::VOLUME
        ];
        
        required_columns.iter().all(|&col| self.schema().contains(col))
    }
    
    fn is_tick(&self) -> bool {
        let required_columns = [
            Column::TIME, 
            Column::PRICE, 
            Column::VOLUME
        ];
        
        required_columns.iter().all(|&col| self.schema().contains(col))
    }
    
    fn time_series(&self) -> PolarsResult<&Series> {
        self.column(Column::TIME)
    }
    
    fn open_series(&self) -> PolarsResult<&Series> {
        self.column(Column::OPEN)
    }
    
    fn high_series(&self) -> PolarsResult<&Series> {
        self.column(Column::HIGH)
    }
    
    fn low_series(&self) -> PolarsResult<&Series> {
        self.column(Column::LOW)
    }
    
    fn close_series(&self) -> PolarsResult<&Series> {
        self.column(Column::CLOSE)
    }
    
    fn volume_series(&self) -> PolarsResult<&Series> {
        self.column(Column::VOLUME)
    }
    
    fn to_market_series(&self, instrument_id: &str, frequency: Frequency) -> PolarsResult<MarketSeries> {
        MarketSeries::new(self.clone(), instrument_id.to_string(), frequency)
    }
    
    fn resample(&self, target_frequency: Frequency) -> PolarsResult<DataFrame> {
        if !self.is_ohlcv() {
            return Err(PolarsError::ComputeError(
                "DataFrame is not in OHLCV format".into()
            ));
        }
        
        let window_size = target_frequency.to_duration();
        
        self.lazy()
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
            ])
            .collect()
    }
}

/// 市場數據框架
/// 
/// 基於 Polars DataFrame 的包裝，專為金融市場數據設計
#[derive(Debug, Clone)]
pub struct MarketFrame {
    pub df: DataFrame,
    pub instrument_id: String,
}

impl MarketFrame {
    pub fn new(df: DataFrame, instrument_id: impl Into<String>) -> PolarsResult<Self> {
        let instrument_id = instrument_id.into();
        
        // 如果 DataFrame 沒有 instrument_id 列，添加它
        let df = if !df.schema().contains(Column::INSTRUMENT_ID) {
            let id_series = Series::new(
                Column::INSTRUMENT_ID,
                vec![instrument_id.clone(); df.height()]
            );
            let mut df = df.clone();
            df.with_column(id_series)?
        } else {
            df
        };
        
        Ok(Self { df, instrument_id })
    }
    
    pub fn filter_by_date_range(&self, start_date: i64, end_date: i64) -> PolarsResult<Self> {
        let filtered_df = self.df.clone().lazy()
            .filter(
                col(Column::TIME).gt_eq(lit(start_date)) &
                col(Column::TIME).lt_eq(lit(end_date))
            )
            .collect()?;
        
        Ok(Self {
            df: filtered_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    // 提供方便的方法轉換為 MarketSeries
    pub fn as_series(&self, frequency: Frequency) -> PolarsResult<MarketSeries> {
        self.df.to_market_series(&self.instrument_id, frequency)
    }
    
    // 實現 Deref 使 MarketFrame 可以像 DataFrame 一樣使用
    pub fn inner(&self) -> &DataFrame {
        &self.df
    }
    
    pub fn into_inner(self) -> DataFrame {
        self.df
    }
}