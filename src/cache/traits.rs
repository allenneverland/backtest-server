use crate::storage::models::market_data::{MinuteBar, Tick as DbTick};
use serde::{Deserialize, Serialize};

/// 可快取資料類型的 trait
///
/// 實現此 trait 的類型可以被存儲在多層級快取中。
/// 要求類型支持克隆、序列化、反序列化，並且是線程安全的。
pub trait CacheableData:
    Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static
{
    /// 返回資料類型名稱，用於監控指標
    ///
    /// 此名稱將用於生成監控指標的標籤，應該是：
    /// - 簡短且描述性的
    /// - 使用小寫字母和下劃線
    /// - 在整個應用中保持一致
    ///
    /// # Examples
    /// ```
    /// use backtest_server::cache::traits::CacheableData;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Clone, Serialize, Deserialize)]
    /// struct MyData;
    ///
    /// impl CacheableData for MyData {
    ///     fn data_type_name() -> &'static str {
    ///         "my_data"
    ///     }
    /// }
    /// ```
    fn data_type_name() -> &'static str;
}

// 為市場資料類型實現 CacheableData trait
impl CacheableData for MinuteBar {
    fn data_type_name() -> &'static str {
        "minute_bars"
    }
}

impl CacheableData for DbTick {
    fn data_type_name() -> &'static str {
        "ticks"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minute_bar_data_type_name() {
        assert_eq!(MinuteBar::data_type_name(), "minute_bars");
    }

    #[test]
    fn test_db_tick_data_type_name() {
        assert_eq!(DbTick::data_type_name(), "ticks");
    }
}
