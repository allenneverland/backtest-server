# Domain Types Module Documentation

## 概述

`domain_types` 模組是 backtest-server 的核心數據結構模組，提供了金融時間序列數據處理的基礎類型和功能。這個模組設計為高度可擴展和類型安全的系統，使用 Rust 的類型系統來確保數據的正確性。

## 模組結構

```
domain_types/
├── frequency.rs      # 頻率定義和配置（從 frequencies.toml 生成）
├── types.rs          # 基礎類型定義（資產類型、數據格式等）
├── series.rs         # 金融時間序列結構（FinancialSeries）
├── resampler.rs      # 時間序列重採樣功能
├── indicators.rs     # 技術指標擴展
└── instrument.rs     # 金融工具定義
```

## 核心概念

### 1. 頻率系統 (Frequency System)

頻率系統是整個時間序列處理的核心。所有頻率定義都集中在 `config/frequencies.toml` 配置文件中，並在編譯時通過 `build.rs` 自動生成相關代碼：

```toml
# config/frequencies.toml
[[frequency]]
name = "Minute"
enum_name = "Minute"
struct_name = "Minute"
seconds = 60
milliseconds = 60000
polars_string = "1m"
display_name = "Minute"
alias_suffix = "Minute"
is_ohlcv = true
```

#### 添加新頻率

要添加新的頻率，只需在 `config/frequencies.toml` 中添加新的頻率定義：

```toml
# 例如添加30分鐘頻率
[[frequency]]
name = "ThirtyMinutes"
enum_name = "ThirtyMinutes"
struct_name = "ThirtyMinutes"
seconds = 1800
milliseconds = 1800000
polars_string = "30m"
display_name = "ThirtyMinutes"
alias_suffix = "ThirtyMinute"
is_ohlcv = true
```

系統會自動生成：
- `Frequency::ThirtyMinutes` 枚舉值
- `ThirtyMinutes` 結構體和 `FrequencyMarker` 實現
- 相關的時間轉換方法
- 頻率元數據訪問方法

### 2. 金融時間序列 (FinancialSeries)

`FinancialSeries<F, D>` 是一個泛型結構，支持不同頻率和數據格式的組合：

```rust
// F: 頻率標記類型 (Day, Minute, Hour 等)
// D: 數據格式類型 (OhlcvFormat, TickFormat 等)
pub struct FinancialSeries<F: FrequencyMarker, D: DataFormat> {
    lazy_frame: LazyFrame,
    instrument_id: String,
    _frequency: PhantomData<F>,
    _format: PhantomData<D>,
}
```

#### 使用泛型類型

系統使用泛型類型來表示不同頻率的時間序列：

```rust
// OHLCV 數據
pub type OhlcvSeries<F> = FinancialSeries<F, OhlcvFormat>;

// Tick 數據
pub type TickSeries<F> = FinancialSeries<F, TickFormat>;

// 特殊情況：Tick 數據的便利別名
pub type TickData = TickSeries<Tick>;
```

使用示例：
```rust
// 分鐘級 OHLCV
let minute_data: OhlcvSeries<Minute> = OhlcvSeries::new(df, "AAPL".to_string())?;

// 日級 OHLCV
let daily_data: OhlcvSeries<Day> = OhlcvSeries::new(df, "AAPL".to_string())?;
```

### 3. 數據格式 (DataFormat)

數據格式定義了時間序列必須包含的列：

```rust
pub trait DataFormat {
    fn required_columns() -> &'static [&'static str];
    fn validate_dataframe(df: &DataFrame) -> PolarsResult<()>;
    fn format_name() -> &'static str;
}
```

目前支持的格式：
- `OhlcvFormat`: 需要 time, open, high, low, close, volume 列
- `TickFormat`: 需要 time, price, volume 列

## 使用指南

### 創建時間序列

```rust
use backtest_server::domain_types::{OhlcvSeries, Day, Minute, TickData};
use polars::prelude::*;

// 創建日線 OHLCV 數據
let df = DataFrame::new(vec![
    Series::new("time", &[1704067200000i64, 1704153600000i64]),
    Series::new("open", &[100.0, 101.0]),
    Series::new("high", &[105.0, 106.0]),
    Series::new("low", &[99.0, 100.0]),
    Series::new("close", &[104.0, 105.0]),
    Series::new("volume", &[1000000.0, 1200000.0]),
])?;

// 使用泛型類型創建日線數據
let daily_data = OhlcvSeries::<Day>::new(df.clone(), "AAPL".to_string())?;

// 或者創建分鐘線數據
let minute_data = OhlcvSeries::<Minute>::new(df, "AAPL".to_string())?;
```

### 時間序列操作

```rust
// 過濾時間範圍
let filtered = daily_data
    .filter_date_range(start_timestamp, end_timestamp)
    .sort_by_time(false);

// 選擇特定列
let close_only = daily_data.select_columns(&["time", "close"]);

// 獲取時間範圍
let (start, end) = daily_data.time_range()?;
```

### 重採樣

```rust
use backtest_server::domain_types::{OhlcvSeries, Minute, Hour};

// 從分鐘數據重採樣到小時數據
let minute_data: OhlcvSeries<Minute> = // ... 載入分鐘數據
let hourly_data: OhlcvSeries<Hour> = minute_data.resample_to::<Hour>()?;
```

### 技術指標

```rust
use backtest_server::domain_types::IndicatorsExt;

// 計算簡單移動平均
let sma_20 = daily_data.sma(20)?;

// 計算指數移動平均
let ema_12 = daily_data.ema(12)?;

// 計算 RSI
let rsi_14 = daily_data.rsi(14)?;
```

## 擴展系統

### 添加新的數據格式

1. 定義新的格式結構：

```rust
pub struct OrderBookFormat;

impl DataFormat for OrderBookFormat {
    fn required_columns() -> &'static [&'static str] {
        &["time", "bid", "ask", "bid_volume", "ask_volume"]
    }
    
    fn format_name() -> &'static str {
        "OrderBook"
    }
}
```

2. 創建類型別名：

```rust
pub type OrderBookSeries<F> = FinancialSeries<F, OrderBookFormat>;
```

### 添加新的技術指標

在 `indicators.rs` 中擴展 `IndicatorsExt` trait：

```rust
impl IndicatorsExt for LazyFrame {
    fn custom_indicator(&self, params: CustomParams) -> PolarsResult<LazyFrame> {
        // 實現你的指標邏輯
    }
}
```

## API 參考

### Frequency 枚舉

```rust
pub enum Frequency {
    Tick,
    Minute,
    FiveMinutes,
    FifteenMinutes,
    Hour,
    Day,
    Week,
    Month,
}
```

方法：
- `to_std_duration()` - 轉換為 `std::time::Duration`
- `to_duration()` - 轉換為 Polars Duration
- `to_polars_duration_string()` - 轉換為 Polars 時間字串
- `seconds()` - 獲取頻率的秒數
- `milliseconds()` - 獲取頻率的毫秒數
- `is_ohlcv()` - 檢查是否為 OHLCV 頻率
- `display_name()` - 獲取顯示名稱
- `alias_suffix()` - 獲取別名後綴
- `all()` - 獲取所有頻率列表
- `all_ohlcv()` - 獲取所有 OHLCV 頻率

### FinancialSeries<F, D>

主要方法：
- `new(df: DataFrame, instrument_id: String)` - 創建新的時間序列
- `collect()` - 執行計算並返回 DataFrame
- `filter_date_range()` - 過濾時間範圍
- `select_columns()` - 選擇特定列
- `sort_by_time()` - 按時間排序
- `resample_to<NewF>()` - 重採樣到新頻率

### IndicatorsExt trait

技術指標方法：
- `sma(period: usize)` - 簡單移動平均
- `ema(period: usize)` - 指數移動平均
- `rsi(period: usize)` - 相對強弱指數
- `macd(fast: usize, slow: usize, signal: usize)` - MACD
- `bollinger_bands(period: usize, std_dev: f64)` - 布林帶
- `atr(period: usize)` - 平均真實範圍

## 最佳實踐

1. **使用泛型類型**：直接使用泛型類型如 `OhlcvSeries<Day>` 來獲得編譯時類型安全。

2. **鏈式操作**：利用方法鏈進行多個操作：
   ```rust
   let result = data
       .filter_date_range(start, end)
       .select_columns(&["time", "close", "volume"])
       .sort_by_time(false)
       .collect()?;
   ```

3. **錯誤處理**：所有操作都返回 `Result`，確保正確處理錯誤。

4. **性能考慮**：使用 `LazyFrame` 進行延遲計算，只在需要時調用 `collect()`。

5. **頻率配置**：通過修改 `config/frequencies.toml` 來添加新頻率，而不是修改代碼。

## 常見問題

### Q: 如何處理缺失數據？
A: Polars 原生支持缺失數據處理。使用 `fill_null()` 或 `drop_nulls()` 方法。

### Q: 如何合併多個時間序列？
A: 使用 Polars 的 `join` 功能：
```rust
let merged = series1.lazy_frame()
    .join(
        series2.lazy_frame(),
        ["time"],
        ["time"],
        JoinArgs::default()
    );
```

### Q: 如何保存和加載時間序列？
A: 可以使用 Polars 的序列化功能：
```rust
// 保存為 Parquet
let df = series.collect()?;
df.write_parquet("data.parquet", CompressionOptions::default())?;

// 加載
let df = LazyFrame::scan_parquet("data.parquet", Default::default())?;
let series = OhlcvSeries::<Day>::from_lazy(df, "AAPL".to_string());
```

## 設計理念

### 泛型優於宏生成

新設計採用泛型類型而非宏生成的類型別名，這帶來了幾個優勢：

1. **編譯時類型安全**：編譯器能夠在編譯時檢查類型的正確性
2. **更好的 IDE 支持**：IDE 可以更好地理解和提供代碼補全
3. **減少編譯時間**：避免了大量宏展開帶來的編譯開銷
4. **更簡潔的代碼**：不需要為每個頻率生成獨立的類型別名

### 配置驅動開發

頻率定義完全由 `config/frequencies.toml` 驅動：

1. **單一真相來源**：所有頻率定義集中在一個配置文件中
2. **易於擴展**：添加新頻率只需修改配置文件
3. **編譯時生成**：通過 `build.rs` 在編譯時生成所需代碼
4. **運行時訪問**：通過 `FrequencyConfig` 可以在運行時訪問頻率元數據

## 相關模組

- `data_provider`: 數據加載和提供（使用泛型 DataLoader trait）
- `storage`: 數據持久化
- `backtest`: 回測引擎
- `strategy`: 策略定義和執行