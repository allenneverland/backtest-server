//! 基本技術指標實現 - 使用 Polars 原生函數優化

use super::types::ColumnName;
use polars::lazy::dsl::max_horizontal;
use polars::prelude::*;
use polars::series::ops::NullBehavior;

/// 為 DataFrame 添加技術指標功能的擴展 trait
pub trait IndicatorsExt {
    /// 簡單移動平均線
    fn sma(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame>;

    /// 指數移動平均線
    fn ema(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame>;

    /// 相對強弱指標
    fn rsi(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame>;

    /// 布林帶指標
    fn bollinger_bands(
        &self,
        column: &str,
        window: usize,
        std_dev: f64,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame>;

    /// 移動平均收斂/發散 (MACD)
    fn macd(
        &self,
        column: &str,
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame>;

    /// 隨機震盪指標 (Stochastic Oscillator)
    fn stochastic(
        &self,
        k_period: usize,
        d_period: usize,
        smooth_k: Option<usize>,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame>;

    /// 平均真實範圍 (ATR)
    fn atr(&self, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame>;

    /// 累積分佈線 (OBV)
    fn obv(&self, alias: Option<&str>) -> PolarsResult<DataFrame>;

    /// 價格通道指標 (Donchian Channel)
    fn donchian_channel(
        &self,
        window: usize,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame>;

    /// 動量指標 (Momentum)
    fn momentum(&self, column: &str, period: usize, alias: Option<&str>)
        -> PolarsResult<DataFrame>;
}

impl IndicatorsExt for DataFrame {
    fn sma(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame> {
        println!("column: {}", column);
        println!("window: {}", window);
        let format_str = format!("sma_{}_{}", column, window);
        let alias_str = alias.unwrap_or(&format_str);

        // 使用基於固定行數的滾動窗口計算 SMA
        let options = RollingOptionsFixedWindow {
            window_size: window,
            min_periods: window, // 確保窗口滿了才計算
            center: false,       // SMA 通常向後看
            ..Default::default()
        };

        let sma_expr = col(column).rolling_mean(options).alias(alias_str);

        self.clone().lazy().with_column(sma_expr).collect()
    }

    fn ema(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame> {
        let format_str = format!("ema_{}_{}", column, window);
        let alias_str = alias.unwrap_or(&format_str);

        // 使用 Polars 原生的指數加權移動平均函數
        let alpha = 2.0 / (window as f64 + 1.0);
        let ema_expr = col(column)
            .ewm_mean(EWMOptions {
                alpha,
                min_periods: 1,
                adjust: false,
                bias: false,
                ignore_nulls: true,
            })
            .alias(alias_str);

        self.clone().lazy().with_column(ema_expr).collect()
    }

    fn rsi(&self, column: &str, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame> {
        let format_str = format!("rsi_{}_{}", column, window);
        let alias_str = alias.unwrap_or(&format_str);

        // 使用 Polars 高效表達式 API 實現 RSI 計算
        // 1. 計算價格變化
        let diff_expr = col(column)
            .diff(lit(1), NullBehavior::Ignore)
            .alias("__price_change");

        // 2. 計算上漲和下跌
        let up_expr = when(col("__price_change").gt(lit(0.0)))
            .then(col("__price_change"))
            .otherwise(lit(0.0))
            .alias("__up");

        let down_expr = when(col("__price_change").lt(lit(0.0)))
            .then(col("__price_change").abs())
            .otherwise(lit(0.0))
            .alias("__down");

        // 3. 計算平均上漲和下跌
        let avg_up_expr = col("__up")
            .ewm_mean(EWMOptions {
                alpha: 1.0 / window as f64,
                min_periods: window,
                adjust: false,
                bias: false,
                ignore_nulls: true,
            })
            .alias("__avg_up");

        let avg_down_expr = col("__down")
            .ewm_mean(EWMOptions {
                alpha: 1.0 / window as f64,
                min_periods: window,
                adjust: false,
                bias: false,
                ignore_nulls: true,
            })
            .alias("__avg_down");

        // 4. 計算 RS 和 RSI
        let rs_expr = (col("__avg_up") / col("__avg_down")).alias("__rs");
        let rsi_expr = (lit(100.0) - (lit(100.0) / (lit(1.0) + col("__rs")))).alias(alias_str);

        // 5. 組合所有表達式
        self.clone()
            .lazy()
            .with_column(diff_expr)
            .with_column(up_expr)
            .with_column(down_expr)
            .with_column(avg_up_expr)
            .with_column(avg_down_expr)
            .with_column(rs_expr)
            .with_column(rsi_expr)
            .drop([
                "__price_change",
                "__up",
                "__down",
                "__avg_up",
                "__avg_down",
                "__rs",
            ])
            .collect()
    }

    fn bollinger_bands(
        &self,
        column: &str,
        window: usize,
        std_dev: f64,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame> {
        let format_str = format!("bb_{}_{}_{}", column, window, std_dev);
        let prefix = alias_prefix.unwrap_or(&format_str);
        let middle_alias = format!("{}_middle", prefix);
        let upper_alias = format!("{}_upper", prefix);
        let lower_alias = format!("{}_lower", prefix);

        // 使用基於固定行數的滾動窗口配置
        let options = RollingOptionsFixedWindow {
            window_size: window,
            min_periods: window,
            center: false,
            ..Default::default()
        };

        // 計算中線(SMA)並加入數據框
        let middle_expr = col(column)
            .rolling_mean(options.clone())
            .alias(&middle_alias);

        // 計算滾動標準差
        let std_expr = col(column).rolling_std(options).alias("__std");

        // 計算上軌和下軌
        let upper_expr = (col(&middle_alias) + (lit(std_dev) * col("__std"))).alias(&upper_alias);

        let lower_expr = (col(&middle_alias) - (lit(std_dev) * col("__std"))).alias(&lower_alias);

        // 組合所有計算並返回結果
        self.clone()
            .lazy()
            .with_column(middle_expr)
            .with_column(std_expr)
            .with_column(upper_expr)
            .with_column(lower_expr)
            .drop(["__std"])
            .collect()
    }

    fn macd(
        &self,
        column: &str,
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame> {
        let format_str = format!(
            "macd_{}_{}_{}_{}",
            column, fast_period, slow_period, signal_period
        );
        let prefix = alias_prefix.unwrap_or(&format_str);
        let macd_alias = format!("{}_line", prefix);
        let signal_alias = format!("{}_signal", prefix);
        let hist_alias = format!("{}_histogram", prefix);

        // 計算快速 EMA
        let fast_ema_expr = col(column)
            .ewm_mean(EWMOptions {
                alpha: 2.0 / (fast_period as f64 + 1.0),
                min_periods: 1,
                adjust: false,
                bias: false,
                ignore_nulls: true,
            })
            .alias("__fast_ema");

        // 計算慢速 EMA
        let slow_ema_expr = col(column)
            .ewm_mean(EWMOptions {
                alpha: 2.0 / (slow_period as f64 + 1.0),
                min_periods: 1,
                adjust: false,
                bias: false,
                ignore_nulls: true,
            })
            .alias("__slow_ema");

        // 計算 MACD 線
        let macd_expr = (col("__fast_ema") - col("__slow_ema")).alias(&macd_alias);

        // 計算信號線(MACD的EMA)
        let signal_expr = col(&macd_alias)
            .ewm_mean(EWMOptions {
                alpha: 2.0 / (signal_period as f64 + 1.0),
                min_periods: 1,
                adjust: false,
                bias: false,
                ignore_nulls: true,
            })
            .alias(&signal_alias);

        // 計算直方圖(MACD - 信號線)
        let hist_expr = (col(&macd_alias) - col(&signal_alias)).alias(&hist_alias);

        // 組合所有計算
        self.clone()
            .lazy()
            .with_column(fast_ema_expr)
            .with_column(slow_ema_expr)
            .with_column(macd_expr)
            .with_column(signal_expr)
            .with_column(hist_expr)
            .drop(["__fast_ema", "__slow_ema"])
            .collect()
    }

    fn stochastic(
        &self,
        k_period: usize,
        d_period: usize,
        smooth_k: Option<usize>,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame> {
        let smooth_k = smooth_k.unwrap_or(1);
        let format_str = format!("stoch_{}_{}_{}", k_period, d_period, smooth_k);
        let prefix = alias_prefix.unwrap_or(&format_str);
        let k_alias = format!("{}_k", prefix);
        let d_alias = format!("{}_d", prefix);

        // 創建時間索引列的名稱
        let time_column = ColumnName::TIME;

        // 滾動窗口配置
        let k_options = RollingGroupOptions {
            index_column: time_column.into(),
            period: Duration::new(k_period as i64),
            offset: Duration::new(0),
            closed_window: ClosedWindow::Right,
        };

        // 計算滾動窗口的最高價和最低價
        let high_max_expr = col(ColumnName::HIGH)
            .rolling(k_options.clone())
            .max()
            .alias("__high_max");

        let low_min_expr = col(ColumnName::LOW)
            .rolling(k_options)
            .min()
            .alias("__low_min");

        // 計算原始 %K
        let raw_k_expr = ((col(ColumnName::CLOSE) - col("__low_min"))
            / (col("__high_max") - col("__low_min"))
            * lit(100.0))
        .alias("__raw_k");

        // 計算平滑後的 %K
        let k_expr = if smooth_k <= 1 {
            col("__raw_k").alias(&k_alias)
        } else {
            let smooth_options = RollingGroupOptions {
                index_column: time_column.into(),
                period: Duration::new(smooth_k as i64),
                offset: Duration::new(0),
                closed_window: ClosedWindow::Right,
            };

            col("__raw_k")
                .rolling(smooth_options)
                .mean()
                .alias(&k_alias)
        };

        // 計算 %D
        let d_options = RollingGroupOptions {
            index_column: time_column.into(),
            period: Duration::new(d_period as i64),
            offset: Duration::new(0),
            closed_window: ClosedWindow::Right,
        };

        let d_expr = col(&k_alias).rolling(d_options).mean().alias(&d_alias);

        // 組合所有計算
        self.clone()
            .lazy()
            .with_column(high_max_expr)
            .with_column(low_min_expr)
            .with_column(raw_k_expr)
            .with_column(k_expr)
            .with_column(d_expr)
            .drop(["__high_max", "__low_min", "__raw_k"])
            .collect()
    }

    fn atr(&self, window: usize, alias: Option<&str>) -> PolarsResult<DataFrame> {
        let format_str = format!("atr_{}", window);
        let alias_str = alias.unwrap_or(&format_str);

        // 計算真實範圍(TR)組件
        let high_low = (col(ColumnName::HIGH) - col(ColumnName::LOW)).alias("__hl");
        let high_close = (col(ColumnName::HIGH) - col(ColumnName::CLOSE).shift(lit(1)))
            .abs()
            .alias("__hc");
        let low_close = (col(ColumnName::LOW) - col(ColumnName::CLOSE).shift(lit(1)))
            .abs()
            .alias("__lc");

        // 計算真實範圍 - 三個元素中的最大值
        let tr_expr = max_horizontal(&[col("__hl"), col("__hc"), col("__lc")])
            .unwrap()
            .alias("__tr");

        // 計算 ATR (TR的滾動平均)
        let atr_expr = col("__tr")
            .ewm_mean(EWMOptions {
                alpha: 1.0 / window as f64,
                min_periods: window,
                adjust: false,
                bias: false,
                ignore_nulls: true,
            })
            .alias(alias_str);

        // 組合所有計算
        self.clone()
            .lazy()
            .with_column(high_low)
            .with_column(high_close)
            .with_column(low_close)
            .with_column(tr_expr)
            .with_column(atr_expr)
            .drop(["__hl", "__hc", "__lc", "__tr"])
            .collect()
    }

    fn obv(&self, alias: Option<&str>) -> PolarsResult<DataFrame> {
        let format_str = "obv".to_string();
        let alias_str = alias.unwrap_or(&format_str);
        // 1. 計算價格變化方向
        let close_diff_expr = col(ColumnName::CLOSE)
            .diff(lit(1), NullBehavior::Ignore)
            .alias("__close_diff");

        // 2. 計算方向值 (1, -1, 0)
        let direction_expr = when(col("__close_diff").gt(lit(0.0)))
            .then(lit(1))
            .when(col("__close_diff").lt(lit(0.0)))
            .then(lit(-1))
            .otherwise(lit(0))
            .alias("__direction");

        // 3. 計算帶方向的交易量
        let dir_volume_expr = (col("__direction") * col(ColumnName::VOLUME)).alias("__dir_volume");

        // 4. 計算 OBV (累積和)
        let obv_expr = col("__dir_volume").cum_sum(false).alias(alias_str);

        // 組合所有計算
        self.clone()
            .lazy()
            .with_column(close_diff_expr)
            .with_column(direction_expr)
            .with_column(dir_volume_expr)
            .with_column(obv_expr)
            .drop(["__close_diff", "__direction", "__dir_volume"])
            .collect()
    }

    fn donchian_channel(
        &self,
        window: usize,
        alias_prefix: Option<&str>,
    ) -> PolarsResult<DataFrame> {
        let format_str = format!("dc_{}", window);
        let prefix = alias_prefix.unwrap_or(&format_str);
        let upper_alias = format!("{}_upper", prefix);
        let middle_alias = format!("{}_middle", prefix);
        let lower_alias = format!("{}_lower", prefix);

        // 創建時間索引列的名稱
        let time_column = ColumnName::TIME;

        // 滾動窗口配置
        let options = RollingGroupOptions {
            index_column: time_column.into(),
            period: Duration::new(window as i64),
            offset: Duration::new(0),
            closed_window: ClosedWindow::Right,
        };

        // 計算上軌(最高價的滾動最大值)
        let upper_expr = col(ColumnName::HIGH)
            .rolling(options.clone())
            .max()
            .alias(&upper_alias);

        // 計算下軌(最低價的滾動最小值)
        let lower_expr = col(ColumnName::LOW)
            .rolling(options)
            .min()
            .alias(&lower_alias);

        // 計算中軌((上軌+下軌)/2)
        let middle_expr = ((col(&upper_alias) + col(&lower_alias)) / lit(2.0)).alias(&middle_alias);

        // 組合結果
        self.clone()
            .lazy()
            .with_column(upper_expr)
            .with_column(lower_expr)
            .with_column(middle_expr)
            .collect()
    }

    fn momentum(
        &self,
        column: &str,
        period: usize,
        alias: Option<&str>,
    ) -> PolarsResult<DataFrame> {
        let format_str = format!("mom_{}_{}", column, period);
        let alias_str = alias.unwrap_or(&format_str);

        // 計算動量指標 (當前價格 - 過去價格)
        let mom_expr = (col(column) - col(column).shift(lit(period as i64))).alias(alias_str);

        // 組合結果
        self.clone().lazy().with_column(mom_expr).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dataframe() -> DataFrame {
        // 使用真實的日期時間戳 (每天一個數據點，從2024-01-01開始)
        let base_timestamp = 1704067200000i64; // 2024-01-01 00:00:00 UTC in milliseconds
        let time_data: Vec<i64> = (0..10).map(|i| base_timestamp + i * 86400000).collect(); // 每天增加86400000ms (24小時)

        let time = Series::new(ColumnName::TIME.into(), &time_data);
        let open = Series::new(
            ColumnName::OPEN.into(),
            &[
                100.0, 102.0, 104.0, 103.0, 105.0, 107.0, 109.0, 108.0, 110.0, 112.0,
            ],
        );
        let high = Series::new(
            ColumnName::HIGH.into(),
            &[
                105.0, 106.0, 107.0, 105.0, 108.0, 110.0, 112.0, 110.0, 115.0, 118.0,
            ],
        );
        let low = Series::new(
            ColumnName::LOW.into(),
            &[
                98.0, 100.0, 102.0, 101.0, 103.0, 105.0, 107.0, 105.0, 108.0, 110.0,
            ],
        );
        let close = Series::new(
            ColumnName::CLOSE.into(),
            &[
                102.0, 104.0, 105.0, 103.0, 107.0, 109.0, 110.0, 107.0, 112.0, 115.0,
            ],
        );
        let volume = Series::new(
            ColumnName::VOLUME.into(),
            &[1000, 1200, 1500, 1300, 1400, 1600, 1800, 1700, 2000, 2200],
        );

        DataFrame::new(vec![
            time.into(),
            open.into(),
            high.into(),
            low.into(),
            close.into(),
            volume.into(),
        ])
        .unwrap()
    }

    #[test]
    fn test_sma() {
        let df = create_test_dataframe();
        let result = df.sma(ColumnName::CLOSE, 3, None).unwrap();

        // 檢查SMA列是否存在
        assert!(result.schema().contains("sma_close_3"));

        // 檢查SMA計算是否正確 (前3個值的平均)
        let sma_series = result.column("sma_close_3").unwrap();
        let sma_vals = sma_series.f64().unwrap();

        // 前三個值的SMA應該是前n個值的平均
        assert!((sma_vals.get(2).unwrap() - (102.0 + 104.0 + 105.0) / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_bollinger_bands() {
        let df = create_test_dataframe();
        let result = df.bollinger_bands(ColumnName::CLOSE, 3, 2.0, None).unwrap();

        // 檢查布林帶三個線是否都存在
        assert!(result.schema().contains("bb_close_3_2_middle"));
        assert!(result.schema().contains("bb_close_3_2_upper"));
        assert!(result.schema().contains("bb_close_3_2_lower"));
    }

    #[test]
    fn test_macd() {
        let df = create_test_dataframe();
        let result = df.macd(ColumnName::CLOSE, 3, 6, 2, None).unwrap();

        // 檢查MACD三個值是否都存在
        assert!(result.schema().contains("macd_close_3_6_2_line"));
        assert!(result.schema().contains("macd_close_3_6_2_signal"));
        assert!(result.schema().contains("macd_close_3_6_2_histogram"));
    }

    #[test]
    fn test_rsi() {
        let df = create_test_dataframe();
        let result = df.rsi(ColumnName::CLOSE, 3, None).unwrap();

        // 檢查RSI列是否存在
        assert!(result.schema().contains("rsi_close_3"));

        // RSI值應該在0-100之間
        let rsi_series = result.column("rsi_close_3").unwrap();
        let rsi_vals = rsi_series.f64().unwrap();

        // 跳過前幾個可能為NaN的值
        for i in 3..rsi_vals.len() {
            let val = rsi_vals.get(i).unwrap();
            if !val.is_nan() {
                assert!(val >= 0.0 && val <= 100.0);
            }
        }
    }

    #[test]
    fn test_atr() {
        let df = create_test_dataframe();
        let result = df.atr(3, None).unwrap();

        // 檢查ATR列是否存在
        assert!(result.schema().contains("atr_3"));
    }

    #[test]
    fn test_obv() {
        let df = create_test_dataframe();
        let result = df.obv(None).unwrap();

        // 檢查OBV列是否存在
        assert!(result.schema().contains("obv"));
    }

    #[test]
    fn test_with_ohlcv_frame() {
        use crate::domain_types::{Day, OhlcvSeries};

        let df = create_test_dataframe();
        let ohlcv_frame = OhlcvSeries::<Day>::new(df, "AAPL".to_string()).unwrap();

        // 在FinancialSeries上應用技術指標
        let with_sma = ohlcv_frame
            .collect()
            .unwrap()
            .sma(ColumnName::CLOSE, 3, None)
            .unwrap();
        let with_indicators = with_sma
            .bollinger_bands(ColumnName::CLOSE, 5, 2.0, None)
            .unwrap();

        // 檢查指標列是否存在
        assert!(with_indicators.schema().contains("sma_close_3"));
        assert!(with_indicators.schema().contains("bb_close_5_2_middle"));
    }
}
