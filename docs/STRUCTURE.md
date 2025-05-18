# BacktestServer 專案結構設計

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
  - [3.2 數據導入模組](#32-數據導入模組)
  - [3.3 數據提供模組](#33-數據提供模組)
  - [3.4 策略DSL模組](#34-策略dsl模組)
  - [3.5 事件處理系統模組](#35-事件處理系統模組)
  - [3.6 隔離運行時模組](#36-隔離運行時模組)
  - [3.7 執行模擬器模組](#37-執行模擬器模組)
  - [3.8 風險管理模組](#38-風險管理模組)
  - [3.9 消息系統模組](#39-消息系統模組)
  - [3.10 配置管理模組](#310-配置管理模組)
  - [3.11 伺服器模組](#311-伺服器模組)
  - [3.12 回測模組](#312-回測模組)
  - [3.13 存儲系統模組](#313-存儲系統模組)
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

## 1. 專案概述

BacktestServer 是一個使用 Rust 開發的高效能金融回測伺服器，專為支持多策略和動態策略管理設計。本文檔詳細描述了專案的整體結構設計。

## 2. 目錄結構

### 2.1 專案根目錄結構

```
backtest_server/                # 專案根目錄
├── Cargo.toml                  # 專案配置和依賴管理
├── Cargo.lock                  # 鎖定的依賴版本
├── Makefile.toml               # cargo-make任務定義
├── .cargo/                     # Cargo配置目錄
│   └── config.toml             # Cargo配置文件
├── .github/                    # GitHub Actions配置
│   └── workflows/
│       ├── ci.yml              # 持續整合配置
│       └── release.yml         # 發布流程配置
├── Dockerfile                  # Docker容器定義
├── Dockerfile.db               # TimescaleDB數據庫容器定義
├── Dockerfile.rabbitmq         # RabbitMQ容器定義
├── docker-compose.yml          # Docker Compose配置
├── docs/                       # 文檔目錄
├── config/                     # 配置文件目錄
├── scripts/                    # 輔助腳本目錄
├── src/                        # 源代碼目錄
├── strategies/                 # 策略存儲目錄
│   └── {strategy_id}/          # 策略ID目錄
│       └── {strategy_id}_v1.dsl # 版本1策略檔案
├── migrations/                 # 數據庫遷移文件目錄
├── tests/                      # 集成測試目錄
├── benches/                    # 性能基準測試目錄
├── examples/                   # 示例代碼目錄
├── raw/                        # 原始數據目錄
└── README.md                   # 專案說明文檔
```

### 2.2 文檔與配置目錄

```
docs/                           # 文檔目錄
├── PLANNING.md                 # 專案規劃文檔
├── TASK.md                     # 任務清單文檔
├── STRUCTURE.md                # 結構說明文檔
├── BACKTEST_ARCHITECTURE.md    # 回測系統架構文檔
└── DB_SCHEMA.md                # 數據庫結構文檔

config/                         # 配置文件目錄
├── development.toml            # 開發環境配置
├── production.toml             # 生產環境配置
└── rabbitmq.conf               # RabbitMQ配置文件

scripts/                        # 輔助腳本目錄
├── db/                         # 數據庫相關腳本
│   └── init.sql                # 數據庫初始化腳本
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
│   └── migrate.rs              # 數據庫遷移工具入口
├── lib.rs                      # 庫入口點，宣告主要模組
├── config.rs                   # 配置管理模組，宣告子模組
├── domain_types.rs             # 核心領域類型模組
├── data_ingestion.rs           # 數據導入模組
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
└── utils.rs                    # 公共工具模組，宣告子模組
```

### 2.4 功能模組目錄結構

```
# 數據相關模組
src/domain_types/                   # 核心領域類型模組目錄
├── asset_types.rs                  # 資產、數據、交易類型定義
├── data_point.rs                   # OHLCVPoint, TickPoint 定義
├── time_series.rs                  # TimeSeries<T> 結構定義
├── data_matrix.rs                  # DataMatrix 結構定義
├── frequency.rs                    # Frequency enum 定義
├── adjustment.rs                   # Adjustment 結構定義
└── aggregation.rs                  # AggregationConfig, AggregationOp 定義

src/data_ingestion/                 # 數據導入模組目錄
├── processor.rs                    # 導入處理流程控制
├── processor/                      # 數據處理器子模組
│   ├── csv_io.rs                   # CSV 文件讀取
│   ├── csv_io/                     # CSV特定邏輯
│   │   ├── format.rs               # CSV 格式化
│   │   └── options.rs              # CSV 選項
│   └── data_loader.rs              # 通用數據加載邏輯
├── validator.rs                    # 數據驗證與清洗流程控制
└── validator/                      # 數據驗證器子模組
    ├── error.rs                    # 驗證錯誤類型
    ├── traits.rs                   # DataValidator, DataCleaner traits
    ├── ohlcv_validator.rs          # OHLCV 數據驗證
    ├── ohlcv_cleaner.rs            # OHLCV 數據清洗
    ├── tick_validator.rs           # Tick 數據驗證
    ├── tick_cleaner.rs             # Tick 數據清洗
    ├── time_series_validator.rs    # 時間序列整體驗證
    ├── report.rs                   # 驗證報告結構
    └── registry.rs                 # 驗證器/清洗器註冊表

src/data_provider/                  # 數據提供模組目錄
├── types.rs                        # 核心類型定義
├── loader.rs                       # 統一的數據加載器實現
├── cache.rs                        # 數據緩存管理
├── resampler.rs                    # 時間序列重採樣功能
├── precalculator.rs                # 技術指標計算
└── iterator.rs                     # 市場數據迭代器實現

# 策略與執行相關模組
src/strategy/                       # 策略管理模組目錄
├── loader.rs                       # 策略加載器
├── lifecycle.rs                    # 策略生命週期管理
├── registry.rs                     # 策略註冊表
├── context.rs                      # 策略執行上下文
├── snapshot.rs                     # 策略快照管理
├── types.rs                        # 策略基本類型定義
├── config_watcher.rs               # 配置文件監控
└── version/                        # 策略版本管理子模組目錄
    ├── manager.rs                  # 版本管理器
    ├── storage.rs                  # 版本存儲實現
    └── metadata.rs                 # 版本元數據結構

src/execution/                      # 執行模擬器模組目錄
├── simulator.rs                    # 訂單執行模擬器
├── matching.rs                     # 訂單匹配引擎
├── position.rs                     # 倉位和資產管理
├── types.rs                        # 執行相關類型宣告與重新導出
└── types/                          # 執行相關類型子模組目錄
    ├── order.rs                    # 訂單定義
    └── trade.rs                    # 交易記錄定義

src/dsl/                            # DSL解釋器模組目錄
├── parser.rs                       # DSL語法解析器
├── runtime.rs                      # DSL運行時
├── stdlib.rs                       # DSL標準庫
└── compiler.rs                     # DSL編譯器

src/risk/                           # 風險管理模組目錄
├── checker.rs                      # 風險檢查器
├── limits.rs                       # 風險限制
└── metrics.rs                      # 風險指標

# 系統與運行時模組
src/runtime/                        # 隔離運行時模組目錄
├── sandbox.rs                      # 策略沙箱
├── resource.rs                     # 資源管理
└── error.rs                        # 錯誤處理

src/event/                          # 事件處理系統模組目錄
├── bus.rs                          # 事件總線
├── queue.rs                        # 事件佇列
└── dispatcher.rs                   # 事件分發器

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
│   └── data.rs                     # 數據相關消息處理
├── models/                         # 消息模型目錄
│   ├── commands.rs                 # 命令消息模型
│   ├── events.rs                   # 事件消息模型
│   └── responses.rs                # 回應消息模型
└── auth.rs                         # 消息認證和授權

# 回測系統模組
src/backtest/                       # 回測系統模組目錄
├── engine.rs                       # 回測引擎核心實現
├── task.rs                         # 回測任務管理
├── results.rs                      # 回測結果處理
├── progress.rs                     # 回測進度監控
├── executor.rs                     # 回測執行調度器
├── context.rs                      # 回測執行上下文
├── metrics.rs                      # 回測性能指標計算
└── storage.rs                      # 回測結果存儲

# 服務與消息系統模組
src/server/                         # 伺服器模組目錄
├── builder.rs                      # 伺服器構建器模式實現
├── config.rs                       # 伺服器特定配置結構
└── error.rs                        # 伺服器級別錯誤處理

# 基礎設施模組
src/config/                         # 配置管理模組目錄
├── loader.rs                       # 配置加載（環境變量、文件等）
└── validation.rs                   # 配置驗證

src/storage/                        # 存儲系統模組目錄
├── database.rs                     # 數據庫連接管理
├── models.rs                       # 數據模型
├── migrations.rs                   # 數據庫遷移管理(對應/migrations目錄)
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
├── serde_helpers.rs                # 序列化與反序列化幫助函數
├── time.rs                         # 時間處理工具
└── error.rs                        # 通用錯誤處理
```

### 2.5 測試與示例目錄

```
tests/                              # 集成測試目錄
├── data_ingestion_tests.rs         # 數據導入模組測試
├── data_provider_tests.rs          # 數據提供模組測試
├── strategy_tests.rs               # 策略模組測試
├── backtest_tests.rs               # 回測系統測試
├── messaging_tests.rs              # 消息系統測試
└── dsl_tests.rs                    # DSL解釋器測試

benches/                            # 性能基準測試目錄
├── data_loading.rs                 # 數據加載性能測試
├── stock_filtering.rs              # 股票篩選性能測試
├── strategy_execution.rs           # 策略執行性能測試
└── messaging_performance.rs        # 消息系統性能測試

examples/                           # 示例代碼目錄
├── simple_strategy.rs              # 簡單策略示例
├── backtest_runner.rs              # 回測運行器示例
└── messaging_client.rs             # 消息客戶端示例
```

## 3. 核心模組

### 3.1 領域類型模組

此模組定義了整個應用程序中共享的核心金融數據結構、枚舉和類型。

**主要功能**:
- 提供標準化的金融數據表示。
- 確保類型安全和數據一致性。

**主要組件** (`src/domain_types/`):
- `asset_types.rs`: 定義資產類型 (`AssetType`)、數據類型 (`DataType`，包括基礎類型如OHLCV、Tick，以及指標類型)、交易類型 (`TradeType`) 等。
- `data_point.rs`: 定義基本的數據點結構，如 `OHLCVPoint` (開高低收量) 和 `TickPoint` (逐筆成交數據)。
- `time_series.rs`: 提供通用的時間序列數據結構 `TimeSeries<T>`，用於管理帶時間戳的數據點集合。
- `data_matrix.rs`: 定義用於高效數值計算的矩陣數據結構。
- `frequency.rs`: 定義數據頻率的枚舉 `Frequency` (例如，分鐘、小時、日、周等)。
- `adjustment.rs`: 定義數據調整（如股票復權）相關的結構。
- `aggregation.rs`: 定義數據聚合操作 (`AggregationOp`) 及聚合配置 (`AggregationConfig`)，用於數據重採樣等場景。

### 3.2 數據導入模組

此模組負責從各種外部來源（如 CSV 文件、數據庫、API）獲取原始市場數據，並對其進行驗證、清洗和初步處理，為後續存儲或實時使用做準備。

**主要功能**:
- 從多種數據源加載數據。
- 驗證數據的完整性和正確性（例如，價格範圍、時間順序）。
- 清洗數據，處理異常值或缺失值。
- 將原始數據轉換為內部標準格式 (`domain_types`)。

**主要組件** (`src/data_ingestion/`):
- `processor.rs`: 協調數據導入的處理流程。
  - `processor/csv_io.rs`: 處理 CSV 格式數據的讀取和解析。
    - `processor/csv_io/format.rs`: 定義 CSV 數據的具體格式映射。
    - `processor/csv_io/options.rs`: 提供 CSV 解析的配置選項。
  - `processor/data_loader.rs`: 提供更通用的數據加載邏輯，可擴展支持不同數據源。
- `validator.rs`: 協調數據驗證和清洗的流程。
  - `validator/traits.rs`: 定義 `DataValidator` 和 `DataCleaner` 等核心特質 (trait)。
  - `validator/error.rs`: 定義數據驗證過程中可能發生的錯誤類型。
  - `validator/ohlcv_validator.rs` 和 `ohlcv_cleaner.rs`: 針對 OHLCV 數據的特定驗證和清洗邏輯。
  - `validator/tick_validator.rs` 和 `tick_cleaner.rs`: 針對 Tick 數據的特定驗證和清洗邏輯。
  - `validator/time_series_validator.rs`: 針對整個時間序列數據的驗證規則（如時間順序）。
  - `validator/report.rs`: 用於生成數據驗證報告的結構。
  - `validator/registry.rs`: 用於註冊和管理不同的驗證器和清洗器實例。

### 3.3 數據提供模組

此模組作為系統中其他部分（如回測引擎、消息系統）獲取處理後數據的統一接口。它負責從存儲系統中高效地檢索數據並提供必要的轉換功能。

**主要功能**:
- 提供統一的數據訪問接口。
- 執行數據轉換，如時間序列重採樣和技術指標計算。
- 高效緩存常用數據。
- 管理數據迭代和流式處理。

**主要組件** (`src/data_provider/`):
- `types.rs`: 定義數據提供模組使用的核心類型。
- `loader.rs`: 實現統一的數據加載器接口。
- `cache.rs`: 管理數據緩存，提高頻繁訪問數據的性能。
- `resampler.rs`: 實現時間序列重採樣（如分鐘資料轉換為小時資料）。
- `precalculator.rs`: 實現技術指標的計算和緩存。
- `iterator.rs`: 提供高效的市場數據迭代器，支持策略遍歷歷史數據。

### 3.4 策略DSL模組

策略DSL模組提供了一種專用的領域特定語言，簡化策略開發並確保策略執行的安全性和一致性。

**主要功能**:
- 解析策略DSL代碼
- 提供安全的執行環境
- 實現DSL標準庫和內建函數
- 支持策略編譯優化

**主要組件** (`src/dsl/`):
- `parser.rs`: 將DSL代碼解析為抽象語法樹
- `runtime.rs`: 執行解析後的DSL代碼
- `stdlib.rs`: 提供DSL標準庫功能
- `compiler.rs`: 將DSL代碼編譯為優化的中間表示

### 3.5 事件處理系統模組

事件處理系統負責系統中各個組件之間的消息傳遞和事件處理，支持鬆耦合的組件通信。

**主要功能**:
- 實現事件發布/訂閱機制
- 管理事件佇列和分發
- 提供異步事件處理
- 支持事件過濾和路由

**主要組件** (`src/event/`):
- `bus.rs`: 實現中央事件總線，用於事件發布和訂閱
- `queue.rs`: 實現高效的事件佇列
- `dispatcher.rs`: 管理事件分發邏輯

### 3.6 隔離運行時模組

隔離運行時模組確保策略在受控環境中執行，防止惡意代碼和資源濫用。

**主要功能**:
- 提供策略沙箱環境
- 管理資源配額和限制
- 處理策略錯誤和異常

**主要組件** (`src/runtime/`):
- `sandbox.rs`: 實現策略沙箱，限制策略的訪問權限
- `resource.rs`: 管理策略使用的計算資源
- `error.rs`: 處理運行時錯誤和異常

### 3.7 執行模擬器模組

執行模擬器負責模擬市場中的訂單執行過程，包括處理不同類型的訂單、計算滑點和佣金等。

**主要功能**:
- 模擬不同類型的訂單執行
- 計算滑點和交易成本
- 管理倉位和資產組合
- 生成執行報告

**主要組件** (`src/execution/`):
- `simulator.rs`: 實現訂單執行模擬器
- `matching.rs`: 實現訂單匹配引擎
- `position.rs`: 管理倉位和資產組合
- `types.rs`: 定義執行相關的核心類型
- `types/order.rs`: 定義訂單類型和結構
- `types/trade.rs`: 定義交易記錄結構

### 3.8 風險管理模組

風險管理模組負責評估和控制交易風險，確保策略符合預設的風險參數。

**主要功能**:
- 實現風險檢查和限制
- 計算風險指標和暴露

**主要組件** (`src/risk/`):
- `checker.rs`: 實現風險檢查邏輯
- `limits.rs`: 定義風險限制規則
- `metrics.rs`: 計算風險指標

### 3.9 消息系統模組

消息系統模組提供基於 RabbitMQ 的消息中間件機制，允許外部系統和用戶與回測伺服器交互。

**主要功能**:
- 提供可靠的消息通訊機制
- 實現多種消息模式（請求/回應、發布/訂閱、工作佇列）
- 處理命令和事件消息
- 提供消息認證和授權

**主要組件** (`src/messaging/`):
- `rabbitmq/connection.rs`: 管理 RabbitMQ 連接
- `rabbitmq/broker.rs`: 實現消息代理和路由
- `rabbitmq/client.rs`: 提供客戶端使用的 API
- `rabbitmq/consumer.rs`: 處理消息消費邏輯
- `rabbitmq/publisher.rs`: 處理消息發布邏輯
- `rabbitmq/rpc.rs`: 實現 RPC 調用模式
- `protocol.rs`: 定義通訊協議和格式
- `handlers/backtest.rs`: 處理回測相關請求
- `handlers/strategy.rs`: 處理策略相關請求
- `handlers/data.rs`: 處理數據相關請求
- `models/commands.rs`: 定義命令消息結構
- `models/events.rs`: 定義事件消息結構
- `models/responses.rs`: 定義回應消息結構
- `auth.rs`: 處理消息認證和授權

### 3.10 配置管理模組

配置管理模組負責加載、驗證和管理系統的配置選項。

**主要功能**:
- 從不同來源加載配置（文件、環境變量等）
- 驗證配置的有效性
- 提供默認配置值

**主要組件** (`src/config/`):
- `loader.rs`: 實現配置加載邏輯
- `validation.rs`: 實現配置驗證
- `defaults.rs`: 定義默認配置值

### 3.11 伺服器模組

伺服器模組管理消息服務的生命週期和配置。

**主要功能**:
- 初始化和配置消息服務系統
- 管理服務生命週期
- 處理基礎設施相關邏輯

**主要組件** (`src/server/`):
- `builder.rs`: 實現伺服器構建器模式
- `config.rs`: 定義伺服器特定配置
- `error.rs`: 處理伺服器級別錯誤

### 3.12 回測模組

回測模組是系統的核心，負責執行金融策略的歷史回測。

**主要功能**:
- 執行五階段回測流程（初始化、數據準備、策略執行、結果收集、結果分析）
- 管理回測任務和資源
- 計算性能指標
- 生成回測報告

**主要組件** (`src/backtest/`):
- `engine.rs`: 實現回測引擎核心
- `task.rs`: 管理回測任務
- `results.rs`: 處理回測結果
- `progress.rs`: 監控回測進度
- `executor.rs`: 調度回測任務執行
- `context.rs`: 管理回測執行上下文
- `metrics.rs`: 計算性能指標
- `storage.rs`: 存儲回測結果

請參閱 [BACKTEST_ARCHITECTURE.md](docs/BACKTEST_ARCHITECTURE.md) 獲取更詳細的回測系統架構說明。

### 3.13 存儲系統模組

存儲系統模組負責管理系統的持久化存儲，包括數據庫和快取操作。

**主要功能**:
- 管理數據庫連接和操作
- 提供數據模型和ORM功能
- 管理Redis快取和分佈式功能

**主要組件** (`src/storage/`):
- `database.rs`: 數據庫連接管理
- `models.rs`: 數據模型定義
- `migrations.rs`: 數據庫遷移管理
- `redis/*`: Redis相關功能實現

## 4. 配置文件

專案使用以下配置文件管理設置：

- **Cargo.toml**: 專案依賴和構建設置
- **.cargo/config.toml**: Rust編譯器配置
- **Makefile.toml**: cargo-make任務定義
- **config/development.toml**: 開發環境配置
- **config/production.toml**: 生產環境配置
- **config/rabbitmq.conf**: RabbitMQ配置

## 5. 資料庫結構

專案使用TimescaleDB（PostgreSQL的時間序列擴展）作為主要數據存儲。詳細資料庫結構請參見 [DB_SCHEMA.md](docs/DB_SCHEMA.md)。

### 5.1 數據庫遷移文件

系統使用Sqlx的數據庫遷移機制來管理數據庫結構的版本控制：

- **migrations/**: 遷移文件目錄
  - **20250501000000_create_base_tables.sql**: 創建基礎表結構
  - **20250502000000_add_indexes.sql**: 添加索引優化
  - **20250503000000_create_views.sql**: 創建視圖和函數

遷移文件按照版本號順序執行，確保數據庫結構能夠可靠地從一個版本更新到下一個版本。

### 5.2 數據庫模型

數據庫模型反映了核心領域模型，主要包括以下表結構：

1. **基礎資產表**:
   - `exchange`: 交易所信息
   - `instrument`: 金融商品基本信息
   - 資產特定表: `stock`, `future`, `option_contract`, `forex`, `crypto`

2. **市場數據表**:
   - `minute_bar`: 分鐘級K線數據
   - `tick`: Tick級別行情數據
   - `market_event`: 市場事件記錄

3. **策略和交易表**:
   - `strategy`: 策略定義
   - `strategy_version`: 策略版本管理
   - `strategy_instance`: 策略實例配置
   - `trade`: 交易記錄
   - `portfolio`: 資產組合管理

4. **回測相關表**:
   - `backtest_config`: 回測配置信息
   - `backtest_result`: 回測結果摘要
   - `backtest_trade`: 回測交易記錄
   - `backtest_position_snapshot`: 回測倉位快照
   - `backtest_portfolio_snapshot`: 回測投資組合快照

5. **技術指標和預計算數據表**:
   - `technical_indicator`: 技術指標定義
   - `instrument_daily_indicator`: 商品日級指標數據
   - `fundamental_indicator`: 基本面指標

完整的數據庫模型和關係圖請參見 [DB_SCHEMA.md](docs/DB_SCHEMA.md)。

## 6. Docker環境

專案提供完整的Docker開發環境，包括：

- **Dockerfile**: 定義Rust開發環境
- **Dockerfile.db**: 定義TimescaleDB數據庫環境，包括：
  - 基於timescale/timescaledb:latest-pg14鏡像
  - 自動加載數據庫遷移文件
  - 配置TimescaleDB優化參數
- **Dockerfile.rabbitmq**: 定義RabbitMQ環境，包括：
  - 基於rabbitmq:3.11-management鏡像
  - 加載預設交換機和佇列定義
  - 配置管理界面和插件
- **docker-compose.yml**: 配置多服務開發環境，包括：
  - Rust開發容器
  - TimescaleDB數據庫
  - Redis緩存和消息系統
  - RabbitMQ消息代理

Docker環境配置確保開發和生產環境的一致性，簡化部署流程。

## 7. 自動化測試

專案包含多個層次的測試：

- **單元測試**: 位於各模組中的`tests`模組，測試單個功能和組件
- **集成測試**: 位於`tests/`目錄，測試多個組件的協同工作
- **性能基準測試**: 位於`benches/`目錄，測量關鍵操作的性能

測試使用以下工具和框架：
- `cargo test`: 運行單元和集成測試
- `cargo-nextest`: 並行運行測試提高效率
- `mockall`: 用於模擬依賴組件
- `proptest`: 基於屬性的測試，生成隨機測試資料

## 8. 效能基準測試

專案使用Criterion.rs進行性能基準測試，主要測試案例包括：

- **數據加載性能**: 測試從CSV和數據庫加載數據的效率
- **篩選和查詢性能**: 測試資產篩選和數據查詢效率
- **策略執行性能**: 測試在不同數據量下的策略執行效率
- **DSL解析和執行性能**: 測試DSL解析器和運行時的效率
- **消息系統性能**: 測試RabbitMQ消息傳輸效率和延遲

基準測試結果用於識別性能瓶頸和驗證優化效果。

## 9. 示例代碼

`examples/`目錄包含使用本庫的示例程序：

- `simple_strategy.rs`: 展示如何創建和測試簡單交易策略
- `backtest_runner.rs`: 展示如何配置和運行完整回測
- `messaging_client.rs`: 展示如何使用RabbitMQ客戶端與系統交互
- 其他專用示例，如技術指標計算、自定義DSL策略等

示例代碼提供了實際使用場景的參考，幫助用戶快速上手。

## 10. 文檔

專案文檔包括：

- **README.md**: 專案概述和快速入門
- **docs/PLANNING.md**: 專案規劃和設計思路
- **docs/TASK.md**: 開發任務清單
- **docs/STRUCTURE.md**: 本文檔，詳細的專案結構
- **docs/DB_SCHEMA.md**: 數據庫結構設計
- **docs/BACKTEST_ARCHITECTURE.md**: 回測系統架構細節
- **消息協議文檔**: 描述RabbitMQ消息格式和協議
- **用戶指南**: 策略開發和系統使用手冊

文檔通過`cargo doc`生成，並可通過GitHub Pages或專用文檔網站訪問。

## 11. Rust 模組系統與組織方式

本專案採用 Rust 現代的模組組織風格，以提高代碼的可讀性和可維護性。

### 11.1 模組結構基本原則

1. **主模組宣告**：在 `lib.rs` 中使用 `pub mod` 宣告所有頂層模組
   ```rust
   // lib.rs
   pub mod data_ingestion;
   pub mod execution;
   // 其他頂層模組...
   ```

2. **子模組宣告**：使用與模組同名的 `.rs` 文件來宣告子模組
   ```rust
   // data_ingestion.rs
   pub mod processor;
   pub mod validator;
   ```

3. **子模組實現**：子模組放在與父模組同名的目錄中
   ```
   src/
   ├── data_ingestion.rs           # 宣告 data_ingestion 模組的子模組
   └── data_ingestion/             # 存放 data_ingestion 子模組實現
       ├── processor.rs            # processor 子模組的實現
       └── validator.rs            # validator 子模組的實現
   ```

4. **深層子模組**：深層子模組也遵循相同的模式
   ```
   src/
   ├── data_ingestion.rs           # 宣告 data_ingestion 模組的子模組
   └── data_ingestion/             
       ├── processor.rs            # 宣告 processor 的子模組
       └── processor/              # 存放 processor 的子模組實現
           ├── csv_io.rs
           └── ...
   ```

### 11.2 好處與最佳實踐

- **清晰的文件結構**：每個模組都有明確的位置
- **易於導航**：IDE和編輯器可以輕鬆定位文件
- **模組重新導出**：使用重新導出簡化外部使用
- **增量編譯優化**：更好的支持Rust的增量編譯

### 11.3 使用示例

**在 `data_ingestion/processor.rs` 中宣告和重新導出子模組**：
```rust
// 宣告子模組
pub mod csv_io;
pub mod data_loader;

// 重新導出，簡化外部使用
pub use csv_io::CsvReader;
pub use data_loader::DataLoader;
```

**從外部使用**：
```rust
// 由於重新導出，可以直接使用
use crate::data_ingestion::processor::CsvReader;
// 而不必
// use crate::data_ingestion::processor::csv_io::CsvReader;
```

## 12. 策略版本管理系統

策略版本管理系統提供基本的版本控制功能，確保策略的開發歷程可追蹤。

### 12.1 版本儲存結構

策略檔案以標準化目錄結構保存：

```
strategies/
└── {strategy_id}/                # 策略ID目錄
    └── {strategy_id}_v{n}.dsl    # 策略檔案
```

### 12.2 版本命名規範

策略檔案採用明確的版本命名規範：
- 格式：`{strategy_id}_v{version}.dsl`
- 例如：`my_strategy_v1.dsl`, `my_strategy_v2.dsl`
- 版本號格式為整數遞增，確保版本排序一致性

### 12.3 版本存取與管理

版本管理完全依賴資料庫，所有策略元數據和版本信息都儲存在資料庫中。系統通過資料庫查詢直接獲取策略信息，而非依賴檔案系統中的元數據檔案。

系統提供簡化的版本管理功能：

**創建新版本**
- 自動生成下一個版本號
- 保留舊版本檔案作為歷史記錄
- 在資料庫中更新相關記錄

## 13. 回測系統架構

回測系統採用五階段協作架構，將回測過程分為明確的階段，確保組件之間的責任分離和協作：

```
┌─── 初始化階段 ───┐   ┌─── 數據準備階段 ───┐   ┌─── 策略執行階段 ───┐   ┌─── 結果收集階段 ───┐   ┌─── 結果分析階段 ───┐
│  使用模組:        │   │  使用模組:          │   │  使用模組:          │   │  使用模組:          │   │  使用模組:          │
│  - 回測模組       │   │  - 數據提供模組     │   │  - 策略模組         │   │  - 執行模擬器模組   │   │  - 回測模組         │
│  - 配置管理模組   │   │  - 運行時模組       │   │  - DSL模組          │   │  - 風險管理模組     │   │  - 消息系統模組    │
│                  │   │  - 回測模組         │   │  - 運行時模組       │   │  - 回測模組         │   │                    │
│                  │   │                    │   │  - 執行模擬器模組    │   │                    │   │                    │
└──────────────────┘   └────────────────────┘   └────────────────────┘   └────────────────────┘   └────────────────────┘
```

每個階段有明確的職責和輸入/輸出，詳細架構請參見 [BACKTEST_ARCHITECTURE.md](docs/BACKTEST_ARCHITECTURE.md)。