-- 創建回測應用程序角色（BacktestServer 使用）
CREATE ROLE backtest_app WITH LOGIN PASSWORD 'backtest_app_password';

-- 創建回測管理員角色
CREATE ROLE backtest_admin WITH LOGIN PASSWORD 'backtest_admin_password';

-- 設置權限
-- 應用程序角色：完整的CRUD權限
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO backtest_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO backtest_app;
GRANT USAGE ON SCHEMA public TO backtest_app;

-- 管理員角色：完全權限
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO backtest_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO backtest_admin;
GRANT USAGE ON SCHEMA public TO backtest_admin;

-- 注意：沒有設置只讀角色，因為外部系統（如 StratPlat）通過 RabbitMQ 獲取數據，而非直接訪問資料庫