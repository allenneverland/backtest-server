-- 創建回測應用程序角色
CREATE ROLE backtest_app WITH LOGIN PASSWORD 'backtest_app_password';

-- 創建回測只讀角色
CREATE ROLE backtest_readonly WITH LOGIN PASSWORD 'backtest_readonly_password';

-- 創建回測管理員角色
CREATE ROLE backtest_admin WITH LOGIN PASSWORD 'backtest_admin_password';

-- 設置權限
-- 只讀角色：只能查詢數據
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backtest_readonly;
GRANT USAGE ON SCHEMA public TO backtest_readonly;

-- 應用程序角色：完整的CRUD權限
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO backtest_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO backtest_app;
GRANT USAGE ON SCHEMA public TO backtest_app;

-- 管理員角色：完全權限
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO backtest_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO backtest_admin;
GRANT USAGE ON SCHEMA public TO backtest_admin;