# backtest-server 專案結構設計

## 目錄

- [1. 專案概述](#1-專案概述)
- [2. 目錄結構](#2-目錄結構)
  - [2.1 專案根目錄結構](#21-專案根目錄結構)
  - [2.2 文檔與配置目錄](#22-文檔與配置目錄)
  - [2.3 核心源代碼結構](#23-核心源代碼結構)
  - [2.4 功能模組目錄結構](#24-功能模組目錄結構)
  - [2.5 測試與示例目錄](#25-測試與示例目錄)
- [3. 核心模組](#3-核心模組)
  - [3.1 領域類型模組](#31-領域類型模組)
  - [3.2 數據提供模組](#32-數據提供模組)
  - [3.3 策略DSL模組](#33-策略dsl模組)
  - [3.4 事件處理系統模組](#34-事件處理系統模組)
  - [3.5 隔離運行時模組](#35-隔離運行時模組)
  - [3.6 執行模擬器模組](#36-執行模擬器模組)
  - [3.7 風險管理模組](#37-風險管理模組)
  - [3.8 消息系統模組](#38-消息系統模組)
  - [3.9 配置管理模組](#39-配置管理模組)
  - [3.10 伺服器模組](#310-伺服器模組)
  - [3.11 回測模組](#311-回測模組)
  - [3.12 存儲系統模組](#312-存儲系統模組)
- [4. 配置文件](#4-配置文件)
- [5. 資料庫結構](#5-資料庫結構)
  - [5.1 數據庫遷移文件](#51-數據庫遷移文件)
  - [5.2 數據庫模型](#52-數據庫模型)
- [6. Docker環境](#6-docker環境)
- [7. 自動化測試](#7-自動化測試)
- [8. 效能基準測試](#8-效能基準測試)
- [9. 示例代碼](#9-示例代碼)
- [10. 文檔](#10-文檔)
- [11. Rust 模組系統與組織方式](#11-rust-模組系統與組織方式)
  - [11.1 模組結構基本原則](#111-模組結構基本原則)
  - [11.2 好處與最佳實踐](#112-好處與最佳實踐)
  - [11.3 使用示例](#113-使用示例)
- [12. 策略版本管理系統](#12-策略版本管理系統)
  - [12.1 版本儲存結構](#121-版本儲存結構)
  - [12.2 版本命名規範](#122-版本命名規範)
  - [12.3 版本存取與管理](#123-版本存取與管理)
- [13. 回測系統架構](#13-回測系統架構)
  - [13.1 事件驅動架構](#131-事件驅動架構)
  - [13.2 統一資金管理](#132-統一資金管理)
- [14. 快取系統架構](#14-快取系統架構)
  - [14.1 快取層級](#141-快取層級)
  - [14.2 排名快取設計](#142-排名快取設計)

## 1. 專案概述

backtest-server 是一個使用 Rust 開發的高效能金融回測伺服器，專為支持多策略和動態策略管理設計。系統採用事件驅動架構，實現真實的市場環境模擬和統一資金管理。本文檔詳細描述了專案的整體結構設計。

## 2. 目錄結構

### 2.1 專案根目錄結構

```
backtest_server/                # 專案根目錄
├── Cargo.toml                  # 專案配置和依賴管理
├── Cargo.lock                  # 鎖定的依賴版本
├── Makefile.toml               # cargo-make任務定義
├── .cargo/                     # Cargo配置目錄
│   └── config.toml             # Cargo配置文件
├── Dockerfile                  # Docker容器定義
├── Dockerfile.db               # TimescaleDB數據庫容器定義
├── Dockerfile.rabbitmq         # RabbitMQ容器定義
├── docker-compose.yml          # Docker Compose配置
├── docs/                       # 文檔目錄
├── config/                     # 配置文件目錄
├── scripts/                    # 輔助腳本目錄
├── src/                        # 源代碼目錄
├── cache/                      # 快取目錄
│   ├── rankings/               # 排名快取檔案
│   └── indicators/             # 技術指標快取
├── migrations/                 # 數據庫遷移文件目錄
├── tests/                      # 集成測試目錄
├── benches/                    # 性能基準測試目錄
├── examples/                   # 示例代碼目錄
└── README.md                   # 專案說明文檔
```

### 2.2 文檔與配置目錄

```
docs/                           # 文檔目錄
├── PLANNING.md                 # 專案規劃文檔
├── TASK.md                     # 任務清單文檔
├── STRUCTURE.md                # 結構說明文檔
├── BACKTEST_ARCHITECTURE.md    # 回測系統架構文檔
├── EVENT_DRIVEN_DESIGN.md      # 事件驅動設計文檔
├── CACHE_STRATEGY.md           # 快取策略文檔
└── DB_SCHEMA.md                # 數據庫結構文檔

config/                         # 配置文件目錄
├── development.toml            # 開發環境配置
├── production.toml             # 生產環境配置
├── cache.toml                  # 快取配置
├── backtest.toml               # 回測配置
└── rabbitmq.conf               # RabbitMQ配置文件

scripts/                        # 輔助腳本目錄
├── db/                         # 數據庫相關腳本
│   └── init.sql                # 數據庫初始化腳本
├── cache/                      # 快取相關腳本
│   ├── build_ranking_cache.sh  # 構建排名快取
│   └── clear_cache.sh          # 清理快取
├── rabbitmq/                   # RabbitMQ相關腳本
│   ├── init.sh                 # RabbitMQ初始化腳本
│   └── definitions.json        # RabbitMQ預設定義
├── install.sh                  # 安裝依賴腳本
├── setup_db.sh                 # 設置數據庫腳本
└── benchmark.sh                # 運行基準測試腳本
```

### 2.3 核心源代碼結構

```
src/                            # 源代碼目錄
├── bin/                        # 程序入口點
│   ├── main.rs                 # 主程序入口
│   ├── migrate.rs              # 數據庫遷移工具入口
│   └── cache_builder.rs        # 快取構建工具
├── lib.rs                      # 庫入口點，宣告主要模組
├── config.rs                   # 配置管理模組，宣告子模組
├── domain_types.rs             # 核心領域類型模組
├── data_provider.rs            # 數據提供模組
├── execution.rs                # 執行模擬器模組，宣告子模組
├── strategy.rs                 # 策略管理模組，宣告子模組
├── dsl.rs                      # DSL解釋器模組，宣告子模組
├── event.rs                    # 事件處理系統模組，宣告子模組
├── runtime.rs                  # 隔離運行時模組，宣告子模組
├── risk.rs                     # 風險管理模組，宣告子模組
├── messaging.rs                # 消息系統模組，宣告子模組
├── server.rs                   # 伺服器核心組件，宣告子模組
├── storage.rs                  # 存儲系統模組，宣告子模組
├── backtest.rs                 # 回測系統模組，宣告子模組
├── cache.rs                    # 快取系統模組，宣告子模組
└── utils.rs                    # 公共工具模組，宣告子模組
```

### 2.4 功能模組目錄結構

```
# 數據相關模組
src/domain_types/                   # 核心領域類型模組目錄
├── types.rs                        # 基本類型定義 (資產類型、頻率等)
├── instrument.rs                   # 金融商品結構
├── frequency.rs                    # 頻率定義
├── frame.rs                        # 基於 Polars 的市場數據框架
├── series.rs                       # 時間序列相關功能
├── indicators.rs                   # 基本技術指標
├── event.rs                        # 市場事件類型定義
└── portfolio.rs                    # 投資組合類型


src/data_provider/                  # 數據提供模組目錄
├── types.rs                        # 核心類型定義
├── loader.rs                       # 統一的數據加載器實現
├── cache.rs                        # 數據緩存管理（分層快取）
├── resampler.rs                    # 時間序列重採樣功能
├── precalculator.rs                # 技術指標計算
├── iterator.rs                     # 市場數據迭代器實現
├── ranking_cache.rs                # 排名快取系統入口
└── ranking_cache/                  # 排名快取子模組
    ├── memory.rs                   # 內存快取實現
    ├── persistent.rs               # 本地檔案持久化
    ├── builder.rs                  # 排名預處理器
    └── types.rs                    # 排名相關類型

# 策略與執行相關模組
src/strategy/                       # 策略管理模組目錄（不存儲策略）
├── parser.rs                       # 策略 DSL 解析器（從 RabbitMQ 接收）
├── executor.rs                     # 策略執行器
├── context.rs                      # 策略執行上下文（含資金狀態）
├── runtime.rs                      # 策略運行時狀態管理
└── types.rs                        # 策略執行相關類型

src/execution/                      # 執行模擬器模組目錄
├── simulator.rs                    # 訂單執行模擬器
├── matching.rs                     # 訂單匹配引擎
├── position.rs                     # 倉位和資產管理
├── portfolio.rs                    # 統一投資組合管理
├── types.rs                        # 執行相關類型宣告與重新導出
└── types/                          # 執行相關類型子模組目錄
    ├── order.rs                    # 訂單定義
    ├── trade.rs                    # 交易記錄定義
    └── fill.rs                     # 成交記錄定義

src/dsl/                            # DSL解釋器模組目錄
├── parser.rs                       # DSL語法解析器
├── runtime.rs                      # DSL運行時
├── stdlib.rs                       # DSL標準庫（含資金查詢）
└── compiler.rs                     # DSL編譯器

src/risk/                           # 風險管理模組目錄
├── checker.rs                      # 風險檢查器
├── limits.rs                       # 風險限制（含資金限制）
├── metrics.rs                      # 風險指標
└── position_sizing.rs              # 倉位大小計算

# 系統與運行時模組
src/runtime/                        # 隔離運行時模組目錄
├── sandbox.rs                      # 策略沙箱
├── resource.rs                     # 資源管理
└── error.rs                        # 錯誤處理

src/event/                          # 事件處理系統模組目錄
├── bus.rs                          # 事件總線
├── queue.rs                        # 事件優先級佇列
├── dispatcher.rs                   # 事件分發器
├── types.rs                        # 事件類型定義
└── generator.rs                    # 事件生成器

# 快取系統模組
src/cache/                          # 快取系統模組目錄
├── hierarchy.rs                    # 分層快取管理
├── memory.rs                       # 內存快取實現
├── file.rs                         # 檔案快取實現
├── redis.rs                        # Redis快取介面
├── strategy.rs                     # 快取策略
└── metrics.rs                      # 快取指標監控

# 消息系統模組
src/messaging/                      # 消息系統模組目錄
├── rabbitmq/                       # RabbitMQ實現
│   ├── connection.rs               # 連接管理
│   ├── broker.rs                   # 消息代理實現
│   ├── client.rs                   # 客戶端實現
│   ├── consumer.rs                 # 消費者實現
│   ├── publisher.rs                # 發布者實現
│   ├── rpc.rs                      # RPC實現
│   └── error.rs                    # 錯誤處理
├── protocol.rs                     # 通訊協議定義
├── handlers/                       # 消息處理器目錄
│   ├── backtest.rs                 # 回測相關消息處理
│   ├── strategy.rs                 # 策略相關消息處理
│   ├── data.rs                     # 數據相關消息處理
│   └── ranking.rs                  # 排名查詢處理
├── models/                         # 消息模型目錄
│   ├── commands.rs                 # 命令消息模型
│   ├── events.rs                   # 事件消息模型
│   └── responses.rs                # 回應消息模型
├── rabbitmq.rs                     # RabbitMQ導出
├── models.rs                       # 模組定義與導出
└── auth.rs                         # 消息認證和授權

# 回測系統模組
src/backtest/                       # 回測系統模組目錄
├── engine.rs                       # 事件驅動回測引擎核心
├── task.rs                         # 回測任務管理
├── results.rs                      # 回測結果處理
├── progress.rs                     # 回測進度監控
├── executor.rs                     # 回測執行調度器
├── context.rs                      # 回測執行上下文（統一資金池）
├── metrics.rs                      # 回測性能指標計算
├── storage.rs                      # 回測結果存儲
├── event_generator.rs              # 市場事件生成器
├── event_queue.rs                  # 事件優先級佇列
├── portfolio.rs                    # 統一投資組合管理
└── time_slicer.rs                  # 時間片執行優化

# 服務與消息系統模組
src/server/                         # 伺服器模組目錄
├── builder.rs                      # 伺服器構建器模式實現
├── error.rs                        # 伺服器級別錯誤處理

# 基礎設施模組
src/config/                         # 配置管理模組目錄
├── loader.rs                       # 配置加載（環境變量、文件等）
├── validation.rs                   # 配置驗證
├── cache_config.rs                 # 快取配置結構
└── backtest_config.rs              # 回測配置結構

src/storage/                        # 存儲系統模組目錄
├── database.rs                     # 數據庫連接管理
├── models.rs                       # 數據模型
├── migrations.rs                   # 數據庫遷移管理(對應/migrations目錄)
├── external_db.rs                  # 外部市場數據庫連接
└── redis/                          # Redis模組目錄
    ├── client.rs                   # Redis客戶端實現
    ├── config.rs                   # Redis配置結構
    ├── error.rs                    # Redis錯誤處理
    ├── pool.rs                     # 連接池管理
    └── operations/                 # 特定業務操作封裝
        ├── cache.rs                # 通用快取操作
        ├── pubsub.rs               # 發布/訂閱操作
        ├── queue.rs                # 任務佇列操作
        └── lock.rs                 # 分散式鎖實現

src/utils/                          # 公共工具模組目錄
├── time_utils.rs                   # 時間轉換工具，處理不同層間的時間格式轉換
├── error.rs                        # 通用錯誤處理
├── metrics.rs                      # 系統指標收集
└── binary_heap.rs                  # 自定義二元堆實現（事件優先級）
```

### 2.5 測試與示例目錄

```
tests/                              # 集成測試目錄
├── data_provider_tests.rs          # 數據提供模組測試
├── ranking_cache_tests.rs          # 排名快取系統測試
├── strategy_tests.rs               # 策略模組測試
├── backtest_tests.rs               # 回測系統測試
├── event_driven_tests.rs           # 事件驅動架構測試
├── portfolio_tests.rs              # 投資組合管理測試
├── messaging_tests.rs              # 消息系統測試
└── dsl_tests.rs                    # DSL解釋器測試

benches/                            # 性能基準測試目錄
├── data_loading.rs                 # 數據加載性能測試
├── stock_filtering.rs              # 股票篩選性能測試
├── ranking_cache_performance.rs    # 排名快取性能測試
├── event_processing.rs             # 事件處理性能測試
├── strategy_execution.rs           # 策略執行性能測試
├── cache_comparison.rs             # 快取層級對比測試
└── messaging_performance.rs        # 消息系統性能測試

examples/                           # 示例代碼目錄
├── simple_strategy.rs              # 簡單策略示例
├── day_trading_strategy.rs         # 當沖策略示例
├── event_driven_backtest.rs        # 事件驅動回測示例
├── ranking_cache_usage.rs          # 排名快取使用示例
├── backtest_runner.rs              # 回測運行器示例
└── messaging_client.rs             # 消息客戶端示例
```

## 3. 核心模組

### 3.1 領域類型模組 (domain_types)

此模組定義了整個應用程序中使用的核心金融數據結構和類型，基於 Polars 提供高效的數據處理能力，並支援事件驅動架構。

**注意**：此模組不包含策略定義相關類型，策略由外部 StratPlat 系統管理。

**主要功能**:
- 提供標準化的金融市場數據表示
- 定義資產類型和交易枚舉
- 整合 Polars 資料結構，實現高效率的數據操作
- 定義事件類型支援事件驅動架構
- 為應用程序提供統一的數據類型系統

**主要組件** (`src/domain_types/`):
- `types.rs`: 基本類型定義 (資產類型、頻率等)
- `instrument.rs`: 金融商品結構
- `frame.rs`: 基於 Polars 的市場數據框架
- `series.rs`: 時間序列相關功能
- `indicators.rs`: 基本技術指標
- `event.rs`: 市場事件類型定義（MarketOpen、TickData 等）
- `portfolio.rs`: 投資組合和資金管理類型

### 3.2 數據提供模組 (data_provider)

此模組作為回測系統的數據來源，專注於高效率地從外部資料庫提取數據，並提供優化的查詢、數據轉換和排名快取功能。

**主要功能**:
- 從外部資料庫高效讀取市場數據
- 提供統一的數據訪問接口給回測系統
- 實現多層快取策略，優化頻繁訪問的數據讀取
- 管理股票排名快取系統
- 執行數據轉換操作，如頻率轉換、技術指標計算等
- 支持流式數據處理和惰性計算

**主要組件** (`src/data_provider/`):
- `loader.rs`: 高層資料加載邏輯，整合排名快取
- `cache.rs`: 實現多層緩存策略（內存優先）
- `iterator.rs`: 提供高效的市場數據迭代器
- `precalculator.rs`: 基於 Polars 向量化操作實現技術指標預計算
- `ranking_cache.rs`: 排名快取系統入口
- `ranking_cache/memory.rs`: 基於 BTreeMap 的內存快取
- `ranking_cache/persistent.rs`: 二進制序列化的本地檔案快取
- `ranking_cache/builder.rs`: 從外部數據庫預處理排名數據

**排名快取設計重點**:
- 預處理：啟動時計算所有歷史排名
- 內存存儲：O(1) 查詢時間
- 增量更新：只計算新增日期
- 壓縮存儲：減少內存佔用

### 3.3 策略DSL模組

策略DSL模組提供了一種專用的領域特定語言，簡化策略開發並確保策略執行的安全性和一致性，支援統一資金管理。

**主要功能**:
- 解析策略DSL代碼
- 提供安全的執行環境
- 實現DSL標準庫和內建函數
- 支持策略編譯優化
- 提供資金查詢和管理接口

**主要組件** (`src/dsl/`):
- `parser.rs`: 將DSL代碼解析為抽象語法樹
- `runtime.rs`: 執行解析後的DSL代碼
- `stdlib.rs`: 提供DSL標準庫功能（包含資金管理函數）
- `compiler.rs`: 將DSL代碼編譯為優化的中間表示

**資金管理功能**:
```rust
// DSL 中可用的資金函數
available_cash()      // 查詢可用資金
position_value(symbol) // 查詢持倉價值
total_equity()        // 查詢總權益
can_afford(order)     // 檢查是否有足夠資金
```

### 3.4 事件處理系統模組

事件處理系統負責系統中各個組件之間的消息傳遞和事件處理，是實現事件驅動回測的核心。

**主要功能**:
- 實現事件發布/訂閱機制
- 管理事件優先級佇列
- 提供異步事件處理
- 支持事件過濾和路由
- 確保事件按時間順序處理

**主要組件** (`src/event/`):
- `bus.rs`: 實現中央事件總線，用於事件發布和訂閱
- `queue.rs`: 基於 BinaryHeap 的優先級事件佇列
- `dispatcher.rs`: 管理事件分發邏輯
- `types.rs`: 定義所有事件類型
- `generator.rs`: 從市場數據生成事件流

**事件優先級設計**:
```rust
// 事件按時間戳排序，同時間按類型優先級
1. MarketOpen/Close
2. OrderFilled
3. TickData/MinuteBar
4. PositionUpdate
```

### 3.5 隔離運行時模組

隔離運行時模組確保策略在受控環境中執行，防止惡意代碼和資源濫用。

**主要功能**:
- 提供策略沙箱環境
- 管理資源配額和限制
- 處理策略錯誤和異常
- 限制系統資源訪問

**主要組件** (`src/runtime/`):
- `sandbox.rs`: 實現策略沙箱，限制策略的訪問權限
- `resource.rs`: 管理策略使用的計算資源
- `error.rs`: 處理運行時錯誤和異常

### 3.6 執行模擬器模組

執行模擬器負責模擬市場中的訂單執行過程，包括處理不同類型的訂單、計算滑點和佣金等，並管理統一的投資組合。

**主要功能**:
- 模擬不同類型的訂單執行
- 計算滑點和交易成本
- 管理統一資金池
- 追蹤所有持倉
- 生成執行報告

**主要組件** (`src/execution/`):
- `simulator.rs`: 實現訂單執行模擬器
- `matching.rs`: 實現訂單匹配引擎
- `position.rs`: 管理個別股票倉位
- `portfolio.rs`: 統一投資組合和資金管理
- `types.rs`: 定義執行相關的核心類型
- `types/order.rs`: 定義訂單類型和結構
- `types/trade.rs`: 定義交易記錄結構
- `types/fill.rs`: 定義成交記錄結構

**統一資金管理**:
```rust
pub struct Portfolio {
    cash: Decimal,
    positions: HashMap<String, Position>,
    pending_orders: Vec<Order>,
    total_value: Decimal,
}
```

### 3.7 風險管理模組

風險管理模組負責評估和控制交易風險，確保策略符合預設的風險參數，特別是資金使用限制。

**主要功能**:
- 實現風險檢查和限制
- 計算風險指標和暴露
- 控制倉位大小
- 監控資金使用

**主要組件** (`src/risk/`):
- `checker.rs`: 實現風險檢查邏輯
- `limits.rs`: 定義風險限制規則（包含資金限制）
- `metrics.rs`: 計算風險指標
- `position_sizing.rs`: 根據資金計算合適倉位

### 3.8 消息系統模組

消息系統模組提供基於 RabbitMQ 的消息中間件機制，允許外部系統和用戶與回測伺服器交互。

**主要功能**:
- 提供可靠的消息通訊機制
- 實現多種消息模式（請求/回應、發布/訂閱、工作佇列）
- 處理命令和事件消息
- 提供消息認證和授權
- 支援排名查詢請求

**主要組件** (`src/messaging/`):
- `rabbitmq/`: RabbitMQ 實現細節
- `protocol.rs`: 定義通訊協議和格式
- `handlers/backtest.rs`: 處理回測相關請求
- `handlers/strategy.rs`: 處理策略相關請求
- `handlers/data.rs`: 處理數據相關請求
- `handlers/ranking.rs`: 處理排名查詢請求
- `models/`: 消息模型定義
- `auth.rs`: 處理消息認證和授權

### 3.9 配置管理模組

配置管理模組負責加載、驗證和管理系統的配置選項，包含快取和回測特定配置。

**主要功能**:
- 從不同來源加載配置（文件、環境變量等）
- 驗證配置的有效性
- 提供默認配置值
- 管理快取策略配置

**主要組件** (`src/config/`):
- `loader.rs`: 實現配置加載邏輯
- `validation.rs`: 實現配置驗證
- `cache_config.rs`: 快取系統配置
- `backtest_config.rs`: 回測引擎配置

### 3.10 伺服器模組

伺服器模組管理消息服務的生命週期和配置。

**主要功能**:
- 初始化和配置消息服務系統
- 管理服務生命週期
- 處理基礎設施相關邏輯

**主要組件** (`src/server/`):
- `builder.rs`: 實現伺服器構建器模式
- `error.rs`: 處理伺服器級別錯誤

### 3.11 回測模組

回測模組是系統的核心，負責執行金融策略的歷史回測，採用事件驅動架構實現真實的市場模擬。

**主要功能**:
- 實現事件驅動回測引擎
- 管理回測任務和資源
- 生成市場事件流
- 協調策略執行和資金管理
- 計算性能指標
- 生成回測報告

**主要組件** (`src/backtest/`):
- `engine.rs`: 事件驅動回測引擎核心
- `task.rs`: 管理回測任務
- `results.rs`: 處理回測結果
- `progress.rs`: 監控回測進度
- `executor.rs`: 調度回測任務執行
- `context.rs`: 管理回測執行上下文（含統一資金池）
- `metrics.rs`: 計算性能指標
- `storage.rs`: 存儲回測結果
- `event_generator.rs`: 從市場數據生成事件
- `event_queue.rs`: 管理事件優先級佇列
- `portfolio.rs`: 統一投資組合狀態管理
- `time_slicer.rs`: 時間片優化執行

**事件驅動執行流程**:
1. 生成所有市場事件
2. 按時間順序處理事件
3. 每個事件可能觸發策略信號
4. 檢查資金約束後執行交易
5. 更新投資組合狀態

### 3.12 存儲系統模組

存儲系統模組負責管理系統的持久化存儲，包括連接外部數據庫、管理系統數據庫和快取操作。

**主要功能**:
- 管理外部市場數據庫連接（只讀）
- 管理系統數據庫（TimescaleDB）
- 提供數據模型和ORM功能
- 管理Redis快取（用於跨進程共享）

**主要組件** (`src/storage/`):
- `database.rs`: 系統數據庫連接管理
- `external_db.rs`: 外部市場數據庫連接
- `models.rs`: 數據模型
- `migrations.rs`: 數據庫遷移管理
- `redis/*`: Redis相關功能實現（主要用於狀態共享）

## 4. 配置文件

專案使用以下配置文件管理設置：

- **Cargo.toml**: 專案依賴和構建設置
- **.cargo/config.toml**: Rust編譯器配置
- **Makefile.toml**: cargo-make任務定義
- **config/development.toml**: 開發環境配置
- **config/production.toml**: 生產環境配置
- **config/cache.toml**: 快取策略配置
- **config/backtest.toml**: 回測引擎配置
- **config/rabbitmq.conf**: RabbitMQ配置

### 快取配置示例：
```toml
# config/cache.toml
[ranking]
type = "memory"
memory_size_mb = 512
file_cache_dir = "./cache/rankings"
ttl_days = 30

[market_data]
type = "hierarchical"
memory_size_mb = 2048
redis_enabled = false
```

### 回測配置示例：
```toml
# config/backtest.toml
[execution]
mode = "event_driven"
time_slice_seconds = 1

[portfolio]
initial_cash = 10000000
max_position_per_symbol = 0.1
max_positions = 10
```

## 5. 資料庫結構

專案採用混合資料庫策略：
- **外部市場數據庫**：只讀訪問原始市場數據
- **TimescaleDB**：存儲回測結果和系統數據
- **本地檔案**：存儲排名快取

詳細資料庫結構請參見 [DB_SCHEMA.md](docs/DB_SCHEMA.md)。

### 5.1 數據庫遷移文件

系統使用Sqlx的數據庫遷移機制來管理數據庫結構的版本控制：

- **migrations/**: 遷移文件目錄
  - **20250501000000_create_system_tables.sql**: 創建系統表
  - **20250502000000_create_backtest_tables.sql**: 創建回測相關表
  - **20250503000000_add_indexes.sql**: 添加索引優化
  - **20250504000000_create_views.sql**: 創建視圖和函數

### 5.2 數據庫模型

數據庫模型主要包括以下表結構：

1. **系統管理表**:
   - `system_config`: 系統配置
   - `task_queue`: 任務佇列
   - `cache_metadata`: 快取元數據

2. **策略管理表**:
   - `strategy`: 策略元數據
   - `strategy_version`: 策略版本
   - `strategy_instance`: 策略實例

3. **回測結果表**:
   - `backtest_config`: 回測配置
   - `backtest_result`: 回測結果摘要
   - `backtest_trade`: 交易記錄
   - `backtest_position_snapshot`: 倉位快照
   - `backtest_portfolio_snapshot`: 投資組合快照
   - `backtest_event_log`: 事件日誌

4. **績效分析表**:
   - `performance_metrics`: 績效指標
   - `drawdown_analysis`: 回撤分析
   - `risk_metrics`: 風險指標

## 6. Docker環境

專案提供完整的Docker開發環境，包括：

- **Dockerfile**: 定義Rust開發環境
- **Dockerfile.db**: 定義TimescaleDB數據庫環境
- **Dockerfile.rabbitmq**: 定義RabbitMQ環境
- **docker-compose.yml**: 配置多服務開發環境

Docker環境配置確保開發和生產環境的一致性，簡化部署流程。

## 7. 自動化測試

專案包含多個層次的測試：

- **單元測試**: 位於各模組中的`tests`模組
- **集成測試**: 位於`tests/`目錄
- **性能基準測試**: 位於`benches/`目錄

重點測試領域：
- 排名快取系統的正確性和效能
- 事件驅動引擎的時序正確性
- 資金計算的準確性
- 多股票並發執行的一致性

## 8. 效能基準測試

專案使用Criterion.rs進行性能基準測試，主要測試案例包括：

- **排名快取效能**: 測試內存查詢 vs 數據庫查詢
- **事件處理效能**: 測試事件佇列吞吐量
- **資金計算效能**: 測試統一資金管理開銷
- **策略執行效能**: 測試多股票並行處理
- **快取層級對比**: 測試內存 vs 檔案 vs Redis

基準測試結果用於驗證設計決策和優化效果。

## 9. 示例代碼

`examples/`目錄包含使用本庫的示例程序：

- `simple_strategy.rs`: 展示基本交易策略
- `day_trading_strategy.rs`: 展示當沖策略實現
- `event_driven_backtest.rs`: 展示事件驅動回測
- `ranking_cache_usage.rs`: 展示排名快取使用
- `backtest_runner.rs`: 展示完整回測流程
- `messaging_client.rs`: 展示RabbitMQ客戶端

示例代碼提供了實際使用場景的參考。

## 10. 文檔

專案文檔包括：

- **README.md**: 專案概述和快速入門
- **docs/PLANNING.md**: 專案規劃和設計思路
- **docs/TASK.md**: 開發任務清單
- **docs/STRUCTURE.md**: 本文檔，詳細的專案結構
- **docs/DB_SCHEMA.md**: 數據庫結構設計
- **docs/BACKTEST_ARCHITECTURE.md**: 回測系統架構細節
- **docs/EVENT_DRIVEN_DESIGN.md**: 事件驅動設計說明
- **docs/CACHE_STRATEGY.md**: 快取策略詳解

## 11. Rust 模組系統與組織方式

本專案採用 Rust 現代的模組組織風格，以提高代碼的可讀性和可維護性。

### 11.1 模組結構基本原則

1. **主模組宣告**：在 `lib.rs` 中使用 `pub mod` 宣告所有頂層模組
2. **子模組宣告**：使用與模組同名的 `.rs` 文件來宣告子模組
3. **子模組實現**：子模組放在與父模組同名的目錄中
4. **深層子模組**：深層子模組也遵循相同的模式

### 11.2 好處與最佳實踐

- **清晰的文件結構**：每個模組都有明確的位置
- **易於導航**：IDE和編輯器可以輕鬆定位文件
- **模組重新導出**：使用重新導出簡化外部使用
- **增量編譯優化**：更好的支持Rust的增量編譯

### 11.3 使用示例

**在 `data_provider/ranking_cache.rs` 中宣告和重新導出子模組**：
```rust
// 宣告子模組
pub mod memory;
pub mod persistent;
pub mod builder;

// 重新導出，簡化外部使用
pub use memory::RankingMemoryCache;
pub use persistent::RankingFileCache;
pub use builder::RankingPreprocessor;
```

## 12. 策略版本管理系統

策略版本管理系統提供基本的版本控制功能，確保策略的開發歷程可追蹤。

### 12.1 版本儲存結構

策略通過 RabbitMQ 接收，系統不存儲策略檔案，只在資料庫中記錄元數據。

### 12.2 版本命名規範

策略版本資訊存儲在資料庫中：
- 策略ID：唯一標識符
- 版本號：整數遞增
- 接收時間：記錄策略接收時間

### 12.3 版本存取與管理

版本管理完全依賴資料庫，所有策略元數據和版本信息都儲存在資料庫中。

## 13. 回測系統架構

### 13.1 事件驅動架構

回測系統採用事件驅動架構，確保真實模擬市場環境：

```
事件生成 → 事件佇列 → 事件處理 → 策略執行 → 訂單生成 → 資金檢查 → 執行模擬
```

**核心組件**：
1. **事件生成器**：從市場數據生成時序事件
2. **事件佇列**：基於 BinaryHeap 的優先級佇列
3. **事件調度器**：按時間順序分發事件
4. **策略執行器**：處理事件並生成交易信號
5. **執行模擬器**：模擬訂單執行和資金變化

### 13.2 統一資金管理

所有股票共享同一資金池，確保資金約束的真實性：

```rust
pub struct UnifiedPortfolio {
    cash: Decimal,
    positions: HashMap<String, Position>,
    pending_orders: Vec<Order>,
    
    // 資金管理方法
    pub fn available_cash(&self) -> Decimal
    pub fn can_afford(&self, order: &Order) -> bool
    pub fn execute_order(&mut self, order: Order) -> Result<Trade>
}
```

## 14. 快取系統架構

### 14.1 快取層級

系統實現三層快取架構：

1. **L1 - 內存快取**：
   - 最快速的訪問（< 100ns）
   - 用於熱數據和當前交易日數據
   - 基於 LRU 策略

2. **L2 - 本地檔案快取**：
   - 持久化存儲（< 1ms）
   - 用於排名數據和預計算結果
   - 使用二進制序列化

3. **L3 - Redis（可選）**：
   - 跨進程共享（< 10ms）
   - 用於系統狀態和分散式鎖
   - 不用於高頻數據

### 14.2 排名快取設計

專門優化的排名快取系統：

```rust
pub struct RankingCache {
    // 內存存儲
    memory: BTreeMap<NaiveDate, Vec<String>>,
    // LRU快取
    lru: LruCache<NaiveDate, Vec<String>>,
    // 檔案路徑
    cache_file: PathBuf,
    
    // 核心方法
    pub fn get_top_stocks(&self, date: NaiveDate) -> &[String]
    pub fn build_from_db(&mut self, date_range: DateRange)
    pub fn update_incremental(&mut self, new_date: NaiveDate)
}
```

**效能指標**：
- 初始構建：< 5分鐘（10年數據）
- 查詢延遲：< 100ns（內存命中）
- 增量更新：< 1秒（單日數據）