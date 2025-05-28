# CI 雙資料庫測試策略

## 概述

本文檔說明在雙資料庫架構下的CI/CD測試策略。系統使用兩個獨立的PostgreSQL數據庫：
- **市場數據庫** (marketdata) - 唯讀，由其他系統維護
- **回測資料庫** (backtest) - 讀寫，由本系統管理

## CI 配置文件

### 主要CI文件
- `.github/workflows/ci-dual-db.yml` - 雙資料庫CI工作流程
- `.github/workflows/ci.yml` - 原始單資料庫CI（向後兼容）

### 配置文件
- `config/test.toml` - 測試環境雙資料庫配置
- `.env.dual-db.example` - 本地開發環境變數範例

## 資料庫設置策略

### 市場數據庫 (Port 5431)
```yaml
postgres-market:
  image: timescale/timescaledb:latest-pg14
  env:
    POSTGRES_USER: market_user
    POSTGRES_PASSWORD: market_pass
    POSTGRES_DB: marketdata
  ports:
    - 5431:5432
```

**特點：**
- 預設為唯讀環境
- 包含測試市場數據
- 模擬外部系統維護的數據結構
- 使用 `scripts/init-market-db.sql` 初始化

### 回測資料庫 (Port 5432)
```yaml
postgres-backtest:
  image: timescale/timescaledb:latest-pg14
  env:
    POSTGRES_USER: backtest_user
    POSTGRES_PASSWORD: backtest_pass
    POSTGRES_DB: backtest
  ports:
    - 5432:5432
```

**特點：**
- 完整讀寫權限
- 運行現有遷移腳本
- 儲存回測結果和策略數據

## 測試流程

### 1. 環境準備
1. 啟動兩個獨立的TimescaleDB實例
2. 在市場數據庫中建立測試數據
3. 在回測資料庫中運行遷移

### 2. 測試階段
1. **格式檢查** - 驗證代碼格式
2. **單元測試** - 測試雙資料庫連接和操作
3. **整合測試** - 測試跨資料庫查詢場景
4. **Clippy檢查** - 代碼品質檢查

### 3. 環境變數
```bash
# CI環境中的資料庫連接
MARKET_DATABASE_URL=postgresql://market_user:market_pass@localhost:5431/marketdata
BACKTEST_DATABASE_URL=postgresql://backtest_user:backtest_pass@localhost:5432/backtest
```

## 本地開發環境

### Docker Compose 設置
使用 `docker-compose-dual-db.yml` 啟動本地雙資料庫環境：

```bash
# 複製環境變數文件
cp .env.dual-db.example .env

# 啟動雙資料庫環境
docker-compose -f docker-compose-dual-db.yml up -d

# 運行測試
cargo test --features dual-db
```

### 遷移管理
```bash
# 僅對回測資料庫運行遷移
cargo run --bin migrate run --target backtest

# 檢查遷移狀態
cargo run --bin migrate status --target backtest
```

## 測試數據管理

### 市場數據庫測試數據
- Bitcoin (BTCUSD) - 5小時歷史數據
- Ethereum (ETHUSD) - 5小時歷史數據  
- Apple (AAPL) - 5小時股票數據

### 數據結構
```sql
-- 基本市場數據表
market_data (timestamp, symbol, open_price, high_price, low_price, close_price, volume)

-- 交易所信息
exchanges (name, code, timezone)

-- 交易品種
instruments (symbol, name, exchange_id, instrument_type, base_currency, quote_currency)
```

## 權限管理

### 市場數據庫權限
- `market_user` - 僅有SELECT權限
- 模擬真實環境的唯讀限制
- 防止意外寫入操作

### 回測資料庫權限
- `backtest_user` - 完整CRUD權限
- 管理回測結果和策略數據
- 運行資料庫遷移

## 故障排除

### 常見問題

1. **連接失敗**
   - 確認端口配置 (市場數據庫: 5431, 回測資料庫: 5432)
   - 檢查服務健康狀態

2. **權限錯誤**
   - 市場數據庫應拒絕寫入操作
   - 回測資料庫應允許完整操作

3. **測試數據不一致**
   - 重新運行 `init-market-db.sql`
   - 確認時區設置正確

### 調試命令
```bash
# 檢查市場數據庫連接
psql -h localhost -p 5431 -U market_user -d marketdata -c "SELECT COUNT(*) FROM market_data;"

# 檢查回測資料庫遷移
psql -h localhost -p 5432 -U backtest_user -d backtest -c "\\dt"

# 運行特定測試
cargo test database_separation_test --features dual-db
```

## 部署考量

### 生產環境
- 市場數據庫由外部系統維護
- 回測資料庫在本系統內部管理
- 使用不同的主機和端口
- 實施適當的網路安全策略

### 監控建議
- 監控兩個資料庫的連接狀態
- 追蹤跨資料庫查詢性能
- 監控市場數據更新頻率
- 追蹤回測資料庫增長

## 遷移策略

### 從單資料庫遷移
1. 保留原有CI配置以確保向後兼容
2. 逐步導入雙資料庫測試
3. 驗證所有測試通過後切換
4. 移除舊的單資料庫依賴

### 版本管理
- 使用功能標籤控制雙資料庫功能
- 維護向後兼容性
- 分階段部署新架構