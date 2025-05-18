use anyhow::{Result, anyhow};
use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use serde_json::Value as JsonValue;

use crate::storage::models::Exchange;
use crate::storage::repository::{DbExecutor};

/// 交易所數據庫操作
pub struct ExchangeRepository {
    pool: PgPool,
}

impl ExchangeRepository {
    /// 創建新的交易所數據庫操作實例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 創建交易所
    pub async fn create(&self, exchange: ExchangeInsert) -> Result<Exchange> {
        let now = Utc::now();
        
        let result = sqlx::query!(
            r#"
            INSERT INTO exchange (
                code, name, country, timezone, operating_hours,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7
            )
            RETURNING 
                exchange_id
            "#,
            exchange.code,
            exchange.name,
            exchange.country,
            exchange.timezone,
            exchange.operating_hours,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        let id = result.exchange_id;
        
        // 重新獲取完整的交易所資訊
        self.get_by_id(id).await?.ok_or_else(|| anyhow!("Failed to retrieve newly created exchange"))
    }

    /// 根據ID獲取交易所
    pub async fn get_by_id(&self, exchange_id: i32) -> Result<Option<Exchange>> {
        let record = sqlx::query!(
            r#"
            SELECT 
                exchange_id, code, name, country, timezone,
                operating_hours as "operating_hours: serde_json::Value",
                created_at, updated_at
            FROM exchange
            WHERE exchange_id = $1
            "#,
            exchange_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(record.map(|r| Exchange {
            exchange_id: r.exchange_id,
            code: r.code,
            name: r.name,
            country: r.country,
            timezone: r.timezone,
            operating_hours: r.operating_hours,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// 根據代碼獲取交易所
    pub async fn get_by_code(&self, code: &str) -> Result<Option<Exchange>> {
        let record = sqlx::query!(
            r#"
            SELECT 
                exchange_id, code, name, country, timezone,
                operating_hours as "operating_hours: serde_json::Value",
                created_at, updated_at
            FROM exchange
            WHERE code = $1
            "#,
            code
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(record.map(|r| Exchange {
            exchange_id: r.exchange_id,
            code: r.code,
            name: r.name,
            country: r.country,
            timezone: r.timezone,
            operating_hours: r.operating_hours,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// 獲取所有交易所
    pub async fn get_all(&self) -> Result<Vec<Exchange>> {
        let records = sqlx::query!(
            r#"
            SELECT 
                exchange_id, code, name, country, timezone,
                operating_hours as "operating_hours: serde_json::Value",
                created_at, updated_at
            FROM exchange
            ORDER BY code
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(records.into_iter().map(|r| Exchange {
            exchange_id: r.exchange_id,
            code: r.code,
            name: r.name,
            country: r.country,
            timezone: r.timezone,
            operating_hours: r.operating_hours,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }

    /// 更新交易所
    pub async fn update(&self, exchange_id: i32, exchange: ExchangeInsert) -> Result<Exchange> {
        let now = Utc::now();
        
        let _result = sqlx::query!(
            r#"
            UPDATE exchange
            SET 
                code = $1,
                name = $2,
                country = $3, 
                timezone = $4,
                operating_hours = $5,
                updated_at = $6
            WHERE exchange_id = $7
            "#,
            exchange.code,
            exchange.name,
            exchange.country,
            exchange.timezone,
            exchange.operating_hours,
            now,
            exchange_id
        )
        .execute(&self.pool)
        .await?;

        // 重新獲取更新後的交易所資訊
        self.get_by_id(exchange_id).await?.ok_or_else(|| anyhow!("Exchange not found after update"))
    }

    /// 刪除交易所
    pub async fn delete(&self, exchange_id: i32) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            DELETE FROM exchange
            WHERE exchange_id = $1
            "#,
            exchange_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
    
    /// 在事務中創建交易所
    pub async fn create_in_tx<'a>(
        &self,
        tx: &mut Transaction<'a, Postgres>,
        exchange: ExchangeInsert,
    ) -> Result<Exchange> {
        let now = Utc::now();
        
        let result = sqlx::query!(
            r#"
            INSERT INTO exchange (
                code, name, country, timezone, operating_hours,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7
            )
            RETURNING 
                exchange_id, code, name, country, timezone,
                operating_hours as "operating_hours: serde_json::Value",
                created_at, updated_at
            "#,
            exchange.code,
            exchange.name,
            exchange.country,
            exchange.timezone,
            exchange.operating_hours,
            now,
            now
        )
        .fetch_one(&mut **tx)
        .await?;

        Ok(Exchange {
            exchange_id: result.exchange_id,
            code: result.code,
            name: result.name,
            country: result.country,
            timezone: result.timezone,
            operating_hours: result.operating_hours,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }
}

impl DbExecutor for ExchangeRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

/// 交易所插入模型
#[derive(Debug, Clone)]
pub struct ExchangeInsert {
    pub code: String,
    pub name: String,
    pub country: String,
    pub timezone: String,
    pub operating_hours: Option<JsonValue>,
} 