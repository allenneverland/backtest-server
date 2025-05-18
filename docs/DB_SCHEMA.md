# TimescaleDB 回測系統數據庫結構設計

## 目錄

- [1. 概述](#1-概述)
- [2. 數據庫配置](#2-數據庫配置)
  - [2.1 基本配置](#21-基本配置)
- [3. 核心數據表結構](#3-核心數據表結構)
  - [3.1 金融商品資訊](#31-金融商品資訊)
  - [3.2 分鐘級別行情數據](#32-分鐘級別行情數據)
  - [3.3 Tick級別行情數據](#33-tick級別行情數據)
  - [3.4 策略定義與管理](#34-策略定義與管理)
  - [3.5 回測系統數據表](#36-回測系統數據表)
- [4. 連續聚合(Continuous Aggregates)](#4-連續聚合continuous-aggregates)
  - [4.1 日內指標聚合](#41-日內指標聚合)
  - [4.2 回測結果聚合](#42-回測結果聚合)
- [5. 預計算指標](#5-預計算指標)
  - [5.1 技術指標表](#51-技術指標表)
- [7. 數據庫權限與角色](#7-數據庫權限與角色)
- [9. 查詢優化示例](#9-查詢優化示例)
  - [9.1 獲取金融商品最近N天的分鐘K線數據](#91-獲取金融商品最近n天的分鐘k線數據)
  - [9.2 跨表查詢金融商品數據](#92-跨表查詢金融商品數據)
  - [9.3 使用預計算指標查詢](#93-使用預計算指標查詢)
  - [9.4 回測結果查詢](#94-回測結果查詢)

## 表結構概覽

| 表名 | 類型 | 主要字段 | 用途 |
|------|------|---------|------|
| `exchange` | 普通表 | exchange_id, code, name, country | 存儲交易所基本信息 |
| `instrument` | 普通表 | instrument_id, symbol, exchange_id, instrument_type, name, attributes | 存儲所有金融商品基本信息 |
| `minute_bar` | 超表 | time, instrument_id, open, high, low, close | 存儲分鐘K線數據 |
| `tick` | 超表 | time, instrument_id, price, volume, bid_prices, ask_prices | 存儲Tick級別數據 |
| `strategy` | 普通表 | strategy_id, name, version, code, parameters | 存儲策略定義 |
| `strategy_version` | 普通表 | version_id, strategy_id, version, source_path | 存儲策略版本信息 |
| `technical_indicator` | 普通表 | indicator_id, code, name, parameters | 存儲技術指標定義 |
| `instrument_daily_indicator` | 超表 | time, instrument_id, indicator_id, values | 存儲預計算技術指標 |
| `backtest_config` | 普通表 | config_id, name, start_date, end_date, strategy_id | 存儲回測配置信息 |
| `backtest_result` | 普通表 | result_id, config_id, status, metrics | 存儲回測結果摘要和績效指標 |
| `backtest_trade` | 超表 | time, result_id, instrument_id, direction, price | 存儲回測交易記錄 |
| `backtest_position_snapshot` | 超表 | time, result_id, instrument_id, quantity, market_value | 存儲回測倉位快照 |
| `backtest_portfolio_snapshot` | 超表 | time, result_id, total_value, cash, equity | 存儲回測投資組合快照 |
| `daily_volume_by_instrument` | 連續聚合 | bucket, instrument_id, open, close, total_volume | 日級成交量聚合 |
| `backtest_daily_returns` | 連續聚合 | bucket, result_id, daily_return, end_of_day_value | 回測日收益率聚合 |

## 表關係圖

```
exchange
    ↑
    │
instrument ─────────┬─────────────────┐
    ↑               │                 │
    │               │                 │
    ├── minute_bar  │                 │
    │               │                 │
    ├── tick        │                 │
    │               │                 │
    │                                │
    └── instrument_daily_indicator ←─┼── technical_indicator
                                     │
strategy ──────────────────┐         │
    ↑                      │         │
    │                      │         │
strategy_version           │         │
                           │         │
                    backtest_config  │
                           ↓         │
                    backtest_result  │
                           ↓         │
          ┌────────────────┼─────────┼───────────────┐
          ↓                ↓         ↓               ↓
   backtest_trade    backtest_position_snapshot    backtest_portfolio_snapshot
          
```

## 1. 概述

BacktestServer專案使用TimescaleDB作為主要的時間序列數據存儲系統，專注於回測功能。TimescaleDB是基於PostgreSQL的時間序列數據庫擴展，提供了高效的時間序列數據存儲和查詢功能，非常適合金融市場數據的管理。系統支援多種金融商品，包括股票、期貨、選擇權、外匯和虛擬貨幣。

本文檔詳細描述了BacktestServer回測系統中TimescaleDB的數據庫結構、表設計、索引策略和查詢優化方案。

## 2. 數據庫配置

### 2.1 基本配置

```sql
-- 創建數據庫
CREATE DATABASE backtest_server_db;

-- 連接到數據庫
\c backtest_server_db

-- 啟用TimescaleDB擴展
CREATE EXTENSION IF NOT EXISTS timescaledb;
```

## 3. 核心數據表結構

### 3.1 金融商品資訊

```sql
-- 交易所表
CREATE TABLE exchange (
    exchange_id SERIAL PRIMARY KEY,
    code VARCHAR(10) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    country VARCHAR(50) NOT NULL,
    timezone VARCHAR(50) NOT NULL,
    operating_hours JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 金融商品表（整合所有商品類型）
CREATE TABLE instrument (
    instrument_id SERIAL PRIMARY KEY,
    symbol VARCHAR(50) NOT NULL,
    exchange_id INTEGER REFERENCES exchange(exchange_id),
    instrument_type VARCHAR(20) NOT NULL CHECK (instrument_type IN ('STOCK', 'FUTURE', 'OPTIONCONTRACT', 'FOREX', 'CRYPTO')),
    name VARCHAR(200) NOT NULL,
    description TEXT,
    currency VARCHAR(10) NOT NULL,
    tick_size NUMERIC(18,6),
    lot_size INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT true,
    trading_start_date DATE,
    trading_end_date DATE,
    -- 使用JSONB存儲不同資產類型的特定屬性
    attributes JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(symbol, exchange_id, instrument_type)
);

-- 創建索引
CREATE INDEX idx_instruments_symbol ON instrument(symbol);
CREATE INDEX idx_instruments_type ON instrument(instrument_type);
CREATE INDEX idx_instruments_exchange ON instrument(exchange_id);
CREATE INDEX idx_instruments_attributes ON instrument USING GIN (attributes);
```

### 3.2 分鐘級別行情數據

```sql
-- 分鐘K線數據表
CREATE TABLE minute_bar (
    time TIMESTAMPTZ NOT NULL,
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    open NUMERIC(18,8) NOT NULL,
    high NUMERIC(18,8) NOT NULL,
    low NUMERIC(18,8) NOT NULL,
    close NUMERIC(18,8) NOT NULL,
    volume NUMERIC(24,8) NOT NULL,
    amount NUMERIC(24,8),
    open_interest NUMERIC(24,8), -- 期貨/選擇權的未平倉量
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('minute_bar', 'time', 
                         chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_minute_bars_instrument_id ON minute_bar(instrument_id);
CREATE INDEX idx_minute_bars_instrument_time ON minute_bar(instrument_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE minute_bar SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'instrument_id'
);

-- 分鐘K線數據將永久保存，設置較長時間的壓縮策略
SELECT add_compression_policy('minute_bar', INTERVAL '90 days');
```

### 3.3 Tick級別行情數據

```sql
-- Tick級別行情數據表
CREATE TABLE tick (
    time TIMESTAMPTZ NOT NULL,
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    price NUMERIC(18,8) NOT NULL,
    volume NUMERIC(24,8) NOT NULL,
    trade_type SMALLINT,
    -- 買賣盤口數據（適用於股票、期貨等）
    bid_price_1 NUMERIC(18,8),
    bid_volume_1 NUMERIC(24,8),
    ask_price_1 NUMERIC(18,8),
    ask_volume_1 NUMERIC(24,8),
    -- 擴展盤口數據（使用數組存儲多檔）
    bid_prices NUMERIC(18,8)[],
    bid_volumes NUMERIC(24,8)[],
    ask_prices NUMERIC(18,8)[],
    ask_volumes NUMERIC(24,8)[],
    -- 期貨/選擇權特有
    open_interest NUMERIC(24,8),
    -- 擴展信息
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('tick', 'time', 
                         chunk_time_interval => INTERVAL '1 hour');

-- 創建索引
CREATE INDEX idx_ticks_instrument_id ON tick(instrument_id);
CREATE INDEX idx_ticks_instrument_time ON tick(instrument_id, time DESC);
CREATE INDEX idx_ticks_trade_type ON tick(trade_type);

-- 設置數據壓縮策略
ALTER TABLE tick SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'instrument_id, trade_type'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('tick', INTERVAL '7 days');
```

### 3.4 策略定義與管理

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

### 3.5 回測系統數據表

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
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
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
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
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

TimescaleDB的連續聚合功能可以預先計算常用的聚合查詢，提高查詢效率。

### 4.1 日內指標聚合

```sql
-- 建立每日交易量聚合視圖
CREATE MATERIALIZED VIEW daily_volume_by_instrument
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    instrument_id,
    first(open, time) AS open,
    max(high) AS high,
    min(low) AS low,
    last(close, time) AS close,
    sum(volume) AS total_volume,
    sum(amount) AS total_amount,
    max(open_interest) AS max_open_interest
FROM minute_bar
GROUP BY bucket, instrument_id;

-- 添加自動刷新策略（每日刷新）
SELECT add_continuous_aggregate_policy('daily_volume_by_instrument',
    start_offset => INTERVAL '1 month',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');
```

### 4.2 回測結果聚合

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

## 5. 預計算指標

為了支持高效的策略回測，我們預先計算一些常用技術指標並存儲在數據庫中。

### 5.1 技術指標表

```sql
-- 技術指標定義表
CREATE TABLE technical_indicator (
    indicator_id SERIAL PRIMARY KEY,
    code VARCHAR(20) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    parameters JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 插入常用指標
INSERT INTO technical_indicator (code, name, description, parameters) VALUES
('SMA', 'Simple Moving Average', '簡單移動平均線', '{"periods": [5, 10, 20, 60, 120, 250]}'),
('EMA', 'Exponential Moving Average', '指數移動平均線', '{"periods": [5, 10, 20, 60, 120, 250]}'),
('RSI', 'Relative Strength Index', '相對強弱指標', '{"periods": [6, 14, 24]}'),
('MACD', 'Moving Average Convergence Divergence', '移動平均匯聚背馳', '{"fast": 12, "slow": 26, "signal": 9}'),
('BOLL', 'Bollinger Bands', '布林帶', '{"period": 20, "std_dev": 2}');

-- 商品日級指標數據表
CREATE TABLE instrument_daily_indicator (
    time TIMESTAMPTZ NOT NULL,
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    indicator_id INTEGER NOT NULL REFERENCES technical_indicator(indicator_id),
    parameters JSONB NOT NULL,
    values JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('instrument_daily_indicator', 'time',
                         chunk_time_interval => INTERVAL '1 month');

-- 創建索引
CREATE INDEX idx_daily_indicators_instrument_time ON instrument_daily_indicator(instrument_id, time DESC);
CREATE INDEX idx_daily_indicators_indicator ON instrument_daily_indicator(indicator_id);
```

## 7. 數據庫權限與角色

為了管理數據庫訪問權限，設置以下角色：

```sql
-- 創建應用程序角色
CREATE ROLE backtest_server_app WITH LOGIN PASSWORD 'secure_password';

-- 創建只讀角色
CREATE ROLE backtest_server_readonly WITH LOGIN PASSWORD 'readonly_password';

-- 創建數據導入角色
CREATE ROLE backtest_server_importer WITH LOGIN PASSWORD 'importer_password';

-- 設置權限
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backtest_server_readonly;

GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO backtest_server_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO backtest_server_app;

GRANT SELECT, INSERT ON minute_bar, tick TO backtest_server_importer;
GRANT SELECT ON instrument, exchange TO backtest_server_importer;
GRANT USAGE, SELECT ON SEQUENCE minute_bars_id_seq, ticks_id_seq TO backtest_server_importer;
```

## 9. 查詢優化示例

下面是一些常見查詢的優化示例：

### 9.1 獲取金融商品最近N天的分鐘K線數據

```sql
-- 使用時間索引優化查詢
SELECT time, open, high, low, close, volume, amount
FROM minute_bar
WHERE instrument_id = 123
  AND time >= now() - INTERVAL '7 days'
ORDER BY time DESC;

-- 獲取特定類型商品的數據
SELECT i.symbol, i.instrument_type, mb.time, mb.open, mb.high, mb.low, mb.close, mb.volume
FROM instrument i
JOIN minute_bar mb ON i.instrument_id = mb.instrument_id
WHERE i.instrument_type = 'FUTURE'
  AND mb.time >= now() - INTERVAL '1 day'
ORDER BY i.symbol, mb.time;
```

### 9.2 跨表查詢金融商品數據

```sql
-- 查詢特定商品的詳細資訊和最新價格
SELECT 
    i.symbol, 
    i.name, 
    i.instrument_type,
    i.attributes->>'sector' as sector,
    mb.time, 
    mb.close as latest_price,
    mb.volume
FROM instrument i
JOIN LATERAL (
    SELECT * FROM minute_bar mb
    WHERE mb.instrument_id = i.instrument_id
    ORDER BY mb.time DESC
    LIMIT 1
) mb ON true
WHERE i.exchange_id = 1
  AND i.is_active = true
ORDER BY i.instrument_type, i.symbol;
```

### 9.3 使用預計算指標查詢

```sql
-- 使用預計算指標表優化技術指標查詢
SELECT 
    i.symbol, 
    i.instrument_type,
    idi.time, 
    idi.values->>'sma20' AS sma20, 
    idi.values->>'sma60' AS sma60
FROM instrument i
JOIN instrument_daily_indicator idi ON i.instrument_id = idi.instrument_id
WHERE i.symbol = 'AAPL'
  AND idi.indicator_id = 1 -- SMA指標
  AND idi.time >= '2023-01-01'
ORDER BY idi.time DESC;
```

### 9.4 回測結果查詢

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
    i.symbol,
    i.name,
    bt.direction,
    bt.price,
    bt.quantity,
    bt.amount,
    bt.commission,
    bt.slippage,
    bt.position_effect,
    bt.order_type
FROM backtest_trade bt
JOIN instrument i ON bt.instrument_id = i.instrument_id
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
