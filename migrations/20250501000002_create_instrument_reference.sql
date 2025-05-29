-- 金融商品參考表（輕量級版本）
-- 此表只包含回測系統需要的核心商品資訊
-- 需要定期從市場資料庫同步更新
CREATE TABLE instrument_reference (
    instrument_id INTEGER PRIMARY KEY,
    symbol VARCHAR(50) NOT NULL,
    exchange_code VARCHAR(10) NOT NULL,
    instrument_type VARCHAR(20) NOT NULL CHECK (instrument_type IN ('STOCK', 'FUTURE', 'OPTIONCONTRACT', 'FOREX', 'CRYPTO')),
    name VARCHAR(200) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    -- 同步相關欄位
    last_sync_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    sync_version BIGINT NOT NULL DEFAULT 0, -- 用於追蹤同步版本
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(symbol, exchange_code, instrument_type)
);

-- 創建索引以優化查詢
CREATE INDEX idx_instrument_ref_symbol ON instrument_reference(symbol);
CREATE INDEX idx_instrument_ref_exchange ON instrument_reference(exchange_code);
CREATE INDEX idx_instrument_ref_type ON instrument_reference(instrument_type);
CREATE INDEX idx_instrument_ref_active ON instrument_reference(is_active);
CREATE INDEX idx_instrument_ref_sync ON instrument_reference(last_sync_at);

-- 創建同步歷史表（用於追蹤同步狀態）
CREATE TABLE instrument_sync_log (
    sync_id SERIAL PRIMARY KEY,
    sync_type VARCHAR(20) NOT NULL, -- 'FULL', 'INCREMENTAL'
    sync_status VARCHAR(20) NOT NULL, -- 'STARTED', 'COMPLETED', 'FAILED'
    records_synced INTEGER,
    records_failed INTEGER,
    error_message TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    metadata JSONB -- 額外的同步資訊
);

-- 創建索引
CREATE INDEX idx_sync_log_status ON instrument_sync_log(sync_status);
CREATE INDEX idx_sync_log_started ON instrument_sync_log(started_at DESC);

-- 添加更新時間戳的觸發器
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_instrument_reference_updated_at BEFORE UPDATE
    ON instrument_reference FOR EACH ROW EXECUTE PROCEDURE 
    update_updated_at_column();

-- 添加註解說明
COMMENT ON TABLE instrument_reference IS '金融商品參考表 - 存儲回測需要的核心商品資訊，定期從市場資料庫同步';
COMMENT ON COLUMN instrument_reference.instrument_id IS '商品ID，與市場資料庫保持一致';
COMMENT ON COLUMN instrument_reference.sync_version IS '同步版本號，用於增量更新';
COMMENT ON COLUMN instrument_reference.last_sync_at IS '最後一次成功同步的時間';

COMMENT ON TABLE instrument_sync_log IS '商品資料同步歷史記錄';
COMMENT ON COLUMN instrument_sync_log.sync_type IS '同步類型：FULL=全量同步, INCREMENTAL=增量同步';
COMMENT ON COLUMN instrument_sync_log.sync_status IS '同步狀態：STARTED=開始, COMPLETED=完成, FAILED=失敗';