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