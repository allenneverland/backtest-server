use crate::storage::models::market_data::{MinuteBar, Tick as DbTick};
use serde::{Deserialize, Serialize};

/// 可快取的數據特徵
pub trait Cacheable:
    Clone + Send + Sync + std::fmt::Debug + Serialize + for<'de> Deserialize<'de> + 'static
{
    /// 快取鍵前綴
    const CACHE_PREFIX: &'static str;
}

impl Cacheable for Vec<MinuteBar> {
    const CACHE_PREFIX: &'static str = "minute_bars";
}

impl Cacheable for Vec<DbTick> {
    const CACHE_PREFIX: &'static str = "ticks";
}
