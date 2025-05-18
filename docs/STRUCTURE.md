# FinRust 專案結構設計

## 目錄

- [1. 專案概述](#1-專案概述)
- [2. 目錄結構](#2-目錄結構)
  - [2.1 專案根目錄結構](#21-專案根目錄結構)
  - [2.2 文檔與配置目錄](#22-文檔與配置目錄)
  - [2.3 核心源代碼結構](#23-核心源代碼結構)
  - [2.4 功能模組目錄結構](#24-功能模組目錄結構)
  - [2.5 測試與示例目錄](#25-測試與示例目錄)
- [3. 核心模組](#3-核心模組)
  - [3.1 領域類型模組 (Domain Types Module)](#31-領域類型模組-domain-types-module)
  - [3.2 數據導入模組 (Data Ingestion Module)](#32-數據導入模組-data-ingestion-module)
  - [3.3 數據提供模組 (Data Provider Module)](#33-數據提供模組-data-provider-module)
  - [3.4 策略DSL模組](#34-策略dsl模組)
  - [3.5 事件處理系統模組](#35-事件處理系統模組)
  - [3.6 隔離運行時模組](#36-隔離運行時模組)
  - [3.7 執行模擬器模組](#37-執行模擬器模組)
  - [3.8 風險管理模組](#38-風險管理模組)
  - [3.9 API服務模組](#39-api服務模組)
  - [3.10 配置管理模組](#310-配置管理模組)
  - [3.11 伺服器模組](#311-伺服器模組)
  - [3.12 回測模組](#312-回測模組)
- [4. 配置文件](#4-配置文件)
- [5. 資料庫結構](#5-資料庫結構)
  - [5.1 數據庫遷移文件](#51-數據庫遷移文件)
  - [5.2 數據庫模型](#52-數據庫模型)
- [6. Docker環境](#6-docker環境)
  - [6.1 Docker目錄結構](#61-docker目錄結構)
- [7. 自動化測試](#7-自動化測試)
- [8. 效能基準測試](#8-效能基準測試)
- [9. 示例代碼](#9-示例代碼)
- [10. 文檔](#10-文檔)
- [11. Rust 模組系統與組織方式](#11-rust-模組系統與組織方式)
  - [11.1 模組結構基本原則](#111-模組結構基本原則)
  - [11.2 新舊風格對比](#112-新舊風格對比)
  - [11.3 好處與最佳實踐](#113-好處與最佳實踐)
  - [11.4 使用示例](#114-使用示例)
- [12. 策略版本管理與回滾系統](#12-策略版本管理與回滾系統)
  - [12.1 版本儲存結構](#121-版本儲存結構)
  - [12.2 版本命名規範](#122-版本命名規範)
  - [12.3 版本元數據管理](#123-版本元數據管理)
  - [12.4 版本管理操作](#124-版本管理操作)
  - [12.5 回滾機制實現](#125-回滾機制實現)

## 1. 專案概述

FinRust是一個使用Rust開發的高效能金融回測伺服器，專為支持多策略和動態策略管理設計。本文檔詳細描述了專案的整體結構設計。

## 2. 目錄結構

### 2.1 專案根目錄結構

```
finrust/                          # 專案根目錄
├── Cargo.toml                    # 專案配置和依賴管理
├── Cargo.lock                    # 鎖定的依賴版本
├── Makefile.toml                 # cargo-make任務定義
├── .cargo/                       # Cargo配置目錄
│   └── config.toml               # Cargo配置文件
├── .github/                      # GitHub Actions配置
│   └── workflows/
│       ├── ci.yml                # 持續整合配置
│       └── release.yml           # 發布流程配置
├── Dockerfile                    # Docker容器定義
├── Dockerfile.db                 # TimescaleDB數據庫容器定義
├── docker-compose.yml            # Docker Compose配置
├── docs/                         # 文檔目錄
├── config/                       # 配置文件目錄
├── scripts/                      # 輔助腳本目錄
├── src/                          # 源代碼目錄
├── strategies/                   # 策略存儲目錄
│   └── {strategy_id}/            # 策略ID目錄
│       ├── {strategy_id}_{version}.dsl  # 版本1策略檔案
│       └── metadata.json         # 策略元數據文件
├── migrations/                   # 數據庫遷移文件目錄
├── tests/                        # 集成測試目錄
├── benches/                      # 性能基準測試目錄
├── examples/                     # 示例代碼目錄
├── raw/                          # 原始數據目錄
└── README.md                     # 專案說明文檔
```

### 2.2 文檔與配置目錄

```
docs/                             # 文檔目錄
├── PLANNING.md                   # 專案規劃文檔
├── TASK.md                       # 任務清單文檔
├── STRUCTURE.md                  # 結構說明文檔
└── DB_SCHEMA.md                  # 數據庫結構文檔

config/                           # 配置文件目錄
├── default.toml                  # 預設配置
└── development.toml              # 開發環境配置

scripts/                          # 輔助腳本目錄
├── db/                           # 數據庫相關腳本
│   └── init.sql                  # 數據庫初始化腳本
├── install.sh                    # 安裝依賴腳本
├── setup_db.sh                   # 設置數據庫腳本
└── benchmark.sh                  # 運行基準測試腳本

migrations/                       # 數據庫遷移文件目錄
├── V1__create_base_tables.sql                # 基礎表創建
├── V2__create_market_data_tables.sql         # 市場數據表創建
├── V3__create_strategy_tables.sql            # 策略表創建
├── V4__create_event_financial_tables.sql     # 事件和財務數據表創建
├── V5__create_portfolio_tables.sql           # 投資組合表創建
├── V6__create_continuous_aggregates.sql      # 連續聚合視圖創建
├── V7__create_indicator_tables.sql           # 指標表創建
└── V8__create_db_management.sql              # 數據庫管理表創建
```

### 2.3 核心源代碼結構

```
src/                              # 源代碼目錄
├── main.rs                       # 程序入口點
├── lib.rs                        # 庫入口點，宣告主要模組
├── config.rs                     # 配置管理模組，宣告子模組
├── domain_types.rs               # 核心領域類型模組
├── data_ingestion.rs             # 數據導入模組
├── data_provider.rs              # 數據提供模組
├── execution.rs                  # 執行模擬器模組，宣告子模組
├── strategy.rs                   # 策略管理模組，宣告子模組
├── dsl.rs                        # DSL解釋器模組，宣告子模組
├── event.rs                      # 事件處理系統模組，宣告子模組
├── runtime.rs                    # 隔離運行時模組，宣告子模組
├── risk.rs                       # 風險管理模組，宣告子模組
├── api.rs                        # API服務模組，宣告子模組
├── server.rs                     # 伺服器核心組件，宣告子模組
├── storage.rs                    # 存儲系統模組，宣告子模組
├── monitor.rs                    # 監控系統模組，宣告子模組
├── backtest.rs                   # 回測系統模組，宣告子模組
└── utils.rs                      # 公共工具模組，宣告子模組
```

### 2.4 功能模組目錄結構

```
# 數據相關模組 (重構後)
src/domain_types/                     # 核心領域類型模組目錄 (NEW)
├── asset_types.rs                  # 資產、數據、交易類型定義
├── data_point.rs                   # OHLCVPoint, TickPoint 定義
├── time_series.rs                  # TimeSeries<T> 結構定義
├── data_matrix.rs                  # DataMatrix 結構定義
├── frequency.rs                    # Frequency enum 定義
├── adjustment.rs                   # Adjustment 結構定義
└── aggregation.rs                  # AggregationConfig, AggregationOp 定義

src/data_ingestion/                   # 數據導入模組目錄 (NEW)
├── processor.rs                      # 導入處理流程控制
├── processor/                        # 數據處理器子模組
│   ├── csv_io.rs                     # CSV 文件讀取
│   ├── csv_io/                       # CSV特定邏輯
│   │   ├── format.rs                 # CSV 格式化
│   │   └── options.rs                # CSV 選項
│   └── data_loader.rs                # 通用數據加載邏輯
├── validator.rs                      # 數據驗證與清洗流程控制
└── validator/                        # 數據驗證器子模組
    ├── error.rs                      # 驗證錯誤類型
    ├── traits.rs                     # DataValidator, DataCleaner traits
    ├── ohlcv_validator.rs            # OHLCV 數據驗證
    ├── ohlcv_cleaner.rs              # OHLCV 數據清洗
    ├── tick_validator.rs             # Tick 數據驗證
    ├── tick_cleaner.rs               # Tick 數據清洗
    ├── time_series_validator.rs      # 時間序列整體驗證
    ├── report.rs                     # 驗證報告結構
    └── registry.rs                   # 驗證器/清洗器註冊表

src/data_provider/                    # 數據提供模組目錄 (NEW)
├── lazy_loader.rs                    # lazy_loader入口，宣告與重新導出組件
├── lazy_loader/                      # 延遲加載模組
│   ├── types.rs                      # LazyLoadStrategy, LazyDataKey 等類型
│   ├── traits.rs                     # LazyLoader trait
│   ├── ohlcv_loader.rs               # LazyOHLCVLoader 實現
│   ├── tick_loader.rs                # LazyTickLoader 實現
│   └── manager.rs                    # LazyDataManager 實現
├── smart_distribution/               # 智能數據分發模組
│   ├── types.rs                      # types 入口
│   ├── dependency_graph.rs           # 數據依賴圖實現
│   └── types/                        # smart_distribution 相關類型
│       ├── config.rs                 # 各類 Config 結構
│       ├── core.rs                   # DataKey, DataRequest, DataResponse 等核心結構
│       ├── error.rs                  # DataDistributionError enum
│       └── status.rs                 # 各類 Status 結構
├── resampler.rs                      # 時間序列重採樣器 (TimeSeriesResampler)
└── precalculator.rs                  # 技術指標預計算器 (StockDataPrecalculator, IndicatorType)

# 策略與執行相關模組
src/strategy/                         # 策略管理模組目錄
├── loader.rs                         # 策略加載器
├── lifecycle.rs                      # 策略生命週期管理
├── registry.rs                       # 策略註冊表
├── context.rs                        # 策略執行上下文
├── snapshot.rs                       # 策略快照管理
├── types.rs                          # 策略基本類型定義
├── config_watcher.rs                 # 配置文件監控
└── version/                          # 策略版本管理子模組目錄
    ├── manager.rs                    # 版本管理器
    ├── storage.rs                    # 版本存儲實現
    ├── metadata.rs                   # 版本元數據結構
    ├── diff.rs                       # 版本差異比較工具
    └── rollback.rs                   # 版本回滾機制實現

src/execution/                        # 執行模擬器模組目錄
├── types.rs                          # 執行相關類型宣告與重新導出
└── types/                            # 執行相關類型子模組目錄
    ├── order.rs                      # 訂單定義
    └── trade.rs                      # 交易記錄定義

src/dsl/                              # DSL解釋器模組目錄
├── parser.rs                         # DSL語法解析器
├── runtime.rs                        # DSL運行時
├── stdlib.rs                         # DSL標準庫
└── compiler.rs                       # DSL編譯器

src/risk/                             # 風險管理模組目錄
├── checker.rs                        # 風險檢查器
├── limits.rs                         # 風險限制
└── metrics.rs                        # 風險指標

# 系統與運行時模組
src/runtime/                          # 隔離運行時模組目錄
├── sandbox.rs                        # 策略沙箱
├── resource.rs                       # 資源管理
└── error.rs                          # 錯誤處理

src/event/                            # 事件處理系統模組目錄
├── bus.rs                            # 事件總線
├── queue.rs                          # 事件佇列
└── dispatcher.rs                     # 事件分發器

# 回測系統模組
src/backtest/                         # 回測系統模組目錄
├── engine.rs                         # 回測引擎核心實現
├── task.rs                           # 回測任務管理
├── results.rs                        # 回測結果處理
├── progress.rs                       # 回測進度監控
├── executor.rs                       # 回測執行調度器
├── context.rs                        # 回測執行上下文
├── metrics.rs                        # 回測性能指標計算
└── storage.rs                        # 回測結果存儲

# 服務與API模組
src/api/                              # API服務模組目錄
├── handlers/                         # 
├── routes/                           # 
├── middleware/                       # 
├── rest.rs                           # REST API
├── handlers.rs                       # 
├── middleware.rs                     # 
├── auth.rs                           # 認證和授權
└── routes/                           # API路由定義

src/server/                           # 伺服器模組目錄
├── builder.rs                        # 伺服器構建器模式實現
├── config.rs                         # 伺服器特定配置結構
└── error.rs                          # 伺服器級別錯誤處理

# 基礎設施模組
src/config/                           # 配置管理模組目錄
├── loader.rs                         # 配置加載（環境變量、文件等）
├── validation.rs                     # 配置驗證
└── defaults.rs                       # 默認配置值

src/storage/                          # 存儲系統模組目錄
├── database.rs                       # 數據庫連接管理
├── models.rs                         # 數據模型
└── migrations.rs                     # 數據庫遷移管理(對應/migrations目錄)

src/monitor/                          # 監控系統模組目錄
├── metrics.rs                        # 監控指標收集
├── logger.rs                         # 日誌記錄
└── alerter.rs                        # 警報系統

src/utils/                            # 公共工具模組目錄
├── time.rs                           # 時間處理
├── math.rs                           # 數學函數
├── concurrency.rs                    # 並發工具
├── shutdown.rs                       # 關閉信號處理
└── graceful.rs                       # 優雅關閉助手
```

### 2.5 測試與示例目錄

```
tests/                                # 集成測試目錄
├── data_ingestion_tests.rs         # (更新)
├── data_provider_tests.rs          # (新增)
├── selector_tests.rs
├── strategy_tests.rs
├── backtest_tests.rs               # 回測系統測試
└── dsl_tests.rs

benches/                              # 性能基準測試目錄
├── data_loading.rs
├── stock_filtering.rs
└── strategy_execution.rs

examples/                             # 示例代碼目錄
├── simple_strategy.rs                # 簡單策略示例
└── backtest_runner.rs                # 回測運行器示例
```

## 3. 核心模組

### 3.1 領域類型模組 (Domain Types Module)

此模組定義了整個應用程序中共享的核心金融數據結構、枚舉和類型。

**主要功能**:
- 提供標準化的金融數據表示。
- 確保類型安全和數據一致性。

**主要組件** (`src/domain_types/`):
- `asset_types.rs`: 定義資產類型 (`AssetType`)、數據類型 (`DataType`，包括基礎類型如OHLCV、Tick，以及指標類型)、交易類型 (`TradeType`) 等。
- `data_point.rs`: 定義基本的數據點結構，如 `OHLCVPoint` (開高低收量) 和 `TickPoint` (逐筆成交數據)。
- `time_series.rs`: 提供通用的時間序列數據結構 `TimeSeries<T>`，用於管理帶時間戳的數據點集合。
- `data_matrix.rs`: (若使用) 定義用於高效數值計算的矩陣數據結構。
- `frequency.rs`: 定義數據頻率的枚舉 `Frequency` (例如，分鐘、小時、日、周等)。
- `adjustment.rs`: (若使用) 定義數據調整（如股票復權）相關的結構。
- `aggregation.rs`: 定義數據聚合操作 (`AggregationOp`) 及聚合配置 (`AggregationConfig`)，用於數據重採樣等場景。

### 3.2 數據導入模組 (Data Ingestion Module)

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
  - `validator/registry.rs`: （可選）用於註冊和管理不同的驗證器和清洗器實例。

### 3.3 數據提供模組 (Data Provider Module)

此模組作為系統中其他部分（如回測引擎、API服務）獲取處理後數據的統一接口。它負責從存儲系統中高效地檢索數據，並可按需進行即時的數據轉換。

**主要功能**:
- 提供統一的數據訪問接口。
- 實現數據的延遲加載 (Lazy Loading)，僅在需要時從存儲中讀取數據。
- 管理智能數據分發，優化數據流和緩存。
- 執行即時數據轉換，如時間序列重採樣和技術指標計算。

**主要組件** (`src/data_provider/`):
- `lazy_loader.rs`: `lazy_loader` 模組的入口
- `lazy_loader/`: 實現數據的延遲加載機制。
  - `types.rs`: 定義延遲加載相關的類型，如 `LazyDataKey`, `LazyLoadStrategy`, `LazyDataSource`。
  - `traits.rs`: 定義 `LazyLoader` 特質。
  - `ohlcv_loader.rs` 和 `tick_loader.rs`: 分別實現 OHLCV 和 Tick 數據的延遲加載器。
  - `manager.rs`: `LazyDataManager` 用於管理多個延遲加載器實例。
- `smart_distribution.rs`: `smart_distribution` 模組的入口。
- `smart_distribution/`: （若已實現或規劃中）實現智能的數據分發、緩存和預測性加載。
  - `dependency_graph.rs`: 管理數據之間的依賴關係。
  - `types.rs`: `types` 模組的入口。
  - `types/`: 包含與智能分發相關的各種配置、核心數據結構、錯誤和狀態類型。
    - `config.rs`, `core.rs`, `error.rs`, `status.rs`。
- `resampler.rs`: `TimeSeriesResampler` 實現，用於將時間序列數據從一個頻率轉換到另一個頻率。
- `precalculator.rs`: `StockDataPrecalculator` 和 `BatchPrecalculator` 實現，用於預計算常用的技術指標 (如 SMA, EMA, MACD, RSI 等)，並定義 `IndicatorType`。

### 3.4 策略DSL模組

策略DSL模組提供了一種專用的領域特定語言，簡化策略開發。

**主要功能**:
- 解析策略DSL代碼
- 提供安全的執行環境
- 實現DSL標準庫和內建函數
- 支持策略編譯優化

**主要組件**:
- `dsl.rs`: DSL模組主檔案，宣告子模組
- `dsl/parser.rs`: DSL語法解析器
- `dsl/runtime.rs`: DSL運行時
- `dsl/stdlib.rs`: DSL標準庫
- `dsl/compiler.rs`: DSL編譯器和優化

### 3.5 事件處理系統模組

事件處理系統負責系統中各個組件之間的消息傳遞和事件處理。

**主要功能**:
- 實現事件發布/訂閱機制
- 管理事件佇列和分發
- 提供異步事件處理
- 支持事件過濾和路由

**主要組件**:
- `event.rs`: 事件系統模組主檔案，宣告子模組
- `event/bus.rs`: 事件總線實現
- `event/queue.rs`: 高效事件佇列
- `event/dispatcher.rs`: 事件分發系統

### 3.6 隔離運行時模組

隔離運行時模組確保不同策略之間的資源隔離和錯誤隔離。

**主要功能**:
- 提供策略沙箱環境
- 管理資源配額和限制
- 處理策略錯誤和異常
- 支持策略熱載入和卸載

**主要組件**:
- `runtime.rs`: 隔離運行時模組主檔案，宣告子模組
- `runtime/sandbox.rs`: 策略安全沙箱
- `runtime/resource.rs`: 資源管理和配額
- `runtime/error.rs`: 錯誤處理和隔離

### 3.7 執行模擬器模組

執行模擬器負責模擬市場中的訂單執行過程。

**主要功能**:
- 模擬不同類型的訂單執行
- 計算滑點和交易成本
- 管理倉位和資產組合
- 生成執行報告

**主要組件**:
- `simulator.rs`: 訂單執行模擬器
- `matching.rs`: 訂單匹配引擎
- `position.rs`: 倉位和資產管理
- `types.rs`: 宣告和重新導出執行相關類型
- `types/` 目錄:
  - `order.rs`: 訂單定義
  - `trade.rs`: 交易記錄定義

### 3.8 風險管理模組

風險管理模組負責評估和控制交易風險。

**主要功能**:
- 實現風險檢查和限制
- 計算風險指標和暴露
- 提供風險預警機制
- 支持自定義風險規則

**主要組件**:
- `risk.rs`: 風險管理模組主檔案，宣告子模組
- `risk/checker.rs`: 風險檢查器
- `risk/limits.rs`: 風險限制規則
- `risk/metrics.rs`: 風險指標計算

### 3.9 API服務模組

API服務模組提供外部接口，使用戶能夠與系統交互。

**主要功能**:
- 提供RESTful API接口
- 實現用戶認證和授權
- 提供API文檔和示例

**主要組件**:
- `api.rs`: API服務模組主檔案，宣告子模組
- `api/rest.rs`: REST API實現
- `api/auth.rs`: 認證和授權系統
- `api/routes/`: API路由定義子目錄

### 3.10 配置管理模組

配置管理模組負責處理系統的各種配置選項和設置。

**主要功能**:
- 從不同來源加載配置（環境變量、文件、命令行參數）
- 驗證配置的正確性和完整性
- 提供默認配置值
- 支持動態配置更新

**主要組件**:
- `config.rs`: 配置管理模組主檔案，宣告子模組
- `config/loader.rs`: 配置加載器，支持多種來源
- `config/validation.rs`: 配置驗證邏輯
- `config/defaults.rs`: 默認配置值定義

### 3.11 伺服器模組

伺服器模組負責管理系統的HTTP服務和生命週期。

**主要功能**:
- 提供HTTP服務器實現
- 管理伺服器生命週期
- 處理請求路由和中間件
- 支持優雅啟動和關閉

**主要組件**:
- `server.rs`: 伺服器模組主檔案，宣告子模組
- `server/builder.rs`: 使用構建器模式實現伺服器配置
- `server/error.rs`: 伺服器級別錯誤處理

### 3.12 回測模組

回測模組負責協調各個組件，執行策略回測，並收集和分析回測結果。

**主要功能**:
- 管理回測任務的創建、調度和監控
- 執行端到端的回測流程
- 記錄回測進度和狀態
- 計算和保存回測結果和績效指標
- 提供結果分析和可視化的基礎

**主要組件**:
- `backtest.rs`: 回測模組主檔案，宣告子模組
- `backtest/engine.rs`: 回測引擎核心實現，負責組織回測流程和數據流
- `backtest/task.rs`: 回測任務管理，包括任務創建、狀態更新、隊列管理
- `backtest/results.rs`: 回測結果處理，包括結果收集、計算和格式化
- `backtest/progress.rs`: 回測進度監控，支持實時追蹤回測執行狀態
- `backtest/executor.rs`: 回測執行調度器，負責任務分配和並行執行
- `backtest/context.rs`: 回測執行上下文，保存回測過程中的狀態和配置
- `backtest/metrics.rs`: 回測性能指標計算，如夏普比率、最大回撤等
- `backtest/storage.rs`: 回測結果存儲，將結果保存到文件系統

回測模組整合了系統中的多個核心組件，包括：
- 從數據提供模組獲取歷史市場數據
- 使用策略模組執行交易邏輯
- 利用執行模擬器模組模擬訂單執行
- 通過風險管理模組評估風險約束
- 將結果存儲到html

## 4. 配置文件

專案使用以下配置文件管理設置：

- **Cargo.toml**: 項目依賴和構建設置
- **.cargo/config.toml**: Rust編譯器配置
- **Makefile.toml**: cargo-make任務定義
- **config/default.toml**: 預設應用程序配置
- **config/development.toml**: 開發環境特定配置

## 5. 資料庫結構

專案使用TimescaleDB（PostgreSQL的時間序列擴展）作為主要數據存儲。詳細資料庫結構請參見 [DB_SCHEMA.md](docs/DB_SCHEMA.md)。

### 5.1 數據庫遷移文件

專案使用Flyway風格的數據庫遷移文件來管理數據庫結構的版本控制和演進。遷移文件位於`/migrations`目錄，按照執行順序編號：

- **V1__create_base_tables.sql**: 創建基礎表結構
- **V2__create_market_data_tables.sql**: 創建市場數據相關表
- **V3__create_strategy_tables.sql**: 創建策略管理相關表
- **V4__create_event_financial_tables.sql**: 創建事件和財務數據表
- **V5__create_portfolio_tables.sql**: 創建投資組合管理表
- **V6__create_continuous_aggregates.sql**: 創建TimescaleDB連續聚合視圖
- **V7__create_indicator_tables.sql**: 創建技術指標相關表
- **V8__create_db_management.sql**: 創建數據庫管理和維護表

這些遷移文件按照版本號順序執行，確保數據庫結構能夠可靠地從一個版本更新到下一個版本。每個文件包含特定版本的數據庫更改，如創建表、添加索引、設置約束等。

### 5.2 數據庫模型

## 6. Docker環境

專案提供完整的Docker開發環境，包括：

- **Dockerfile**: 定義Rust開發環境
- **Dockerfile.db**: 定義TimescaleDB數據庫環境，包括：
  - 基於timescale/timescaledb:latest-pg14鏡像
  - 自動加載數據庫遷移文件
  - 配置TimescaleDB優化參數
  - 設置自動執行的初始化腳本：
    - 000_load_custom_config.sh: 載入自定義PostgreSQL配置
    - 001_setup_extensions.sh: 設置必要的擴展和輔助函數
    - 002_run_migrations.sh: 按順序執行數據庫遷移文件
- **docker-compose.yml**: 配置多服務開發環境，包括：
  - Rust開發容器
  - TimescaleDB數據庫
  - Redis緩存和消息系統
  - Grafana數據可視化（可選）

### 6.1 Docker目錄結構

Docker環境中的關鍵目錄結構：

```
/app/                             # 容器內工作目錄
├── migrations/                   # 數據庫遷移文件目錄
│   ├── V1__create_base_tables.sql
│   ├── V2__create_market_data_tables.sql
│   └── ...
└── ...

/docker-entrypoint-initdb.d/      # 數據庫初始化腳本目錄
├── 000_load_custom_config.sh     # 載入自定義PostgreSQL配置
├── 001_setup_extensions.sh       # 設置必要的擴展
└── 002_run_migrations.sh         # 執行數據庫遷移
```

## 7. 自動化測試

專案包含多個測試類型：

- **單元測試**: 位於各模組中的`tests`模組
- **集成測試**: 位於`tests/`目錄
- **性能基準測試**: 位於`benches/`目錄

## 8. 效能基準測試

專案使用Criterion.rs進行性能基準測試，測試案例包括：

- 數據加載性能
- 策略執行性能
- DSL解析和執行性能

## 9. 示例代碼

`examples/`目錄包含多個示例程序，演示系統的核心功能：

- 簡單交易策略示例
- 完整回測流程示例

## 10. 文檔

專案文檔包括：

- **README.md**: 專案概述和快速入門
- **docs/PLANNING.md**: 專案規劃和設計思路
- **docs/TASK.md**: 開發任務清單
- **docs/STRUCTURE.md**: 本文檔，詳細的專案結構
- **docs/DB_SCHEMA.md**: 數據庫結構設計
- 源代碼文檔（通過`cargo doc`生成）

## 11. Rust 模組系統與組織方式

本專案採用 Rust 最新的模組組織風格，避免使用舊版的 `mod.rs` 文件，以提高代碼的可讀性和可維護性。

### 11.1 模組結構基本原則

1. **主模組宣告**：在 `lib.rs` 中使用 `pub mod` 宣告所有頂層模組
   ```rust
   // lib.rs
   pub mod data_engine;
   pub mod execution;
   // 其他頂層模組...
   ```

2. **子模組宣告**：使用與模組同名的 `.rs` 文件來宣告子模組
   ```rust
   // data_engine.rs
   pub mod types;
   ```

3. **子模組實現**：子模組放在與父模組同名的目錄中
   ```
   src/
   ├── data_engine.rs           # 宣告 data_engine 模組的子模組
   └── data_engine/             # 存放 data_engine 子模組實現
       └── types.rs             # types 子模組的實現
   ```

4. **再次子模組**：深層子模組也遵循相同的模式
   ```
   src/
   ├── data_engine.rs           # 宣告 data_engine 模組的子模組
   └── data_engine/             
       ├── types.rs             # 宣告 types 的子模組
       └── types/               # 存放 types 的子模組實現
           ├── data_point.rs
           └── ...
   ```

### 11.2 新舊風格對比

**新風格**（本專案採用）：
```
src/
├── lib.rs                  # 宣告 data_engine 模組
├── data_engine.rs          # 宣告 data_engine 的子模組
└── data_engine/            
    ├── types.rs            # 宣告 types 的子模組
    └── types/              
        ├── data_point.rs
        └── ...
```

### 11.3 好處與最佳實踐

1. **避免多個同名文件**：新風格避免了多個名為 `mod.rs` 的文件，使編輯器中的文件標籤更清晰
2. **路徑直觀**：目錄結構和模組結構有明確對應，更容易導航
3. **模組邊界明確**：每個 `.rs` 文件清晰標示了一個模組的邊界
4. **統一性**：專案中所有模組使用相同的組織方式
5. **導入簡化**：使用 `pub use` 重新導出子模組的項目，簡化外部模組的使用

### 11.4 使用示例

**在 `data_engine/types.rs` 中宣告和重新導出子模組**：
```rust
// 宣告子模組
pub mod data_point;
pub mod time_series;
// ... 其他子模組 ...

// 重新導出，簡化外部使用
pub use data_point::*;
pub use time_series::*;
// ... 重新導出其他子模組 ...
```

**從外部使用**：
```rust
// 由於重新導出，可以直接使用
use crate::data_engine::types::DataPoint;
// 而不必
// use crate::data_engine::types::data_point::DataPoint;
```

遵循此模式可確保代碼組織一致、清晰且易於維護。

## 12. 策略版本管理與回滾系統

策略版本管理系統是確保策略穩定性和可追蹤性的關鍵組件。本專案採用以下策略版本管理與回滾架構：

### 12.1 版本儲存結構

策略檔案以標準化目錄結構保存：

```
strategies/
└── {strategy_id}/              # 策略ID目錄
    ├── {strategy_id}_v1.dsl    # 版本1策略檔案
    ├── {strategy_id}_v2.dsl    # 版本2策略檔案
    └── metadata.json           # 策略元數據文件
```

### 12.2 版本命名規範

策略檔案採用明確的版本命名規範：
- 格式：`{strategy_id}_v{version}.dsl`
- 例如：`my_strategy_v1.dsl`, `my_strategy_v2.dsl`
- 版本號格式為整數遞增，確保版本排序一致性

### 12.3 版本元數據管理

### 12.4 版本管理操作

系統提供以下版本管理核心功能：

1. **創建新版本**
   - 自動生成下一個版本號
   - 保留舊版本檔案
   - 更新元數據記錄

2. **切換版本**
   - 允許指定版本號載入舊版本
   - 支援歷史版本回測比較

3. **自動備份**
   - 每次更新前自動創建舊版本備份
   - 確保版本更新安全性

4. **自動回滾**
   - 檢測新版本載入或執行失敗
   - 自動回滾至最近的穩定版本
   - 恢復策略狀態（如適用）
   - 記錄回滾事件與原因

5. **版本比較**
   - 提供不同版本間的代碼差異比較
   - 支援參數變化追蹤

6. **版本清理**
   - 根據配置的保留策略清理舊版本
   - 支援版本存檔和刪除操作

### 12.5 回滾機制實現

回滾機制設計為防止策略更新失敗時系統穩定性受損：

1. **回滾觸發條件**
   - 新版本策略載入編譯失敗
   - 策略執行時出現嚴重錯誤
   - 策略資源使用超出配額

2. **回滾流程**
   - 立即停止故障版本執行
   - 自動載入上一個穩定版本
   - 恢復策略狀態（如適用）
   - 記錄回滾事件與原因
   - 通知系統管理員

3. **多級回滾**
   - 若回滾版本也失敗，繼續嘗試更早的版本
   - 最多嘗試三個歷史版本
   - 若全部失敗則關閉策略並報警

此版本管理系統確保策略更新過程安全可控，同時保留完整的版本歷史記錄，支援回溯測試和問題診斷。
```
┌─── 初始化階段 ───┐   ┌─── 數據準備階段 ───┐   ┌─── 策略執行階段 ───┐   ┌─── 結果收集階段 ───┐   ┌─── 結果分析階段 ───┐
│                  │   │                    │   │                    │   │                    │   │                    │
│ ┌──────────────┐ │   │ ┌──────────────┐   │   │ ┌──────────────┐   │   │ ┌──────────────┐   │   │ ┌──────────────┐   │
│ │ 配置管理模組  │ │   │ │ 數據提供模組 │   │   │ │ 策略模組     │   │   │ │ 執行模擬器   │   │   │ │ 回測模組     │   │
│ └──────────────┘ │   │ └──────────────┘   │   │ └──────────────┘   │   │ └──────────────┘   │   │ └──────────────┘   │
│         │        │   │         │          │   │         │          │   │         │          │   │         │          │
│ ┌──────────────┐ │   │ ┌──────────────┐   │   │ ┌──────────────┐   │   │ ┌──────────────┐   │   │ ┌──────────────┐   │
│ │ 回測模組     │ │   │ │ 運行時模組   │   │   │ │ DSL模組      │   │   │ │ 風險管理模組 │   │   │ │ 事件處理模組 │   │
│ └──────────────┘ │   │ └──────────────┘   │   │ └──────────────┘   │   │ └──────────────┘   │   │ └──────────────┘   │
│                  │   │                    │   │                    │   │                    │   │                    │
└──────────────────┘   └────────────────────┘   └────────────────────┘   └────────────────────┘   └────────────────────┘
```