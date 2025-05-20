//! 基本技術指標實現

use polars::prelude::*;
use polars::lazy::prelude::*;
use super::types::Column;

/// 為 DataFrame 添加技術指標功能的擴展 trait
pub trait IndicatorsExt {
    /// 簡單移動平均線
    fn sma(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame>;
    
    /// 指數移動平均線
    fn ema(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame>;
    
    /// 相對強弱指標
    fn rsi(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame>;
}

impl IndicatorsExt for DataFrame {
    fn sma(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame> {
        let alias = alias.unwrap_or(&format!("sma_{}_{}",column, window));
        
        let expr = col(column)
            .rolling_mean(RollingOptions {
                window_size: window,
                min_periods: 1,
                center: false,
                weights: None,
            })
            .alias(alias);
        
        self.clone().lazy().with_column(expr).collect()
    }
    
    fn ema(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame> {
        let alias = alias.unwrap_or(&format!("ema_{}_{}",column, window));
        
        let alpha = 2.0 / (window as f64 + 1.0);
        
        // Polars 目前不直接支援 EMA，但可以用 ewm_mean 實現
        let expr = col(column)
            .ewm_mean(
                EWMOptions {
                    alpha,
                    adjust: false,
                    min_periods: 1,
                }
            )
            .alias(alias);
        
        self.clone().lazy().with_column(expr).collect()
    }
    
    fn rsi(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame> {
        let alias = alias.unwrap_or(&format!("rsi_{}_{}",column, window));
        
        // 簡化的 RSI 實現
        // 實際實現可能需要更複雜的邏輯
        
        // 計算價格變化
        let diff_column = "__temp_diff";
        let up_column = "__temp_up";
        let down_column = "__temp_down";
        let avg_up_column = "__temp_avg_up";
        let avg_down_column = "__temp_avg_down";
        
        let with_diff = self.clone().lazy()
            .with_column(
                col(column).diff(1).alias(diff_column)
            )
            .collect()?;
        
        // 計算上漲和下跌
        let with_up_down = with_diff.clone().lazy()
            .with_column(
                when(col(diff_column).gt(lit(0.0)))
                    .then(col(diff_column))
                    .otherwise(lit(0.0))
                    .alias(up_column)
            )
            .with_column(
                when(col(diff_column).lt(lit(0.0)))
                    .then(lit(0.0) - col(diff_column))
                    .otherwise(lit(0.0))
                    .alias(down_column)
            )
            .collect()?;
        
        // 計算平均上漲和下跌
        let with_avgs = with_up_down.clone().lazy()
            .with_column(
                col(up_column)
                    .rolling_mean(RollingOptions {
                        window_size: window,
                        min_periods: 1,
                        center: false,
                        weights: None,
                    })
                    .alias(avg_up_column)
            )
            .with_column(
                col(down_column)
                    .rolling_mean(RollingOptions {
                        window_size: window,
                        min_periods: 1,
                        center: false,
                        weights: None,
                    })
                    .alias(avg_down_column)
            )
            .collect()?;
        
        // 計算 RSI
        let final_df = with_avgs.clone().lazy()
            .with_column(
                (lit(100.0) - (lit(100.0) / (lit(1.0) + col(avg_up_column) / col(avg_down_column))))
                    .alias(alias)
            )
            // 移除臨時列
            .drop_columns([diff_column, up_column, down_column, avg_up_column, avg_down_column])
            .collect()?;
        
        Ok(final_df)
    }
}