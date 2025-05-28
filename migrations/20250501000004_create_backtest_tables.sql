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
    instrument_id INTEGER NOT NULL REFERENCES instrument_reference(instrument_id),
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
    instrument_id INTEGER NOT NULL REFERENCES instrument_reference(instrument_id),
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