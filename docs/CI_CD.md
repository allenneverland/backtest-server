# CI/CD 流程文件

## 概述

backtest-server 專案使用 GitHub Actions 進行持續整合與持續部署 (CI/CD)。此流程會在每次推送到 main/develop 分支或建立 pull request 時，自動執行測試、檢查程式碼品質並驗證建置。

## 工作流程結構

### 主要 CI 工作流程 (`ci.yml`)

CI 工作流程包含五個平行執行的任務：

1. **測試套件 (Test Suite)** - 執行所有單元測試與整合測試
2. **格式檢查 (Format Check)** - 使用 rustfmt 驗證程式碼格式
3. **Clippy 檢查** - 執行 linting 與靜態分析
4. **建置檢查 (Build Check)** - 驗證 debug 與 release 模式的建置
5. **整合測試 (Integration Tests)** - 單獨執行整合測試（單執行緒）

### 觸發條件

CI 流程會在以下情況觸發：
- 推送到 `main` 或 `develop` 分支
- 針對 `main` 或 `develop` 分支的 pull request

## 服務配置

測試任務使用以下服務：

### PostgreSQL (TimescaleDB)
- 映像檔：`timescale/timescaledb:latest-pg16`
- 資料庫：`backtest_server_test`
- 認證資訊：postgres/postgres
- 連接埠：5432

### Redis
- 映像檔：`redis:7-alpine`
- 連接埠：6379

### RabbitMQ
- 映像檔：`rabbitmq:3-management-alpine`
- 認證資訊：guest/guest
- 連接埠：5672 (AMQP)、15672 (管理介面)

## 任務詳情

### 測試套件任務
- 安裝包含 rustfmt 和 clippy 的 Rust stable 工具鏈
- 快取相依套件以加速建置
- 執行資料庫遷移
- 使用 `cargo test --all-features` 執行所有測試
- 單獨執行文件測試

### 格式檢查任務
- 輕量級任務，僅檢查格式
- 若程式碼格式不正確則失敗
- 在本地執行 `cargo fmt` 以修正格式問題

### Clippy 任務
- 對所有目標和功能執行 clippy
- 將警告視為錯誤 (`-D warnings`)
- 需要資料庫連線以進行巨集展開

### 建置檢查任務
- 使用矩陣策略測試 debug 和 release 建置
- 驗證所有目標都能成功編譯
- 為每個建置模式分別快取相依套件
- 需要資料庫連線以支援 sqlx 編譯時驗證

### 整合測試任務
- 使用單執行緒執行整合測試 (`--test-threads=1`)
- 確保測試之間不會相互干擾
- 完整的服務堆疊可用（PostgreSQL、Redis、RabbitMQ）

## 環境變數

CI 中設定的環境變數：

```yaml
CARGO_TERM_COLOR: always      # 彩色輸出
RUST_BACKTRACE: 1             # 顯示 panic 時的堆疊追蹤
DATABASE_URL: postgres://...   # PostgreSQL 連線
REDIS_URL: redis://...        # Redis 連線
RABBITMQ_URL: amqp://...      # RabbitMQ 連線
RUST_LOG: debug               # 日誌等級
```

## 快取策略

工作流程使用 `Swatinem/rust-cache@v2` 進行智慧快取：
- 快取已編譯的相依套件
- 不同建置配置使用獨立的快取
- 快取鍵包含作業系統和 Cargo.lock 雜湊值

## 本地測試

推送前，您可以在本地執行相同的檢查：

```bash
# 執行測試
cargo test --all-features

# 檢查格式
cargo fmt --all -- --check

# 執行 clippy
cargo clippy --all-targets --all-features -- -D warnings

# 建置 debug 版本
cargo build --all-targets

# 建置 release 版本
cargo build --release --all-targets

# 執行整合測試
cargo test --test '*' -- --test-threads=1
```

### 使用 Docker 環境

為了與 CI 保持一致，請使用 Docker 環境：

```bash
# 啟動服務
cargo make docker-up

# 在 Docker 中執行測試
cargo make docker-c cargo test

# 檢查格式
cargo make docker-c cargo format-check

# 執行 linter
cargo make docker-c cargo lint
```

## 驗證腳本

專案在 `scripts/` 目錄中包含驗證腳本：

- `test-ci-commands.sh` - 測試所有 CI 指令在本地是否可用
- `validate-github-workflows.sh` - 驗證工作流程 YAML 語法
- `test-ci-readiness.sh` - 快速就緒檢查

在修改 CI 前執行這些腳本：

```bash
./scripts/test-ci-commands.sh
./scripts/validate-github-workflows.sh
```

## 疑難排解

### 常見問題

1. **格式檢查失敗**
   - 推送前在本地執行 `cargo fmt`
   - 確保您的 rustfmt 版本與 CI 相符

2. **Clippy 警告**
   - 使用 `cargo clippy --fix` 在本地修正警告
   - 某些警告可能需要手動修正

3. **資料庫連線錯誤**
   - 確保遷移成功執行
   - 檢查 DATABASE_URL 是否正確設定

4. **不穩定的整合測試**
   - 整合測試使用單執行緒執行以避免衝突
   - 確保適當的測試隔離和清理

### 偵錯 CI 失敗

1. 在 GitHub Actions 中檢查失敗的特定任務
2. 查看錯誤輸出和堆疊追蹤
3. 使用 Docker 環境在本地重現問題
4. 使用 `RUST_LOG=debug` 獲得更詳細的輸出

## 未來改進

CI/CD 流程的潛在改進：

- [ ] 新增程式碼覆蓋率報告
- [ ] 實作安全掃描
- [ ] 新增效能基準測試
- [ ] 設定部署工作流程
- [ ] 新增 Docker 映像建置和發布
- [ ] 實作發布自動化

## 維護

### 更新相依套件

更新 Rust 工具鏈或相依套件時：
1. 先在本地測試變更
2. 如需要，更新 CI 工作流程
3. 若出現建置問題，清除快取

### 新增服務

要在 CI 中新增服務：
1. 在工作流程中新增服務定義
2. 配置健康檢查
3. 設定適當的環境變數
4. 更新文件

## 相關文件

- [專案結構](./STRUCTURE.md)
- [開發規劃](./PLANNING.md)
- [相依套件](./DEPENDENCIES.md)