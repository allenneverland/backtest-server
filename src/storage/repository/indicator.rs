use crate::storage::models::*;
use crate::storage::repository::{DbExecutor, Page, PageQuery, TimeRange};
use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

/// 技術指標儲存庫特徵
pub trait IndicatorRepository: Send + Sync {
    /// 創建技術指標定義
    async fn create_technical_indicator(&self, indicator: TechnicalIndicatorInsert) -> Result<TechnicalIndicator>;

    /// 根據ID獲取技術指標定義
    async fn get_technical_indicator(&self, indicator_id: i32) -> Result<Option<TechnicalIndicator>>;

    /// 根據代碼獲取技術指標定義
    async fn get_technical_indicator_by_code(&self, code: &str) -> Result<Option<TechnicalIndicator>>;

    /// 獲取所有技術指標定義
    async fn list_technical_indicators(&self) -> Result<Vec<TechnicalIndicator>>;

    /// 添加商品日級指標數據
    async fn add_instrument_daily_indicator(&self, indicator: InstrumentDailyIndicatorInsert) -> Result<()>;
    
    /// 批量添加商品日級指標數據
    async fn add_instrument_daily_indicators(&self, indicators: Vec<InstrumentDailyIndicatorInsert>) -> Result<()>;

    /// 獲取商品日級指標數據
    async fn get_instrument_daily_indicators(
        &self, 
        instrument_id: i32, 
        indicator_id: i32, 
        time_range: TimeRange,
        page: PageQuery
    ) -> Result<Page<InstrumentDailyIndicator>>;
}

/// PostgreSQL 技術指標儲存庫實現
pub struct PgIndicatorRepository {
    pool: Arc<PgPool>,
}

impl PgIndicatorRepository {
    /// 創建新的技術指標儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgIndicatorRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

impl IndicatorRepository for PgIndicatorRepository {
    async fn create_technical_indicator(&self, indicator: TechnicalIndicatorInsert) -> Result<TechnicalIndicator> {
        let result = sqlx::query_as!(
            TechnicalIndicator,
            r#"
            INSERT INTO technical_indicator (
                code, name, description, parameters
            ) VALUES (
                $1, $2, $3, $4
            )
            RETURNING 
                indicator_id, code, name, description, 
                parameters as "parameters!: _", created_at, updated_at
            "#,
            indicator.code,
            indicator.name,
            indicator.description,
            indicator.parameters as _
        )
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_technical_indicator(&self, indicator_id: i32) -> Result<Option<TechnicalIndicator>> {
        let result = sqlx::query_as!(
            TechnicalIndicator,
            r#"
            SELECT 
                indicator_id, code, name, description, 
                parameters as "parameters!: _", created_at, updated_at
            FROM technical_indicator
            WHERE indicator_id = $1
            "#,
            indicator_id
        )
        .fetch_optional(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn get_technical_indicator_by_code(&self, code: &str) -> Result<Option<TechnicalIndicator>> {
        let result = sqlx::query_as!(
            TechnicalIndicator,
            r#"
            SELECT 
                indicator_id, code, name, description, 
                parameters as "parameters!: _", created_at, updated_at
            FROM technical_indicator
            WHERE code = $1
            "#,
            code
        )
        .fetch_optional(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn list_technical_indicators(&self) -> Result<Vec<TechnicalIndicator>> {
        let results = sqlx::query_as!(
            TechnicalIndicator,
            r#"
            SELECT 
                indicator_id, code, name, description, 
                parameters as "parameters!: _", created_at, updated_at
            FROM technical_indicator
            ORDER BY code
            "#
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }

    async fn add_instrument_daily_indicator(&self, indicator: InstrumentDailyIndicatorInsert) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO instrument_daily_indicator (
                time, instrument_id, indicator_id, parameters, values
            ) VALUES (
                $1, $2, $3, $4, $5
            )
            "#,
            indicator.time,
            indicator.instrument_id,
            indicator.indicator_id,
            indicator.parameters as _,
            indicator.values as _
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(())
    }

    async fn add_instrument_daily_indicators(&self, indicators: Vec<InstrumentDailyIndicatorInsert>) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;
        
        for indicator in indicators {
            sqlx::query!(
                r#"
                INSERT INTO instrument_daily_indicator (
                    time, instrument_id, indicator_id, parameters, values
                ) VALUES (
                    $1, $2, $3, $4, $5
                )
                "#,
                indicator.time,
                indicator.instrument_id,
                indicator.indicator_id,
                indicator.parameters as _,
                indicator.values as _
            )
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    async fn get_instrument_daily_indicators(
        &self, 
        instrument_id: i32, 
        indicator_id: i32, 
        time_range: TimeRange,
        page: PageQuery
    ) -> Result<Page<InstrumentDailyIndicator>> {
        let offset = (page.page - 1) * page.page_size;
        
        let indicators = sqlx::query_as!(
            InstrumentDailyIndicator,
            r#"
            SELECT 
                time, instrument_id, indicator_id, 
                parameters as "parameters!: _", values as "values!: _", created_at
            FROM instrument_daily_indicator
            WHERE instrument_id = $1
            AND indicator_id = $2
            AND time BETWEEN $3 AND $4
            ORDER BY time DESC
            LIMIT $5 OFFSET $6
            "#,
            instrument_id,
            indicator_id,
            time_range.start,
            time_range.end,
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM instrument_daily_indicator WHERE instrument_id = $1 AND indicator_id = $2 AND time BETWEEN $3 AND $4"
        )
        .bind(instrument_id)
        .bind(indicator_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(indicators, total, page.page, page.page_size))
    }
} 