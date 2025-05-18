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