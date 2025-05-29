-- 執行任務記錄表
CREATE TABLE execution_runs (
    run_id SERIAL PRIMARY KEY,
    external_backtest_id INTEGER NOT NULL,      -- StratPlat 的 backtest ID
    request_id UUID NOT NULL UNIQUE,            -- RabbitMQ 請求 ID
    strategy_dsl TEXT NOT NULL,                 -- 從 RabbitMQ 接收的策略 DSL
    parameters JSONB NOT NULL,                  -- 執行參數（開始日期、結束日期、初始資金等）
    status VARCHAR(20) NOT NULL DEFAULT 'INITIALIZING', -- INITIALIZING, RUNNING, COMPLETED, FAILED
    progress INTEGER DEFAULT 0,                 -- 執行進度 (0-100)
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    execution_time_ms INTEGER,                  -- 執行時間（毫秒）
    error_code VARCHAR(50),                     -- 錯誤代碼
    error_message TEXT,                         -- 錯誤訊息
    error_details JSONB,                        -- 詳細錯誤信息
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_execution_runs_external_backtest_id ON execution_runs(external_backtest_id);
CREATE INDEX idx_execution_runs_request_id ON execution_runs(request_id);
CREATE INDEX idx_execution_runs_status ON execution_runs(status);
CREATE INDEX idx_execution_runs_started_at ON execution_runs(started_at);

-- 執行日誌表
CREATE TABLE execution_logs (
    log_id BIGSERIAL PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    log_level VARCHAR(10) NOT NULL,            -- DEBUG, INFO, WARN, ERROR
    component VARCHAR(50),                      -- 組件名稱（如：data_loader, strategy_executor）
    message TEXT NOT NULL,
    details JSONB,                              -- 額外的結構化數據
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 創建索引
CREATE INDEX idx_execution_logs_run_id ON execution_logs(run_id);
CREATE INDEX idx_execution_logs_timestamp ON execution_logs(timestamp);
CREATE INDEX idx_execution_logs_log_level ON execution_logs(log_level);
CREATE INDEX idx_execution_logs_component ON execution_logs(component);

-- 執行交易記錄表（臨時存儲）
CREATE TABLE execution_trades (
    time TIMESTAMPTZ NOT NULL,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
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
SELECT create_hypertable('execution_trades', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_execution_trades_run ON execution_trades(run_id);
CREATE INDEX idx_execution_trades_instrument ON execution_trades(instrument_id);
CREATE INDEX idx_execution_trades_run_time ON execution_trades(run_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE execution_trades SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'run_id, instrument_id'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('execution_trades', INTERVAL '7 days');

-- 執行倉位快照表（臨時存儲）
CREATE TABLE execution_positions (
    time TIMESTAMPTZ NOT NULL,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
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
SELECT create_hypertable('execution_positions', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_execution_positions_run ON execution_positions(run_id);
CREATE INDEX idx_execution_positions_instrument ON execution_positions(instrument_id);
CREATE INDEX idx_execution_positions_run_time ON execution_positions(run_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE execution_positions SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'run_id, instrument_id'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('execution_positions', INTERVAL '7 days');

-- 執行投資組合快照表（臨時存儲）
CREATE TABLE execution_portfolios (
    time TIMESTAMPTZ NOT NULL,
    run_id INTEGER NOT NULL REFERENCES execution_runs(run_id) ON DELETE CASCADE,
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
SELECT create_hypertable('execution_portfolios', 'time',
                        chunk_time_interval => INTERVAL '1 day');

-- 創建索引
CREATE INDEX idx_execution_portfolios_run ON execution_portfolios(run_id);
CREATE INDEX idx_execution_portfolios_run_time ON execution_portfolios(run_id, time DESC);

-- 設置數據壓縮策略
ALTER TABLE execution_portfolios SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'run_id'
);

-- 添加壓縮策略（7天前的數據自動壓縮）
SELECT add_compression_policy('execution_portfolios', INTERVAL '7 days');