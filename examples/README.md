# backtest-server 範例程式碼

本目錄包含 backtest-server 項目的範例程式碼，用於展示各個模組的功能和使用方式。

## 可用範例

### 1. Domain Types 模組範例 (`domain_types_demo.rs`)

展示 `domain_types` 模組的核心功能，包括：

#### 主要功能演示
- **時間序列創建**: 如何創建不同頻率和格式的金融時間序列
- **數據格式支持**: OHLCV 和 Tick 數據格式的使用
- **技術指標計算**: 移動平均線等技術指標的應用
- **金融工具管理**: 股票、外匯、加密貨幣等不同資產類型的創建
- **類型安全性**: 編譯時類型檢查的展示

#### 支持的資產類型
- 股票 (`Stock`)
- 期貨 (`Future`)
- 選擇權 (`Option`)
- 外匯 (`Forex`)
- 加密貨幣 (`Crypto`)

#### 核心設計特點
- **泛型時間序列**: `FinancialSeries<F: FrequencyMarker, D: DataFormat>`
- **頻率標記系統**: 支援編譯時頻率檢查
- **數據格式驗證**: 自動檢查數據完整性
- **高效數據處理**: 基於 Polars LazyFrame 的延遲計算

## 執行範例

### 使用 Docker 開發環境 (推薦)

```bash
# 執行 domain_types 範例
cargo make docker-c cargo run --example domain_types_demo

# 執行範例測試
cargo make docker-c cargo test --example domain_types_demo

# 檢查範例編譯
cargo make docker-c cargo check --example domain_types_demo
```

### 本地執行 (需要配置環境)

```bash
# 執行範例
cargo run --example domain_types_demo

# 執行測試
cargo test --example domain_types_demo
```

## 範例輸出

執行 `domain_types_demo` 範例會展示：

1. **OHLCV 時間序列範例**: 創建分鐘級和日級數據，展示數據操作
2. **Tick 數據範例**: 創建高頻 Tick 數據，展示數據預覽
3. **技術指標範例**: 計算移動平均線並展示結果
4. **金融工具創建**: 創建不同類型的金融商品
5. **類型安全性展示**: 展示編譯時類型檢查特性

## 開發指引

### 添加新範例

1. 在 `examples/` 目錄下創建新的 `.rs` 文件
2. 遵循現有範例的結構和註解風格
3. 包含完整的錯誤處理和測試
4. 更新此 README 檔案
5. 在 `CLAUDE.md` 中添加相關說明

### 範例代碼規範

- 使用繁體中文註解和輸出文字
- 包含完整的錯誤處理
- 提供單元測試
- 使用清晰的函數分組和註解
- 展示實際使用場景

### 測試要求

每個範例都應包含：
- 基本功能測試
- 錯誤情況處理
- 邊界條件檢查
- 性能考量 (如適用)

## 相關文檔

- [項目結構文檔](../docs/STRUCTURE.md)
- [Domain Types 架構](../docs/BACKTEST_ARCHITECTURE.md)
- [開發指引](../CLAUDE.md)
- [依賴說明](../docs/DEPENDENCIES.md)