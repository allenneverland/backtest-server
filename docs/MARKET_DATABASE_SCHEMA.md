# 市場數據資料庫結構設計

## 目錄

- [1. 概述](#1-概述)
- [2. 資料庫配置](#2-資料庫配置)
  - [2.1 基本配置](#21-基本配置)
- [3. 核心數據表結構](#3-核心數據表結構)
  - [3.1 金融商品資訊](#31-金融商品資訊)
  - [3.2 分鐘級別行情數據](#32-分鐘級別行情數據)
  - [3.3 Tick級別行情數據](#33-tick級別行情數據)
- [4. 連續聚合(Continuous Aggregates)](#4-連續聚合continuous-aggregates)
  - [4.1 日內指標聚合](#41-日內指標聚合)
- [5. 預計算指標](#5-預計算指標)
  - [5.1 技術指標表](#51-技術指標表)
- [6. 資料庫權限與角色](#6-資料庫權限與角色)
- [7. 查詢優化示例](#7-查詢優化示例)
  - [7.1 獲取金融商品最近N天的分鐘K線數據](#71-獲取金融商品最近n天的分鐘k線數據)
  - [7.2 跨表查詢金融商品數據](#72-跨表查詢金融商品數據)
  - [7.3 使用預計算指標查詢](#73-使用預計算指標查詢)

## 表結構概覽

| 表名 | 類型 | 主要字段 | 用途 |
|------|------|---------|------|
| `exchange` | 普通表 | exchange_id, code, name, country | 存儲交易所基本信息 |
| `instrument` | 普通表 | instrument_id, symbol, exchange_id, instrument_type, name, attributes | 存儲所有金融商品基本信息 |
| `minute_bar` | 超表 | time, instrument_id, open, high, low, close | 存儲分鐘K線數據 |
| `tick` | 超表 | time, instrument_id, price, volume, bid_prices, ask_prices | 存儲Tick級別數據 |
| `technical_indicator` | 普通表 | indicator_id, code, name, parameters | 存儲技術指標定義 |
| `instrument_daily_indicator` | 超表 | time, instrument_id, indicator_id, values | 存儲預計算技術指標 |
| `daily_volume_by_instrument` | 連續聚合 | bucket, instrument_id, open, close, total_volume | 日級成交量聚合 |

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
    └── instrument_daily_indicator ←─┼── technical_indicator
                                     │
```

## 1. 概述

市場數據資料庫專門存儲所有市場相關的數據，包括金融商品信息、價格數據、技術指標等。該資料庫使用 TimescaleDB 作為時間序列數據存儲系統，提供高效的時間序列數據存儲和查詢功能。

本資料庫為唯讀模式，不允許應用程式進行寫入操作，確保數據完整性和一致性。

## 2. 資料庫配置

### 2.1 基本配置

```sql
-- 創建市場數據資料庫
CREATE DATABASE marketdata;

-- 連接到資料庫
\c marketdata

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

## 6. 資料庫權限與角色

為了管理市場數據資料庫的訪問權限，設置以下角色：

```sql
-- 創建市場數據只讀角色（用於回測服務）
CREATE ROLE market_reader WITH LOGIN PASSWORD 'market_reader_password';

-- 創建市場數據管理員角色（用於數據導入）
CREATE ROLE market_admin WITH LOGIN PASSWORD 'market_admin_password';

-- 創建數據導入角色
CREATE ROLE market_importer WITH LOGIN PASSWORD 'market_importer_password';

-- 設置權限
-- 只讀角色：只能查詢數據
GRANT SELECT ON ALL TABLES IN SCHEMA public TO market_reader;
GRANT USAGE ON SCHEMA public TO market_reader;

-- 管理員角色：完全權限
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO market_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO market_admin;
GRANT USAGE ON SCHEMA public TO market_admin;

-- 導入角色：可插入市場數據
GRANT SELECT, INSERT ON minute_bar, tick, instrument_daily_indicator TO market_importer;
GRANT SELECT ON instrument, exchange, technical_indicator TO market_importer;
GRANT USAGE ON SCHEMA public TO market_importer;
```

## 7. 查詢優化示例

下面是一些常見查詢的優化示例：

### 7.1 獲取金融商品最近N天的分鐘K線數據

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

### 7.2 跨表查詢金融商品數據

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

### 7.3 使用預計算指標查詢

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

-- 獲取多種技術指標
SELECT 
    i.symbol,
    idi.time,
    ti.code AS indicator_code,
    idi.values
FROM instrument i
JOIN instrument_daily_indicator idi ON i.instrument_id = idi.instrument_id
JOIN technical_indicator ti ON idi.indicator_id = ti.indicator_id
WHERE i.symbol = 'AAPL'
  AND idi.time >= '2023-01-01'
ORDER BY idi.time DESC, ti.code;
```