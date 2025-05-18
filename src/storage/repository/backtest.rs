use crate::storage::models::*;
use crate::storage::repository::{DbExecutor, Page, PageQuery, TimeRange};
use anyhow::Result;
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;

/// 回測系統儲存庫特徵
pub trait BacktestRepository: Send + Sync {
    /// 創建回測配置
    async fn create_backtest_config(&self, config: BacktestConfigInsert) -> Result<BacktestConfig>;

    /// 根據ID獲取回測配置
    async fn get_backtest_config(&self, config_id: i32) -> Result<Option<BacktestConfig>>;

    /// 獲取回測配置列表
    async fn list_backtest_configs(&self, page: PageQuery) -> Result<Page<BacktestConfig>>;

    /// 創建回測結果
    async fn create_backtest_result(&self, result: BacktestResultInsert) -> Result<BacktestResult>;

    /// 更新回測結果
    async fn update_backtest_result(&self, result_id: i32, result: BacktestResultInsert) -> Result<BacktestResult>;
    
    /// 根據ID獲取回測結果
    async fn get_backtest_result(&self, result_id: i32) -> Result<Option<BacktestResult>>;

    /// 根據配置ID獲取回測結果
    async fn get_backtest_result_by_config(&self, config_id: i32) -> Result<Option<BacktestResult>>;

    /// 添加回測交易記錄
    async fn add_backtest_trade(&self, trade: BacktestTradeInsert) -> Result<()>;
    
    /// 批量添加回測交易記錄
    async fn add_backtest_trades(&self, trades: Vec<BacktestTradeInsert>) -> Result<()>;

    /// 獲取回測交易記錄
    async fn get_backtest_trades(&self, result_id: i32, time_range: TimeRange, page: PageQuery) -> Result<Page<BacktestTrade>>;

    /// 添加回測倉位快照
    async fn add_backtest_position_snapshot(&self, snapshot: BacktestPositionSnapshotInsert) -> Result<()>;
    
    /// 批量添加回測倉位快照
    async fn add_backtest_position_snapshots(&self, snapshots: Vec<BacktestPositionSnapshotInsert>) -> Result<()>;

    /// 獲取回測倉位快照
    async fn get_backtest_position_snapshots(&self, result_id: i32, time_range: TimeRange, page: PageQuery) -> Result<Page<BacktestPositionSnapshot>>;

    /// 添加回測投資組合快照
    async fn add_backtest_portfolio_snapshot(&self, snapshot: BacktestPortfolioSnapshotInsert) -> Result<()>;
    
    /// 批量添加回測投資組合快照
    async fn add_backtest_portfolio_snapshots(&self, snapshots: Vec<BacktestPortfolioSnapshotInsert>) -> Result<()>;

    /// 獲取回測投資組合快照
    async fn get_backtest_portfolio_snapshots(&self, result_id: i32, time_range: TimeRange, page: PageQuery) -> Result<Page<BacktestPortfolioSnapshot>>;

    /// 獲取回測日收益率聚合
    async fn get_backtest_daily_returns(&self, result_id: i32, time_range: TimeRange) -> Result<Vec<BacktestDailyReturns>>;
}

/// PostgreSQL 回測系統儲存庫實現
pub struct PgBacktestRepository {
    pool: Arc<PgPool>,
}

impl PgBacktestRepository {
    /// 創建新的回測系統儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgBacktestRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

impl BacktestRepository for PgBacktestRepository {
    async fn create_backtest_config(&self, config: BacktestConfigInsert) -> Result<BacktestConfig> {
        let result = sqlx::query_as!(
            BacktestConfig,
            r#"
            INSERT INTO backtest_config (
                name, description, start_date, end_date, initial_capital, 
                currency, instruments, strategy_id, execution_settings, 
                risk_settings
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
            )
            RETURNING 
                config_id, name, description, start_date, end_date, 
                initial_capital, currency, instruments as "instruments!: _", 
                strategy_id, execution_settings as "execution_settings!: _", 
                risk_settings as "risk_settings!: _", created_at, updated_at
            "#,
            config.name,
            config.description,
            config.start_date,
            config.end_date,
            config.initial_capital,
            config.currency,
            &config.instruments as &[i32],
            config.strategy_id,
            config.execution_settings as _,
            config.risk_settings as _
        )
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_backtest_config(&self, config_id: i32) -> Result<Option<BacktestConfig>> {
        let result = sqlx::query_as!(
            BacktestConfig,
            r#"
            SELECT 
                config_id, name, description, start_date, end_date, 
                initial_capital, currency, instruments as "instruments!: _", 
                strategy_id, execution_settings as "execution_settings!: _", 
                risk_settings as "risk_settings!: _", created_at, updated_at
            FROM backtest_config
            WHERE config_id = $1
            "#,
            config_id
        )
        .fetch_optional(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn list_backtest_configs(&self, page: PageQuery) -> Result<Page<BacktestConfig>> {
        let offset = (page.page - 1) * page.page_size;
        
        let configs = sqlx::query_as!(
            BacktestConfig,
            r#"
            SELECT 
                config_id, name, description, start_date, end_date, 
                initial_capital, currency, instruments as "instruments!: _", 
                strategy_id, execution_settings as "execution_settings!: _", 
                risk_settings as "risk_settings!: _", created_at, updated_at
            FROM backtest_config
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM backtest_config")
            .fetch_one(DbExecutor::get_pool(self))
            .await?;

        Ok(Page::new(configs, total, page.page, page.page_size))
    }

    async fn create_backtest_result(&self, result: BacktestResultInsert) -> Result<BacktestResult> {
        let result = sqlx::query_as!(
            BacktestResult,
            r#"
            INSERT INTO backtest_result (
                config_id, status, start_time, end_time, execution_time,
                metrics, benchmark_comparison, error_message
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8
            )
            RETURNING 
                result_id, config_id, status, start_time, end_time, execution_time,
                metrics as "metrics!: _", benchmark_comparison as "benchmark_comparison: _",
                error_message, created_at, updated_at
            "#,
            result.config_id,
            result.status,
            result.start_time,
            result.end_time,
            result.execution_time,
            result.metrics as _,
            result.benchmark_comparison as _,
            result.error_message,
        )
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn update_backtest_result(&self, result_id: i32, result: BacktestResultInsert) -> Result<BacktestResult> {
        let result = sqlx::query_as!(
            BacktestResult,
            r#"
            UPDATE backtest_result SET
                status = $2,
                start_time = $3,
                end_time = $4,
                execution_time = $5,
                metrics = $6,
                benchmark_comparison = $7,
                error_message = $8,
                updated_at = NOW()
            WHERE result_id = $1
            RETURNING 
                result_id, config_id, status, start_time, end_time, execution_time,
                metrics as "metrics!: _", benchmark_comparison as "benchmark_comparison: _",
                error_message, created_at, updated_at
            "#,
            result_id,
            result.status,
            result.start_time,
            result.end_time,
            result.execution_time,
            result.metrics as _,
            result.benchmark_comparison as _,
            result.error_message,
        )
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_backtest_result(&self, result_id: i32) -> Result<Option<BacktestResult>> {
        let result = sqlx::query_as!(
            BacktestResult,
            r#"
            SELECT 
                result_id, config_id, status, start_time, end_time, execution_time,
                metrics as "metrics!: _", benchmark_comparison as "benchmark_comparison: _",
                error_message, created_at, updated_at
            FROM backtest_result
            WHERE result_id = $1
            "#,
            result_id
        )
        .fetch_optional(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_backtest_result_by_config(&self, config_id: i32) -> Result<Option<BacktestResult>> {
        let result = sqlx::query_as!(
            BacktestResult,
            r#"
            SELECT 
                result_id, config_id, status, start_time, end_time, execution_time,
                metrics as "metrics!: _", benchmark_comparison as "benchmark_comparison: _",
                error_message, created_at, updated_at
            FROM backtest_result
            WHERE config_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            config_id
        )
        .fetch_optional(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn add_backtest_trade(&self, trade: BacktestTradeInsert) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO backtest_trade (
                time, result_id, instrument_id, direction, price, 
                quantity, amount, commission, slippage, trade_id, 
                position_effect, order_type, metadata
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
            "#,
            trade.time,
            trade.result_id,
            trade.instrument_id,
            trade.direction,
            trade.price,
            trade.quantity,
            trade.amount,
            trade.commission,
            trade.slippage,
            trade.trade_id,
            trade.position_effect,
            trade.order_type,
            trade.metadata as _,
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(())
    }

    async fn add_backtest_trades(&self, trades: Vec<BacktestTradeInsert>) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;
        
        for trade in trades {
            sqlx::query!(
                r#"
                INSERT INTO backtest_trade (
                    time, result_id, instrument_id, direction, price, 
                    quantity, amount, commission, slippage, trade_id, 
                    position_effect, order_type, metadata
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
                )
                "#,
                trade.time,
                trade.result_id,
                trade.instrument_id,
                trade.direction,
                trade.price,
                trade.quantity,
                trade.amount,
                trade.commission,
                trade.slippage,
                trade.trade_id,
                trade.position_effect,
                trade.order_type,
                trade.metadata as _,
            )
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    async fn get_backtest_trades(&self, result_id: i32, time_range: TimeRange, page: PageQuery) -> Result<Page<BacktestTrade>> {
        let offset = (page.page - 1) * page.page_size;
        
        let trades = sqlx::query_as!(
            BacktestTrade,
            r#"
            SELECT
                time, result_id, instrument_id, direction, price,
                quantity, amount, commission, slippage, trade_id,
                position_effect, order_type, metadata as "metadata: _",
                created_at
            FROM backtest_trade
            WHERE result_id = $1
            AND time BETWEEN $2 AND $3
            ORDER BY time DESC
            LIMIT $4 OFFSET $5
            "#,
            result_id,
            time_range.start,
            time_range.end,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM backtest_trade WHERE result_id = $1 AND time BETWEEN $2 AND $3"
        )
        .bind(result_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(trades, total, page.page, page.page_size))
    }

    async fn add_backtest_position_snapshot(&self, snapshot: BacktestPositionSnapshotInsert) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO backtest_position_snapshot (
                time, result_id, instrument_id, quantity, avg_cost,
                market_value, unrealized_pl, realized_pl, margin_used
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9
            )
            "#,
            snapshot.time,
            snapshot.result_id,
            snapshot.instrument_id,
            snapshot.quantity,
            snapshot.avg_cost,
            snapshot.market_value,
            snapshot.unrealized_pl,
            snapshot.realized_pl,
            snapshot.margin_used,
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(())
    }

    async fn add_backtest_position_snapshots(&self, snapshots: Vec<BacktestPositionSnapshotInsert>) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;
        
        for snapshot in snapshots {
            sqlx::query!(
                r#"
                INSERT INTO backtest_position_snapshot (
                    time, result_id, instrument_id, quantity, avg_cost,
                    market_value, unrealized_pl, realized_pl, margin_used
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9
                )
                "#,
                snapshot.time,
                snapshot.result_id,
                snapshot.instrument_id,
                snapshot.quantity,
                snapshot.avg_cost,
                snapshot.market_value,
                snapshot.unrealized_pl,
                snapshot.realized_pl,
                snapshot.margin_used,
            )
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    async fn get_backtest_position_snapshots(&self, result_id: i32, time_range: TimeRange, page: PageQuery) -> Result<Page<BacktestPositionSnapshot>> {
        let offset = (page.page - 1) * page.page_size;
        
        let snapshots = sqlx::query_as!(
            BacktestPositionSnapshot,
            r#"
            SELECT
                time, result_id, instrument_id, quantity, avg_cost,
                market_value, unrealized_pl, realized_pl, margin_used,
                created_at
            FROM backtest_position_snapshot
            WHERE result_id = $1
            AND time BETWEEN $2 AND $3
            ORDER BY time DESC
            LIMIT $4 OFFSET $5
            "#,
            result_id,
            time_range.start,
            time_range.end,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM backtest_position_snapshot WHERE result_id = $1 AND time BETWEEN $2 AND $3"
        )
        .bind(result_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(snapshots, total, page.page, page.page_size))
    }

    async fn add_backtest_portfolio_snapshot(&self, snapshot: BacktestPortfolioSnapshotInsert) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO backtest_portfolio_snapshot (
                time, result_id, total_value, cash, equity,
                margin, daily_pnl, total_pnl, daily_return, total_return,
                metadata
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            "#,
            snapshot.time,
            snapshot.result_id,
            snapshot.total_value,
            snapshot.cash,
            snapshot.equity,
            snapshot.margin,
            snapshot.daily_pnl,
            snapshot.total_pnl,
            snapshot.daily_return,
            snapshot.total_return,
            snapshot.metadata as _,
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(())
    }

    async fn add_backtest_portfolio_snapshots(&self, snapshots: Vec<BacktestPortfolioSnapshotInsert>) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;
        
        for snapshot in snapshots {
            sqlx::query!(
                r#"
                INSERT INTO backtest_portfolio_snapshot (
                    time, result_id, total_value, cash, equity,
                    margin, daily_pnl, total_pnl, daily_return, total_return,
                    metadata
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
                )
                "#,
                snapshot.time,
                snapshot.result_id,
                snapshot.total_value,
                snapshot.cash,
                snapshot.equity,
                snapshot.margin,
                snapshot.daily_pnl,
                snapshot.total_pnl,
                snapshot.daily_return,
                snapshot.total_return,
                snapshot.metadata as _,
            )
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    async fn get_backtest_portfolio_snapshots(&self, result_id: i32, time_range: TimeRange, page: PageQuery) -> Result<Page<BacktestPortfolioSnapshot>> {
        let offset = (page.page - 1) * page.page_size;
        
        let snapshots = sqlx::query_as!(
            BacktestPortfolioSnapshot,
            r#"
            SELECT
                time, result_id, total_value, cash, equity,
                margin, daily_pnl, total_pnl, daily_return, total_return,
                metadata as "metadata: _", created_at
            FROM backtest_portfolio_snapshot
            WHERE result_id = $1
            AND time BETWEEN $2 AND $3
            ORDER BY time DESC
            LIMIT $4 OFFSET $5
            "#,
            result_id,
            time_range.start,
            time_range.end,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM backtest_portfolio_snapshot WHERE result_id = $1 AND time BETWEEN $2 AND $3"
        )
        .bind(result_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(snapshots, total, page.page, page.page_size))
    }

    async fn get_backtest_daily_returns(&self, result_id: i32, time_range: TimeRange) -> Result<Vec<BacktestDailyReturns>> {
        let returns = sqlx::query_as!(
            BacktestDailyReturns,
            r#"
            SELECT
                bucket as "bucket!", 
                result_id as "result_id!", 
                daily_return as "daily_return!: _",
                end_of_day_value as "end_of_day_value!: _",
                end_of_day_equity as "end_of_day_equity!: _"
            FROM backtest_daily_returns
            WHERE result_id = $1
            AND bucket BETWEEN $2 AND $3
            ORDER BY bucket
            "#,
            result_id,
            time_range.start,
            time_range.end
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(returns)
    }
} 