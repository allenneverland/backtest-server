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
GROUP BY bucket, instrument_id
WITH NO DATA;

-- 添加自動刷新策略（每日刷新）
SELECT add_continuous_aggregate_policy('daily_volume_by_instrument',
    start_offset => INTERVAL '1 month',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

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
GROUP BY bucket, result_id
WITH NO DATA;

-- 添加自動刷新策略（每日刷新）
SELECT add_continuous_aggregate_policy('backtest_daily_returns',
    start_offset => INTERVAL '1 month',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day'); 