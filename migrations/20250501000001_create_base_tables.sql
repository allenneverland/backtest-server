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