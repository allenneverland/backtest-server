# 資料庫分離架構

## 概述

backtest-server 現在支援雙資料庫架構，將市場數據與回測相關數據分離。這種分離提供了更好的安全性、可擴展性和資源管理。

## 架構

### 資料庫結構

1. **市場數據資料庫 (`marketdata`)** - 唯讀
   - 包含所有市場數據（OHLCV、tick、指標）
   - 可被多個系統存取
   - 從回測伺服器配置為唯讀存取
   - 針對高讀取吞吐量進行優化

2. **回測資料庫 (`backtest`)** - 讀寫
   - 包含回測相關數據（結果、策略、配置）
   - 與市場數據操作隔離
   - 完全讀寫存取權限
   - 針對事務性操作進行優化

### 配置

資料庫分離在環境特定的配置檔案中設定：

#### 開發環境配置 (`config/development.toml`)
```toml
[market_database]
host = "db"
port = 5432
username = "market_reader"
password = "market_reader_pass"
database = "marketdata"
# ... 連接池設定

[backtest_database]
host = "db"
port = 5432
username = "backtest_server"
password = "backtest_server"
database = "backtest_server"
# ... 連接池設定
```

#### 生產環境配置 (`config/production.toml`)
```toml
[market_database]
host = "marketdb.production"
port = 5432
username = "market_reader"
password = "market_reader_prod"
database = "marketdata_prod"
# ... 連接池設定

[backtest_database]
host = "backtestdb.production"
port = 5432
username = "backtest_server"
password = "production_password"
database = "backtest_server_prod"
# ... 連接池設定
```

## 實作細節

### 資料庫模組更新

`storage::database` 模組現在提供：

1. **獨立的連接池**
   - `get_market_data_pool()` - 返回市場數據資料庫池
   - `get_backtest_pool()` - 返回回測資料庫池

2. **資料庫包裝器**
   - `MarketDataDatabase` - 強制唯讀操作
   - `BacktestDatabase` - 允許完整讀寫操作

3. **池管理器**
   - `DatabasePoolManager` - 管理兩個資料庫池
   - 提供兩個資料庫的健康檢查

### Repository 模式

Repository 綁定到特定的資料庫：

- **MarketDataRepository** → 使用市場數據資料庫（唯讀）
- **BacktestRepository** → 使用回測資料庫（讀寫）
- **StrategyRepository** → 使用回測資料庫（讀寫）
- **其他 repository** → 根據數據類型使用適當的資料庫

### 遷移支援

遷移工具現在支援針對特定資料庫：

```bash
# 在兩個資料庫上運行遷移（預設）
cargo make docker-c cargo run --bin migrate run

# 僅在市場數據資料庫上運行遷移
cargo make docker-c cargo run --bin migrate run --target market

# 僅在回測資料庫上運行遷移
cargo make docker-c cargo run --bin migrate run --target backtest
```

## 優點

1. **安全性**
   - 市場數據資料庫是唯讀的，防止意外修改
   - 每個資料庫使用獨立的憑證
   - 市場數據和回測操作之間的隔離

2. **效能**
   - 資料庫的獨立擴展
   - 針對每個用例優化的連接池
   - 減少資源競爭

3. **維護**
   - 更容易的備份策略（市場數據可以較少頻率備份）
   - 獨立的維護視窗
   - 清晰的關注點分離

## 使用範例

### 存取市場數據
```rust
use crate::storage::database::get_market_data_pool;
use crate::storage::repository::{MarketDataRepo, DbExecutor};

let pool = get_market_data_pool(false).await?;
let repo = MarketDataRepo::new(pool.clone());
// 市場數據操作是唯讀的
```

### 存取回測數據
```rust
use crate::storage::database::get_backtest_pool;
use crate::storage::repository::{BacktestRepo, DbExecutor};

let pool = get_backtest_pool(false).await?;
let repo = BacktestRepo::new(pool.clone());
// 回測操作支援讀寫
```

### 健康檢查
```rust
use crate::storage::database::DatabasePoolManager;

let manager = DatabasePoolManager::new(market_config, backtest_config).await?;
let health = manager.health_check().await?;

if health.market_data_healthy && health.backtest_healthy {
    println!("兩個資料庫都是健康的");
}
```

## 遷移指南

將現有部署遷移到雙資料庫架構：

1. **備份現有資料庫**
2. **建立新的市場數據資料庫**
3. **將市場數據表複製到新資料庫**
4. **使用新的資料庫設定更新配置檔案**
5. **部署更新的應用程式**
6. **驗證兩個資料庫都可以存取**
7. **從回測資料庫中刪除市場數據表（可選）**

## 疑難排解

### 常見問題

1. **連接池超時**
   - 檢查資料庫連接性
   - 驗證憑證和權限
   - 確保連接池設定適當

2. **在市場數據資料庫上的寫入操作**
   - 這會按設計失敗
   - 確保市場數據操作是唯讀的
   - 檢查 repository 綁定

3. **遷移失敗**
   - 使用特定目標運行遷移
   - 檢查遷移日誌以獲取詳細資訊
   - 驗證資料庫權限