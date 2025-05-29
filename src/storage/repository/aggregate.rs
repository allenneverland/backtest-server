use crate::storage::models::*;
use crate::storage::repository::{DbExecutor, TimeRange};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

/// 連續聚合視圖儲存庫特徵
#[async_trait]
pub trait AggregateRepository: Send + Sync {
    /// 獲取指定商品的日級成交量聚合數據
    async fn get_daily_volume_by_instrument(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<DailyVolumeByInstrument>>;

    /// 獲取多個商品的日級成交量聚合數據
    async fn get_daily_volume_by_instruments(
        &self,
        instrument_ids: &[i32],
        time_range: TimeRange,
    ) -> Result<Vec<DailyVolumeByInstrument>>;

    /// 獲取指定執行任務的日收益率聚合數據
    async fn get_execution_daily_returns(
        &self,
        run_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<ExecutionDailyReturns>>;

    /// 獲取多個執行任務的日收益率聚合數據（用於比較）
    async fn get_execution_daily_returns_multi(
        &self,
        run_ids: &[i32],
        time_range: TimeRange,
    ) -> Result<Vec<ExecutionDailyReturns>>;
}

/// PostgreSQL 連續聚合視圖儲存庫實現
pub struct PgAggregateRepository {
    pool: Arc<PgPool>,
}

impl PgAggregateRepository {
    /// 創建新的連續聚合視圖儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgAggregateRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl AggregateRepository for PgAggregateRepository {
    async fn get_daily_volume_by_instrument(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<DailyVolumeByInstrument>> {
        let results = sqlx::query_as::<_, DailyVolumeByInstrument>(
            r#"
            SELECT
                bucket, 
                instrument_id, 
                open,
                high,
                low,
                close,
                total_volume,
                total_amount,
                max_open_interest
            FROM daily_volume_by_instrument
            WHERE instrument_id = $1
            AND bucket BETWEEN $2 AND $3
            ORDER BY bucket
            "#,
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }

    async fn get_daily_volume_by_instruments(
        &self,
        instrument_ids: &[i32],
        time_range: TimeRange,
    ) -> Result<Vec<DailyVolumeByInstrument>> {
        let results = sqlx::query_as::<_, DailyVolumeByInstrument>(
            r#"
            SELECT
                bucket,
                instrument_id,
                open,
                high,
                low,
                close,
                total_volume,
                total_amount,
                max_open_interest
            FROM daily_volume_by_instrument
            WHERE instrument_id = ANY($1)
            AND bucket BETWEEN $2 AND $3
            ORDER BY instrument_id, bucket
            "#,
        )
        .bind(instrument_ids)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }

    async fn get_execution_daily_returns(
        &self,
        run_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<ExecutionDailyReturns>> {
        let results = sqlx::query_as!(
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

        Ok(results)
    }

    async fn get_execution_daily_returns_multi(
        &self,
        run_ids: &[i32],
        time_range: TimeRange,
    ) -> Result<Vec<ExecutionDailyReturns>> {
        let results = sqlx::query_as!(
            ExecutionDailyReturns,
            r#"
            SELECT
                bucket as "bucket!", 
                run_id as "run_id!", 
                daily_return as "daily_return!: _",
                end_of_day_value as "end_of_day_value!: _",
                end_of_day_equity as "end_of_day_equity!: _"
            FROM execution_daily_returns
            WHERE run_id = ANY($1)
            AND bucket BETWEEN $2 AND $3
            ORDER BY run_id, bucket
            "#,
            run_ids,
            time_range.start,
            time_range.end
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }
}
