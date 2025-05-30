# 資料快取策略 (Data Caching Strategy)

## 概述

本文檔描述 backtest-server 的多層級資料快取策略實現，旨在減少資料庫查詢壓力並提高資料存取效能。

## 架構設計

### 多層級快取架構

系統實現了兩層快取架構：

1. **L1 - 內存快取**（最快）
   - 使用 LRU (Least Recently Used) 演算法
   - 存取速度：< 100ns
   - 適用於高頻存取的熱數據
   - 容量有限，自動淘汰最少使用的項目

2. **L2 - Redis 快取**（跨進程共享）
   - 持久化存儲，支援跨進程共享
   - 存取速度：< 1ms
   - 適用於需要在多個服務實例間共享的數據
   - 支援 TTL 自動過期

### 快取流程

#### 讀取流程
```
請求數據
  ↓
檢查 L1 內存快取
  ├─ 命中 → 返回數據
  └─ 未命中
      ↓
    檢查 L2 Redis 快取
      ├─ 命中 → 更新 L1 → 返回數據
      └─ 未命中
          ↓
        從資料庫載入
          ↓
        更新 L1 和 L2
          ↓
        返回數據
```

#### 寫入流程
```
寫入數據
  ↓
同時更新
  ├─ L1 內存快取
  └─ L2 Redis 快取（帶 TTL）
```

## 實現細節

### MultiLevelCache 結構

```rust
pub struct MultiLevelCache<P: RedisPool> {
    /// L1: 內存 LRU 快取
    memory_cache: Arc<RwLock<LruCache<String, Vec<u8>>>>,
    /// L2: Redis 快取
    redis_cache: Arc<CacheManager<P>>,
    /// 快取 TTL（秒）
    cache_ttl: u64,
}
```

### 主要方法

1. **get** - 從快取獲取數據
   - 優先查詢內存快取
   - 若未命中，查詢 Redis
   - Redis 命中時自動更新內存快取

2. **set** - 設置快取數據
   - 同時更新內存和 Redis
   - 使用 write-through 策略確保一致性

3. **delete** - 刪除快取項目
   - 同時從兩層快取刪除

4. **warm_cache** - 預熱快取
   - 批量載入常用數據
   - 提高後續查詢效能

### 快取鍵生成

```rust
pub fn generate_cache_key(
    instrument_id: i32,
    frequency: &str,
    start_ts: i64,
    end_ts: i64,
) -> String {
    format!("market_data:{}:{}:{}:{}", 
            instrument_id, frequency, start_ts, end_ts)
}
```

## 使用範例

### 基本使用

```rust
// 創建資料載入器
let loader = MarketDataLoader::new(database)
    .with_redis(redis_pool)
    .with_cache_ttl(300);

// 載入 OHLCV 數據（自動使用快取）
let ohlcv = loader.load_ohlcv::<Hour>(
    instrument_id,
    start_time,
    end_time
).await?;
```

### 快取預熱

```rust
// 預熱常用數據
loader.warm_cache(
    &[100, 101, 102],  // instrument IDs
    &["1m", "1h"],     // frequencies
    start_time,
    end_time
).await?;
```

## 配置選項

### MarketDataLoader 配置

- **cache_ttl_seconds**: 快取過期時間（秒），預設 300
- **memory_capacity**: 內存快取容量，預設 1000 項

### 建議配置

- 高頻交易策略：較短 TTL（60-300 秒）
- 日線級策略：較長 TTL（3600-86400 秒）
- 記憶體受限環境：減少 memory_capacity

## 效能優化

### LRU 淘汰策略

- 自動淘汰最少使用的項目
- 保持熱數據在內存中
- 避免內存無限增長

### 序列化優化

- 使用 bincode 進行高效二進制序列化
- 減少序列化開銷
- 壓縮存儲空間

### 錯誤處理

- 快取錯誤不影響正常流程
- 自動降級到資料庫查詢
- 記錄警告日誌供監控

## 監控指標

### 可用指標

```rust
pub struct CacheStats {
    /// 當前快取項目數
    pub size: usize,
    /// 快取容量
    pub capacity: usize,
}
```

### 建議監控

1. 快取命中率
2. 平均查詢延遲
3. 內存使用量
4. Redis 連接狀態

## 最佳實踐

1. **合理設置 TTL**
   - 根據數據更新頻率設置
   - 避免過長導致數據陳舊
   - 避免過短導致快取失效

2. **選擇性快取**
   - 只快取頻繁存取的數據
   - 避免快取一次性數據
   - 考慮數據大小

3. **快取預熱**
   - 系統啟動時預熱熱門數據
   - 定期預熱即將使用的數據
   - 避免冷啟動問題

4. **監控和調優**
   - 定期檢查快取命中率
   - 根據使用模式調整容量
   - 監控記憶體使用情況

## 故障處理

### Redis 不可用

- 自動降級到僅使用內存快取
- 繼續從資料庫載入數據
- 不影響系統正常運行

### 內存不足

- LRU 自動淘汰舊數據
- 可動態調整快取容量
- 考慮增加服務器記憶體

## 未來改進

1. **快取統計**
   - 實現詳細的命中率統計
   - 提供快取效能報告

2. **智能預熱**
   - 基於歷史使用模式
   - 自動預測熱點數據

3. **分布式一致性**
   - 實現快取失效廣播
   - 確保多實例一致性