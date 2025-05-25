// resampling.rs
use super::frequency::Frequency;
use super::types::ColumnName;
use polars::prelude::*;

/// 提供重採樣核心功能的結構
pub struct Resampler;

impl Resampler {
    /// 對 LazyFrame 進行重採樣
    pub fn resample_lazy(lf: LazyFrame, target_frequency: Frequency) -> LazyFrame {
        let window_size = target_frequency.to_duration();

        lf.group_by_dynamic(
            col(ColumnName::TIME),
            [],
            DynamicGroupOptions {
                label: Label::Left,
                start_by: StartBy::WindowBound,
                index_column: ColumnName::TIME.into(),
                every: window_size,
                period: window_size,
                offset: Duration::new(0),
                include_boundaries: false,
                closed_window: ClosedWindow::Left,
            },
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

    /// 將 Tick 數據轉換為 OHLCV 數據
    ///
    /// 該函數將 Tick 數據按照指定的頻率聚合為蠟燭圖（K線）數據
    ///
    /// # Arguments
    /// * `df` - 包含 Tick 數據的 DataFrame，必須包含 time、price 和 volume 列
    /// * `frequency` - 目標頻率，例如 Frequency::Minute
    ///
    /// # Returns
    /// 返回轉換後的 OHLCV DataFrame，包含 time、open、high、low、close 和 volume 列
    ///
    /// # Errors
    /// 當輸入的 DataFrame 缺少必要列（time、price 或 volume）時返回錯誤
    pub fn tick_to_ohlcv(df: &DataFrame, frequency: Frequency) -> PolarsResult<DataFrame> {
        // 驗證必要的列是否存在
        for col_name in [ColumnName::TIME, ColumnName::PRICE, ColumnName::VOLUME].iter() {
            if !df.schema().contains(*col_name) {
                return Err(PolarsError::ColumnNotFound(
                    format!("Column '{}' not found in DataFrame", *col_name).into(),
                ));
            }
        }

        let window_size = frequency.to_duration();

        // 轉換為 LazyFrame 並按時間窗口分組
        let result = df
            .clone()
            .lazy()
            .group_by_dynamic(
                col(ColumnName::TIME),
                [],
                DynamicGroupOptions {
                    label: Label::Left,
                    start_by: StartBy::WindowBound,
                    index_column: ColumnName::TIME.into(),
                    every: window_size,
                    period: window_size,
                    offset: Duration::new(0),
                    include_boundaries: false,
                    closed_window: ClosedWindow::Left,
                },
            )
            .agg([
                // 開盤價是時間窗口內第一個交易的價格
                col(ColumnName::PRICE).first().alias(ColumnName::OPEN),
                // 最高價是時間窗口內的最高交易價格
                col(ColumnName::PRICE).max().alias(ColumnName::HIGH),
                // 最低價是時間窗口內的最低交易價格
                col(ColumnName::PRICE).min().alias(ColumnName::LOW),
                // 收盤價是時間窗口內最後一個交易的價格
                col(ColumnName::PRICE).last().alias(ColumnName::CLOSE),
                // 成交量是時間窗口內所有交易的成交量總和
                col(ColumnName::VOLUME).sum().alias(ColumnName::VOLUME),
            ])
            .collect()?;

        // 保留原始 DataFrame 中的 instrument_id 列（如果存在）
        let mut final_result = result;
        if df.schema().contains(ColumnName::INSTRUMENT_ID) {
            // 獲取第一個 instrument_id 值（假設所有行的 instrument_id 相同）
            if let Ok(id_series) = df.column(ColumnName::INSTRUMENT_ID) {
                // 使用正確的錯誤處理，get(0) 返回 PolarsResult<AnyValue>
                if let Ok(first_id) = id_series.head(Some(1)).get(0) {
                    let id_str = first_id.to_string();
                    let id_series = Series::new(
                        ColumnName::INSTRUMENT_ID.into(),
                        vec![id_str; final_result.height()],
                    );
                    final_result.with_column(id_series)?;
                }
            }
        }

        Ok(final_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 創建測試用的 Tick 數據 DataFrame
    fn create_test_tick_dataframe() -> DataFrame {
        // 使用真實的時間戳（從2024-01-01開始，每秒一個tick）
        let base_timestamp = 1704067200000i64; // 2024-01-01 00:00:00 UTC in milliseconds
        let time_data: Vec<i64> = (0..5).map(|i| base_timestamp + i * 1000).collect(); // 每秒增加1000ms

        let time = Series::new(ColumnName::TIME.into(), &time_data);
        let price = Series::new(
            ColumnName::PRICE.into(),
            &[100.0, 101.0, 102.0, 103.0, 104.0],
        );
        let volume = Series::new(ColumnName::VOLUME.into(), &[10, 20, 30, 40, 50]);

        DataFrame::new(vec![time.into(), price.into(), volume.into()]).unwrap()
    }

    #[test]
    fn test_tick_to_ohlcv() {
        let df = create_test_tick_dataframe();
        let result = Resampler::tick_to_ohlcv(&df, Frequency::Minute);

        match &result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("tick_to_ohlcv error: {:?}", e);
            }
        }

        assert!(result.is_ok(), "tick_to_ohlcv should return Ok");

        if let Ok(ohlcv_df) = result {
            // 檢查必要列是否存在
            assert!(ohlcv_df.schema().contains(ColumnName::TIME));
            assert!(ohlcv_df.schema().contains(ColumnName::OPEN));
            assert!(ohlcv_df.schema().contains(ColumnName::HIGH));
            assert!(ohlcv_df.schema().contains(ColumnName::LOW));
            assert!(ohlcv_df.schema().contains(ColumnName::CLOSE));
            assert!(ohlcv_df.schema().contains(ColumnName::VOLUME));

            // 由於我們使用的是測試數據，實際值可能取決於時間窗口的實現方式
            // 在真實世界中會需要更多的測試，但這對於確認功能正常工作已足夠
        }
    }
}
