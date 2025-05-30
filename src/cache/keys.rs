use rustc_hash::FxHasher;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};

/// 預計算的快取鍵雜湊值，用於加速查找
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CacheKeyHash(pub u64);

impl CacheKeyHash {
    /// 使用 FxHasher 計算鍵的雜湊值
    pub fn new(key: &str) -> Self {
        let mut hasher = FxHasher::default();
        key.hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// 快取鍵構建器，重用內部緩衝區以提升性能
pub struct OptimizedKeyBuilder {
    buffer: Vec<u8>,
    itoa_buffer: itoa::Buffer,
}

impl OptimizedKeyBuilder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(128),
            itoa_buffer: itoa::Buffer::new(),
        }
    }

    pub fn generate_key(
        &mut self,
        instrument_id: i32,
        frequency: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> &str {
        self.buffer.clear();

        // 手動拼接以重用 itoa_buffer，避免重複分配
        self.buffer.extend_from_slice(b"market_data:");
        self.buffer
            .extend_from_slice(self.itoa_buffer.format(instrument_id).as_bytes());
        self.buffer.push(b':');
        self.buffer.extend_from_slice(frequency.as_bytes());
        self.buffer.push(b':');
        self.buffer
            .extend_from_slice(self.itoa_buffer.format(start_ts).as_bytes());
        self.buffer.push(b':');
        self.buffer
            .extend_from_slice(self.itoa_buffer.format(end_ts).as_bytes());

        // 安全：我們知道內容是有效的 UTF-8
        unsafe { std::str::from_utf8_unchecked(&self.buffer) }
    }

    pub fn generate_key_owned(
        &mut self,
        instrument_id: i32,
        frequency: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> String {
        self.generate_key(instrument_id, frequency, start_ts, end_ts)
            .to_owned()
    }
}

impl Default for OptimizedKeyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static KEY_BUILDER: RefCell<OptimizedKeyBuilder> = RefCell::new(OptimizedKeyBuilder::new());
}

/// 生成市場數據快取鍵（優化版本）
///
/// 使用高性能的實現，適合高頻調用場景。
/// 使用 thread_local 重用內部緩衝區，避免頻繁的記憶體分配。
///
/// # Arguments
/// * `instrument_id` - 金融工具 ID
/// * `frequency` - 數據頻率
/// * `start_ts` - 開始時間戳
/// * `end_ts` - 結束時間戳
pub fn generate_cache_key(
    instrument_id: i32,
    frequency: &str,
    start_ts: i64,
    end_ts: i64,
) -> String {
    KEY_BUILDER.with(|builder| {
        builder
            .borrow_mut()
            .generate_key_owned(instrument_id, frequency, start_ts, end_ts)
    })
}
