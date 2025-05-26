# backtest-server - 高性能金融回測伺服器

## 概述

BacktestServer是一個使用Rust語言開發的高性能金融回測伺服器，專為支持DSL策略、多數據類型和動態策略管理而設計。系統採用模組化架構，以事件驅動方式構建，優化處理金融交易數據。本系統主要用於運行一個網站服務，讓客戶通過API將DSL策略傳輸到伺服器進行編譯和回測，完成後將結果回傳給客戶。

### 主要特性

- **高性能**: 利用Rust的高效能特性，實現低延遲數據處理和回測
- **強大的數據處理**: 支持多種數據格式和來源的高效處理
- **模組化設計**: 清晰分離的組件使系統易於擴展和維護
- **高效數據結構**: 針對金融數據的專門優化數據結構
- **多用戶同時服務**: 支持多客戶並行提交不同策略進行回測
- **動態策略管理**: 系統支持策略的注冊、加載、初始化等標準生命週期管理
- **完整的回測環境**: 提供隔離的回測環境，確保多用戶間策略互不干擾

## 系統架構

BacktestServer採用模組化的架構設計，目前已實現的主要模組：

### 核心組件

- **數據引擎**: 管理市場數據的獲取、標準化和存取
  - 支持CSV格式
  - 數據加載和轉換
  - 高效數據緩存

- **策略管理器**: 管理DSL策略的整個生命週期
  - 策略動態載入和卸載
  - 策略沙箱隔離環境
  - 標準化的策略生命週期管理


## 回測環境特別說明

在回測環境中，策略遵循簡化的生命週期：

1. 註冊和加載策略
2. 初始化回測環境
3. 執行回測
4. 停止和卸載策略
5. 分析回測結果

以下是回測環境的關鍵限制：

1. 每個策略執行完成後會結束沙箱
2. 不允許在回測中途調整參數或更新策略
3. 策略修改後需要重新啟動新的沙箱進行回測

這種設計確保了回測結果的一致性和可重現性，但也意味著每次參數調整都需要重新執行完整的回測流程。

## 使用範例

以下是使用BacktestServer導入市場數據的簡單範例：

```rust
use backtest_server::DataEngine;
use backtest_server::market_types::{AssetType, Exchange};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 創建數據引擎實例
    let engine = DataEngine::new()?;
    
    // 載入市場數據
    let symbol = "AAPL";
    let start_date = "2024-01-01";
    let end_date = "2024-01-10";
    
    let data = engine.load_market_data(symbol, start_date, end_date).await?;
    println!("成功載入數據: {} 條記錄", data.len());

    Ok(())
}
```

## 開發工具

BacktestServer專案使用以下工具簡化開發和部署流程：

### Cargo-Make

使用cargo-make作為主要任務執行工具：

```bash
# 安裝cargo-make
cargo install cargo-make

# 運行默認任務
cargo make

# 運行特定任務
cargo make build-release
```

## 文檔

專案提供了詳細的文檔，幫助開發者和用戶了解系統架構、開發計劃和使用方法：

### 架構和設計

- [專案規劃](docs/PLANNING.md) - 詳細的專案規劃、目標和技術選擇說明
- [系統結構](docs/STRUCTURE.md) - 完整的系統架構和組件設計文檔

### 開發指南

- [任務清單](docs/TASK.md) - 專案開發進度和任務跟蹤
- [Rust學習指南](docs/rust-learning-guide.md) - Rust語言學習資源和最佳實踐

### 數據庫設計

- [TimescaleDB結構設計](docs/timescaledb-schema.md) - 時序數據庫結構設計文檔

### CI/CD 流程

專案使用 GitHub Actions 進行持續整合，確保程式碼品質和測試覆蓋：

- [CI/CD 文檔](docs/CI_CD.md) - 持續整合流程設定與使用指南

### Docker 與遷移

數據庫容器在首次啟動時會自動執行所有遷移。這是通過自定義 Dockerfile.db 實現的：

1. **啟動容器**：
   ```bash
   docker-compose up -d
   ```

2. **檢查遷移是否成功**：
   ```bash
   docker-compose logs db
   ```

3. **手動觸發遷移**（如果需要）：
   ```bash
   docker-compose exec db migrate run
   ```

**添加新遷移時**：只需將新的遷移文件添加到 `migrations/` 目錄中，然後:
- 對於已存在的數據庫容器：使用上面的手動觸發命令
- 對於新容器：遷移將在首次啟動時自動運行

這種方法使您可以輕鬆地維護和版本控制數據庫架構，同時確保所有環境（本地開發、測試和生產）使用相同的數據庫結構。