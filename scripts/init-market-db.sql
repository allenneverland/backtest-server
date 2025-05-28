-- 市場數據資料庫初始化腳本
-- 基於 docs/MARKET_DATABASE_SCHEMA.md 的規範
-- 注意：確保連接到正確的數據庫 "marketdata"，而不是 "market_user"

-- 啟用TimescaleDB擴展
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- 交易所表
CREATE TABLE IF NOT EXISTS exchange (
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
CREATE TABLE IF NOT EXISTS instrument (
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
CREATE INDEX IF NOT EXISTS idx_instruments_symbol ON instrument(symbol);
CREATE INDEX IF NOT EXISTS idx_instruments_type ON instrument(instrument_type);
CREATE INDEX IF NOT EXISTS idx_instruments_exchange ON instrument(exchange_id);
CREATE INDEX IF NOT EXISTS idx_instruments_attributes ON instrument USING GIN (attributes);

-- 分鐘K線數據表
CREATE TABLE IF NOT EXISTS minute_bar (
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
                         chunk_time_interval => INTERVAL '1 day',
                         if_not_exists => TRUE);

-- 創建索引
CREATE INDEX IF NOT EXISTS idx_minute_bars_instrument_id ON minute_bar(instrument_id);
CREATE INDEX IF NOT EXISTS idx_minute_bars_instrument_time ON minute_bar(instrument_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE minute_bar SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'instrument_id'
);

-- 分鐘K線數據將永久保存，設置較長時間的壓縮策略
SELECT add_compression_policy('minute_bar', INTERVAL '90 days');

-- Tick級別行情數據表
CREATE TABLE IF NOT EXISTS tick (
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
                         chunk_time_interval => INTERVAL '1 hour',
                         if_not_exists => TRUE);

-- 創建索引
CREATE INDEX IF NOT EXISTS idx_ticks_instrument_id ON tick(instrument_id);
CREATE INDEX IF NOT EXISTS idx_ticks_instrument_time ON tick(instrument_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_ticks_trade_type ON tick(trade_type);

-- 設置數據壓縮策略
ALTER TABLE tick SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'instrument_id, trade_type'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('tick', INTERVAL '7 days');

-- 建立每日交易量聚合視圖
CREATE MATERIALIZED VIEW IF NOT EXISTS daily_volume_by_instrument
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

-- 技術指標定義表
CREATE TABLE IF NOT EXISTS technical_indicator (
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
('BOLL', 'Bollinger Bands', '布林帶', '{"period": 20, "std_dev": 2}')
ON CONFLICT (code) DO NOTHING;

-- 商品日級指標數據表
CREATE TABLE IF NOT EXISTS instrument_daily_indicator (
    time TIMESTAMPTZ NOT NULL,
    instrument_id INTEGER NOT NULL REFERENCES instrument(instrument_id),
    indicator_id INTEGER NOT NULL REFERENCES technical_indicator(indicator_id),
    parameters JSONB NOT NULL,
    values JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 轉換為超表
SELECT create_hypertable('instrument_daily_indicator', 'time',
                         chunk_time_interval => INTERVAL '1 month',
                         if_not_exists => TRUE);

-- 創建索引
CREATE INDEX IF NOT EXISTS idx_daily_indicators_instrument_time ON instrument_daily_indicator(instrument_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_daily_indicators_indicator ON instrument_daily_indicator(indicator_id);

-- 創建市場數據只讀角色（用於回測服務）
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'market_reader') THEN
        CREATE ROLE market_reader WITH LOGIN PASSWORD 'market_reader_password';
    END IF;
END
$$;

-- 創建市場數據管理員角色（用於數據導入）
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'market_admin') THEN
        CREATE ROLE market_admin WITH LOGIN PASSWORD 'market_admin_password';
    END IF;
END
$$;

-- 創建數據導入角色
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'market_importer') THEN
        CREATE ROLE market_importer WITH LOGIN PASSWORD 'market_importer_password';
    END IF;
END
$$;

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