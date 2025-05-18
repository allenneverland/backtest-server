use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use chrono::{DateTime, Utc};

use crate::storage::{
    models::portfolio::{
        Portfolio, PortfolioHolding, PortfolioInsert, PortfolioHoldingInsert, PortfolioPerformance
    },
    repository::{DbExecutor, PageQuery, Page, TimeRange}
};

/// 投資組合儲存庫特性
#[async_trait]
pub trait PortfolioRepository: Send + Sync + DbExecutor {
    // 投資組合基本操作
    async fn get_portfolio_by_id(&self, portfolio_id: i32) -> Result<Option<Portfolio>>;
    async fn get_portfolio_by_name(&self, name: &str) -> Result<Option<Portfolio>>;
    async fn get_portfolios(&self, active_only: bool, page: PageQuery) -> Result<Page<Portfolio>>;
    async fn insert_portfolio(&self, portfolio: &PortfolioInsert) -> Result<i32>;
    async fn update_portfolio(&self, portfolio: &Portfolio) -> Result<()>;
    async fn delete_portfolio(&self, portfolio_id: i32) -> Result<bool>;
    
    // 投資組合持倉操作
    async fn get_portfolio_holdings(&self, portfolio_id: i32, time: Option<DateTime<Utc>>) -> Result<Vec<PortfolioHolding>>;
    async fn get_portfolio_holding_history(&self, portfolio_id: i32, instrument_id: i32, time_range: TimeRange) -> Result<Vec<PortfolioHolding>>;
    async fn insert_portfolio_holding(&self, holding: &PortfolioHoldingInsert) -> Result<()>;
    async fn batch_insert_portfolio_holdings(&self, holdings: &[PortfolioHoldingInsert]) -> Result<()>;
    
    // 投資組合表現聚合操作
    async fn get_portfolio_performance(&self, portfolio_id: i32, time_range: TimeRange) -> Result<Vec<PortfolioPerformance>>;
}

/// PostgreSQL 投資組合儲存庫實現
pub struct PgPortfolioRepository {
    pool: PgPool,
}

impl PgPortfolioRepository {
    /// 創建新的 PostgreSQL 投資組合儲存庫實例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgPortfolioRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl PortfolioRepository for PgPortfolioRepository {
    async fn get_portfolio_by_id(&self, portfolio_id: i32) -> Result<Option<Portfolio>> {
        let portfolio = sqlx::query_as::<_, Portfolio>(
            "SELECT * FROM portfolio WHERE portfolio_id = $1"
        )
        .bind(portfolio_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(portfolio)
    }

    async fn get_portfolio_by_name(&self, name: &str) -> Result<Option<Portfolio>> {
        let portfolio = sqlx::query_as::<_, Portfolio>(
            "SELECT * FROM portfolio WHERE name = $1"
        )
        .bind(name)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(portfolio)
    }

    async fn get_portfolios(&self, active_only: bool, page: PageQuery) -> Result<Page<Portfolio>> {
        let offset = (page.page - 1) * page.page_size;
        
        let (portfolios, total) = if active_only {
            let portfolios = sqlx::query_as::<_, Portfolio>(
                "SELECT * FROM portfolio
                 WHERE is_active = true
                 ORDER BY name
                 LIMIT $1 OFFSET $2"
            )
            .bind(page.page_size)
            .bind(offset)
            .fetch_all(self.get_pool())
            .await?;

            let total = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM portfolio WHERE is_active = true"
            )
            .fetch_one(self.get_pool())
            .await?;
            
            (portfolios, total)
        } else {
            let portfolios = sqlx::query_as::<_, Portfolio>(
                "SELECT * FROM portfolio
                 ORDER BY name
                 LIMIT $1 OFFSET $2"
            )
            .bind(page.page_size)
            .bind(offset)
            .fetch_all(self.get_pool())
            .await?;

            let total = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM portfolio"
            )
            .fetch_one(self.get_pool())
            .await?;
            
            (portfolios, total)
        };

        Ok(Page::new(portfolios, total, page.page, page.page_size))
    }

    async fn insert_portfolio(&self, portfolio: &PortfolioInsert) -> Result<i32> {
        let id = sqlx::query_scalar::<_, i32>(
            "INSERT INTO portfolio (
                name, description, initial_capital, currency, risk_tolerance,
                start_date, end_date, is_active, strategy_instance, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING portfolio_id"
        )
        .bind(&portfolio.name)
        .bind(&portfolio.description)
        .bind(&portfolio.initial_capital)
        .bind(&portfolio.currency)
        .bind(&portfolio.risk_tolerance)
        .bind(&portfolio.start_date)
        .bind(&portfolio.end_date)
        .bind(&portfolio.is_active)
        .bind(&portfolio.strategy_instance)
        .bind(&portfolio.metadata)
        .fetch_one(self.get_pool())
        .await?;

        Ok(id)
    }

    async fn update_portfolio(&self, portfolio: &Portfolio) -> Result<()> {
        sqlx::query(
            "UPDATE portfolio SET 
                name = $1, description = $2, initial_capital = $3, currency = $4,
                risk_tolerance = $5, start_date = $6, end_date = $7, is_active = $8,
                strategy_instance = $9, metadata = $10, updated_at = now()
             WHERE portfolio_id = $11"
        )
        .bind(&portfolio.name)
        .bind(&portfolio.description)
        .bind(&portfolio.initial_capital)
        .bind(&portfolio.currency)
        .bind(&portfolio.risk_tolerance)
        .bind(&portfolio.start_date)
        .bind(&portfolio.end_date)
        .bind(&portfolio.is_active)
        .bind(&portfolio.strategy_instance)
        .bind(&portfolio.metadata)
        .bind(portfolio.portfolio_id)
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn delete_portfolio(&self, portfolio_id: i32) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM portfolio WHERE portfolio_id = $1"
        )
        .bind(portfolio_id)
        .execute(self.get_pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_portfolio_holdings(&self, portfolio_id: i32, time: Option<DateTime<Utc>>) -> Result<Vec<PortfolioHolding>> {
        let holdings = match time {
            Some(time) => {
                sqlx::query_as::<_, PortfolioHolding>(
                    "SELECT * FROM portfolio_holding
                     WHERE portfolio_id = $1 AND time <= $2
                     ORDER BY instrument_id, time DESC"
                )
                .bind(portfolio_id)
                .bind(time)
                .fetch_all(self.get_pool())
                .await?
            },
            None => {
                // 獲取最新持倉
                sqlx::query_as::<_, PortfolioHolding>(
                    "WITH latest_times AS (
                        SELECT instrument_id, MAX(time) as latest_time
                        FROM portfolio_holding
                        WHERE portfolio_id = $1
                        GROUP BY instrument_id
                     )
                     SELECT ph.*
                     FROM portfolio_holding ph
                     JOIN latest_times lt ON ph.instrument_id = lt.instrument_id AND ph.time = lt.latest_time
                     WHERE ph.portfolio_id = $1
                     ORDER BY ph.instrument_id"
                )
                .bind(portfolio_id)
                .fetch_all(self.get_pool())
                .await?
            }
        };

        Ok(holdings)
    }

    async fn get_portfolio_holding_history(&self, portfolio_id: i32, instrument_id: i32, time_range: TimeRange) -> Result<Vec<PortfolioHolding>> {
        let holdings = sqlx::query_as::<_, PortfolioHolding>(
            "SELECT * FROM portfolio_holding
             WHERE portfolio_id = $1 AND instrument_id = $2 AND time >= $3 AND time <= $4
             ORDER BY time DESC"
        )
        .bind(portfolio_id)
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(self.get_pool())
        .await?;

        Ok(holdings)
    }

    async fn insert_portfolio_holding(&self, holding: &PortfolioHoldingInsert) -> Result<()> {
        sqlx::query(
            "INSERT INTO portfolio_holding (
                time, portfolio_id, instrument_id, quantity, cost_basis,
                market_value, profit_loss, allocation_percentage
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(holding.time)
        .bind(holding.portfolio_id)
        .bind(holding.instrument_id)
        .bind(&holding.quantity)
        .bind(&holding.cost_basis)
        .bind(&holding.market_value)
        .bind(&holding.profit_loss)
        .bind(&holding.allocation_percentage)
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn batch_insert_portfolio_holdings(&self, holdings: &[PortfolioHoldingInsert]) -> Result<()> {
        if holdings.is_empty() {
            return Ok(());
        }

        let mut tx = self.get_pool().begin().await?;
        
        for holding in holdings {
            sqlx::query(
                "INSERT INTO portfolio_holding (
                    time, portfolio_id, instrument_id, quantity, cost_basis,
                    market_value, profit_loss, allocation_percentage
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
            )
            .bind(holding.time)
            .bind(holding.portfolio_id)
            .bind(holding.instrument_id)
            .bind(&holding.quantity)
            .bind(&holding.cost_basis)
            .bind(&holding.market_value)
            .bind(&holding.profit_loss)
            .bind(&holding.allocation_percentage)
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    async fn get_portfolio_performance(&self, portfolio_id: i32, time_range: TimeRange) -> Result<Vec<PortfolioPerformance>> {
        let performance = sqlx::query_as::<_, PortfolioPerformance>(
            "SELECT * FROM portfolio_performance
             WHERE portfolio_id = $1 AND bucket >= $2 AND bucket <= $3
             ORDER BY bucket DESC"
        )
        .bind(portfolio_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(self.get_pool())
        .await?;

        Ok(performance)
    }
} 