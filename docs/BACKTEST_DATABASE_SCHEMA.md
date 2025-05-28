# 回測資料庫結構設計

## 目錄

- [1. 概述](#1-概述)
- [2. 資料庫配置](#2-資料庫配置)
  - [2.1 基本配置](#21-基本配置)
- [3. 核心數據表結構](#3-核心數據表結構)
  - [3.1 策略定義與管理](#31-策略定義與管理)
  - [3.2 回測系統數據表](#32-回測系統數據表)
- [4. 連續聚合(Continuous Aggregates)](#4-連續聚合continuous-aggregates)
  - [4.1 回測結果聚合](#41-回測結果聚合)
- [5. 資料庫權限與角色](#5-資料庫權限與角色)
- [6. 查詢優化示例](#6-查詢優化示例)
  - [6.1 回測結果查詢](#61-回測結果查詢)
  - [6.2 策略管理查詢](#62-策略管理查詢)
  - [6.3 效能分析查詢](#63-效能分析查詢)

## 表結構概覽

| 表名 | 類型 | 主要字段 | 用途 |
|------|------|---------|------|
| `strategy` | 普通表 | strategy_id, name, version, code, parameters | 存儲策略定義 |
| `strategy_version` | 普通表 | version_id, strategy_id, version, source_path | 存儲策略版本信息 |
| `backtest_config` | 普通表 | config_id, name, start_date, end_date, strategy_id | 存儲回測配置信息 |
| `backtest_result` | 普通表 | result_id, config_id, status, metrics | 存儲回測結果摘要和績效指標 |
| `backtest_trade` | 超表 | time, result_id, instrument_id, direction, price | 存儲回測交易記錄 |
| `backtest_position_snapshot` | 超表 | time, result_id, instrument_id, quantity, market_value | 存儲回測倉位快照 |
| `backtest_portfolio_snapshot` | 超表 | time, result_id, total_value, cash, equity | 存儲回測投資組合快照 |
| `backtest_daily_returns` | 連續聚合 | bucket, result_id, daily_return, end_of_day_value | 回測日收益率聚合 |

## 表關係圖

```
strategy ──────────────────┐
    ↑                      │
    │                      │
strategy_version           │
                           │
                    backtest_config
                           ↓
                    backtest_result
                           ↓
          ┌────────────────┼─────────────────┐
          ↓                ↓                 ↓
   backtest_trade    backtest_position_snapshot    backtest_portfolio_snapshot
```

## 1. 概述

回測資料庫專門存儲所有回測相關的數據，包括策略定義、回測配置、執行結果和交易記錄等。該資料庫支援完整的讀寫操作，使用 TimescaleDB 來處理大量的時間序列回測數據。

本資料庫與市場數據資料庫完全分離，確保回測操作不會影響市場數據的完整性。

## 2. 資料庫配置

### 2.1 基本配置

```sql
-- 創建回測資料庫
CREATE DATABASE backtest;

-- 連接到資料庫
\c backtest

-- 啟用TimescaleDB擴展
CREATE EXTENSION IF NOT EXISTS timescaledb;
```

## 3. 核心數據表結構

### 3.1 策略定義與管理

```sql
-- 策略定義表（整合所有策略相關信息）
CREATE TABLE strategy (
    strategy_id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    version VARCHAR(50) NOT NULL,
    code TEXT NOT NULL,
    code_path VARCHAR(255),
    parameters JSONB NOT NULL DEFAULT '{}',
    active BOOLEAN DEFAULT true,
    author VARCHAR(100),
    tags VARCHAR[] NOT NULL DEFAULT '{}',
    dependencies VARCHAR[] NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name, version)
);

-- 創建索引
CREATE INDEX idx_strategy_name_version ON strategy(name, version);
CREATE INDEX idx_strategy_active ON strategy(active);
CREATE INDEX idx_strategy_tags ON strategy USING GIN (tags);

-- 策略版本表（簡化版本管理）
CREATE TABLE strategy_version (
    version_id SERIAL PRIMARY KEY,
    strategy_id INTEGER NOT NULL REFERENCES strategy(strategy_id),
    version VARCHAR(50) NOT NULL,
    source_path VARCHAR(255) NOT NULL,
    description TEXT,
    is_stable BOOLEAN NOT NULL DEFAULT false,
    created_by VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(strategy_id, version)
);

-- 創建索引
CREATE INDEX idx_strategy_versions_strategy_id ON strategy_version(strategy_id);
CREATE INDEX idx_strategy_versions_is_stable ON strategy_version(is_stable);
```

### 3.2 回測系統數據表

```sql
-- 回測配置表
CREATE TABLE backtest_config (
    config_id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    initial_capital NUMERIC(24,6) NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    instruments INTEGER[] NOT NULL, -- 關聯的金融商品ID數組
    strategy_id INTEGER NOT NULL REFERENCES strategy(strategy_id),
    execution_settings JSONB NOT NULL DEFAULT '{}', -- 執行設置：滑點、費用等
    risk_settings JSONB NOT NULL DEFAULT '{}', -- 風險管理設置
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_backtest_config_strategy ON backtest_config(strategy_id);

-- 回測結果表（整合績效指標）
CREATE TABLE backtest_result (
    result_id SERIAL PRIMARY KEY,
    config_id INTEGER NOT NULL REFERENCES backtest_config(config_id),
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING', -- 'PENDING', 'RUNNING', 'COMPLETED', 'FAILED'
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    execution_time INTEGER, -- 執行時間（秒）
    
    -- 整合所有績效指標
    metrics JSONB NOT NULL DEFAULT '{}', -- 包含所有指標：收益率、夏普比率、最大回撤等
    
    -- 基準比較（如有）
    benchmark_comparison JSONB, -- 相對於基準的指標
    
    -- 錯誤信息
    error_message TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_backtest_result_config ON backtest_result(config_id);
CREATE INDEX idx_backtest_result_status ON backtest_result(status);
CREATE INDEX idx_backtest_result_metrics ON backtest_result USING GIN (metrics);

-- 回測交易記錄表
CREATE TABLE backtest_trade (
    time TIMESTAMPTZ NOT NULL,
    result_id INTEGER NOT NULL REFERENCES backtest_result(result_id),
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
SELECT create_hypertable('backtest_trade', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_backtest_trade_result ON backtest_trade(result_id);
CREATE INDEX idx_backtest_trade_instrument ON backtest_trade(instrument_id);
CREATE INDEX idx_backtest_trade_result_time ON backtest_trade(result_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE backtest_trade SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'result_id, instrument_id'
);

-- 添加壓縮策略（60天前的數據自動壓縮）
SELECT add_compression_policy('backtest_trade', INTERVAL '60 days');

-- 回測倉位快照表
CREATE TABLE backtest_position_snapshot (
    time TIMESTAMPTZ NOT NULL,
    result_id INTEGER NOT NULL REFERENCES backtest_result(result_id),
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
SELECT create_hypertable('backtest_position_snapshot', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_backtest_position_result ON backtest_position_snapshot(result_id);
CREATE INDEX idx_backtest_position_instrument ON backtest_position_snapshot(instrument_id);
CREATE INDEX idx_backtest_position_result_time ON backtest_position_snapshot(result_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE backtest_position_snapshot SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'result_id, instrument_id'
);

-- 添加壓縮策略（60天前的數據自動壓縮）
SELECT add_compression_policy('backtest_position_snapshot', INTERVAL '60 days');

-- 回測投資組合快照表
CREATE TABLE backtest_portfolio_snapshot (
    time TIMESTAMPTZ NOT NULL,
    result_id INTEGER NOT NULL REFERENCES backtest_result(result_id),
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
SELECT create_hypertable('backtest_portfolio_snapshot', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_backtest_portfolio_result ON backtest_portfolio_snapshot(result_id);
CREATE INDEX idx_backtest_portfolio_result_time ON backtest_portfolio_snapshot(result_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE backtest_portfolio_snapshot SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'result_id'
);

-- 添加壓縮策略（60天前的數據自動壓縮）
SELECT add_compression_policy('backtest_portfolio_snapshot', INTERVAL '60 days');
```

## 4. 連續聚合(Continuous Aggregates)

### 4.1 回測結果聚合

```sql
-- 回測日收益率聚合視圖
CREATE MATERIALIZED VIEW backtest_daily_returns
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    result_id,
    last(total_return, time) AS daily_return,
    last(total_value, time) AS end_of_day_value,
    last(equity, time) AS end_of_day_equity
FROM backtest_portfolio_snapshot
GROUP BY bucket, result_id;

-- 添加自動刷新策略（每日刷新）
SELECT add_continuous_aggregate_policy('backtest_daily_returns',
    start_offset => INTERVAL '1 month',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');
```

## 5. 資料庫權限與角色

為了管理回測資料庫的訪問權限，設置以下角色：

```sql
-- 創建回測應用程序角色
CREATE ROLE backtest_app WITH LOGIN PASSWORD 'backtest_app_password';

-- 創建回測只讀角色
CREATE ROLE backtest_readonly WITH LOGIN PASSWORD 'backtest_readonly_password';

-- 創建回測管理員角色
CREATE ROLE backtest_admin WITH LOGIN PASSWORD 'backtest_admin_password';

-- 設置權限
-- 只讀角色：只能查詢數據
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backtest_readonly;
GRANT USAGE ON SCHEMA public TO backtest_readonly;

-- 應用程序角色：完整的CRUD權限
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO backtest_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO backtest_app;
GRANT USAGE ON SCHEMA public TO backtest_app;

-- 管理員角色：完全權限
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO backtest_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO backtest_admin;
GRANT USAGE ON SCHEMA public TO backtest_admin;
```

## 6. 查詢優化示例

### 6.1 回測結果查詢

```sql
-- 查詢特定回測結果的摘要信息
SELECT 
    bc.name AS backtest_name,
    bc.start_date,
    bc.end_date,
    s.name AS strategy_name,
    br.status,
    br.metrics->>'total_trades' AS total_trades,
    br.metrics->>'win_rate' AS win_rate,
    br.metrics->>'profit_loss' AS profit_loss,
    br.metrics->>'max_drawdown' AS max_drawdown,
    br.metrics->>'sharpe_ratio' AS sharpe_ratio,
    br.metrics->>'sortino_ratio' AS sortino_ratio,
    br.metrics->>'calmar_ratio' AS calmar_ratio
FROM backtest_result br
JOIN backtest_config bc ON br.config_id = bc.config_id
JOIN strategy s ON bc.strategy_id = s.strategy_id
WHERE br.result_id = 123;

-- 查詢回測交易記錄
SELECT 
    bt.time,
    bt.instrument_id, -- 需要跨資料庫查詢 instrument 表以獲取 symbol
    bt.direction,
    bt.price,
    bt.quantity,
    bt.amount,
    bt.commission,
    bt.slippage,
    bt.position_effect,
    bt.order_type
FROM backtest_trade bt
WHERE bt.result_id = 123
ORDER BY bt.time;

-- 查詢回測投資組合表現曲線數據
SELECT 
    time,
    total_value,
    cash,
    equity,
    daily_pnl,
    total_pnl,
    daily_return,
    total_return
FROM backtest_portfolio_snapshot
WHERE result_id = 123
ORDER BY time;

-- 比較多個回測結果的性能
SELECT 
    bc.name AS backtest_name,
    s.name AS strategy_name,
    br.metrics->>'total_trades' AS total_trades,
    br.metrics->>'win_rate' AS win_rate,
    br.metrics->>'profit_loss' AS profit_loss,
    br.metrics->>'max_drawdown' AS max_drawdown,
    br.metrics->>'sharpe_ratio' AS sharpe_ratio,
    RANK() OVER (ORDER BY (br.metrics->>'sharpe_ratio')::numeric DESC) AS sharpe_rank,
    RANK() OVER (ORDER BY (br.metrics->>'profit_loss')::numeric DESC) AS profit_rank
FROM backtest_result br
JOIN backtest_config bc ON br.config_id = bc.config_id
JOIN strategy s ON bc.strategy_id = s.strategy_id
WHERE br.status = 'COMPLETED'
ORDER BY (br.metrics->>'sharpe_ratio')::numeric DESC;
```

### 6.2 策略管理查詢

```sql
-- 查詢活躍策略列表
SELECT 
    s.strategy_id,
    s.name,
    s.version,
    s.description,
    s.author,
    s.tags,
    s.created_at,
    s.updated_at
FROM strategy s
WHERE s.active = true
ORDER BY s.created_at DESC;

-- 查詢策略的所有版本
SELECT 
    s.name AS strategy_name,
    sv.version,
    sv.source_path,
    sv.description,
    sv.is_stable,
    sv.created_by,
    sv.created_at
FROM strategy s
JOIN strategy_version sv ON s.strategy_id = sv.strategy_id
WHERE s.name = 'MyTradingStrategy'
ORDER BY sv.created_at DESC;

-- 查詢策略的回測歷史
SELECT 
    s.name AS strategy_name,
    s.version,
    bc.name AS backtest_name,
    bc.start_date,
    bc.end_date,
    br.status,
    br.metrics->>'profit_loss' AS profit_loss,
    br.metrics->>'sharpe_ratio' AS sharpe_ratio,
    br.created_at
FROM strategy s
JOIN backtest_config bc ON s.strategy_id = bc.strategy_id
JOIN backtest_result br ON bc.config_id = br.config_id
WHERE s.name = 'MyTradingStrategy'
ORDER BY br.created_at DESC;
```

### 6.3 效能分析查詢

```sql
-- 計算策略的月度回報統計
SELECT 
    s.name AS strategy_name,
    DATE_TRUNC('month', bdr.bucket) AS month,
    COUNT(*) AS trading_days,
    AVG(bdr.daily_return) AS avg_daily_return,
    STDDEV(bdr.daily_return) AS volatility,
    MIN(bdr.daily_return) AS min_daily_return,
    MAX(bdr.daily_return) AS max_daily_return
FROM backtest_daily_returns bdr
JOIN backtest_result br ON bdr.result_id = br.result_id
JOIN backtest_config bc ON br.config_id = bc.config_id
JOIN strategy s ON bc.strategy_id = s.strategy_id
WHERE br.status = 'COMPLETED'
  AND bdr.bucket >= '2023-01-01'
GROUP BY s.name, DATE_TRUNC('month', bdr.bucket)
ORDER BY s.name, month;

-- 分析不同策略的風險調整後回報
SELECT 
    s.name AS strategy_name,
    COUNT(br.result_id) AS total_backtests,
    AVG((br.metrics->>'profit_loss')::numeric) AS avg_profit_loss,
    AVG((br.metrics->>'sharpe_ratio')::numeric) AS avg_sharpe_ratio,
    AVG((br.metrics->>'max_drawdown')::numeric) AS avg_max_drawdown,
    AVG((br.metrics->>'win_rate')::numeric) AS avg_win_rate
FROM strategy s
JOIN backtest_config bc ON s.strategy_id = bc.strategy_id
JOIN backtest_result br ON bc.config_id = br.config_id
WHERE br.status = 'COMPLETED'
  AND s.active = true
GROUP BY s.name
HAVING COUNT(br.result_id) >= 3 -- 至少有3次回測
ORDER BY avg_sharpe_ratio DESC;

-- 查詢特定時間段的交易頻率分析
SELECT 
    s.name AS strategy_name,
    COUNT(bt.time) AS total_trades,
    COUNT(bt.time) / EXTRACT(DAYS FROM (bc.end_date - bc.start_date)) AS trades_per_day,
    AVG(bt.amount) AS avg_trade_amount,
    SUM(bt.commission) AS total_commission
FROM strategy s
JOIN backtest_config bc ON s.strategy_id = bc.strategy_id
JOIN backtest_result br ON bc.config_id = br.config_id
JOIN backtest_trade bt ON br.result_id = bt.result_id
WHERE br.status = 'COMPLETED'
  AND bc.start_date >= '2023-01-01'
  AND bc.end_date <= '2023-12-31'
GROUP BY s.name, bc.start_date, bc.end_date
ORDER BY trades_per_day DESC;
```