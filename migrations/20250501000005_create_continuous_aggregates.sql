-- 執行日收益率聚合視圖
CREATE MATERIALIZED VIEW execution_daily_returns
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    run_id,
    last(total_return, time) AS daily_return,
    last(total_value, time) AS end_of_day_value,
    last(equity, time) AS end_of_day_equity
FROM execution_portfolios
GROUP BY bucket, run_id
WITH NO DATA;

-- 添加自動刷新策略（每小時刷新）
SELECT add_continuous_aggregate_policy('execution_daily_returns',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');