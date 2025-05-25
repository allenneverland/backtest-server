# GitLab CI/CD 配置指南

本文件說明 BacktestServer 專案的 GitLab CI/CD 配置和使用方式。

## 概述

我們使用 GitLab CI/CD 來自動化以下流程：
- 代碼構建和編譯
- 單元測試和整合測試
- 代碼品質檢查
- 測試覆蓋率報告
- Docker 映像建置
- 自動部署（需手動觸發）

## Pipeline 階段

### 1. Prepare（準備）
- 安裝必要的系統依賴
- 安裝 Rust 工具鏈組件（rustfmt、clippy）
- 安裝 cargo 擴展工具

### 2. Build（構建）
- **build:check** - 檢查代碼是否能成功編譯
- **build:debug** - 構建 debug 版本（分支和 MR）
- **build:release** - 構建 release 版本（僅 main 分支和標籤）

### 3. Test（測試）
- **test:unit** - 執行單元測試
- **test:integration** - 執行整合測試（允許失敗）

### 4. Quality（品質檢查）
- **quality:format** - 檢查代碼格式（rustfmt）
- **quality:lint** - 執行 lint 檢查（clippy）
- **security:dependency-check** - 安全依賴檢查（cargo-audit）

### 5. Coverage（覆蓋率）
- **coverage:report** - 生成測試覆蓋率報告

### 6. Deploy（部署）
- **deploy:staging** - 部署到測試環境（手動觸發）
- **deploy:production** - 部署到生產環境（手動觸發）

## 環境變數配置

在 GitLab 專案設定中需要配置以下變數：

### 資料庫相關
```
DB_HOST=postgres
DB_PORT=5432
DB_USER=backtest
DB_PASSWORD=<your-secure-password>
DB_NAME=backtest_db
```

### Redis 相關
```
REDIS_HOST=redis
REDIS_PORT=6379
```

### RabbitMQ 相關
```
RABBITMQ_HOST=rabbitmq
RABBITMQ_PORT=5672
RABBITMQ_USER=guest
RABBITMQ_PASSWORD=<your-secure-password>
```

### Docker Registry（如需要）
```
CI_REGISTRY_USER=<your-username>
CI_REGISTRY_PASSWORD=<your-password>
```

## 本地驗證

在推送到 GitLab 之前，可以使用驗證腳本檢查配置：

```bash
./.gitlab/ci-lint.sh
```

## 使用指南

### 1. 觸發 Pipeline

Pipeline 會在以下情況自動觸發：
- 推送到任何分支
- 建立或更新 Merge Request
- 建立新標籤

### 2. 查看測試報告

測試報告會自動生成並顯示在：
- Merge Request 頁面
- Pipeline 詳情頁面
- 覆蓋率報告可在 Pipeline artifacts 中下載

### 3. 手動部署

部署階段需要手動觸發：
1. 進入 Pipeline 頁面
2. 找到 deploy 階段
3. 點擊播放按鈕開始部署

### 4. 快取管理

Pipeline 使用快取來加速構建：
- 基於 `Cargo.lock` 和 `Cargo.toml` 的變化管理快取
- 快取包含 cargo 依賴和 target 目錄

## 故障排除

### 常見問題

1. **資料庫連接失敗**
   - 確認環境變數正確設置
   - 檢查服務容器是否正常啟動

2. **測試超時**
   - 可能是整合測試需要更多時間
   - 考慮調整超時設定

3. **快取問題**
   - 清除快取：在 Pipeline 頁面選擇 "Clear Runner Caches"
   - 更新快取鍵值

### 調試技巧

1. 在本地模擬 CI 環境：
   ```bash
   docker run -it rust:1.87-slim-bullseye bash
   # 執行 CI 指令
   ```

2. 查看詳細日誌：
   - 在 Pipeline 頁面點擊失敗的 job
   - 查看完整的執行日誌

## 最佳實踐

1. **保持 Pipeline 快速**
   - 使用並行執行
   - 合理使用快取
   - 避免不必要的步驟

2. **測試覆蓋率**
   - 維持高測試覆蓋率（目標 > 80%）
   - 定期檢查覆蓋率報告

3. **安全性**
   - 定期更新依賴
   - 使用 cargo-audit 檢查安全漏洞
   - 不要在代碼中硬編碼敏感資訊

## 擴展配置

如需添加新的 CI/CD 功能：

1. 編輯 `.gitlab-ci.yml`
2. 使用驗證腳本檢查
3. 在 feature branch 測試
4. 建立 MR 進行代碼審查

## 相關資源

- [GitLab CI/CD 文檔](https://docs.gitlab.com/ee/ci/)
- [Rust CI 最佳實踐](https://github.com/rust-lang/rust/blob/master/.github/workflows/ci.yml)
- [cargo-make 文檔](https://github.com/sagiegurari/cargo-make)