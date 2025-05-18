-- 創建應用程序角色
CREATE ROLE backtest_server_app WITH LOGIN PASSWORD 'secure_password';

-- 創建只讀角色
CREATE ROLE backtest_server_readonly WITH LOGIN PASSWORD 'readonly_password';

-- 創建數據導入角色
CREATE ROLE backtest_server_importer WITH LOGIN PASSWORD 'importer_password';

-- 設置權限
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backtest_server_readonly;

GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO backtest_server_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO backtest_server_app;

GRANT SELECT, INSERT ON minute_bar, tick TO backtest_server_importer;
GRANT SELECT ON instrument, exchange TO backtest_server_importer;
GRANT USAGE, SELECT ON SEQUENCE minute_bars_id_seq, ticks_id_seq TO backtest_server_importer; 