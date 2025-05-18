# FinRust TimescaleDB 數據庫結構設計

## 目錄

- [1. 概述](#1-概述)
- [2. 數據庫配置](#2-數據庫配置)
  - [2.1 基本配置](#21-基本配置)
  - [2.2 TimescaleDB專用參數](#22-timescaledb專用參數)
- [3. 核心數據表結構](#3-核心數據表結構)
  - [3.1 金融商品基本信息](#31-金融商品基本信息)
    - [3.1.1 金融商品便利視圖](#311-金融商品便利視圖)
  - [3.2 分鐘級別行情數據](#32-分鐘級別行情數據)
  - [3.3 Tick級別行情數據](#33-tick級別行情數據)
  - [3.4 策略運行和交易記錄](#34-策略運行和交易記錄)
  - [3.5 市場事件和財務報告](#35-市場事件和財務報告)
  - [3.6 資金管理和投資組合](#36-資金管理和投資組合)
  - [3.7 回測系統數據表](#37-回測系統數據表)
- [4. 連續聚合(Continuous Aggregates)](#4-連續聚合continuous-aggregates)
  - [4.1 日內指標聚合](#41-日內指標聚合)
  - [4.2 交易量指標聚合](#42-交易量指標聚合)
  - [4.3 策略性能聚合](#43-策略性能聚合)
  - [4.4 投資組合表現聚合](#44-投資組合表現聚合)
  - [4.5 回測結果聚合](#45-回測結果聚合)
- [5. 預計算指標和序列化存儲](#5-預計算指標和序列化存儲)
  - [5.1 技術指標表](#51-技術指標表)
  - [5.2 基本面指標表](#52-基本面指標表)
- [6. 保留策略和數據生命週期管理](#6-保留策略和數據生命週期管理)
- [7. 數據庫權限與角色](#7-數據庫權限與角色)
- [8. 數據庫維護任務](#8-數據庫維護任務)
- [9. 查詢優化示例](#9-查詢優化示例)
  - [9.1 獲取金融商品最近N天的分鐘K線數據](#91-獲取金融商品最近n天的分鐘k線數據)
  - [9.2 跨表查詢金融商品數據](#92-跨表查詢金融商品數據)
  - [9.3 使用預計算指標查詢](#93-使用預計算指標查詢)
  - [9.4 回測結果查詢](#94-回測結果查詢)
- [10. 部署與備份策略](#10-部署與備份策略)
  - [10.1 備份配置](#101-備份配置)
  - [10.2 高可用性配置](#102-高可用性配置)
- [附錄：性能優化建議](#附錄性能優化建議)

## 表結構概覽

| 表名 | 類型 | 主要字段 | 用途 |
|------|------|---------|------|
| `exchange` | 普通表 | exchange_id, code, name, country | 存儲交易所基本信息 |
| `instrument` | 普通表 | instrument_id, symbol, exchange_id, instrument_type, name | 存儲所有金融商品基本信息 |
| `stock` | 普通表 | instrument_id, sector, industry, market_cap | 存儲股票特定屬性 |
| `future` | 普通表 | instrument_id, underlying_asset, contract_size, delivery_date | 存儲期貨特定屬性 |
| `option_contract` | 普通表 | instrument_id, option_type, strike_price, expiration_date | 存儲選擇權特定屬性 |
| `forex` | 普通表 | instrument_id, base_currency, quote_currency | 存儲外匯特定屬性 |
| `crypto` | 普通表 | instrument_id, blockchain_network, total_supply | 存儲虛擬貨幣特定屬性 |
| `minute_bar` | 超表 | time, instrument_id, open, high, low, close | 存儲分鐘K線數據 |
| `tick` | 超表 | time, instrument_id, price, volume, bid_prices, ask_prices | 存儲Tick級別數據 |
| `market_event` | 超表 | time, event_type, description, impact_level | 存儲市場重大事件 |
| `financial_report` | 普通表 | report_id, instrument_id, report_date, report_type | 存儲公司財務報告數據 |
| `strategy` | 普通表 | strategy_id, name, code, parameters | 存儲策略定義 |
| `strategy_version` | 普通表 | version_id, strategy_id, version, source_path, is_stable | 存儲策略版本信息 |
| `strategy_instance` | 普通表 | instance_id, strategy_id, parameters | 存儲策略實例 |
| `strategy_signal` | 超表 | time, instance_id, instrument_id, signal_type | 存儲策略信號 |
| `portfolio` | 普通表 | portfolio_id, name, initial_capital, currency | 存儲投資組合數據 |
| `portfolio_holding` | 超表 | time, portfolio_id, instrument_id, quantity | 存儲投資組合持倉 |
| `trade` | 普通表 | trade_id, time, instance_id, instrument_id | 存儲交易記錄 |
| `technical_indicator` | 普通表 | indicator_id, code, name, parameters | 存儲技術指標定義 |
| `instrument_daily_indicator` | 超表 | time, instrument_id, indicator_id, values | 存儲預計算技術指標 |
| `fundamental_indicator` | 超表 | time, instrument_id, indicator_type, values | 存儲基本面指標 |
| `hourly_volume_by_instrument` | 連續聚合 | bucket, instrument_id, total_volume | 小時級成交量聚合 |
| `daily_volume_by_instrument` | 連續聚合 | bucket, instrument_id, open, close, total_volume | 日級成交量聚合 |
| `strategy_performance` | 連續聚合 | bucket, instance_id, total_buy, total_sell | 策略性能聚合 |
| `portfolio_performance` | 連續聚合 | bucket, portfolio_id, total_value, profit_loss | 投資組合表現聚合 |
| `backtest_config` | 普通表 | config_id, name, start_date, end_date, strategy_id | 存儲回測配置信息 |
| `backtest_param_set` | 普通表 | set_id, config_id, set_name, parameters | 存儲回測參數集 |
| `backtest_result` | 普通表 | result_id, config_id, status, profit_loss, sharpe_ratio | 存儲回測結果摘要 |
| `backtest_trade` | 超表 | time, result_id, instrument_id, direction, price | 存儲回測交易記錄 |
| `backtest_position_snapshot` | 超表 | time, result_id, instrument_id, quantity, market_value | 存儲回測倉位快照 |
| `backtest_portfolio_snapshot` | 超表 | time, result_id, total_value, cash, equity | 存儲回測投資組合快照 |
| `backtest_performance_metrics` | 普通表 | metric_id, result_id, sharpe_ratio, sortino_ratio | 存儲詳細績效指標 |
| `benchmark_comparison` | 普通表 | comparison_id, result_id, alpha, beta, correlation | 存儲基準比較數據 |
| `backtest_price_override` | 普通表 | override_id, config_id, instrument_id, price_adjustment | 存儲回測價格覆蓋設置 |
| `backtest_event_override` | 普通表 | event_id, config_id, event_time, event_type | 存儲回測事件覆蓋設置 |
| `backtest_daily_returns` | 連續聚合 | bucket, result_id, daily_return, end_of_day_value | 回測日收益率聚合 |
| `backtest_trade_stats` | 連續聚合 | bucket, result_id, instrument_id, trade_count | 回測交易統計聚合 |
| `risk_limit` | 普通表 | limit_id, name, limit_type, max_value | 存儲風險限制設置 |
| `risk_check_record` | 超表 | time, limit_id, current_value, status | 存儲風險檢查記錄 |
| `risk_alert` | 普通表 | alert_id, time, check_id, alert_type, message | 存儲風險警報記錄 |

## 表關係圖

```
exchange
    ↑
    │
instrument ─────────────────────┐
    ↑                           │
    ├── stock                   │
    ├── future                  │
    ├── option_contract         │
    ├── forex                   │
    ├── crypto                  │
    │                           │
    ├── minute_bar              │
    │                           │
    ├── tick                    │
    │                           │
    ├── financial_report        │
    │                           │
    ├── instrument_daily_indicator ←┼── technical_indicator
    │                           │
    └── fundamental_indicator   │
                         
strategy
    ↑                           
    │                           
strategy_instance                
    ↑                           ┌── backtest_config ──┬── backtest_param_set
    │                           │         ↑          │
    ├── strategy_signal         │         │          │
    │      ↓                    │     backtest_result ┼── backtest_price_override
    └── trade ───────────┐      │         ↓          │
                         │      │         │          └── backtest_event_override
                         ↓      │         │
portfolio ───────→ portfolio_holding     ┌┴──────────┬─────────────┬───────────────────┐
    │                                    ↓           ↓             ↓                   ↓
    ↓                               backtest_trade  backtest_position_snapshot  backtest_portfolio_snapshot  backtest_performance_metrics
portfolio_performance                                                                  ↑                        
                                                                                       │
                                                                                       │
                                                                          benchmark_comparison
                                                                                
market_event

risk_limit
    ↓
risk_check_record
    ↓
risk_alert

backtest_daily_returns ← backtest_portfolio_snapshot

backtest_trade_stats ← backtest_trade
```

## 1. 概述

FinRust專案使用TimescaleDB作為主要的時間序列數據存儲系統。TimescaleDB是基於PostgreSQL的時間序列數據庫擴展，提供了高效的時間序列數據存儲和查詢功能，非常適合金融市場數據的管理。系統支援多種金融商品，包括股票、期貨、選擇權、外匯和虛擬貨幣。

本文檔詳細描述了FinRust專案中TimescaleDB的數據庫結構、表設計、索引策略和查詢優化方案。

## 2. 數據庫配置

### 2.1 基本配置

```sql
-- 創建數據庫
CREATE DATABASE finrust_db;

-- 連接到數據庫
\c finrust_db

-- 啟用TimescaleDB擴展
CREATE EXTENSION IF NOT EXISTS timescaledb;
```

### 2.2 TimescaleDB專用參數

以下是推薦的TimescaleDB配置參數，應添加到`postgresql.conf`文件中：

```
# 記憶體設置
shared_buffers = '4GB'               -- 根據可用記憶體調整，建議總記憶體的25%
work_mem = '256MB'                   -- 複雜查詢的工作記憶體
maintenance_work_mem = '1GB'         -- 維護操作的記憶體

# TimescaleDB特定設置
timescaledb.max_background_workers = 8
timescaledb.max_insert_batch_size = 10000
timescaledb.telemetry_level = 'off'  -- 禁用遙測

# 查詢優化
random_page_cost = 1.1               -- 假設使用SSD
effective_cache_size = '12GB'        -- 根據可用記憶體調整，建議總記憶體的75%

# 壓縮設置
timescaledb.enable_compression = true
```

## 3. 核心數據表結構

### 3.1 金融商品基本信息

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

-- 金融商品基礎表（所有商品的共同屬性）
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(symbol, exchange_id, instrument_type)
);

-- 創建索引
CREATE INDEX idx_instruments_symbol ON instrument(symbol);
CREATE INDEX idx_instruments_type ON instrument(instrument_type);
CREATE INDEX idx_instruments_exchange ON instrument(exchange_id);

-- 股票特定屬性表
CREATE TABLE stock (
    instrument_id INTEGER PRIMARY KEY REFERENCES instrument(instrument_id),
    sector VARCHAR(100),
    industry VARCHAR(100),
    market_cap NUMERIC(24,6),
    shares_outstanding BIGINT,
    free_float BIGINT,
    listing_date DATE,
    delisting_date DATE,
    dividend_yield NUMERIC(10,6),
    pe_ratio NUMERIC(12,6),
    CONSTRAINT fk_stock_instrument FOREIGN KEY (instrument_id) 
        REFERENCES instrument(instrument_id) ON DELETE CASCADE
);

-- 期貨特定屬性表
CREATE TABLE future (
    instrument_id INTEGER PRIMARY KEY REFERENCES instrument(instrument_id),
    underlying_asset VARCHAR(100) NOT NULL,
    contract_size NUMERIC(18,6) NOT NULL,
    contract_unit VARCHAR(50),
    delivery_date DATE NOT NULL,
    first_notice_date DATE,
    last_trading_date DATE NOT NULL,
    settlement_type VARCHAR(20) NOT NULL CHECK (settlement_type IN ('PHYSICAL', 'CASH')),
    initial_margin NUMERIC(18,6),
    maintenance_margin NUMERIC(18,6),
    price_quotation VARCHAR(100),
    CONSTRAINT fk_future_instrument FOREIGN KEY (instrument_id) 
        REFERENCES instrument(instrument_id) ON DELETE CASCADE
);

-- 選擇權特定屬性表
CREATE TABLE option_contract (
    instrument_id INTEGER PRIMARY KEY REFERENCES instrument(instrument_id),
    underlying_instrument_id INTEGER REFERENCES instrument(instrument_id),
    option_type VARCHAR(10) NOT NULL CHECK (option_type IN ('CALL', 'PUT')),
    strike_price NUMERIC(18,6) NOT NULL,
    expiration_date DATE NOT NULL,
    exercise_style VARCHAR(20) NOT NULL CHECK (exercise_style IN ('AMERICAN', 'EUROPEAN', 'ASIAN')),
    contract_size INTEGER NOT NULL,
    implied_volatility NUMERIC(10,6),
    delta NUMERIC(10,6),
    gamma NUMERIC(10,6),
    theta NUMERIC(10,6),
    vega NUMERIC(10,6),
    rho NUMERIC(10,6),
    CONSTRAINT fk_option_instrument FOREIGN KEY (instrument_id) 
        REFERENCES instrument(instrument_id) ON DELETE CASCADE
);

-- 外匯特定屬性表
CREATE TABLE forex (
    instrument_id INTEGER PRIMARY KEY REFERENCES instrument(instrument_id),
    base_currency VARCHAR(10) NOT NULL,
    quote_currency VARCHAR(10) NOT NULL,
    pip_value NUMERIC(18,8) NOT NULL,
    typical_spread NUMERIC(10,6),
    margin_requirement NUMERIC(10,6),
    trading_hours JSONB,
    CONSTRAINT fk_forex_instrument FOREIGN KEY (instrument_id) 
        REFERENCES instrument(instrument_id) ON DELETE CASCADE
);

-- 虛擬貨幣特定屬性表
CREATE TABLE crypto (
    instrument_id INTEGER PRIMARY KEY REFERENCES instrument(instrument_id),
    blockchain_network VARCHAR(100),
    total_supply NUMERIC(24,8),
    circulating_supply NUMERIC(24,8),
    max_supply NUMERIC(24,8),
    mining_algorithm VARCHAR(50),
    consensus_mechanism VARCHAR(50),
    website_url VARCHAR(200),
    whitepaper_url VARCHAR(200),
    github_url VARCHAR(200),
    CONSTRAINT fk_crypto_instrument FOREIGN KEY (instrument_id) 
        REFERENCES instrument(instrument_id) ON DELETE CASCADE
);
```

#### 3.1.1 金融商品便利視圖

為了便於應用程序查詢各類金融商品的完整信息，我們創建以下便利視圖，將核心表和擴展表的數據整合在一起：

```sql
-- 股票完整信息視圖
CREATE VIEW stock_complete AS
SELECT 
    i.*,  -- instrument表的所有欄位
    e.code AS exchange_code,
    e.name AS exchange_name,
    e.country AS exchange_country,
    s.sector,
    s.industry,
    s.market_cap,
    s.shares_outstanding,
    s.free_float,
    s.listing_date,
    s.delisting_date,
    s.dividend_yield,
    s.pe_ratio
FROM instrument i
JOIN exchange e ON i.exchange_id = e.exchange_id
JOIN stock s ON i.instrument_id = s.instrument_id
WHERE i.instrument_type = 'STOCK';

-- 期貨完整信息視圖
CREATE VIEW future_complete AS
SELECT 
    i.*,  -- instrument表的所有欄位
    e.code AS exchange_code,
    e.name AS exchange_name,
    e.country AS exchange_country,
    f.underlying_asset,
    f.contract_size,
    f.contract_unit,
    f.delivery_date,
    f.first_notice_date,
    f.last_trading_date,
    f.settlement_type,
    f.initial_margin,
    f.maintenance_margin,
    f.price_quotation
FROM instrument i
JOIN exchange e ON i.exchange_id = e.exchange_id
JOIN future f ON i.instrument_id = f.instrument_id
WHERE i.instrument_type = 'FUTURE';

-- 選擇權完整信息視圖
CREATE VIEW option_complete AS
SELECT 
    i.*,  -- instrument表的所有欄位
    e.code AS exchange_code,
    e.name AS exchange_name,
    e.country AS exchange_country,
    o.underlying_instrument_id,
    ui.symbol AS underlying_symbol,
    ui.name AS underlying_name,
    o.option_type,
    o.strike_price,
    o.expiration_date,
    o.exercise_style,
    o.contract_size,
    o.implied_volatility,
    o.delta,
    o.gamma,
    o.theta,
    o.vega,
    o.rho
FROM instrument i
JOIN exchange e ON i.exchange_id = e.exchange_id
JOIN option_contract o ON i.instrument_id = o.instrument_id
LEFT JOIN instrument ui ON o.underlying_instrument_id = ui.instrument_id
WHERE i.instrument_type = 'OPTIONCONTRACT';

-- 外匯完整信息視圖
CREATE VIEW forex_complete AS
SELECT 
    i.*,  -- instrument表的所有欄位
    e.code AS exchange_code,
    e.name AS exchange_name,
    e.country AS exchange_country,
    f.base_currency,
    f.quote_currency,
    f.pip_value,
    f.typical_spread,
    f.margin_requirement,
    f.trading_hours
FROM instrument i
JOIN exchange e ON i.exchange_id = e.exchange_id
JOIN forex f ON i.instrument_id = f.instrument_id
WHERE i.instrument_type = 'FOREX';

-- 虛擬貨幣完整信息視圖
CREATE VIEW crypto_complete AS
SELECT 
    i.*,  -- instrument表的所有欄位
    e.code AS exchange_code,
    e.name AS exchange_name,
    e.country AS exchange_country,
    c.blockchain_network,
    c.total_supply,
    c.circulating_supply,
    c.max_supply,
    c.mining_algorithm,
    c.consensus_mechanism,
    c.website_url,
    c.whitepaper_url,
    c.github_url
FROM instrument i
JOIN exchange e ON i.exchange_id = e.exchange_id
JOIN crypto c ON i.instrument_id = c.instrument_id
WHERE i.instrument_type = 'CRYPTO';
```

這些便利視圖的優勢和用途：

1. **數據訪問簡化**：允許應用程序通過單一查詢獲取金融商品的完整信息，無需手動聯接多個表
2. **統一查詢接口**：為不同類型的金融商品提供一致的查詢模式
3. **保持底層數據完整性**：不影響底層表結構和關聯關係
4. **提高開發效率**：減少應用程序中重複的表聯接代碼

視圖使用示例：

```sql
-- 查詢特定股票的完整信息
SELECT * FROM stock_complete WHERE symbol = 'AAPL';

-- 查詢即將到期的期貨合約
SELECT symbol, name, underlying_asset, delivery_date 
FROM future_complete 
WHERE delivery_date BETWEEN CURRENT_DATE AND (CURRENT_DATE + INTERVAL '30 days')
ORDER BY delivery_date;

-- 查詢某個交易所的所有選擇權合約
SELECT symbol, underlying_symbol, option_type, strike_price, expiration_date
FROM option_complete
WHERE exchange_code = 'CBOE'
ORDER BY underlying_symbol, expiration_date, strike_price;
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
    -- 外匯特有
    spread NUMERIC(10,6),
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

### 3.4 策略運行和交易記錄

```sql
-- 策略定義表
CREATE TABLE strategy (
    strategy_id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    code TEXT NOT NULL,
    parameters JSONB DEFAULT '{}',
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 策略配置表
CREATE TABLE strategy_configs (
    strategy_id VARCHAR(100) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    version VARCHAR(50) NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}',
    code_path VARCHAR(255),
    enabled BOOLEAN NOT NULL DEFAULT true,
    author VARCHAR(100),
    tags VARCHAR[] NOT NULL DEFAULT '{}',
    dependencies VARCHAR[] NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_strategy_configs_name ON strategy_configs(name);
CREATE INDEX idx_strategy_configs_enabled ON strategy_configs(enabled);
CREATE INDEX idx_strategy_configs_updated_at ON strategy_configs(updated_at DESC);

-- 策略實例表
CREATE TABLE strategy_instance (
    instance_id SERIAL PRIMARY KEY,
    strategy_id INTEGER NOT NULL REFERENCES strategy(strategy_id),
    name VARCHAR(100) NOT NULL,
    parameters JSONB DEFAULT '{}',
    active BOOLEAN DEFAULT true,
    last_run_time TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 策略信號表
CREATE TABLE strategy_signal (
    time TIMESTAMPTZ NOT NULL,
    instance_id INTEGER NOT NULL REFERENCES strategy_instance(instance_id),
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    signal_type VARCHAR(20) NOT NULL, -- 'BUY', 'SELL', etc.
    price NUMERIC(18,8),
    quantity NUMERIC(24,8),
    reason TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('strategy_signal', 'time');

-- 創建索引
CREATE INDEX idx_signals_instance_time ON strategy_signal(instance_id, time DESC);
CREATE INDEX idx_signals_instrument_time ON strategy_signal(instrument_id, time DESC);

-- 交易記錄表
CREATE TABLE trade (
    trade_id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL,
    instance_id INTEGER NOT NULL REFERENCES strategy_instance(instance_id),
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    direction VARCHAR(10) NOT NULL, -- 'BUY', 'SELL'
    price NUMERIC(18,8) NOT NULL,
    quantity NUMERIC(24,8) NOT NULL,
    amount NUMERIC(24,8) NOT NULL,
    commission NUMERIC(18,8) NOT NULL,
    slippage NUMERIC(18,8),
    -- 期貨/選擇權特有
    contract_month VARCHAR(10),
    -- 外匯特有
    exchange_rate NUMERIC(18,8),
    signal_id INTEGER,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_trades_time ON trade(time DESC);
CREATE INDEX idx_trades_instance_time ON trade(instance_id, time DESC);
CREATE INDEX idx_trades_instrument_time ON trade(instrument_id, time DESC);
```

```sql
-- 策略版本表
CREATE TABLE strategy_version (
    version_id SERIAL PRIMARY KEY,
    strategy_id INTEGER NOT NULL REFERENCES strategy(strategy_id),
    version VARCHAR(50) NOT NULL,
    source_path VARCHAR(255) NOT NULL,
    is_stable BOOLEAN NOT NULL DEFAULT false,
    description TEXT,
    created_by VARCHAR(100) NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_strategy_versions_strategy_id ON strategy_version(strategy_id);
CREATE INDEX idx_strategy_versions_is_stable ON strategy_version(is_stable);
CREATE INDEX idx_strategy_versions_created_at ON strategy_version(created_at DESC);
```

### 3.5 市場事件和財務報告

```sql
-- 市場事件表
CREATE TABLE market_event (
    time TIMESTAMPTZ NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    impact_level SMALLINT, -- 1-5，表示影響程度
    related_instruments JSONB, -- 相關的金融商品ID
    related_exchanges JSONB, -- 相關的交易所ID
    source VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('market_event', 'time',
                        chunk_time_interval => INTERVAL '1 month');

-- 創建索引
CREATE INDEX idx_market_events_type ON market_event(event_type);
CREATE INDEX idx_market_events_impact ON market_event(impact_level);

-- 設置數據壓縮策略
ALTER TABLE market_event SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'event_type'
);

-- 添加壓縮策略（180天前的數據自動壓縮）
SELECT add_compression_policy('market_event', INTERVAL '180 days');

-- 財務報告表（主要適用於股票）
CREATE TABLE financial_report (
    report_id SERIAL PRIMARY KEY,
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    report_date DATE NOT NULL,
    report_type VARCHAR(20) NOT NULL, -- 'Q1', 'Q2', 'Q3', 'Annual'
    fiscal_year INTEGER NOT NULL,
    publish_date DATE NOT NULL,
    revenue NUMERIC(24,6),
    net_income NUMERIC(24,6),
    eps NUMERIC(12,6),
    pe_ratio NUMERIC(12,6),
    pb_ratio NUMERIC(12,6),
    roe NUMERIC(10,6),
    debt_to_equity NUMERIC(10,6),
    current_ratio NUMERIC(10,6),
    metrics JSONB, -- 存儲其他財務指標
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(instrument_id, report_date, report_type)
);

-- 創建索引
CREATE INDEX idx_financial_reports_instrument ON financial_report(instrument_id);
CREATE INDEX idx_financial_reports_date ON financial_report(report_date DESC);
CREATE INDEX idx_financial_reports_type ON financial_report(report_type);
```

### 3.6 資金管理和投資組合

```sql
-- 投資組合表
CREATE TABLE portfolio (
    portfolio_id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    initial_capital NUMERIC(24,6) NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    risk_tolerance SMALLINT, -- 1-5，表示風險容忍度
    start_date DATE NOT NULL,
    end_date DATE,
    is_active BOOLEAN NOT NULL DEFAULT true,
    strategy_instance JSONB, -- 關聯的策略實例
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 投資組合持倉表
CREATE TABLE portfolio_holding (
    time TIMESTAMPTZ NOT NULL,
    portfolio_id INTEGER NOT NULL REFERENCES portfolio(portfolio_id),
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    quantity NUMERIC(24,8) NOT NULL,
    cost_basis NUMERIC(18,8) NOT NULL,
    market_value NUMERIC(18,8) NOT NULL,
    profit_loss NUMERIC(18,8),
    allocation_percentage NUMERIC(8,4),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('portfolio_holding', 'time',
                        chunk_time_interval => INTERVAL '1 week');

-- 創建索引
CREATE INDEX idx_portfolio_holdings_portfolio ON portfolio_holding(portfolio_id);
CREATE INDEX idx_portfolio_holdings_instrument ON portfolio_holding(instrument_id);
CREATE INDEX idx_portfolio_holdings_portfolio_time ON portfolio_holding(portfolio_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE portfolio_holding SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'portfolio_id, instrument_id'
);

-- 添加壓縮策略（30天前的數據自動壓縮）
SELECT add_compression_policy('portfolio_holding', INTERVAL '30 days');
```

### 3.7 回測系統數據表

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

-- 回測結果表
CREATE TABLE backtest_result (
    result_id SERIAL PRIMARY KEY,
    config_id INTEGER NOT NULL REFERENCES backtest_config(config_id),
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING', -- 'PENDING', 'RUNNING', 'COMPLETED', 'FAILED'
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    total_trades INTEGER,
    win_rate NUMERIC(10,6),
    profit_loss NUMERIC(24,6),
    max_drawdown NUMERIC(10,6),
    sharpe_ratio NUMERIC(10,6),
    summary_metrics JSONB, -- 其他摘要指標
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_backtest_result_config ON backtest_result(config_id);
CREATE INDEX idx_backtest_result_status ON backtest_result(status);

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
    metrics JSONB, -- 其他指標
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

-- 回測資產價格覆寫表（用於壓力測試和情景分析）
CREATE TABLE backtest_price_override (
    override_id SERIAL PRIMARY KEY,
    config_id INTEGER NOT NULL REFERENCES backtest_config(config_id),
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    price_adjustment NUMERIC(10,6) NOT NULL, -- 價格調整因子（百分比）
    volume_adjustment NUMERIC(10,6), -- 成交量調整因子（百分比）
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_backtest_price_override_config ON backtest_price_override(config_id);
CREATE INDEX idx_backtest_price_override_instrument ON backtest_price_override(instrument_id);

-- 回測市場事件覆寫表（用於壓力測試和情景分析）
CREATE TABLE backtest_event_override (
    event_id SERIAL PRIMARY KEY,
    config_id INTEGER NOT NULL REFERENCES backtest_config(config_id),
    event_time TIMESTAMPTZ NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    description TEXT,
    impact_settings JSONB NOT NULL, -- 事件影響設置
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_backtest_event_override_config ON backtest_event_override(config_id);
CREATE INDEX idx_backtest_event_override_time ON backtest_event_override(event_time);
```

## 4. 連續聚合(Continuous Aggregates)

TimescaleDB的連續聚合功能可以預先計算常用的聚合查詢，提高查詢效率。

### 4.1 日內指標聚合

```sql
-- 建立每小時成交量聚合視圖
CREATE MATERIALIZED VIEW hourly_volume_by_instrument
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', time) AS bucket,
    instrument_id,
    sum(volume) AS total_volume,
    sum(amount) AS total_amount,
    count(*) AS trade_count
FROM minute_bar
GROUP BY bucket, instrument_id;

-- 添加自動刷新策略（每小時刷新）
SELECT add_continuous_aggregate_policy('hourly_volume_by_instrument',
    start_offset => INTERVAL '2 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');
```

### 4.2 交易量指標聚合

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

### 4.3 策略性能聚合

```sql
-- 建立策略性能聚合視圖
CREATE MATERIALIZED VIEW strategy_performance
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    instance_id,
    sum(CASE WHEN direction = 'BUY' THEN amount ELSE 0 END) AS total_buy,
    sum(CASE WHEN direction = 'SELL' THEN amount ELSE 0 END) AS total_sell,
    sum(commission) AS total_commission,
    count(*) AS trade_count
FROM trade
GROUP BY bucket, instance_id;

-- 添加自動刷新策略（每日刷新）
SELECT add_continuous_aggregate_policy('strategy_performance',
    start_offset => INTERVAL '1 year',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');
```

### 4.4 投資組合表現聚合

```sql
-- 建立投資組合表現聚合視圖
CREATE MATERIALIZED VIEW portfolio_performance
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    portfolio_id,
    sum(market_value) AS total_value,
    sum(profit_loss) AS total_profit_loss,
    avg(allocation_percentage) AS avg_allocation,
    count(DISTINCT instrument_id) AS instrument_count
FROM portfolio_holding
GROUP BY bucket, portfolio_id;

-- 添加自動刷新策略（每日刷新）
SELECT add_continuous_aggregate_policy('portfolio_performance',
    start_offset => INTERVAL '1 year',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');
```

### 4.5 回測結果聚合

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

-- 回測交易統計聚合視圖
CREATE MATERIALIZED VIEW backtest_trade_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    result_id,
    instrument_id,
    count(*) AS trade_count,
    sum(CASE WHEN direction = 'BUY' THEN amount ELSE 0 END) AS total_buy_amount,
    sum(CASE WHEN direction = 'SELL' THEN amount ELSE 0 END) AS total_sell_amount,
    sum(commission) AS total_commission,
    sum(slippage) AS total_slippage
FROM backtest_trade
GROUP BY bucket, result_id, instrument_id;

-- 添加自動刷新策略（每日刷新）
SELECT add_continuous_aggregate_policy('backtest_trade_stats',
    start_offset => INTERVAL '1 month',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');
```

## 5. 預計算指標和序列化存儲

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

### 5.2 基本面指標表

```sql
-- 基本面指標表（主要適用於股票）
CREATE TABLE fundamental_indicator (
    time TIMESTAMPTZ NOT NULL,
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    indicator_type VARCHAR(50) NOT NULL, -- 'Valuation', 'Growth', 'Profitability', etc.
    values JSONB NOT NULL, -- 存儲多個相關指標
    source VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('fundamental_indicator', 'time',
                         chunk_time_interval => INTERVAL '1 month');

-- 創建索引
CREATE INDEX idx_fundamental_indicators_instrument_time ON fundamental_indicator(instrument_id, time DESC);
CREATE INDEX idx_fundamental_indicators_type ON fundamental_indicator(indicator_type);
```

## 6. 保留策略和數據生命週期管理

為了有效管理數據存儲空間，設置以下數據保留策略：

```sql
-- 設置Tick數據保留策略（保留60天）
SELECT add_retention_policy('tick', INTERVAL '60 days');

-- 分鐘數據將被永久保存，不設置保留策略
```

## 7. 數據庫權限與角色

為了管理數據庫訪問權限，設置以下角色：

```sql
-- 創建應用程序角色
CREATE ROLE finrust_app WITH LOGIN PASSWORD 'secure_password';

-- 創建只讀角色
CREATE ROLE finrust_readonly WITH LOGIN PASSWORD 'readonly_password';

-- 創建數據導入角色
CREATE ROLE finrust_importer WITH LOGIN PASSWORD 'importer_password';

-- 設置權限
GRANT SELECT ON ALL TABLES IN SCHEMA public TO finrust_readonly;

GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO finrust_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO finrust_app;

GRANT SELECT, INSERT ON minute_bar, tick TO finrust_importer;
GRANT SELECT ON instrument, exchange TO finrust_importer;
GRANT USAGE, SELECT ON SEQUENCE minute_bars_id_seq, ticks_id_seq TO finrust_importer;
```

## 8. 數據庫維護任務

為確保數據庫性能保持最佳狀態，設置以下維護任務：

```sql
-- 創建定期維護函數
CREATE OR REPLACE FUNCTION maintenance_job()
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    -- 重新索引
    REINDEX DATABASE finrust_db;
    
    -- 更新統計信息
    ANALYZE;
    
    -- 清理已刪除的數據
    VACUUM;
END $$;

-- 設置每週維護任務
SELECT cron.schedule('weekly-maintenance', '0 0 * * 0', 'SELECT maintenance_job()');
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
-- 查詢股票的詳細資訊和最新價格
SELECT 
    i.symbol, 
    i.name, 
    s.sector,
    s.industry,
    mb.time, 
    mb.close as latest_price,
    mb.volume
FROM instrument i
JOIN stock s ON i.instrument_id = s.instrument_id
JOIN LATERAL (
    SELECT * FROM minute_bar mb
    WHERE mb.instrument_id = i.instrument_id
    ORDER BY mb.time DESC
    LIMIT 1
) mb ON true
WHERE i.exchange_id = 1
  AND i.is_active = true
ORDER BY s.sector, i.symbol;

-- 查詢即將到期的期貨合約
SELECT 
    i.symbol,
    i.name,
    f.underlying_asset,
    f.delivery_date,
    f.last_trading_date,
    mb.close as current_price
FROM instrument i
JOIN future f ON i.instrument_id = f.instrument_id
JOIN LATERAL (
    SELECT * FROM minute_bar mb
    WHERE mb.instrument_id = i.instrument_id
    ORDER BY mb.time DESC
    LIMIT 1
) mb ON true
WHERE f.delivery_date BETWEEN now() AND now() + INTERVAL '30 days'
  AND i.is_active = true
ORDER BY f.delivery_date;
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

-- 查詢多個商品的技術指標
SELECT 
    i.symbol,
    i.instrument_type,
    ti.code as indicator_code,
    idi.time,
    idi.values
FROM instrument i
JOIN instrument_daily_indicator idi ON i.instrument_id = idi.instrument_id
JOIN technical_indicator ti ON idi.indicator_id = ti.indicator_id
WHERE i.instrument_type IN ('STOCK', 'FUTURE')
  AND ti.code IN ('RSI', 'MACD')
  AND idi.time >= now() - INTERVAL '1 month'
ORDER BY i.symbol, ti.code, idi.time DESC;
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
    br.total_trades,
    br.win_rate,
    br.profit_loss,
    br.max_drawdown,
    br.sharpe_ratio,
    br.summary_metrics->>'sortino_ratio' AS sortino_ratio,
    br.summary_metrics->>'calmar_ratio' AS calmar_ratio
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
    br.total_trades,
    br.win_rate,
    br.profit_loss,
    br.max_drawdown,
    br.sharpe_ratio,
    RANK() OVER (ORDER BY br.sharpe_ratio DESC) AS sharpe_rank,
    RANK() OVER (ORDER BY br.profit_loss DESC) AS profit_rank
FROM backtest_result br
JOIN backtest_config bc ON br.config_id = bc.config_id
JOIN strategy s ON bc.strategy_id = s.strategy_id
WHERE br.status = 'COMPLETED'
ORDER BY br.sharpe_ratio DESC;

-- 策略優化參數比較
WITH param_comparison AS (
    SELECT 
        bc.config_id,
        bc.name,
        bc.execution_settings->>'slippage' AS slippage,
        bc.risk_settings->>'max_position_size' AS max_position_size,
        br.profit_loss,
        br.sharpe_ratio,
        br.max_drawdown
    FROM backtest_config bc
    JOIN backtest_result br ON bc.config_id = br.config_id
    WHERE bc.strategy_id = 42
    AND br.status = 'COMPLETED'
)
SELECT * FROM param_comparison
ORDER BY sharpe_ratio DESC;
```

## 10. 部署與備份策略

### 10.1 備份配置

```sql
-- 配置備份（使用pgBackRest）
-- 相關配置應添加到pgBackRest配置文件中

-- 全量備份設置
-- 每週日凌晨2點執行全量備份
SELECT cron.schedule('weekly-full-backup', '0 2 * * 0', 
    'SELECT pg_backup(''full'')');

-- 增量備份設置
-- 每天凌晨3點執行增量備份
SELECT cron.schedule('daily-incremental-backup', '0 3 * * 1-6', 
    'SELECT pg_backup(''incremental'')');
```

### 10.2 高可用性配置

對於生產環境，建議配置TimescaleDB的高可用性設置，可使用以下方案：

1. 主從複製設置（Master-Replica）
2. 使用Patroni進行自動故障轉移
3. 使用PgBouncer進行連接池管理
4. 考慮使用TimescaleDB Cloud托管服務

## 附錄：性能優化建議

1. **適當調整分區大小（chunk_time_interval）**：
   - Tick數據：小時級分區
   - 分鐘數據：天級分區
   - 日線數據：月級分區

2. **針對查詢模式優化索引**：
   - 分析常用查詢模式
   - 為常用過濾條件添加複合索引
   - 考慮不同商品類型的查詢特性
   - 避免過多索引導致寫入性能下降

3. **利用TimescaleDB壓縮功能**：
   - 為歷史數據啟用壓縮
   - 根據數據特性調整壓縮設置
   - 考慮不同商品類型的數據特性
   - 監控壓縮效果並調整策略

4. **利用連續聚合提高查詢性能**：
   - 識別常用聚合查詢模式
   - 為高頻率訪問的聚合數據建立連續聚合視圖
   - 針對不同商品類型建立專門的聚合視圖
   - 調整刷新策略平衡實時性和性能

5. **定期維護**：
   - 監控和調整數據分區
   - 定期更新統計信息
   - 監控查詢性能並優化慢查詢
   - 針對不同商品類型進行專門的性能調優

6. **商品類型特定優化**：
   - 針對高頻交易的商品（如外匯、加密貨幣）優化Tick數據處理
   - 為期貨和選擇權數據預先計算到期日相關查詢
   - 優化股票基本面數據的存儲和查詢
   - 考慮不同市場的交易時間特性進行分區優化

7. **視圖和函數優化**：
   - 使用materialized view緩存常用的複雜查詢
   - 創建針對不同商品類型的專用查詢函數
   - 定期重建視圖以保持性能

8. **監控和告警**：
   - 設置性能監控指標
   - 配置慢查詢日誌
   - 監控存儲空間使用情況
   - 設置數據異常警告