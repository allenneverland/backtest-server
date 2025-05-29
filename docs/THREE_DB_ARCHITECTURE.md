# StratPlat 三資料庫架構設計

## 概述
整個交易系統生態包含三個獨立的資料庫，分別由不同的專案管理。StratPlat 作為前端平台，只直接管理 Website DB，並通過 RabbitMQ 與其他服務互動。

## 資料庫架構

### 1. Website DB (PostgreSQL) - StratPlat 專案（外部專案）
**擁有者**: StratPlat 專案
**權限**: StratPlat 讀寫
**用途**: 儲存核心業務資料和回測結果
**內容**:
- 用戶管理 (users, sessions, api_keys)
- 策略定義 (strategies, strategy_versions)
- 回測記錄 (backtests) - 儲存回測基本資訊
- 回測結果數據:
  - 交易記錄 (backtest_trades) - 每筆交易的詳細資訊
  - 持倉歷史 (backtest_positions) - 持倉快照記錄
  - 績效指標 (backtest_performance_metrics) - 各項績效指標
- 系統管理 (audit_logs, user_favorites)

### 2. History Data DB (TimescaleDB) - 外部專案維護（外部專案）
**擁有者**: 獨立的市場數據專案
**權限**: 多個服務只讀訪問（包括 BacktestServer）
**用途**: 儲存歷史市場數據
**StratPlat 關係**: 無直接連接，未來可能整合
**內容**:
- 市場價格數據 (OHLCV)
- 技術指標數據
- 交易所資訊
- 其他時序數據

### 3. Backtest DB (TimescaleDB) - BacktestServer 專案 (本專案)
**擁有者**: BacktestServer 專案（獨立專案）
**權限**: BacktestServer 讀寫, StratPlat 無權限
**用途**: 儲存回測執行過程的臨時數據
**StratPlat 關係**: 通過 RabbitMQ 間接獲取結果
**內容**:
- 回測執行細節 (backtest_runs)
- 執行過程的中間數據
- 臨時計算結果
- 執行日誌 (execution_logs)
- 注意：完成後的結果會透過 RabbitMQ 傳送給 StratPlat 儲存

## 系統架構圖

```
┌─────────────────────────────────────────────────────────────────────┐
│                              用戶介面                                 │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                           StratPlat (FastAPI)                        │
│                                                                      │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐             │
│  │   Auth      │  │  Strategies  │  │   Backtests   │             │
│  │  Service    │  │   Service    │  │    Service    │             │
│  └─────────────┘  └──────────────┘  └───────────────┘             │
│         │                 │                   │                      │
│         └─────────────────┴───────────────────┘                     │
│                           │                                          │
│                    ┌──────▼──────┐                                  │
│                    │  Website DB  │                                  │
│                    │ (PostgreSQL) │                                  │
│                    └──────────────┘                                  │
│                                                                      │
│  ┌────────────────────────────────────────────┐                    │
│  │          RabbitMQ Client                    │                    │
│  │  - 發送策略和回測請求                         │                    │
│  │  - 接收回測結果和狀態更新                    │                    │
│  └────────────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ RabbitMQ
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        BacktestServer                                │
│                                                                      │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐             │
│  │  Strategy   │  │   Backtest   │  │    Market     │             │
│  │   Parser    │  │    Engine    │  │     Data      │             │
│  └─────────────┘  └──────────────┘  └───────────────┘             │
│         │                 │                   │                      │
│         └─────────────────┴───────────────────┘                     │
│                           │                                          │
│         ┌─────────────────┴────────────────┐                       │
│         │                                   │                       │
│    ┌────▼─────┐                  ┌─────────▼────┐                 │
│    │Backtest DB│                  │History Data DB│                 │
│    │(TimeScale)│                  │ (TimescaleDB) │                 │
│    └───────────┘                  └───────────────┘                 │
│     (Read/Write)                      (Read Only)                   │
└─────────────────────────────────────────────────────────────────────┘
```

## 資料流程

### 1. 提交回測請求
```
User → StratPlat → Website DB (創建 backtest 記錄, status='pending')
                 ↓
                 → RabbitMQ → BacktestServer
```

### 2. 執行回測
```
BacktestServer → History Data DB (讀取歷史數據)
               ↓
               → 執行回測邏輯
               ↓
               → Backtest DB (寫入詳細執行記錄)
               ↓
               → RabbitMQ (發送結果和狀態更新)
```

### 3. 接收回測結果
```
RabbitMQ → StratPlat → Website DB (更新 backtest 記錄，儲存交易、持倉、績效數據)
                     ↓
                     → Redis (快取常用結果)
                     ↓
                     → User (顯示結果)
```

## RabbitMQ 訊息設計

### 1. 回測請求訊息 (StratPlat → BacktestServer)
```json
{
  "message_type": "backtest_request",
  "request_id": "uuid",
  "timestamp": "2024-01-20T10:00:00Z",
  "payload": {
    "backtest_id": 123,
    "strategy_dsl": "...",
    "parameters": {
      "start_date": "2023-01-01",
      "end_date": "2023-12-31",
      "initial_capital": 100000,
      "symbols": ["AAPL", "GOOGL"]
    }
  }
}
```

### 2. 狀態更新訊息 (BacktestServer → StratPlat)
```json
{
  "message_type": "status_update",
  "request_id": "uuid",
  "timestamp": "2024-01-20T10:01:00Z",
  "payload": {
    "backtest_id": 123,
    "status": "running",
    "progress": 45,
    "message": "Processing 2023-06-15..."
  }
}
```

### 3. 回測結果訊息 (BacktestServer → StratPlat)
```json
{
  "message_type": "backtest_result",
  "request_id": "uuid",
  "timestamp": "2024-01-20T10:05:00Z",
  "payload": {
    "backtest_id": 123,
    "status": "completed",
    "execution_time_ms": 5000,
    "results": {
      "total_return": 0.15,
      "sharpe_ratio": 1.2,
      "max_drawdown": -0.08,
      "trade_count": 150,
      "win_rate": 0.58,
      "profit_factor": 1.8
    },
    "trades": [
      {
        "timestamp": "2023-01-15T10:30:00Z",
        "symbol": "AAPL",
        "action": "buy",
        "quantity": 100,
        "price": 150.25,
        "commission": 1.0
      }
      // ... 更多交易記錄
    ],
    "positions": [
      {
        "date": "2023-01-15",
        "symbol": "AAPL",
        "quantity": 100,
        "avg_price": 150.25,
        "market_value": 15025.0
      }
      // ... 更多持倉記錄
    ],
    "performance_metrics": {
      "annualized_return": 0.18,
      "volatility": 0.15,
      "calmar_ratio": 2.25,
      "sortino_ratio": 1.8,
      "max_consecutive_wins": 8,
      "max_consecutive_losses": 3
      // ... 更多績效指標
    },
    "summary_charts": {
      "equity_curve": "base64_encoded_data",
      "drawdown_chart": "base64_encoded_data"
    }
  }
}
```

### 4. 錯誤訊息 (BacktestServer → StratPlat)
```json
{
  "message_type": "error",
  "request_id": "uuid",
  "timestamp": "2024-01-20T10:02:00Z",
  "payload": {
    "backtest_id": 123,
    "error_code": "INVALID_STRATEGY",
    "error_message": "策略語法錯誤: 第 15 行",
    "details": "..."
  }
}
```

## 設計優勢

### 1. 解耦性
- 每個服務管理自己的資料庫
- 服務之間通過明確的消息介面通訊
- 便於獨立開發、測試和部署

### 2. 擴展性
- BacktestServer 可以水平擴展
- 資料庫可以獨立優化和擴展
- 支援多租戶架構

### 3. 安全性
- 最小權限原則
- StratPlat 無法直接訪問敏感的回測執行數據
- 審計追蹤更清晰

### 4. 效能優化
- 專用資料庫避免資源競爭
- TimescaleDB 優化時序數據查詢
- Redis 快取減少資料庫壓力

## 實作注意事項

### 1. 數據一致性
- 使用事務確保 Website DB 的一致性
- RabbitMQ 消息需要確認機制
- 實作重試和錯誤處理邏輯

### 2. 監控和告警
- 監控 RabbitMQ 隊列長度
- 追蹤消息處理延遲
- 資料庫連接池狀態

### 3. 備份策略
- Website DB: 每日完整備份
- History Data DB: 增量備份
- Backtest DB: 定期清理舊數據

### 4. 災難恢復
- RabbitMQ 持久化配置
- 資料庫主從複製
- 定期演練恢復流程

## 未來擴展

### 1. 實時回測
- WebSocket 支援即時狀態更新
- 流式處理回測結果

### 2. 分散式回測
- 支援將大型回測任務分片
- 多個 BacktestServer 協同工作

### 3. 結果分析服務
- 獨立的分析服務處理回測結果
- 機器學習模型優化策略參數

## 相關文檔
- [DB_SCHEMA.md](./DB_SCHEMA.md) - Website DB 詳細架構
- [PLANNING.md](./PLANNING.md) - 系統整體規劃
- [STRUCTURE.md](./STRUCTURE.md) - 專案結構