# 回測伺服器專案任務清單（含套件建議）

## 圖標說明
- 🔴 高優先級：必須優先完成的關鍵任務
- 🟡 中優先級：重要但可靈活安排的任務
- 🟢 低優先級：有助於提升系統功能但非緊急的任務
- ⚡ 依賴關係：表示此任務依賴於其他任務（括號中說明依賴項）
- ✅ 已完成：已經完成的任務
- 🚧 進行中：正在進行的任務
- 📋 待辦：尚未開始的任務
- 🚀 MVP：最小可行產品所需的任務

## 第一階段：專案設置和基礎架構（1-2週）

### 1. 專案初始化
- ✅ 設置Git倉庫
- ✅ 初始化Cargo專案結構
- ✅ 設置開發環境（Rust工具鏈、編輯器配置）
- 🚧 配置CI/CD流程（GitHub Actions）
- ✅ 建立開發指南文檔

### 2. 項目基礎設施
- ✅ 設置Docker開發環境
- ✅ 創建TimescaleDB資料庫Docker配置
- ✅ 配置cargo-make任務
- ✅ 建立開發和測試環境分離配置 `[config, toml]`
- 📋 🔴 創建RabbitMQ Docker配置 `[docker-compose]`

### 3. 領域類型建立
- ✅ 實現基本資產類型（`asset_types.rs`）`[serde, rust_decimal, thiserror]`
- ✅ 實現時間序列數據結構（`time_series.rs`）`[chrono, serde, ndarray]`
- ✅ 實現數據點結構（`data_point.rs`）`[serde, chrono, rust_decimal]`
- ✅ 實現頻率枚舉（`frequency.rs`）`[serde, chrono]`
- ✅ 實現數據聚合操作（`aggregation.rs`）`[ndarray, serde]`
- ✅ 實現資料調整相關結構（`adjustment.rs`）`[rust_decimal, serde]`

### 4. 數據庫結構
- ✅ 設計並實現資料庫基本表結構 `[sqlx]`
- ✅ 實現遷移腳本（基本表）`[sqlx]`
- ✅ 設計並實現資料庫索引優化 `[sqlx]`
- ✅ 建立數據庫連接管理（`database.rs`）`[sqlx, tokio, async-trait]`

## 第二階段：核心數據功能（2-3週）

### 新增：Redis存儲模組
- ✅ 🔴 實現Redis客戶端（`redis/client.rs`）`[redis, tokio, async-trait]` ⚡(依賴任務7)
- ✅ 🔴 實現Redis連接池（`redis/pool.rs`）`[bb8-redis, tokio]` ⚡(依賴任務21.1)
- ✅ 🟡 實現快取操作（`redis/operations/cache.rs`）`[redis, serde, serde_json]` ⚡(依賴任務21.2)
- 📋 🟡 實現發布/訂閱操作（`redis/operations/pubsub.rs`）`[redis, tokio, futures]` ⚡(依賴任務21.2)
- 📋 🟢 實現任務佇列操作（`redis/operations/queue.rs`）`[redis, tokio, serde_json]` ⚡(依賴任務21.2)
- 📋 🟢 實現分散式鎖（`redis/operations/lock.rs`）`[redis, tokio, parking_lot]` ⚡(依賴任務21.2)

### 新增：RabbitMQ消息系統模組
- ✅ 實現RabbitMQ連接管理（`messaging/rabbitmq/connection.rs`）`[lapin, tokio, deadpool-lapin]` ⚡(依賴任務7)
- ✅ 🔴 🚀 實現消息代理（`messaging/rabbitmq/broker.rs`）`[lapin, tokio, async-trait]` ⚡(依賴任務15.1)
- ✅ 🔴 實現消息客戶端（`messaging/rabbitmq/client.rs`）`[lapin, tokio, uuid]` ⚡(依賴任務15.1)
- ✅ 🔴 實現通訊協議（`messaging/protocol.rs`）`[serde, chrono, uuid]`
- ✅ 🟡 實現RPC模式（`messaging/rabbitmq/rpc.rs`）`[lapin, tokio, futures]` ⚡(依賴任務15.1, 15.2)
- ✅ 🟡 實現消息處理器（`messaging/models/`）`[lapin, serde_json, tokio]` ⚡(依賴任務15.2, 15.3)
- ✅ 🟡 實現消息模型（`messaging/models/`）`[serde, chrono, serde_json]`
- 📋 🟢 實現消息認證（`messaging/auth.rs`）`[jsonwebtoken, sha2]` ⚡(依賴任務15.2)

### 5. 數據導入模組
- ✅ 實現CSV讀取功能（`csv_io.rs`）`[csv, serde, tokio]` ⚡(依賴任務3)
- ✅ 實現數據驗證流程（`validator.rs`）`[thiserror, serde]` ⚡(依賴任務3)
- ✅ 實現OHLCV數據驗證與清洗（`ohlcv_validator.rs`, `ohlcv_cleaner.rs`）`[chrono, rust_decimal, thiserror]`
- ✅ 實現Tick數據驗證與清洗（`tick_validator.rs`, `tick_cleaner.rs`）`[chrono, rust_decimal, thiserror]`
- ✅ 實現時間序列整體驗證（`time_series_validator.rs`）`[chrono, statrs]`
- ✅ 實現驗證器註冊表（`registry.rs`）`[once_cell]`
- 📋 🟢 實現驗證報告生成（`report.rs`）`[serde, serde_json]`

### 6. 數據提供模組
- 📋 🔴 🚀 實現統一數據加載器（`loader.rs`）`[tokio, sqlx, async-trait]` ⚡(依賴任務3, 4)
- 📋 🔴 🚀 實現時間序列重採樣（`resampler.rs`）`[chrono, ndarray, polars]` ⚡(依賴任務3)
- 📋 🔴 實現市場數據迭代器（`iterator.rs`）`[tokio, futures, async-trait]`
- 📋 🟡 實現數據緩存管理（`cache.rs`）`[redis, parking_lot, lru_time_cache]`
- 📋 🟡 實現技術指標計算（`precalculator.rs`）`[ndarray, statrs, rayon]`

### 7. 配置管理模組
- ✅ 實現配置加載功能（`loader.rs`）`[config, serde, toml]`
- ✅ 實現配置驗證（`validation.rs`）`[thiserror, serde]`
- ✅ 設定默認配置值（`defaults.rs`）`[once_cell, serde]`
- ✅ 實現環境變量支持 `[config]`
- ✅ 實現RabbitMQ配置 `[config, serde, toml]`

## 第三階段：回測與執行模組（3-4週）

### 8. 執行模擬器模組
- 📋 🔴 🚀 實現訂單和交易類型（`types/order.rs`, `types/trade.rs`）`[serde, rust_decimal, uuid, chrono]` ⚡(依賴任務3)
- 📋 🔴 🚀 實現訂單執行模擬器（`simulator.rs`）`[tokio, rust_decimal, thiserror]` ⚡(依賴任務8.1)
- 📋 🔴 實現倉位和資產管理（`position.rs`）`[rust_decimal, serde, parking_lot]` ⚡(依賴任務8.1)
- 📋 🟡 實現訂單匹配引擎（`matching.rs`）`[rust_decimal, tokio, dashmap]` ⚡(依賴任務8.1, 8.2)
- 📋 🟢 增加複雜訂單類型支持 `[serde, thiserror]`

### 9. 風險管理模組
- 📋 🔴 🚀 實現風險檢查器（`checker.rs`）`[rust_decimal, async-trait, thiserror]` ⚡(依賴任務8)
- 📋 🟡 實現風險限制（`limits.rs`）`[serde, rust_decimal]`
- 📋 🟡 實現風險指標計算（`metrics.rs`）`[statrs, ndarray, chrono]`

### 10. 回測模組
- 📋 🔴 🚀 實現回測引擎核心（`engine.rs`）`[tokio, async-trait, parking_lot]` ⚡(依賴任務6, 8)
- 📋 🔴 🚀 實現回測任務管理（`task.rs`）`[tokio, serde, uuid]`
- 📋 🔴 🚀 實現回測結果處理（`results.rs`）`[serde, chrono, rust_decimal]`
- 📋 🔴 實現回測執行上下文（`context.rs`）`[tokio, parking_lot, serde]` ⚡(依賴任務10.1)
- 📋 🟡 實現性能指標計算（`metrics.rs`）`[statrs, ndarray, chrono, rust_decimal, polars]` ⚡(依賴任務10.3)
- 📋 🟡 實現回測進度監控（`progress.rs`）`[tokio, serde_json]`
- 📋 🟡 實現回測結果存儲（`storage.rs`）`[sqlx, tokio, serde_json]` ⚡(依賴任務4, 10.3)
- 📋 🟢 實現回測執行調度器（`executor.rs`）`[tokio, futures, rayon]` ⚡(依賴任務10.1, 10.2)

### 11. 事件處理系統模組
- 📋 🔴 實現事件總線（`bus.rs`）`[tokio, futures, parking_lot]`
- 📋 🟡 實現事件佇列（`queue.rs`）`[tokio, crossbeam]`
- 📋 🟡 實現事件分發器（`dispatcher.rs`）`[tokio, futures, async-trait]` ⚡(依賴任務11.1, 11.2)
- 📋 🟡 將事件發布整合到RabbitMQ `[lapin, tokio]` ⚡(依賴任務11.1, 15.1)

## 第四階段：策略與隔離運行時（3-4週）

### 12. 策略DSL模組
- 📋 🔴 🚀 實現DSL語法解析器（`parser.rs`）`[serde_yaml_bw, thiserror]`
- 📋 🔴 🚀 實現DSL運行時（`runtime.rs`）`[tokio, thiserror, parking_lot]` ⚡(依賴任務12.1)
- 📋 🔴 實現DSL標準庫（`stdlib.rs`）`[rust_decimal, chrono, statrs]` ⚡(依賴任務12.2)
- 📋 🟡 實現DSL編譯器（`compiler.rs`）`[serde_yaml_bw, thiserror]` ⚡(依賴任務12.1)

### 13. 隔離運行時模組
- 📋 🔴 🚀 實現策略沙箱（`sandbox.rs`）`[tokio, parking_lot, thiserror]` ⚡(依賴任務12)
- 📋 🟡 實現資源管理（`resource.rs`）`[parking_lot, mimalloc]`
- 📋 🟡 實現錯誤處理機制（`error.rs`）`[thiserror, anyhow]`

### 14. 策略管理模組
- 📋 🔴 🚀 實現策略加載器（`loader.rs`）`[tokio, serde_yaml_bw, glob]` ⚡(依賴任務12, 13)
- 📋 🔴 實現策略生命週期管理（`lifecycle.rs`）`[tokio, parking_lot]` ⚡(依賴任務14.1)
- 📋 🔴 實現策略執行上下文（`context.rs`）`[tokio, parking_lot, serde]` ⚡(依賴任務14.1)
- 📋 🟡 實現策略版本管理（`version/manager.rs`）`[semver, serde]` ⚡(依賴任務14.1)
- 📋 🟡 實現策略註冊表（`registry.rs`）`[dashmap, uuid]` ⚡(依賴任務14.1)
- 📋 🟢 實現策略快照管理（`snapshot.rs`）`[serde, serde_json, chrono]` ⚡(依賴任務14.3)
- 📋 🟢 實現配置文件監控（`config_watcher.rs`）`[tokio, futures, glob]`

## 第五階段：消息系統集成與伺服器功能（2-3週）

### 15. 消息系統集成
- 📋 🔴 🚀 實現回測消息處理器（`messaging/handlers/backtest.rs`）`[lapin, tokio, serde_json]` ⚡(依賴任務10, 15.2)
- 📋 🔴 🚀 實現策略消息處理器（`messaging/handlers/strategy.rs`）`[lapin, tokio, serde_json]` ⚡(依賴任務14, 15.2)
- 📋 🔴 實現數據消息處理器（`messaging/handlers/data.rs`）`[lapin, tokio, serde_json]` ⚡(依賴任務5, 6, 15.2)
- 📋 🟡 實現消息協議文檔 `[markdown]`
- 📋 🟡 實現消息錯誤處理 `[thiserror, serde_json]`
- ✅ 🟢 實現消息響應格式標準化 `[serde, serde_json]`

### 16. 伺服器模組
- 📋 🔴 🚀 實現伺服器構建器（`builder.rs`）`[lapin, tokio]` ⚡(依賴任務15)
- 📋 🔴 實現伺服器配置結構（`config.rs`）`[serde, config]`
- 📋 🟡 實現伺服器錯誤處理（`error.rs`）`[thiserror, tracing]`
- 📋 🟡 實現優雅關閉機制 `[tokio, futures]`

## 第六階段：集成與測試（3-4週）

### 17. 自動化測試
- 📋 🔴 🚀 實現數據導入和提供模組測試 `[mockall, tokio-test, fake]`
- 📋 🔴 實現回測系統測試 `[mockall, tokio-test, proptest]`
- 📋 🟡 實現策略DSL測試 `[assert_matches, test-case]`
- 📋 🟡 實現消息系統測試 `[tokio-test, lapin-test-utils]`
- 📋 🟢 實現性能基準測試 `[criterion, fake]`

### 18. 示例代碼
- 📋 🔴 實現簡單策略示例 `[serde_yaml_bw, chrono, rust_decimal]`
- 📋 🟡 實現完整回測流程示例 `[tokio, serde_yaml_bw, chrono]`
- 📋 🟡 實現消息客戶端示例 `[lapin, serde_json, tokio]`
- 📋 🟢 實現系統監控示例 `[lapin, tokio, tracing]`

### 19. 文檔
- 📋 🔴 🚀 完善消息協議參考文檔
- 📋 🔴 完善使用者指南
- 📋 🟡 建立開發者文檔
- 📋 🟢 建立部署指南

### 20. 部署
- 📋 🔴 建立生產環境配置 `[config, toml, rabbitmq-conf]`
- 📋 🟡 優化Docker配置
- 📋 🟡 建立自動部署流程
- 📋 🟢 實現性能監控和日誌管理 `[tracing, metrics-exporter-prometheus]`