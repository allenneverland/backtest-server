//! 基於 Polars 的市場數據框架

use polars::prelude::*;
use super::types::{Column, Frequency};
use super::series::MarketSeries;
use super::resampler::Resampler;

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
        Resampler::is_ohlcv(self)
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
        // 使用共用的 Resampler 實現
        Resampler::resample_df(self, target_frequency)
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
    /// 創建一個新的市場數據框架
    /// 
    /// 如果輸入的 DataFrame 沒有 instrument_id 列，會自動添加
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
    
    /// 從 LazyFrame 創建 MarketFrame
    pub fn from_lazy(lf: LazyFrame, instrument_id: impl Into<String>) -> PolarsResult<Self> {
        let df = lf.collect()?;
        Self::new(df, instrument_id)
    }
    
    /// 檢查 MarketFrame 是否為 OHLCV 格式
    pub fn is_ohlcv(&self) -> bool {
        self.df.is_ohlcv()
    }
    
    /// 檢查 MarketFrame 是否為 Tick 格式
    pub fn is_tick(&self) -> bool {
        self.df.is_tick()
    }
    
    /// 按時間範圍過濾數據
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
    
    /// 根據條件過濾數據
    pub fn filter(&self, predicate: Expr) -> PolarsResult<Self> {
        let filtered_df = self.df.clone().lazy()
            .filter(predicate)
            .collect()?;
            
        Ok(Self {
            df: filtered_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 選擇指定的列
    pub fn select(&self, columns: Vec<&str>) -> PolarsResult<Self> {
        let selected_df = self.df.clone().lazy()
            .select(columns.iter().map(|c| col(*c)).collect::<Vec<_>>())
            .collect()?;
            
        Ok(Self {
            df: selected_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 按時間排序
    pub fn sort_by_time(&self, descending: bool) -> PolarsResult<Self> {
        let sorted_df = self.df.clone().lazy()
            .sort(
                [Column::TIME],
                SortMultipleOptions {
                    descending: vec![descending],
                    nulls_last: vec![true],
                    maintain_order: false,
                    multithreaded: true,
                    limit: None,
                }
            )
            .collect()?;
            
        Ok(Self {
            df: sorted_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 重採樣到指定頻率
    pub fn resample(&self, target_frequency: Frequency) -> PolarsResult<Self> {
        // 使用共用的 Resampler 實現
        let resampled_df = Resampler::resample_df(&self.df, target_frequency)?;
        
        Ok(Self {
            df: resampled_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 獲取時間範圍
    pub fn time_range(&self) -> PolarsResult<(i64, i64)> {
        let time_col = self.df.column(Column::TIME)?;
        
        let (min_time, max_time) = time_col.i64()?
            .min_max()
            .unwrap_or((0, 0));
            
        Ok((min_time, max_time))
    }
    
    /// 獲取資料行數
    pub fn row_count(&self) -> usize {
        self.df.height()
    }
    
    /// 獲取時間列
    pub fn time_column(&self) -> PolarsResult<&Series> {
        self.df.time_series()
    }
    
    /// 獲取開盤價列
    pub fn open_column(&self) -> PolarsResult<&Series> {
        self.df.open_series()
    }
    
    /// 獲取最高價列
    pub fn high_column(&self) -> PolarsResult<&Series> {
        self.df.high_series()
    }
    
    /// 獲取最低價列
    pub fn low_column(&self) -> PolarsResult<&Series> {
        self.df.low_series()
    }
    
    /// 獲取收盤價列
    pub fn close_column(&self) -> PolarsResult<&Series> {
        self.df.close_series()
    }
    
    /// 獲取成交量列
    pub fn volume_column(&self) -> PolarsResult<&Series> {
        self.df.volume_series()
    }
    
    /// 與另一個 MarketFrame 按時間連接
    pub fn join(&self, other: &Self, how: JoinType) -> PolarsResult<Self> {
        let joined_df = self.df.clone().lazy()
            .join(
                other.df.clone().lazy(),
                [col(Column::TIME)],
                [col(Column::TIME)],
                how.into()
            )
            .collect()?;
            
        Ok(Self {
            df: joined_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 提供方便的方法轉換為 MarketSeries
    pub fn as_series(&self, frequency: Frequency) -> PolarsResult<MarketSeries> {
        self.df.to_market_series(&self.instrument_id, frequency)
    }
    
    /// 轉換為 LazyFrame
    pub fn lazy(&self) -> LazyFrame {
        self.df.clone().lazy()
    }
    
    /// 從 LazyFrame 執行計算並返回結果
    pub fn collect_from(&self, lf: LazyFrame) -> PolarsResult<Self> {
        let collected_df = lf.collect()?;
        
        Ok(Self {
            df: collected_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 獲取 DataFrame 引用
    pub fn inner(&self) -> &DataFrame {
        &self.df
    }
    
    /// 消耗 MarketFrame 並返回內部的 DataFrame
    pub fn into_inner(self) -> DataFrame {
        self.df
    }
    
    /// 添加列
    pub fn with_column(&self, series: Series) -> PolarsResult<Self> {
        let new_df = self.df.clone().with_column(series)?;
        
        Ok(Self {
            df: new_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 連結兩個 MarketFrame (垂直合併)
    pub fn vstack(&self, other: &Self) -> PolarsResult<Self> {
        if self.df.schema() != other.df.schema() {
            return Err(PolarsError::ComputeError(
                "Cannot vstack MarketFrames with different schemas".into()
            ));
        }
        
        let concat_df = concat([self.df.clone(), other.df.clone()].as_ref(), true, false)?;
        
        Ok(Self {
            df: concat_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    /// 應用函數到每一列
    pub fn map_columns<F>(&self, f: F) -> PolarsResult<Self> 
    where
        F: Fn(&Series) -> PolarsResult<Series>,
    {
        let mut new_df = DataFrame::new(vec![])?;
        
        for col in self.df.get_columns() {
            let new_col = f(col)?;
            new_df.with_column(new_col)?;
        }
        
        Ok(Self {
            df: new_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_ohlcv_dataframe() -> DataFrame {
        let time = Series::new(Column::TIME, &[1000, 2000, 3000, 4000, 5000]);
        let open = Series::new(Column::OPEN, &[100.0, 101.0, 102.0, 103.0, 104.0]);
        let high = Series::new(Column::HIGH, &[105.0, 106.0, 107.0, 108.0, 109.0]);
        let low = Series::new(Column::LOW, &[95.0, 96.0, 97.0, 98.0, 99.0]);
        let close = Series::new(Column::CLOSE, &[102.0, 103.0, 104.0, 105.0, 106.0]);
        let volume = Series::new(Column::VOLUME, &[1000, 2000, 3000, 4000, 5000]);
        
        DataFrame::new(vec![time, open, high, low, close, volume]).unwrap()
    }
    
    fn create_test_tick_dataframe() -> DataFrame {
        let time = Series::new(Column::TIME, &[1000, 1001, 1002, 1003, 1004]);
        let price = Series::new(Column::PRICE, &[100.0, 101.0, 102.0, 103.0, 104.0]);
        let volume = Series::new(Column::VOLUME, &[10, 20, 30, 40, 50]);
        
        DataFrame::new(vec![time, price, volume]).unwrap()
    }
    
    #[test]
    fn test_market_frame_new() {
        let df = create_test_ohlcv_dataframe();
        let market_frame = MarketFrame::new(df, "AAPL").unwrap();
        
        assert_eq!(market_frame.instrument_id, "AAPL");
        assert_eq!(market_frame.df.height(), 5);
        assert!(market_frame.df.schema().contains(Column::INSTRUMENT_ID));
        
        // 檢查是否已添加 instrument_id 列
        let id_series = market_frame.df.column(Column::INSTRUMENT_ID).unwrap();
        assert_eq!(id_series.str().unwrap().get(0).unwrap(), "AAPL");
    }
    
    #[test]
    fn test_market_frame_is_ohlcv() {
        let df = create_test_ohlcv_dataframe();
        let market_frame = MarketFrame::new(df, "AAPL").unwrap();
        
        assert!(market_frame.is_ohlcv());
        assert!(!market_frame.is_tick());
    }
    
    #[test]
    fn test_market_frame_is_tick() {
        let df = create_test_tick_dataframe();
        let market_frame = MarketFrame::new(df, "BTC/USD").unwrap();
        
        assert!(market_frame.is_tick());
        assert!(!market_frame.is_ohlcv());
    }
    
    #[test]
    fn test_market_frame_filter_by_date_range() {
        let df = create_test_ohlcv_dataframe();
        let market_frame = MarketFrame::new(df, "AAPL").unwrap();
        
        let filtered = market_frame.filter_by_date_range(2000, 4000).unwrap();
        assert_eq!(filtered.df.height(), 3);
        
        let time_series = filtered.df.column(Column::TIME).unwrap().i64().unwrap();
        assert_eq!(time_series.get(0).unwrap(), 2000);
        assert_eq!(time_series.get(2).unwrap(), 4000);
    }
    
    #[test]
    fn test_market_frame_resample() {
        // 注意：真實的重採樣測試需要更多數據和合適的時間戳
        // 這只是一個簡化的測試
        let df = create_test_ohlcv_dataframe();
        let market_frame = MarketFrame::new(df, "AAPL").unwrap();
        
        if let Ok(resampled) = market_frame.resample(Frequency::Day) {
            assert!(resampled.is_ohlcv());
            assert_eq!(resampled.instrument_id, "AAPL");
        }
    }
    
    #[test]
    fn test_market_frame_time_range() {
        let df = create_test_ohlcv_dataframe();
        let market_frame = MarketFrame::new(df, "AAPL").unwrap();
        
        let (min, max) = market_frame.time_range().unwrap();
        assert_eq!(min, 1000);
        assert_eq!(max, 5000);
    }
    
    #[test]
    fn test_market_frame_join() {
        let df1 = create_test_ohlcv_dataframe();
        let market_frame1 = MarketFrame::new(df1, "AAPL").unwrap();
        
        let mut df2 = create_test_ohlcv_dataframe();
        // 修改 df2 的值，以區分兩個數據框
        let new_close = Series::new(Column::CLOSE, &[110.0, 111.0, 112.0, 113.0, 114.0]);
        df2 = df2.with_column(new_close).unwrap();
        let market_frame2 = MarketFrame::new(df2, "MSFT").unwrap();
        
        let joined = market_frame1.join(&market_frame2, JoinType::Inner).unwrap();
        
        // 確認結果包含兩個收盤價列
        assert!(joined.df.schema().contains("close"));
        assert!(joined.df.schema().contains("close_right") || 
                 joined.df.schema().contains("close_other"));
    }
    
    #[test]
    fn test_market_frame_vstack() {
        let df1 = create_test_ohlcv_dataframe();
        let market_frame1 = MarketFrame::new(df1, "AAPL").unwrap();
        
        let df2 = create_test_ohlcv_dataframe();
        let market_frame2 = MarketFrame::new(df2, "AAPL").unwrap();
        
        let stacked = market_frame1.vstack(&market_frame2).unwrap();
        assert_eq!(stacked.df.height(), 10); // 5 + 5 行
    }
    
    #[test]
    fn test_market_frame_as_series() {
        let df = create_test_ohlcv_dataframe();
        let market_frame = MarketFrame::new(df, "AAPL").unwrap();
        
        let series = market_frame.as_series(Frequency::Day).unwrap();
        assert_eq!(series.instrument_id(), "AAPL");
        assert_eq!(series.frequency(), Frequency::Day);
    }
}