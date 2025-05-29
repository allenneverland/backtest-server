# 回測執行資料庫結構設計

## 目錄

- [1. 概述](#1-概述)
- [2. 資料庫配置](#2-資料庫配置)
  - [2.1 基本配置](#21-基本配置)
- [3. 核心數據表結構](#3-核心數據表結構)
  - [3.1 執行管理表](#31-執行管理表)
  - [3.2 執行數據表](#32-執行數據表)
- [4. 連續聚合(Continuous Aggregates)](#4-連續聚合continuous-aggregates)
  - [4.1 執行結果聚合](#41-執行結果聚合)
- [5. 資料庫權限與角色](#5-資料庫權限與角色)
- [6. 數據生命週期管理](#6-數據生命週期管理)
- [7. 查詢優化示例](#7-查詢優化示例)
  - [7.1 執行狀態查詢](#71-執行狀態查詢)
  - [7.2 執行數據查詢](#72-執行數據查詢)
  - [7.3 效能分析查詢](#73-效能分析查詢)

## 表結構概覽

| 表名 | 類型 | 主要字段 | 用途 |
|------|------|---------|------|
| `execution_runs` | 普通表 | run_id, external_backtest_id, request_id, status | 存儲執行任務記錄 |
| `execution_logs` | 普通表 | log_id, run_id, timestamp, log_level, message | 存儲執行日誌 |
| `execution_trades` | 超表 | time, run_id, instrument_id, direction, price | 存儲臨時交易記錄 |
| `execution_positions` | 超表 | time, run_id, instrument_id, quantity, market_value | 存儲臨時倉位快照 |
| `execution_portfolios` | 超表 | time, run_id, total_value, cash, equity | 存儲臨時投資組合快照 |
| `execution_daily_returns` | 連續聚合 | bucket, run_id, daily_return, end_of_day_value | 執行日收益率聚合 |

## 表關係圖

```
                    execution_runs
                           │
                           ├──── execution_logs
                           │
                           ▼
          ┌────────────────┼─────────────────┐
          ▼                ▼                 ▼
   execution_trades   execution_positions   execution_portfolios
                                                    │
                                                    ▼
                                            execution_daily_returns
```

## 1. 概述

回測執行資料庫（Backtest DB）是 BacktestServer 專案專屬的資料庫，專門存儲回測執行過程中的臨時數據。根據三資料庫架構設計，此資料庫：

- **不存儲策略定義**：策略通過 RabbitMQ 從 StratPlat 接收
- **不存儲最終結果**：執行完成後的結果通過 RabbitMQ 發送給 StratPlat
- **只保留臨時數據**：執行過程的中間數據、日誌和計算結果
- **自動清理機制**：舊數據定期清理，避免無限增長

本資料庫使用 TimescaleDB 來高效處理時間序列數據，並與 History Data DB（市場數據）和 Website DB（業務數據）完全分離。

## 2. 資料庫配置

### 2.1 基本配置

```sql
-- 創建回測執行資料庫
CREATE DATABASE backtest_execution;

-- 連接到資料庫
\c backtest_execution

-- 啟用必要的擴展
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS pg_cron;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
```

## 3. 核心數據表結構

### 3.1 執行管理表

```sql
-- 回測執行記錄表
CREATE TABLE execution_runs (
    run_id SERIAL PRIMARY KEY,
    external_backtest_id INTEGER NOT NULL,      -- StratPlat 的 backtest ID
    request_id UUID NOT NULL UNIQUE,            -- RabbitMQ 請求 ID
    strategy_dsl TEXT NOT NULL,                 -- 從 RabbitMQ 接收的策略 DSL
    parameters JSONB NOT NULL,                  -- 執行參數（開始日期、結束日期、初始資金等）
    status VARCHAR(20) NOT NULL DEFAULT 'INITIALIZING', -- INITIALIZING, RUNNING, COMPLETED, FAILED
    progress INTEGER DEFAULT 0,                 -- 執行進度 (0-100)
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    execution_time_ms INTEGER,                  -- 執行時間（毫秒）
    error_code VARCHAR(50),                     -- 錯誤代碼
    error_message TEXT,                         -- 錯誤訊息
    error_details JSONB,                        -- 詳細錯誤信息
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_execution_runs_external_backtest_id ON execution_runs(external_backtest_id);
CREATE INDEX idx_execution_runs_request_id ON execution_runs(request_id);
CREATE INDEX idx_execution_runs_status ON execution_runs(status);
CREATE INDEX idx_execution_runs_started_at ON execution_runs(started_at);

-- 執行日誌表
CREATE TABLE execution_logs (
    log_id BIGSERIAL PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    log_level VARCHAR(10) NOT NULL,            -- DEBUG, INFO, WARN, ERROR
    component VARCHAR(50),                      -- 組件名稱（如：data_loader, strategy_executor）
    message TEXT NOT NULL,
    details JSONB,                              -- 額外的結構化數據
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_execution_logs_run_id ON execution_logs(run_id);
CREATE INDEX idx_execution_logs_timestamp ON execution_logs(timestamp);
CREATE INDEX idx_execution_logs_log_level ON execution_logs(log_level);
CREATE INDEX idx_execution_logs_component ON execution_logs(component);
```

### 3.2 執行數據表

```sql
-- 執行交易記錄表（臨時存儲）
CREATE TABLE execution_trades (
    time TIMESTAMPTZ NOT NULL,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
    instrument_id INTEGER NOT NULL, -- 參考市場數據庫的 instrument
    direction VARCHAR(10) NOT NULL, -- 'BUY', 'SELL'
    price NUMERIC(18,8) NOT NULL,
    quantity NUMERIC(24,8) NOT NULL,
    amount NUMERIC(24,8) NOT NULL,
    commission NUMERIC(18,8) NOT NULL,
    slippage NUMERIC(18,8),
    trade_id VARCHAR(50), -- 回測系統內部交易ID
    position_effect VARCHAR(20), -- 'OPEN', 'CLOSE', 'ADJUST'
    order_type VARCHAR(20), -- 'MARKET', 'LIMIT', 'STOP'
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('execution_trades', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_execution_trades_run ON execution_trades(run_id);
CREATE INDEX idx_execution_trades_instrument ON execution_trades(instrument_id);
CREATE INDEX idx_execution_trades_run_time ON execution_trades(run_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE execution_trades SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'run_id, instrument_id'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('execution_trades', INTERVAL '7 days');

-- 執行倉位快照表（臨時存儲）
CREATE TABLE execution_positions (
    time TIMESTAMPTZ NOT NULL,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
    instrument_id INTEGER NOT NULL, -- 參考市場數據庫的 instrument
    quantity NUMERIC(24,8) NOT NULL,
    avg_cost NUMERIC(18,8) NOT NULL,
    market_value NUMERIC(18,8) NOT NULL,
    unrealized_pl NUMERIC(18,8) NOT NULL,
    realized_pl NUMERIC(18,8) NOT NULL,
    margin_used NUMERIC(18,8),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('execution_positions', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_execution_positions_run ON execution_positions(run_id);
CREATE INDEX idx_execution_positions_instrument ON execution_positions(instrument_id);
CREATE INDEX idx_execution_positions_run_time ON execution_positions(run_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE execution_positions SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'run_id, instrument_id'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('execution_positions', INTERVAL '7 days');

-- 執行投資組合快照表（臨時存儲）
CREATE TABLE execution_portfolios (
    time TIMESTAMPTZ NOT NULL,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
    total_value NUMERIC(24,8) NOT NULL, -- 投資組合總價值
    cash NUMERIC(24,8) NOT NULL, -- 可用資金
    equity NUMERIC(24,8) NOT NULL, -- 權益
    margin NUMERIC(24,8), -- 保證金
    daily_pnl NUMERIC(18,8), -- 當日盈虧
    total_pnl NUMERIC(18,8), -- 總盈虧
    daily_return NUMERIC(10,6), -- 當日回報率
    total_return NUMERIC(10,6), -- 總回報率
    metadata JSONB, -- 其他指標
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('execution_portfolios', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_execution_portfolios_run ON execution_portfolios(run_id);
CREATE INDEX idx_execution_portfolios_run_time ON execution_portfolios(run_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE execution_portfolios SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'run_id'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('execution_portfolios', INTERVAL '7 days');
```

## 4. 連續聚合(Continuous Aggregates)

### 4.1 執行結果聚合

```sql
-- 執行日收益率聚合視圖
CREATE MATERIALIZED VIEW execution_daily_returns
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    run_id,
    last(total_return, time) AS daily_return,
    last(total_value, time) AS end_of_day_value,
    last(equity, time) AS end_of_day_equity
FROM execution_portfolios
GROUP BY bucket, run_id;

-- 添加自動刷新策略（每小時刷新）
SELECT add_continuous_aggregate_policy('execution_daily_returns',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');
```

## 5. 資料庫權限與角色

根據三資料庫架構設計，BacktestServer 是此資料庫的唯一擁有者，StratPlat 無直接訪問權限：

```sql
-- 創建回測應用程序角色（BacktestServer 使用）
CREATE ROLE backtest_app WITH LOGIN PASSWORD 'backtest_app_password';

-- 創建回測管理員角色
CREATE ROLE backtest_admin WITH LOGIN PASSWORD 'backtest_admin_password';

-- 設置權限
-- 應用程序角色：完整的CRUD權限
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO backtest_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO backtest_app;
GRANT USAGE ON SCHEMA public TO backtest_app;

-- 管理員角色：完全權限
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO backtest_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO backtest_admin;
GRANT USAGE ON SCHEMA public TO backtest_admin;

-- 注意：沒有設置只讀角色，因為外部系統（如 StratPlat）通過 RabbitMQ 獲取數據，而非直接訪問資料庫
```

## 6. 數據生命週期管理

由於此資料庫只存儲臨時執行數據，實施自動清理策略以防止數據無限增長：

```sql
-- 設置數據保留策略（30天）
SELECT add_retention_policy('execution_trades', INTERVAL '30 days');
SELECT add_retention_policy('execution_positions', INTERVAL '30 days');
SELECT add_retention_policy('execution_portfolios', INTERVAL '30 days');

-- 執行日誌保留更短時間（7天）
CREATE OR REPLACE FUNCTION cleanup_old_logs()
RETURNS void AS $$
BEGIN
    DELETE FROM execution_logs 
    WHERE created_at < NOW() - INTERVAL '7 days';
    
    -- 清理已完成超過30天的執行記錄
    DELETE FROM execution_runs 
    WHERE status IN ('COMPLETED', 'FAILED') 
    AND completed_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- 創建定期清理任務
SELECT cron.schedule('cleanup_old_data', '0 2 * * *', 'SELECT cleanup_old_logs();');
```

## 7. 查詢優化示例

### 7.1 執行狀態查詢

```sql
-- 查詢執行狀態
SELECT 
    er.run_id,
    er.external_backtest_id,
    er.request_id,
    er.status,
    er.progress,
    er.started_at,
    er.completed_at,
    er.execution_time_ms,
    er.error_code,
    er.error_message
FROM execution_runs er
WHERE er.external_backtest_id = 123;

-- 查詢最近的執行日誌
SELECT 
    el.timestamp,
    el.log_level,
    el.component,
    el.message,
    el.details
FROM execution_logs el
JOIN execution_runs er ON el.run_id = er.run_id
WHERE er.external_backtest_id = 123
AND el.log_level IN ('WARN', 'ERROR')
ORDER BY el.timestamp DESC
LIMIT 100;

-- 監控執行進度
SELECT 
    er.run_id,
    er.external_backtest_id,
    er.status,
    er.progress,
    er.started_at,
    EXTRACT(EPOCH FROM (NOW() - er.started_at)) AS elapsed_seconds,
    COUNT(et.time) AS trades_processed
FROM execution_runs er
LEFT JOIN execution_trades et ON er.run_id = et.run_id
WHERE er.status = 'RUNNING'
GROUP BY er.run_id;
```

### 7.2 執行數據查詢

```sql
-- 查詢執行交易記錄（準備發送給 StratPlat）
SELECT 
    et.time,
    et.instrument_id,
    et.direction,
    et.price,
    et.quantity,
    et.amount,
    et.commission,
    et.slippage,
    et.position_effect,
    et.order_type
FROM execution_trades et
JOIN execution_runs er ON et.run_id = er.run_id
WHERE er.external_backtest_id = 123
ORDER BY et.time;

-- 查詢執行投資組合快照（用於生成績效曲線）
SELECT 
    time,
    total_value,
    cash,
    equity,
    daily_pnl,
    total_pnl,
    daily_return,
    total_return
FROM execution_portfolios ep
JOIN execution_runs er ON ep.run_id = er.run_id
WHERE er.external_backtest_id = 123
ORDER BY time;

-- 計算執行績效指標（準備發送給 StratPlat）
WITH portfolio_data AS (
    SELECT 
        ep.run_id,
        ep.total_value,
        ep.total_return,
        ep.daily_return
    FROM execution_portfolios ep
    JOIN execution_runs er ON ep.run_id = er.run_id
    WHERE er.external_backtest_id = 123
    ORDER BY ep.time
),
trade_data AS (
    SELECT 
        COUNT(*) AS total_trades,
        COUNT(CASE WHEN (amount * CASE WHEN direction = 'SELL' THEN -1 ELSE 1 END) > 0 THEN 1 END) AS winning_trades
    FROM execution_trades et
    JOIN execution_runs er ON et.run_id = er.run_id
    WHERE er.external_backtest_id = 123
)
SELECT 
    MAX(pd.total_return) AS total_return,
    MIN(pd.total_return) AS max_drawdown,
    STDDEV(pd.daily_return) * SQRT(252) AS annualized_volatility,
    td.total_trades,
    td.winning_trades,
    CASE WHEN td.total_trades > 0 THEN td.winning_trades::numeric / td.total_trades ELSE 0 END AS win_rate
FROM portfolio_data pd
CROSS JOIN trade_data td
GROUP BY td.total_trades, td.winning_trades;
```

### 7.3 效能分析查詢

```sql
-- 監控資料庫使用情況
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size,
    n_live_tup AS row_count
FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- 分析執行效能
SELECT 
    DATE_TRUNC('hour', er.started_at) AS hour,
    COUNT(*) AS executions,
    AVG(er.execution_time_ms) AS avg_execution_time_ms,
    MIN(er.execution_time_ms) AS min_execution_time_ms,
    MAX(er.execution_time_ms) AS max_execution_time_ms,
    COUNT(CASE WHEN er.status = 'FAILED' THEN 1 END) AS failed_count
FROM execution_runs er
WHERE er.started_at >= NOW() - INTERVAL '24 hours'
GROUP BY DATE_TRUNC('hour', er.started_at)
ORDER BY hour DESC;

-- 查詢待清理的數據量
SELECT 
    'execution_trades' AS table_name,
    COUNT(*) AS old_records,
    pg_size_pretty(pg_total_relation_size('execution_trades')) AS table_size
FROM execution_trades
WHERE time < NOW() - INTERVAL '30 days'
UNION ALL
SELECT 
    'execution_positions' AS table_name,
    COUNT(*) AS old_records,
    pg_size_pretty(pg_total_relation_size('execution_positions')) AS table_size
FROM execution_positions
WHERE time < NOW() - INTERVAL '30 days'
UNION ALL
SELECT 
    'execution_portfolios' AS table_name,
    COUNT(*) AS old_records,
    pg_size_pretty(pg_total_relation_size('execution_portfolios')) AS table_size
FROM execution_portfolios
WHERE time < NOW() - INTERVAL '30 days';
```

## 備註

1. **數據流向**：
   - 策略 DSL 通過 RabbitMQ 從 StratPlat 接收
   - 執行結果通過 RabbitMQ 發送回 StratPlat
   - 市場數據從 History Data DB 讀取（只讀）

2. **數據保留**：
   - 執行數據保留 30 天
   - 日誌保留 7 天
   - 舊數據自動清理

3. **與其他資料庫的關係**：
   - 不直接連接 Website DB（StratPlat 的資料庫）
   - 只讀訪問 History Data DB（市場數據）
   - 所有跨系統通信通過 RabbitMQ