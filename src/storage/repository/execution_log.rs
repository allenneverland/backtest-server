use crate::storage::models::execution_log::*;
use crate::storage::repository::{DbExecutor, Page, PageQuery};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;

/// 執行日誌儲存庫特徵
#[async_trait]
pub trait ExecutionLogRepository: Send + Sync {
    /// 添加執行日誌
    async fn add_execution_log(&self, log: ExecutionLogInsert) -> Result<ExecutionLog>;

    /// 批量添加執行日誌
    async fn add_execution_logs(&self, logs: Vec<ExecutionLogInsert>) -> Result<()>;

    /// 根據執行ID獲取日誌
    async fn get_execution_logs(
        &self,
        run_id: i32,
        filter: ExecutionLogFilter,
        page: PageQuery,
    ) -> Result<Page<ExecutionLog>>;

    /// 根據日誌級別獲取日誌
    async fn get_execution_logs_by_level(
        &self,
        run_id: i32,
        log_levels: Vec<String>,
        page: PageQuery,
    ) -> Result<Page<ExecutionLog>>;

    /// 獲取最近的錯誤日誌
    async fn get_recent_error_logs(&self, run_id: i32, limit: i64) -> Result<Vec<ExecutionLog>>;

    /// 清理舊的執行日誌
    async fn cleanup_old_logs(&self, days: i32) -> Result<u64>;

    /// 獲取日誌統計信息
    async fn get_log_stats(&self, run_id: i32) -> Result<LogStats>;
}

/// 日誌統計信息
#[derive(Debug, Clone)]
pub struct LogStats {
    pub total_logs: i64,
    pub debug_count: i64,
    pub info_count: i64,
    pub warn_count: i64,
    pub error_count: i64,
}

/// PostgreSQL 執行日誌儲存庫實現
pub struct PgExecutionLogRepository {
    pool: Arc<PgPool>,
}

impl PgExecutionLogRepository {
    /// 創建新的執行日誌儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgExecutionLogRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl ExecutionLogRepository for PgExecutionLogRepository {
    async fn add_execution_log(&self, log: ExecutionLogInsert) -> Result<ExecutionLog> {
        let result = sqlx::query_as!(
            ExecutionLog,
            r#"
            INSERT INTO execution_logs (
                run_id, timestamp, log_level, component, message, details
            ) VALUES (
                $1, $2, $3, $4, $5, $6
            )
            RETURNING 
                log_id, run_id, timestamp, log_level, component, message, 
                details as "details: _", created_at
            "#,
            log.run_id,
            log.timestamp.unwrap_or_else(Utc::now),
            log.log_level,
            log.component,
            log.message,
            log.details as _,
        )
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(result)
    }

    async fn add_execution_logs(&self, logs: Vec<ExecutionLogInsert>) -> Result<()> {
        let mut tx = DbExecutor::get_pool(self).begin().await?;

        for log in logs {
            sqlx::query!(
                r#"
                INSERT INTO execution_logs (
                    run_id, timestamp, log_level, component, message, details
                ) VALUES (
                    $1, $2, $3, $4, $5, $6
                )
                "#,
                log.run_id,
                log.timestamp.unwrap_or_else(Utc::now),
                log.log_level,
                log.component,
                log.message,
                log.details as _,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_execution_logs(
        &self,
        run_id: i32,
        filter: ExecutionLogFilter,
        page: PageQuery,
    ) -> Result<Page<ExecutionLog>> {
        let offset = (page.page - 1) * page.page_size;
        let limit = filter.limit.unwrap_or(page.page_size);

        // Simple implementation without dynamic query building for now
        let logs = if let Some(ref log_levels) = filter.log_levels {
            sqlx::query_as!(
                ExecutionLog,
                r#"
                SELECT 
                    log_id, run_id, timestamp, log_level, component, message, 
                    details as "details: _", created_at
                FROM execution_logs
                WHERE run_id = $1
                AND log_level = ANY($2)
                ORDER BY timestamp DESC
                LIMIT $3 OFFSET $4
                "#,
                run_id,
                &log_levels as &[String],
                limit,
                offset
            )
            .fetch_all(DbExecutor::get_pool(self))
            .await?
        } else {
            sqlx::query_as!(
                ExecutionLog,
                r#"
                SELECT 
                    log_id, run_id, timestamp, log_level, component, message, 
                    details as "details: _", created_at
                FROM execution_logs
                WHERE run_id = $1
                ORDER BY timestamp DESC
                LIMIT $2 OFFSET $3
                "#,
                run_id,
                limit,
                offset
            )
            .fetch_all(DbExecutor::get_pool(self))
            .await?
        };

        // Count total for pagination
        let total = if let Some(ref log_levels) = filter.log_levels {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM execution_logs WHERE run_id = $1 AND log_level = ANY($2)",
                run_id,
                &log_levels as &[String]
            )
            .fetch_one(DbExecutor::get_pool(self))
            .await?
            .unwrap_or(0)
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM execution_logs WHERE run_id = $1",
                run_id
            )
            .fetch_one(DbExecutor::get_pool(self))
            .await?
            .unwrap_or(0)
        };

        Ok(Page::new(logs, total, page.page, page.page_size))
    }

    async fn get_execution_logs_by_level(
        &self,
        run_id: i32,
        log_levels: Vec<String>,
        page: PageQuery,
    ) -> Result<Page<ExecutionLog>> {
        let offset = (page.page - 1) * page.page_size;

        let logs = sqlx::query_as!(
            ExecutionLog,
            r#"
            SELECT 
                log_id, run_id, timestamp, log_level, component, message, 
                details as "details: _", created_at
            FROM execution_logs
            WHERE run_id = $1
            AND log_level = ANY($2)
            ORDER BY timestamp DESC
            LIMIT $3 OFFSET $4
            "#,
            run_id,
            &log_levels as &[String],
            page.page_size,
            offset
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM execution_logs WHERE run_id = $1 AND log_level = ANY($2)",
        )
        .bind(run_id)
        .bind(&log_levels as &[String])
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(Page::new(logs, total, page.page, page.page_size))
    }

    async fn get_recent_error_logs(&self, run_id: i32, limit: i64) -> Result<Vec<ExecutionLog>> {
        let logs = sqlx::query_as!(
            ExecutionLog,
            r#"
            SELECT 
                log_id, run_id, timestamp, log_level, component, message, 
                details as "details: _", created_at
            FROM execution_logs
            WHERE run_id = $1
            AND log_level IN ('WARN', 'ERROR')
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
            run_id,
            limit
        )
        .fetch_all(DbExecutor::get_pool(self))
        .await?;

        Ok(logs)
    }

    async fn cleanup_old_logs(&self, days: i32) -> Result<u64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(days as i64);

        let result = sqlx::query!(
            "DELETE FROM execution_logs WHERE created_at < $1",
            cutoff_date
        )
        .execute(DbExecutor::get_pool(self))
        .await?;

        Ok(result.rows_affected())
    }

    async fn get_log_stats(&self, run_id: i32) -> Result<LogStats> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_logs,
                COUNT(CASE WHEN log_level = 'DEBUG' THEN 1 END) as debug_count,
                COUNT(CASE WHEN log_level = 'INFO' THEN 1 END) as info_count,
                COUNT(CASE WHEN log_level = 'WARN' THEN 1 END) as warn_count,
                COUNT(CASE WHEN log_level = 'ERROR' THEN 1 END) as error_count
            FROM execution_logs
            WHERE run_id = $1
            "#,
            run_id
        )
        .fetch_one(DbExecutor::get_pool(self))
        .await?;

        Ok(LogStats {
            total_logs: stats.total_logs.unwrap_or(0),
            debug_count: stats.debug_count.unwrap_or(0),
            info_count: stats.info_count.unwrap_or(0),
            warn_count: stats.warn_count.unwrap_or(0),
            error_count: stats.error_count.unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::create_test_pool;
    use sqlx::types::Json;

    #[tokio::test]
    async fn test_execution_log_crud() {
        let pool = create_test_pool().await;
        let repo = PgExecutionLogRepository::new(Arc::new(pool));

        // Create test log
        let log_insert = ExecutionLogInsert {
            run_id: 1,
            timestamp: Some(Utc::now()),
            log_level: LogLevel::Info.as_str().to_string(),
            component: Some("test_component".to_string()),
            message: "Test log message".to_string(),
            details: Some(Json(serde_json::json!({"key": "value"}))),
        };

        // Test create
        let created = repo.add_execution_log(log_insert).await.unwrap();
        assert_eq!(created.message, "Test log message");
        assert_eq!(created.log_level, LogLevel::Info.as_str());

        // Test get logs
        let filter = ExecutionLogFilter {
            run_id: Some(1),
            ..Default::default()
        };
        let page = PageQuery::new(1, 10);
        let logs_page = repo.get_execution_logs(1, filter, page).await.unwrap();
        assert!(!logs_page.data.is_empty());

        // Test get log stats
        let stats = repo.get_log_stats(1).await.unwrap();
        assert!(stats.total_logs > 0);
    }
}
