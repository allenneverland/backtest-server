-- 創建數據庫
CREATE DATABASE backtest_server_db;

-- 連接到數據庫
\c backtest_server_db

-- 啟用TimescaleDB擴展
CREATE EXTENSION IF NOT EXISTS timescaledb; 