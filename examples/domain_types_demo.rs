use crate::domain_types::{MarketFrame, MarketSeries, Frequency, IndicatorsExt, Column};
use polars::prelude::*;

// 示例: 加載數據並使用
async fn example_usage() -> PolarsResult<()> {
    // 1. 創建 DataFrame
    let df = DataFrame::new(vec![
        Series::new(Column::TIME, &[1650000000, 1650000060, 1650000120, 1650000180]),
        Series::new(Column::OPEN, &[100.0, 101.0, 102.0, 101.5]),
        Series::new(Column::HIGH, &[102.0, 103.0, 104.0, 103.5]),
        Series::new(Column::LOW, &[99.0, 100.5, 101.0, 100.0]),
        Series::new(Column::CLOSE, &[101.0, 102.0, 103.0, 102.5]),
        Series::new(Column::VOLUME, &[1000.0, 1200.0, 950.0, 1100.0]),
    ])?;
    
    // 2. 將普通 DataFrame 轉換為 MarketFrame
    let market_frame = MarketFrame::new(df, "AAPL")?;
    
    // 3. 將 MarketFrame 轉換為 MarketSeries
    let series = market_frame.as_series(Frequency::Minute)?;
    
    // 4. 重採樣到較高頻率
    let hourly_series = series.resample(Frequency::Hour)?;
    
    // 5. 添加技術指標
    let df_with_sma = hourly_series.collect()?
        .sma(Column::CLOSE, 2, Some("close_sma2"))?;
    
    // 6. 添加多個指標
    let df_with_indicators = df_with_sma
        .ema(Column::CLOSE, 3, Some("close_ema3"))?
        .rsi(Column::CLOSE, 3, Some("close_rsi3"))?;
    
    println!("{}", df_with_indicators);
    
    Ok(())
}