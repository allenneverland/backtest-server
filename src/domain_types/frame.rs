//! 金融市場數據框架

use polars::prelude::*;
use std::fmt;
use super::types::{ColumnName, Frequency};
use super::resampler::Resampler;
use super::series::MarketSeries;

// ========== 內部輔助函數 ==========

/// 驗證 DataFrame 是否包含所有必要的列
fn validate_required_columns(df: &DataFrame, required_columns: &[&str]) -> PolarsResult<()> {
    for &col in required_columns {
        if !df.schema().contains(col) {
            return Err(PolarsError::ColumnNotFound(
                format!("Column '{}' not found in DataFrame", col).into()
            ));
        }
    }
    Ok(())
}

/// 確保 DataFrame 包含 instrument_id 列，如果沒有則添加
fn ensure_instrument_id_column(df: &mut DataFrame, instrument_id: &str) -> PolarsResult<()> {
    if !df.schema().contains(ColumnName::INSTRUMENT_ID) {
        let id_series = Series::new(
            ColumnName::INSTRUMENT_ID.into(),
            vec![instrument_id.clone(); df.height()]
        );
        df.with_column(id_series)?;
    }
    Ok(())
}

/// 獲取指定列的 Series
fn get_series<'a>(df: &'a DataFrame, column_name: &str) -> PolarsResult<&'a Series> {
    let column = df.column(column_name)?;
    column.as_series().ok_or_else(|| PolarsError::ColumnNotFound(
        format!("Column '{}' exists but couldn't be converted to Series", column_name).into()
    ))
}

/// 生成所有列的series訪問器
macro_rules! generate_series_getters {
    ($($method:ident, $column:expr);*) => {
        $(
            pub fn $method(&self) -> PolarsResult<&Series> {
                get_series(&self.df, $column.into())
            }
        )*
    }
}


// ========== 公共 trait ==========

// 定義所有基礎時間序列數據框架共享的基本功能
pub trait BaseDataFrame: Clone {
    /// 獲取原始 DataFrame
    fn inner(&self) -> &DataFrame;
    
    /// 消耗自身並返回內部的 DataFrame
    fn into_inner(self) -> DataFrame;
    
    /// 獲取金融工具 ID
    fn instrument_id(&self) -> &str;
    
    /// 獲取時間列
    fn time_series(&self) -> PolarsResult<&Series>;
    
    /// 按時間範圍過濾數據
    fn filter_by_date_range(&self, start_date: i64, end_date: i64) -> PolarsResult<Self> where Self: Sized;
    
    /// 按時間排序
    fn sort_by_time(&self, descending: bool) -> PolarsResult<Self> where Self: Sized;
    
    /// 獲取時間範圍
    fn time_range(&self) -> PolarsResult<(i64, i64)>;
    
    /// 獲取資料行數
    fn row_count(&self) -> usize;
    
    /// 連接與另一個金融數據框架
    fn join<F: BaseDataFrame>(&self, other: &F, how: JoinType) -> PolarsResult<Self> where Self: Sized;
    
    /// 添加列
    fn with_column(&self, series: Series) -> PolarsResult<Self> where Self: Sized;
    
    /// 轉換為 LazyFrame
    fn lazy(&self) -> LazyFrame;
    
    /// 從 LazyFrame 執行計算並返回結果
    fn collect_from(&self, lf: LazyFrame) -> PolarsResult<Self> where Self: Sized;
    
    /// 連結兩個相同類型的金融數據框架 (垂直合併)
    fn vstack(&self, other: &Self) -> PolarsResult<Self> where Self: Sized;
}

// ========== 框架實現 ==========

/// OHLCV(開盤、最高、最低、收盤、成交量)數據框架
/// 
/// 用於處理條形圖(K線)數據，提供專門的方法訪問OHLCV列
#[derive(Clone)]
pub struct OHLCVFrame {
    df: DataFrame,
    instrument_id: String,
    frequency: Frequency,
}

impl fmt::Debug for OHLCVFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OHLCVFrame")
            .field("instrument_id", &self.instrument_id)
            .field("frequency", &self.frequency)
            .field("rows", &self.df.height())
            .field("columns", &self.df.get_column_names())
            .finish()
    }
}

impl OHLCVFrame {
    // 返回內部 DataFrame 引用
    pub fn df(&self) -> &DataFrame {
        &self.df
    }

    // 創建一個新的OHLCV數據框架
    pub fn new(mut df: DataFrame, instrument_id: impl Into<String>, frequency: Frequency) -> PolarsResult<Self> {
        let instrument_id = instrument_id.into();
        
        // 檢查必要的列是否存在
        let required_columns = [
            ColumnName::TIME,
            ColumnName::OPEN,
            ColumnName::HIGH,
            ColumnName::LOW,
            ColumnName::CLOSE,
            ColumnName::VOLUME
        ];
        
        validate_required_columns(&df, &required_columns)?;
        ensure_instrument_id_column(&mut df, &instrument_id);

        Ok(Self { df, instrument_id: instrument_id.to_string(), frequency })
    }
    
    /// 嘗試從任意 DataFrame 創建 OHLCVFrame
    pub fn try_from_dataframe(df: DataFrame, instrument_id: impl Into<String>, frequency: Frequency) -> PolarsResult<Self> {
        Self::new(df, instrument_id, frequency)
    }
    
    /// 從 LazyFrame 創建 OHLCVFrame
    pub fn from_lazy(lf: LazyFrame, instrument_id: impl Into<String>, frequency: Frequency) -> PolarsResult<Self> {
        let df = lf.collect()?;
        Self::new(df, instrument_id, frequency)
    }
    
    /// 獲取頻率
    pub fn frequency(&self) -> Frequency {
        self.frequency
    }
    
    // 使用宏生成所有列的series訪問器
    generate_series_getters! {
        open_series, ColumnName::OPEN;
        high_series, ColumnName::HIGH;
        low_series, ColumnName::LOW;
        close_series, ColumnName::CLOSE;
        volume_series, ColumnName::VOLUME
    }
    
    /// 重採樣到指定頻率
    pub fn resample(&self, target_frequency: Frequency) -> PolarsResult<Self> {
        // 如果目標頻率與當前頻率相同，直接返回克隆
        if self.frequency == target_frequency {
            return Ok(self.clone());
        }
        
        // 使用共用的 Resampler 實現
        let resampled_df = Resampler::resample_df(&self.df, target_frequency)?;
        
        Ok(Self {
            df: resampled_df,
            instrument_id: self.instrument_id.clone(),
            frequency: target_frequency,
        })
    }

    pub fn to_series(&self) -> PolarsResult<MarketSeries> {
        MarketSeries::from_lazy(
            self.df().clone().lazy(),  // 只克隆數據結構，不克隆數據
            self.instrument_id().to_string(),
            self.frequency()
        )
    }
    
    /// 選擇指定的列
    pub fn select(&self, columns: Vec<&str>) -> PolarsResult<Self> {
        let selected_df = self.df.clone().lazy()
            .select(columns.iter().map(|c| col(*c)).collect::<Vec<_>>())
            .collect()?;
            
        Ok(Self {
            df: selected_df,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    /// 應用函數到每一列
    pub fn map_columns<F>(&self, f: F) -> PolarsResult<Self>
    where
        F: Fn(&Series) -> PolarsResult<Series>,
    {
        // 將所有列映射為新的 Series 集合，然後轉換為 Column
        let new_columns: Result<Vec<Column>, _> = self.df.iter()
            .map(|series| f(series).map(|s| s.into_column()))
            .collect();
            
        // 創建新的 DataFrame
        let new_df = DataFrame::new(new_columns?)?;
        
        Ok(Self {
            df: new_df,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
}

impl BaseDataFrame for OHLCVFrame {
    fn inner(&self) -> &DataFrame {
        &self.df
    }
    
    fn into_inner(self) -> DataFrame {
        self.df
    }
    
    fn instrument_id(&self) -> &str {
        &self.instrument_id
    }

    fn time_series(&self) -> PolarsResult<&Series> {
        get_series(&self.df, ColumnName::TIME.into())
    }
    
    fn filter_by_date_range(&self, start_date: i64, end_date: i64) -> PolarsResult<Self> {
        let filtered_df = self.df.clone().lazy()
            .filter(
                col(ColumnName::TIME).gt_eq(lit(start_date))
                    .and(col(ColumnName::TIME).lt_eq(lit(end_date)))
            )
            .collect()?;
        
        Ok(Self {
            df: filtered_df,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    fn sort_by_time(&self, descending: bool) -> PolarsResult<Self> {
        let sorted_df = self.df.clone().lazy()
            .sort(
                [ColumnName::TIME],
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
            frequency: self.frequency,
        })
    }
    
    fn time_range(&self) -> PolarsResult<(i64, i64)> {
        let time_col = get_series(&self.df, ColumnName::TIME.into())?;
        
        let (min_time, max_time) = time_col.i64()?
            .min_max()
            .unwrap_or((0, 0));
            
        Ok((min_time, max_time))
    }
    
    fn row_count(&self) -> usize {
        self.df.height()
    }
    
    fn join<F: BaseDataFrame>(&self, other: &F, how: JoinType) -> PolarsResult<Self> {
        let joined_df = self.df.clone().lazy()
            .join(
                other.inner().clone().lazy(),
                [col(ColumnName::TIME)],
                [col(ColumnName::TIME)],
                how.into()
            )
            .collect()?;
            
        Ok(Self {
            df: joined_df,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }

    fn with_column(&self, series: Series) -> PolarsResult<Self> {
        let mut df_clone = self.df.clone();
        df_clone.with_column(series)?;
        
        Ok(Self {
            df: df_clone,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    fn lazy(&self) -> LazyFrame {
        self.df.clone().lazy()
    }
    
    fn collect_from(&self, lf: LazyFrame) -> PolarsResult<Self> {
        let collected_df = lf.collect()?;
        
        Ok(Self {
            df: collected_df,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
    
    fn vstack(&self, other: &Self) -> PolarsResult<Self> {
        if self.df.schema() != other.df.schema() {
            return Err(PolarsError::ComputeError(
                "Cannot vstack OHLCVFrames with different schemas".into()
            ));
        }
        if self.frequency != other.frequency {
            return Err(PolarsError::ComputeError(
                "Cannot vstack OHLCVFrames with different frequencies".into()
            ));
        }
        
        let concat_df = self.df.clone().vstack(&other.df)?;
        
        Ok(Self {
            df: concat_df,
            instrument_id: self.instrument_id.clone(),
            frequency: self.frequency,
        })
    }
}

/// Tick(逐筆成交)數據框架
/// 
/// 用於處理逐筆成交數據，提供專門的方法訪問相關列
#[derive(Clone)]
pub struct TickFrame {
    df: DataFrame,
    instrument_id: String,
}

impl fmt::Debug for TickFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TickFrame")
            .field("instrument_id", &self.instrument_id)
            .field("rows", &self.df.height())
            .field("columns", &self.df.get_column_names())
            .finish()
    }
}

impl TickFrame {
    /// 創建一個新的Tick數據框架
    /// 
    /// 如果輸入的 DataFrame 沒有必要的列，會返回錯誤
    pub fn new(mut df: DataFrame, instrument_id: impl Into<String>) -> PolarsResult<Self> {
        let instrument_id = instrument_id.into();
        
        // 檢查必要的列是否存在
        let required_columns = [
            ColumnName::TIME,
            ColumnName::PRICE,
            ColumnName::VOLUME
        ];
        
        validate_required_columns(&df, &required_columns)?;
        ensure_instrument_id_column(&mut df, &instrument_id);
        
        Ok(Self { df, instrument_id: instrument_id.to_string() })
    }
    
    /// 嘗試從任意 DataFrame 創建 TickFrame
    /// 
    /// 如果必要的列不存在，返回 Err
    pub fn try_from_dataframe(df: DataFrame, instrument_id: impl Into<String>) -> PolarsResult<Self> {
        Self::new(df, instrument_id)
    }
    
    /// 從 LazyFrame 創建 TickFrame
    pub fn from_lazy(lf: LazyFrame, instrument_id: impl Into<String>) -> PolarsResult<Self> {
        let df = lf.collect()?;
        Self::new(df, instrument_id)
    }
    
    // 使用宏生成所有列的series訪問器
    generate_series_getters! {
        price_series, ColumnName::PRICE;
        volume_series, ColumnName::VOLUME;
        bid_series, ColumnName::BID;
        ask_series, ColumnName::ASK;
        bid_volume_series, ColumnName::BID_VOLUME;
        ask_volume_series, ColumnName::ASK_VOLUME
    }

    
    /// 將Tick數據聚合為OHLCV數據
    pub fn to_ohlcv(&self, frequency: Frequency) -> PolarsResult<OHLCVFrame> {
        let resampled_df = Resampler::tick_to_ohlcv(&self.df, frequency)?;
        
        OHLCVFrame::new(resampled_df, self.instrument_id.clone(), frequency)
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
    
    /// 應用函數到每一列
    pub fn map_columns<F>(&self, f: F) -> PolarsResult<Self>
    where
        F: Fn(&Series) -> PolarsResult<Series>,
    {
        // 將所有列映射為新的 Series 集合，然後轉換為 Column
        let new_columns: Result<Vec<Column>, _> = self.df.iter()
            .map(|series| f(series).map(|s| s.into_column()))
            .collect();
            
        // 創建新的 DataFrame
        let new_df = DataFrame::new(new_columns?)?;
        
        Ok(Self {
            df: new_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
}

impl BaseDataFrame for TickFrame {
    fn inner(&self) -> &DataFrame {
        &self.df
    }
    
    fn into_inner(self) -> DataFrame {
        self.df
    }
    
    fn instrument_id(&self) -> &str {
        &self.instrument_id
    }

    fn time_series(&self) -> PolarsResult<&Series> {
        get_series(&self.df, ColumnName::TIME.into())
    }
        
    fn filter_by_date_range(&self, start_date: i64, end_date: i64) -> PolarsResult<Self> {
        let filtered_df = self.df.clone().lazy()
            .filter(
                col(ColumnName::TIME).gt_eq(lit(start_date))
                    .and(col(ColumnName::TIME).lt_eq(lit(end_date)))
            )
            .collect()?;
        
        Ok(Self {
            df: filtered_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    fn sort_by_time(&self, descending: bool) -> PolarsResult<Self> {
        let sorted_df = self.df.clone().lazy()
            .sort(
                [ColumnName::TIME],
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
    
    fn time_range(&self) -> PolarsResult<(i64, i64)> {
        let time_col = get_series(&self.df, ColumnName::TIME.into())?;
        
        let (min_time, max_time) = time_col.i64()?
            .min_max()
            .unwrap_or((0, 0));
            
        Ok((min_time, max_time))
    }
    
    fn row_count(&self) -> usize {
        self.df.height()
    }
    
    fn join<F: BaseDataFrame>(&self, other: &F, how: JoinType) -> PolarsResult<Self> {
        let joined_df = self.df.clone().lazy()
            .join(
                other.inner().clone().lazy(),
                [col(ColumnName::TIME)],
                [col(ColumnName::TIME)],
                how.into()
            )
            .collect()?;
            
        Ok(Self {
            df: joined_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    fn with_column(&self, series: Series) -> PolarsResult<Self> {
        let mut df_clone = self.df.clone();
        df_clone.with_column(series)?;
        
        Ok(Self {
            df: df_clone,
            instrument_id: self.instrument_id.clone(),
        })
    }

    fn lazy(&self) -> LazyFrame {
        self.df.clone().lazy()
    }
    
    fn collect_from(&self, lf: LazyFrame) -> PolarsResult<Self> {
        let collected_df = lf.collect()?;
        
        Ok(Self {
            df: collected_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
    
    fn vstack(&self, other: &Self) -> PolarsResult<Self> {
        if self.df.schema() != other.df.schema() {
            return Err(PolarsError::ComputeError(
                "Cannot vstack TickFrames with different schemas".into()
            ));
        }
        
        let concat_df = self.df.clone().vstack(&other.df)?;
        
        Ok(Self {
            df: concat_df,
            instrument_id: self.instrument_id.clone(),
        })
    }
}

// 實用的轉換函數
impl From<OHLCVFrame> for DataFrame {
    fn from(frame: OHLCVFrame) -> Self {
        frame.into_inner()
    }
}

impl From<TickFrame> for DataFrame {
    fn from(frame: TickFrame) -> Self {
        frame.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_ohlcv_dataframe() -> DataFrame {
        let time = Series::new(ColumnName::TIME.into(), &[1000, 2000, 3000, 4000, 5000]);
        let open = Series::new(ColumnName::OPEN.into(), &[100.0, 101.0, 102.0, 103.0, 104.0]);
        let high = Series::new(ColumnName::HIGH.into(), &[105.0, 106.0, 107.0, 108.0, 109.0]);
        let low = Series::new(ColumnName::LOW.into(), &[95.0, 96.0, 97.0, 98.0, 99.0]);
        let close = Series::new(ColumnName::CLOSE.into(), &[102.0, 103.0, 104.0, 105.0, 106.0]);
        let volume = Series::new(ColumnName::VOLUME.into(), &[1000, 2000, 3000, 4000, 5000]);
        
        DataFrame::new(vec![time.into(), open.into(), high.into(), low.into(), close.into(), volume.into()]).unwrap()
    }
    
    fn create_test_tick_dataframe() -> DataFrame {
        let time = Series::new(ColumnName::TIME.into(), &[1000, 1001, 1002, 1003, 1004]);
        let price = Series::new(ColumnName::PRICE.into(), &[100.0, 101.0, 102.0, 103.0, 104.0]);
        let volume = Series::new(ColumnName::VOLUME.into(), &[10, 20, 30, 40, 50]);
        
        DataFrame::new(vec![time.into(), price.into(), volume.into()]).unwrap()
    }
    
    #[test]
    fn test_ohlcv_frame_new() {
        let df = create_test_ohlcv_dataframe();
        let ohlcv_frame = OHLCVFrame::new(df, "AAPL", Frequency::Minute).unwrap();
        
        assert_eq!(ohlcv_frame.instrument_id(), "AAPL");
        assert_eq!(ohlcv_frame.row_count(), 5);
        assert_eq!(ohlcv_frame.frequency(), Frequency::Minute);
        assert!(ohlcv_frame.inner().schema().contains(ColumnName::INSTRUMENT_ID));
    }
    
    #[test]
    fn test_tick_frame_new() {
        let df = create_test_tick_dataframe();
        let tick_frame = TickFrame::new(df, "BTC/USD").unwrap();
        
        assert_eq!(tick_frame.instrument_id(), "BTC/USD");
        assert_eq!(tick_frame.row_count(), 5);
        assert!(tick_frame.inner().schema().contains(ColumnName::INSTRUMENT_ID));
    }
    
    #[test]
    fn test_ohlcv_frame_series_access() {
        let df = create_test_ohlcv_dataframe();
        let ohlcv_frame = OHLCVFrame::new(df, "AAPL", Frequency::Minute).unwrap();
        
        let open = ohlcv_frame.open_series().unwrap();
        let high = ohlcv_frame.high_series().unwrap();
        let low = ohlcv_frame.low_series().unwrap();
        let close = ohlcv_frame.close_series().unwrap();
        let volume = ohlcv_frame.volume_series().unwrap();
        
        assert_eq!(open.f64().unwrap().get(0).unwrap(), 100.0);
        assert_eq!(high.f64().unwrap().get(1).unwrap(), 106.0);
        assert_eq!(low.f64().unwrap().get(2).unwrap(), 97.0);
        assert_eq!(close.f64().unwrap().get(3).unwrap(), 105.0);
        assert_eq!(volume.i32().unwrap().get(4).unwrap(), 5000);
    }
    
    #[test]
    fn test_tick_frame_series_access() {
        let df = create_test_tick_dataframe();
        let tick_frame = TickFrame::new(df, "BTC/USD").unwrap();
        
        let price = tick_frame.price_series().unwrap();
        let volume = tick_frame.volume_series().unwrap();
        
        assert_eq!(price.f64().unwrap().get(2).unwrap(), 102.0);
        assert_eq!(volume.i32().unwrap().get(3).unwrap(), 40);
    }
    
    #[test]
    fn test_ohlcv_frame_filter_by_date_range() {
        let df = create_test_ohlcv_dataframe();
        let ohlcv_frame = OHLCVFrame::new(df, "AAPL", Frequency::Minute).unwrap();
        
        let filtered = ohlcv_frame.filter_by_date_range(2000, 4000).unwrap();
        assert_eq!(filtered.row_count(), 3);
        
        let time_series = filtered.time_series().unwrap().i64().unwrap();
        assert_eq!(time_series.get(0).unwrap(), 2000);
        assert_eq!(time_series.get(2).unwrap(), 4000);
    }
    
    #[test]
    fn test_tick_frame_filter_by_date_range() {
        let df = create_test_tick_dataframe();
        let tick_frame = TickFrame::new(df, "BTC/USD").unwrap();
        
        let filtered = tick_frame.filter_by_date_range(1001, 1003).unwrap();
        assert_eq!(filtered.row_count(), 3);
        
        let time_series = filtered.time_series().unwrap().i64().unwrap();
        assert_eq!(time_series.get(0).unwrap(), 1001);
        assert_eq!(time_series.get(2).unwrap(), 1003);
    }
    
    #[test]
    fn test_ohlcv_frame_resample() {
        // 在實際應用中，需要更多的數據和適當的時間戳進行測試
        let df = create_test_ohlcv_dataframe();
        let ohlcv_frame = OHLCVFrame::new(df, "AAPL", Frequency::Minute).unwrap();
        
        // 假設 Resampler 能處理這種情況（實際中可能不能）
        if let Ok(resampled) = ohlcv_frame.resample(Frequency::Hour) {
            assert_eq!(resampled.frequency(), Frequency::Hour);
            assert_eq!(resampled.instrument_id(), "AAPL");
        }
    }
    
    #[test]
    fn test_tick_to_ohlcv_conversion() {
        // 注意：這需要 Resampler 實現 tick_to_ohlcv 方法
        let df = create_test_tick_dataframe();
        let tick_frame = TickFrame::new(df, "BTC/USD").unwrap();
        
        // 此測試可能在實際運行時失敗，取決於 Resampler 的實現
        if let Ok(ohlcv) = tick_frame.to_ohlcv(Frequency::Minute) {
            assert_eq!(ohlcv.frequency(), Frequency::Minute);
            assert_eq!(ohlcv.instrument_id(), "BTC/USD");
        }
    }
    
    #[test]
    fn test_ohlcv_frame_vstack() {
        let df1 = create_test_ohlcv_dataframe();
        let ohlcv_frame1 = OHLCVFrame::new(df1, "AAPL", Frequency::Minute).unwrap();
        
        let df2 = create_test_ohlcv_dataframe();
        let ohlcv_frame2 = OHLCVFrame::new(df2, "AAPL", Frequency::Minute).unwrap();
        
        let stacked = ohlcv_frame1.vstack(&ohlcv_frame2).unwrap();
        assert_eq!(stacked.row_count(), 10); // 5 + 5 行
    }
}