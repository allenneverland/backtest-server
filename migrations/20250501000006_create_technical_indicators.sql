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