use crate::storage::models::execution_run::*;
use crate::storage::models::execution_data::*;
use crate::storage::repository::{DbExecutor, Page, PageQuery, TimeRange};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// 執行系統儲存庫特徵
#[async_trait]
pub trait ExecutionRunRepository: Send + Sync {
    /// 創建執行任務
    async fn create_execution_run(&self, run: ExecutionRunInsert) -> Result<ExecutionRun>;

    /// 根據ID獲取執行任務
    async fn get_execution_run(&self, run_id: i32) -> Result<Option<ExecutionRun>>;

    /// 根據請求ID獲取執行任務
    async fn get_execution_run_by_request_id(&self, request_id: Uuid) -> Result<Option<ExecutionRun>>;

    /// 根據外部回測ID獲取執行任務
    async fn get_execution_runs_by_backtest_id(
        &self,
        external_backtest_id: i32,
    ) -> Result<Vec<ExecutionRun>>;

    /// 更新執行任務
    async fn update_execution_run(
        &self,
        run_id: i32,
        update: ExecutionRunUpdate,
    ) -> Result<ExecutionRun>;

    /// 獲取執行任務列表
    async fn list_execution_runs(
        &self,
        status: Option<String>,
        page: PageQuery,
    ) -> Result<Page<ExecutionRun>>;

    /// 添加執行交易記錄
    async fn add_execution_trade(&self, trade: ExecutionTradeInsert) -> Result<()>;

    /// 批量添加執行交易記錄
    async fn add_execution_trades(&self, trades: Vec<ExecutionTradeInsert>) -> Result<()>;

    /// 獲取執行交易記錄
    async fn get_execution_trades(
        &self,
        run_id: i32,
        time_range: TimeRange,
        page: PageQuery,
    ) -> Result<Page<ExecutionTrade>>;

    /// 添加執行倉位快照
    async fn add_execution_position(&self, position: ExecutionPositionInsert) -> Result<()>;

    /// 批量添加執行倉位快照
    async fn add_execution_positions(&self, positions: Vec<ExecutionPositionInsert>) -> Result<()>;

    /// 獲取執行倉位快照
    async fn get_execution_positions(
        &self,
        run_id: i32,
        time_range: TimeRange,
        page: PageQuery,
    ) -> Result<Page<ExecutionPosition>>;

    /// 添加執行投資組合快照
    async fn add_execution_portfolio(&self, portfolio: ExecutionPortfolioInsert) -> Result<()>;

    /// 批量添加執行投資組合快照
    async fn add_execution_portfolios(
        &self,
        portfolios: Vec<ExecutionPortfolioInsert>,
    ) -> Result<()>;

    /// 獲取執行投資組合快照
    async fn get_execution_portfolios(
        &self,
        run_id: i32,
        time_range: TimeRange,
        page: PageQuery,
    ) -> Result<Page<ExecutionPortfolio>>;

    /// 獲取執行日收益率聚合
    async fn get_execution_daily_returns(
        &self,
        run_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<ExecutionDailyReturns>>;

    /// 清理舊的執行數據
    async fn cleanup_old_execution_data(&self, days: i32) -> Result<u64>;
}

/// PostgreSQL 執行系統儲存庫實現
pub struct PgExecutionRunRepository {
    pool: Arc<PgPool>,
}

impl PgExecutionRunRepository {
    /// 創建新的執行系統儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgExecutionRunRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl ExecutionRunRepository for PgExecutionRunRepository {
    async fn create_execution_run(&self, run: ExecutionRunInsert) -> Result<ExecutionRun> {
        let result = sqlx::query_as!(
            ExecutionRun,
            r#"
            INSERT INTO execution_runs (
                external_backtest_id, request_id, strategy_dsl, parameters, 
                status, progress
            ) VALUES (
                $1, $2, $3, $4, $5, $6
            )
            RETURNING 
                run_id, external_backtest_id, request_id, strategy_dsl, 
                parameters as "parameters!: _", status, progress, started_at, 
                completed_at, execution_time_ms, error_code, error_message, 
                error_details as "error_details: _", created_at
            "#,
            run.external_backtest_id,
            run.request_id,
            run.strategy_dsl,
            run.parameters as _,
            run.status.unwrap_or_else(|| ExecutionStatus::Initializing.as_str().to_string()),
            run.progress.unwrap_or(0)
        )
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_execution_run(&self, run_id: i32) -> Result<Option<ExecutionRun>> {
        let result = sqlx::query_as!(
            ExecutionRun,
            r#"
            SELECT 
                run_id, external_backtest_id, request_id, strategy_dsl, 
                parameters as "parameters!: _", status, progress, started_at, 
                completed_at, execution_time_ms, error_code, error_message, 
                error_details as "error_details: _", created_at
            FROM execution_runs
            WHERE run_id = $1
            "#,
            run_id
        )
        .fetch_optional(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_execution_run_by_request_id(&self, request_id: Uuid) -> Result<Option<ExecutionRun>> {
        let result = sqlx::query_as!(
            ExecutionRun,
            r#"
            SELECT 
                run_id, external_backtest_id, request_id, strategy_dsl, 
                parameters as "parameters!: _", status, progress, started_at, 
                completed_at, execution_time_ms, error_code, error_message, 
                error_details as "error_details: _", created_at
            FROM execution_runs
            WHERE request_id = $1
            "#,
            request_id
        )
        .fetch_optional(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_execution_runs_by_backtest_id(
        &self,
        external_backtest_id: i32,
    ) -> Result<Vec<ExecutionRun>> {
        let results = sqlx::query_as!(
            ExecutionRun,
            r#"
            SELECT 
                run_id, external_backtest_id, request_id, strategy_dsl, 
                parameters as "parameters!: _", status, progress, started_at, 
                completed_at, execution_time_ms, error_code, error_message, 
                error_details as "error_details: _", created_at
            FROM execution_runs
            WHERE external_backtest_id = $1
            ORDER BY created_at DESC
            "#,
            external_backtest_id
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }

    async fn update_execution_run(
        &self,
        run_id: i32,
        update: ExecutionRunUpdate,
    ) -> Result<ExecutionRun> {
        let mut query = String::from("UPDATE execution_runs SET ");
        let mut set_clauses = vec![];
        let mut bind_count = 1;

        if update.status.is_some() {
            set_clauses.push(format!("status = ${}", bind_count));
            bind_count += 1;
        }
        if update.progress.is_some() {
            set_clauses.push(format!("progress = ${}", bind_count));
            bind_count += 1;
        }
        if update.completed_at.is_some() {
            set_clauses.push(format!("completed_at = ${}", bind_count));
            bind_count += 1;
        }
        if update.execution_time_ms.is_some() {
            set_clauses.push(format!("execution_time_ms = ${}", bind_count));
            bind_count += 1;
        }
        if update.error_code.is_some() {
            set_clauses.push(format!("error_code = ${}", bind_count));
            bind_count += 1;
        }
        if update.error_message.is_some() {
            set_clauses.push(format!("error_message = ${}", bind_count));
            bind_count += 1;
        }
        if update.error_details.is_some() {
            set_clauses.push(format!("error_details = ${}", bind_count));
            bind_count += 1;
        }

        if set_clauses.is_empty() {
            return self
                .get_execution_run(run_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Execution run not found"));
        }

        query.push_str(&set_clauses.join(", "));
        query.push_str(&format!(" WHERE run_id = ${} RETURNING *", bind_count));

        let mut query_builder = sqlx::query_as::<_, ExecutionRun>(&query);
        
        if let Some(status) = update.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(progress) = update.progress {
            query_builder = query_builder.bind(progress);
        }
        if let Some(completed_at) = update.completed_at {
            query_builder = query_builder.bind(completed_at);
        }
        if let Some(execution_time_ms) = update.execution_time_ms {
            query_builder = query_builder.bind(execution_time_ms);
        }
        if let Some(error_code) = update.error_code {
            query_builder = query_builder.bind(error_code);
        }
        if let Some(error_message) = update.error_message {
            query_builder = query_builder.bind(error_message);
        }
        if let Some(error_details) = update.error_details {
            query_builder = query_builder.bind(error_details);
        }

        query_builder = query_builder.bind(run_id);

        let result = query_builder
            .fetch_one(DbExecutor::get_pool(self))
            .await?;

        Ok(result)
    }

    async fn list_execution_runs(
        &self,
        status: Option<String>,
        page: PageQuery,
    ) -> Result<Page<ExecutionRun>> {
        let offset = (page.page - 1) * page.page_size;

        let (runs, total) = if let Some(status) = status {
            let runs = sqlx::query_as!(
                ExecutionRun,
                r#"
                SELECT 
                    run_id, external_backtest_id, request_id, strategy_dsl, 
                    parameters as "parameters!: _", status, progress, started_at, 
                    completed_at, execution_time_ms, error_code, error_message, 
                    error_details as "error_details: _", created_at
                FROM execution_runs
                WHERE status = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
                status,
                page.page_size,
                offset
            )
            .fetch_all(DbExecutor::get_pool(self))
            .await?;

            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM execution_runs WHERE status = $1")
                .bind(&status)
                .fetch_one(DbExecutor::get_pool(self))
                .await?;

            (runs, total)
        } else {
            let runs = sqlx::query_as!(
                ExecutionRun,
                r#"
                SELECT 
                    run_id, external_backtest_id, request_id, strategy_dsl, 
                    parameters as "parameters!: _", status, progress, started_at, 
                    completed_at, execution_time_ms, error_code, error_message, 
                    error_details as "error_details: _", created_at
                FROM execution_runs
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
                page.page_size,
                offset
            )
            .fetch_all(DbExecutor::get_pool(self))
            .await?;

            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM execution_runs")
                .fetch_one(DbExecutor::get_pool(self))
                .await?;

            (runs, total)
        };

        Ok(Page::new(runs, total, page.page, page.page_size))
    }

    async fn add_execution_trade(&self, trade: ExecutionTradeInsert) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO execution_trades (
                time, run_id, instrument_id, direction, price, 
                quantity, amount, commission, slippage, trade_id, 
                position_effect, order_type, metadata
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
            "#,
            trade.time,
            trade.run_id,
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

    async fn add_execution_trades(&self, trades: Vec<ExecutionTradeInsert>) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;

        for trade in trades {
            sqlx::query!(
                r#"
                INSERT INTO execution_trades (
                    time, run_id, instrument_id, direction, price, 
                    quantity, amount, commission, slippage, trade_id, 
                    position_effect, order_type, metadata
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
                )
                "#,
                trade.time,
                trade.run_id,
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

    async fn get_execution_trades(
        &self,
        run_id: i32,
        time_range: TimeRange,
        page: PageQuery,
    ) -> Result<Page<ExecutionTrade>> {
        let offset = (page.page - 1) * page.page_size;

        let trades = sqlx::query_as!(
            ExecutionTrade,
            r#"
            SELECT
                time, run_id, instrument_id, direction, price,
                quantity, amount, commission, slippage, trade_id,
                position_effect, order_type, metadata as "metadata: _",
                created_at
            FROM execution_trades
            WHERE run_id = $1
            AND time BETWEEN $2 AND $3
            ORDER BY time DESC
            LIMIT $4 OFFSET $5
            "#,
            run_id,
            time_range.start,
            time_range.end,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM execution_trades WHERE run_id = $1 AND time BETWEEN $2 AND $3",
        )
        .bind(run_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(trades, total, page.page, page.page_size))
    }

    async fn add_execution_position(&self, position: ExecutionPositionInsert) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO execution_positions (
                time, run_id, instrument_id, quantity, avg_cost,
                market_value, unrealized_pl, realized_pl, margin_used
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9
            )
            "#,
            position.time,
            position.run_id,
            position.instrument_id,
            position.quantity,
            position.avg_cost,
            position.market_value,
            position.unrealized_pl,
            position.realized_pl,
            position.margin_used,
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(())
    }

    async fn add_execution_positions(&self, positions: Vec<ExecutionPositionInsert>) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;

        for position in positions {
            sqlx::query!(
                r#"
                INSERT INTO execution_positions (
                    time, run_id, instrument_id, quantity, avg_cost,
                    market_value, unrealized_pl, realized_pl, margin_used
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9
                )
                "#,
                position.time,
                position.run_id,
                position.instrument_id,
                position.quantity,
                position.avg_cost,
                position.market_value,
                position.unrealized_pl,
                position.realized_pl,
                position.margin_used,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_execution_positions(
        &self,
        run_id: i32,
        time_range: TimeRange,
        page: PageQuery,
    ) -> Result<Page<ExecutionPosition>> {
        let offset = (page.page - 1) * page.page_size;

        let positions = sqlx::query_as!(
            ExecutionPosition,
            r#"
            SELECT
                time, run_id, instrument_id, quantity, avg_cost,
                market_value, unrealized_pl, realized_pl, margin_used,
                created_at
            FROM execution_positions
            WHERE run_id = $1
            AND time BETWEEN $2 AND $3
            ORDER BY time DESC
            LIMIT $4 OFFSET $5
            "#,
            run_id,
            time_range.start,
            time_range.end,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM execution_positions WHERE run_id = $1 AND time BETWEEN $2 AND $3"
        )
        .bind(run_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(positions, total, page.page, page.page_size))
    }

    async fn add_execution_portfolio(&self, portfolio: ExecutionPortfolioInsert) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO execution_portfolios (
                time, run_id, total_value, cash, equity,
                margin, daily_pnl, total_pnl, daily_return, total_return,
                metadata
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            "#,
            portfolio.time,
            portfolio.run_id,
            portfolio.total_value,
            portfolio.cash,
            portfolio.equity,
            portfolio.margin,
            portfolio.daily_pnl,
            portfolio.total_pnl,
            portfolio.daily_return,
            portfolio.total_return,
            portfolio.metadata as _,
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(())
    }

    async fn add_execution_portfolios(
        &self,
        portfolios: Vec<ExecutionPortfolioInsert>,
    ) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;

        for portfolio in portfolios {
            sqlx::query!(
                r#"
                INSERT INTO execution_portfolios (
                    time, run_id, total_value, cash, equity,
                    margin, daily_pnl, total_pnl, daily_return, total_return,
                    metadata
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
                )
                "#,
                portfolio.time,
                portfolio.run_id,
                portfolio.total_value,
                portfolio.cash,
                portfolio.equity,
                portfolio.margin,
                portfolio.daily_pnl,
                portfolio.total_pnl,
                portfolio.daily_return,
                portfolio.total_return,
                portfolio.metadata as _,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_execution_portfolios(
        &self,
        run_id: i32,
        time_range: TimeRange,
        page: PageQuery,
    ) -> Result<Page<ExecutionPortfolio>> {
        let offset = (page.page - 1) * page.page_size;

        let portfolios = sqlx::query_as!(
            ExecutionPortfolio,
            r#"
            SELECT
                time, run_id, total_value, cash, equity,
                margin, daily_pnl, total_pnl, daily_return, total_return,
                metadata as "metadata: _", created_at
            FROM execution_portfolios
            WHERE run_id = $1
            AND time BETWEEN $2 AND $3
            ORDER BY time DESC
            LIMIT $4 OFFSET $5
            "#,
            run_id,
            time_range.start,
            time_range.end,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM execution_portfolios WHERE run_id = $1 AND time BETWEEN $2 AND $3"
        )
        .bind(run_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(portfolios, total, page.page, page.page_size))
    }

    async fn get_execution_daily_returns(
        &self,
        run_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<ExecutionDailyReturns>> {
        let returns = sqlx::query_as!(
            ExecutionDailyReturns,
            r#"
            SELECT
                bucket as "bucket!", 
                run_id as "run_id!", 
                daily_return as "daily_return!: _",
                end_of_day_value as "end_of_day_value!: _",
                end_of_day_equity as "end_of_day_equity!: _"
            FROM execution_daily_returns
            WHERE run_id = $1
            AND bucket BETWEEN $2 AND $3
            ORDER BY bucket
            "#,
            run_id,
            time_range.start,
            time_range.end
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(returns)
    }

    async fn cleanup_old_execution_data(&self, days: i32) -> Result<u64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(days as i64);
        
        let result = sqlx::query!(
            r#"
            DELETE FROM execution_runs
            WHERE status IN ('COMPLETED', 'FAILED')
            AND completed_at < $1
            "#,
            cutoff_date
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::create_test_pool;
    use chrono::Utc;
    use sqlx::types::Json;

    #[tokio::test]
    async fn test_execution_run_crud() {
        let pool = create_test_pool().await;
        let repo = PgExecutionRunRepository::new(Arc::new(pool));

        // Create test data
        let run_insert = ExecutionRunInsert {
            external_backtest_id: 123,
            request_id: Uuid::new_v4(),
            strategy_dsl: "test strategy".to_string(),
            parameters: Json(serde_json::json!({"initial_capital": 100000})),
            status: Some(ExecutionStatus::Initializing.as_str().to_string()),
            progress: Some(0),
        };

        // Test create
        let created = repo.create_execution_run(run_insert).await.unwrap();
        assert_eq!(created.external_backtest_id, 123);
        assert_eq!(created.status, ExecutionStatus::Initializing.as_str());

        // Test get
        let fetched = repo
            .get_execution_run(created.run_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.run_id, created.run_id);

        // Test update
        let update = ExecutionRunUpdate {
            status: Some(ExecutionStatus::Running.as_str().to_string()),
            progress: Some(50),
            ..Default::default()
        };
        let updated = repo
            .update_execution_run(created.run_id, update)
            .await
            .unwrap();
        assert_eq!(updated.status, ExecutionStatus::Running.as_str());
        assert_eq!(updated.progress, Some(50));
    }
}