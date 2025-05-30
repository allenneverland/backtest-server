/// 單一快取統計信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 當前快取項目數
    pub size: usize,
    /// 快取容量
    pub capacity: usize,
}

/// 多快取統計信息
#[derive(Debug, Clone)]
pub struct MultiCacheStats {
    /// MinuteBars 快取統計
    pub minute_bars: CacheStats,
    /// Ticks 快取統計
    pub ticks: CacheStats,
    /// Hash 映射大小
    pub mapping_size: usize,
}
