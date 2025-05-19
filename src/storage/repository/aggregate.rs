use crate::storage::models::*;
use crate::storage::repository::{DbExecutor, TimeRange};
use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use async_trait::async_trait;

/// 連續聚合視圖儲存庫特徵
#[async_trait]
pub trait AggregateRepository: Send + Sync {
    /// 獲取指定商品的日級成交量聚合數據
    async fn get_daily_volume_by_instrument(&self, instrument_id: i32, time_range: TimeRange) -> Result<Vec<DailyVolumeByInstrument>>;

    /// 獲取多個商品的日級成交量聚合數據
    async fn get_daily_volume_by_instruments(&self, instrument_ids: &[i32], time_range: TimeRange) -> Result<Vec<DailyVolumeByInstrument>>;

    /// 獲取指定回測結果的日收益率聚合數據
    async fn get_backtest_daily_returns(&self, result_id: i32, time_range: TimeRange) -> Result<Vec<BacktestDailyReturns>>;

    /// 獲取多個回測結果的日收益率聚合數據（用於比較）
    async fn get_backtest_daily_returns_multi(&self, result_ids: &[i32], time_range: TimeRange) -> Result<Vec<BacktestDailyReturns>>;
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
    async fn get_daily_volume_by_instrument(&self, instrument_id: i32, time_range: TimeRange) -> Result<Vec<DailyVolumeByInstrument>> {
        let results = sqlx::query_as!(
            DailyVolumeByInstrument,
            r#"
            SELECT
                bucket as "bucket!", 
                instrument_id as "instrument_id!", 
                open as "open!: _",
                high as "high!: _",
                low as "low!: _",
                close as "close!: _",
                total_volume as "total_volume!: _",
                total_amount as "total_amount: _",
                max_open_interest as "max_open_interest: _"
            FROM daily_volume_by_instrument
            WHERE instrument_id = $1
            AND bucket BETWEEN $2 AND $3
            ORDER BY bucket
            "#,
            instrument_id,
            time_range.start,
            time_range.end
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }

    async fn get_daily_volume_by_instruments(&self, instrument_ids: &[i32], time_range: TimeRange) -> Result<Vec<DailyVolumeByInstrument>> {
        let results = sqlx::query_as!(
            DailyVolumeByInstrument,
            r#"
            SELECT
                bucket as "bucket!",
                instrument_id as "instrument_id!", 
                open as "open!: _",
                high as "high!: _",
                low as "low!: _",
                close as "close!: _",
                total_volume as "total_volume!: _",
                total_amount as "total_amount: _",
                max_open_interest as "max_open_interest: _"
            FROM daily_volume_by_instrument
            WHERE instrument_id = ANY($1)
            AND bucket BETWEEN $2 AND $3
            ORDER BY instrument_id, bucket
            "#,
            instrument_ids,
            time_range.start,
            time_range.end
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }

    async fn get_backtest_daily_returns(&self, result_id: i32, time_range: TimeRange) -> Result<Vec<BacktestDailyReturns>> {
        let results = sqlx::query_as!(
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

        Ok(results)
    }

    async fn get_backtest_daily_returns_multi(&self, result_ids: &[i32], time_range: TimeRange) -> Result<Vec<BacktestDailyReturns>> {
        let results = sqlx::query_as!(
            BacktestDailyReturns,
            r#"
            SELECT
                bucket as "bucket!", 
                result_id as "result_id!", 
                daily_return as "daily_return!: _",
                end_of_day_value as "end_of_day_value!: _",
                end_of_day_equity as "end_of_day_equity!: _"
            FROM backtest_daily_returns
            WHERE result_id = ANY($1)
            AND bucket BETWEEN $2 AND $3
            ORDER BY result_id, bucket
            "#,
            result_ids,
            time_range.start,
            time_range.end
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(results)
    }
} 