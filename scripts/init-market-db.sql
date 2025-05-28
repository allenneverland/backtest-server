-- 市場數據資料庫初始化腳本
-- 模擬其他系統維護的市場數據結構

-- 創建 TimescaleDB 擴展
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- 基本市場數據表
CREATE TABLE IF NOT EXISTS market_data (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    symbol VARCHAR(32) NOT NULL,
    open_price DECIMAL(20,8) NOT NULL,
    high_price DECIMAL(20,8) NOT NULL,
    low_price DECIMAL(20,8) NOT NULL,
    close_price DECIMAL(20,8) NOT NULL,
    volume DECIMAL(20,8) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 創建時間序列超表
SELECT create_hypertable('market_data', 'timestamp', if_not_exists => TRUE);

-- 創建索引以提高查詢性能
CREATE INDEX IF NOT EXISTS idx_market_data_symbol_timestamp ON market_data (symbol, timestamp DESC);

-- 交易所信息表
CREATE TABLE IF NOT EXISTS exchanges (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL UNIQUE,
    code VARCHAR(16) NOT NULL UNIQUE,
    timezone VARCHAR(32) NOT NULL DEFAULT 'UTC',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 交易品種表
CREATE TABLE IF NOT EXISTS instruments (
    id BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(32) NOT NULL,
    name VARCHAR(128),
    exchange_id BIGINT REFERENCES exchanges(id),
    instrument_type VARCHAR(32) NOT NULL,
    base_currency VARCHAR(8),
    quote_currency VARCHAR(8),
    tick_size DECIMAL(20,8),
    lot_size DECIMAL(20,8),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(symbol, exchange_id)
);

-- 插入一些測試數據
INSERT INTO exchanges (name, code, timezone) VALUES
('Binance', 'BINANCE', 'UTC'),
('Coinbase Pro', 'COINBASE', 'UTC'),
('NYSE', 'NYSE', 'America/New_York')
ON CONFLICT (code) DO NOTHING;

INSERT INTO instruments (symbol, name, exchange_id, instrument_type, base_currency, quote_currency, tick_size, lot_size) VALUES
('BTCUSD', 'Bitcoin USD', (SELECT id FROM exchanges WHERE code = 'BINANCE'), 'CRYPTO', 'BTC', 'USD', 0.01, 0.0001),
('ETHUSD', 'Ethereum USD', (SELECT id FROM exchanges WHERE code = 'BINANCE'), 'CRYPTO', 'ETH', 'USD', 0.01, 0.001),
('AAPL', 'Apple Inc.', (SELECT id FROM exchanges WHERE code = 'NYSE'), 'STOCK', NULL, 'USD', 0.01, 1)
ON CONFLICT (symbol, exchange_id) DO NOTHING;

-- 插入測試市場數據
INSERT INTO market_data (timestamp, symbol, open_price, high_price, low_price, close_price, volume) VALUES
-- Bitcoin 數據
('2024-01-01 00:00:00+00', 'BTCUSD', 45000.0, 45100.0, 44900.0, 45050.0, 100.5),
('2024-01-01 01:00:00+00', 'BTCUSD', 45050.0, 45200.0, 45000.0, 45150.0, 120.3),
('2024-01-01 02:00:00+00', 'BTCUSD', 45150.0, 45300.0, 45100.0, 45250.0, 95.7),
('2024-01-01 03:00:00+00', 'BTCUSD', 45250.0, 45400.0, 45200.0, 45350.0, 87.2),
('2024-01-01 04:00:00+00', 'BTCUSD', 45350.0, 45500.0, 45300.0, 45450.0, 92.8),

-- Ethereum 數據
('2024-01-01 00:00:00+00', 'ETHUSD', 3500.0, 3520.0, 3480.0, 3510.0, 200.1),
('2024-01-01 01:00:00+00', 'ETHUSD', 3510.0, 3540.0, 3500.0, 3535.0, 180.5),
('2024-01-01 02:00:00+00', 'ETHUSD', 3535.0, 3560.0, 3525.0, 3550.0, 165.8),
('2024-01-01 03:00:00+00', 'ETHUSD', 3550.0, 3580.0, 3540.0, 3570.0, 172.3),
('2024-01-01 04:00:00+00', 'ETHUSD', 3570.0, 3600.0, 3560.0, 3590.0, 155.9),

-- Apple 股票數據
('2024-01-01 09:30:00-05', 'AAPL', 185.50, 186.20, 185.10, 185.80, 1000000),
('2024-01-01 10:30:00-05', 'AAPL', 185.80, 187.00, 185.60, 186.50, 850000),
('2024-01-01 11:30:00-05', 'AAPL', 186.50, 187.20, 186.20, 186.90, 750000),
('2024-01-01 12:30:00-05', 'AAPL', 186.90, 188.00, 186.70, 187.60, 920000),
('2024-01-01 13:30:00-05', 'AAPL', 187.60, 188.50, 187.30, 188.20, 680000)

ON CONFLICT DO NOTHING;

-- 創建權限（模擬唯讀環境）
-- 注意：在實際環境中，市場數據用戶應該只有讀取權限
GRANT CONNECT ON DATABASE marketdata TO market_user;
GRANT USAGE ON SCHEMA public TO market_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO market_user;
GRANT SELECT ON ALL SEQUENCES IN SCHEMA public TO market_user;

-- 設置默認權限以確保新表也只能讀取
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO market_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON SEQUENCES TO market_user;

-- 創建視圖以提供常用查詢
CREATE OR REPLACE VIEW latest_prices AS
SELECT DISTINCT ON (symbol) 
    symbol,
    timestamp,
    close_price as price,
    volume
FROM market_data 
ORDER BY symbol, timestamp DESC;

CREATE OR REPLACE VIEW daily_ohlcv AS
SELECT 
    symbol,
    date_trunc('day', timestamp) as date,
    first(open_price, timestamp) as open,
    max(high_price) as high,
    min(low_price) as low,
    last(close_price, timestamp) as close,
    sum(volume) as volume
FROM market_data
GROUP BY symbol, date_trunc('day', timestamp)
ORDER BY symbol, date DESC;

GRANT SELECT ON latest_prices TO market_user;
GRANT SELECT ON daily_ohlcv TO market_user;