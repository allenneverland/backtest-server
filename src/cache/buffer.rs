/// 快取緩衝區，用於減少記憶體分配
pub struct CacheBuffer {
    /// 快取鍵緩衝區
    pub keys: Vec<String>,
    /// 索引緩衝區
    pub indices: Vec<usize>,
}

impl CacheBuffer {
    /// 創建具有指定容量的緩衝區
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            keys: Vec::with_capacity(cap),
            indices: Vec::with_capacity(cap),
        }
    }

    /// 清空緩衝區內容（保留容量）
    pub fn clear(&mut self) {
        self.keys.clear();
        self.indices.clear();
    }
}
