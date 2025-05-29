use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use crate::storage::models::instrument_reference::InstrumentReference;

#[derive(Debug)]
pub struct InstrumentReferenceInsert {
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_code: String,
    pub instrument_type: String,
    pub name: String,
    pub currency: String,
    pub is_active: bool,
}

/// 金融商品參考資料倉庫（回測資料庫）
#[derive(Debug, Clone)]
pub struct InstrumentReferenceRepository {
    pool: PgPool,
}

impl InstrumentReferenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 創建金融商品參考
    pub async fn create(
        &self,
        instrument: InstrumentReferenceInsert,
    ) -> Result<InstrumentReference> {
        let now = Utc::now();

        let result = sqlx::query!(
            r#"
            INSERT INTO instrument_reference (
                instrument_id, symbol, exchange_code, instrument_type, name,
                currency, is_active, last_sync_at, sync_version,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            RETURNING 
                instrument_id
            "#,
            instrument.instrument_id,
            instrument.symbol,
            instrument.exchange_code,
            instrument.instrument_type,
            instrument.name,
            instrument.currency,
            instrument.is_active,
            now,
            1i64, // 初始同步版本
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        let id = result.instrument_id;

        // 重新獲取完整的金融商品參考資訊
        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch created instrument reference"))
    }

    /// 根據ID獲取金融商品參考
    pub async fn get_by_id(&self, instrument_id: i32) -> Result<Option<InstrumentReference>> {
        let record = sqlx::query_as!(
            InstrumentReference,
            r#"
            SELECT 
                instrument_id, symbol, exchange_code, instrument_type, name, 
                currency, is_active, last_sync_at, sync_version,
                created_at, updated_at
            FROM instrument_reference
            WHERE instrument_id = $1
            "#,
            instrument_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 根據符號和交易所獲取金融商品參考
    pub async fn get_by_symbol_and_exchange(
        &self,
        symbol: &str,
        exchange_code: &str,
    ) -> Result<Option<InstrumentReference>> {
        let record = sqlx::query_as!(
            InstrumentReference,
            r#"
            SELECT 
                instrument_id, symbol, exchange_code, instrument_type, name, 
                currency, is_active, last_sync_at, sync_version,
                created_at, updated_at
            FROM instrument_reference
            WHERE symbol = $1 AND exchange_code = $2
            "#,
            symbol,
            exchange_code
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 獲取活躍的金融商品參考列表
    pub async fn list_active(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<InstrumentReference>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let records = sqlx::query_as!(
            InstrumentReference,
            r#"
            SELECT 
                instrument_id, symbol, exchange_code, instrument_type, name, 
                currency, is_active, last_sync_at, sync_version,
                created_at, updated_at
            FROM instrument_reference
            WHERE is_active = true
            ORDER BY symbol, exchange_code
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 更新金融商品參考
    pub async fn update(
        &self,
        instrument_id: i32,
        updates: InstrumentReferenceInsert,
    ) -> Result<InstrumentReference> {
        let now = Utc::now();

        sqlx::query!(
            r#"
            UPDATE instrument_reference
            SET 
                symbol = $2,
                exchange_code = $3,
                instrument_type = $4,
                name = $5,
                currency = $6,
                is_active = $7,
                updated_at = $8
            WHERE instrument_id = $1
            "#,
            instrument_id,
            updates.symbol,
            updates.exchange_code,
            updates.instrument_type,
            updates.name,
            updates.currency,
            updates.is_active,
            now
        )
        .execute(&self.pool)
        .await?;

        self.get_by_id(instrument_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Instrument reference not found after update"))
    }

    /// 刪除金融商品參考
    pub async fn delete(&self, instrument_id: i32) -> Result<()> {
        sqlx::query!(
            "DELETE FROM instrument_reference WHERE instrument_id = $1",
            instrument_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 更新同步狀態
    pub async fn update_sync_status(&self, instrument_id: i32, sync_version: i64) -> Result<()> {
        let now = Utc::now();

        sqlx::query!(
            r#"
            UPDATE instrument_reference
            SET 
                last_sync_at = $2,
                sync_version = $3,
                updated_at = $2
            WHERE instrument_id = $1
            "#,
            instrument_id,
            now,
            sync_version
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn setup_test_db() -> PgPool {
        // InstrumentReferenceRepository 應該連接到回測資料庫
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            // 檢查是否在 Docker 環境中
            if std::path::Path::new("/.dockerenv").exists() {
                // 在 Docker 環境中，使用回測資料庫
                "postgresql://backtest_user:backtest_pass@backtest-db:5432/backtest".to_string()
            } else {
                // 在 CI 或本地測試環境中
                "postgresql://backtest_user:backtest_pass@localhost:5432/backtest".to_string()
            }
        });

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to backtest database")
    }

    #[sqlx::test]
    async fn test_create_and_get_instrument_reference() -> Result<()> {
        let pool = setup_test_db().await;
        let repo = InstrumentReferenceRepository::new(pool);

        // 創建金融商品參考
        let instrument = InstrumentReferenceInsert {
            instrument_id: 10001,
            symbol: "AAPL".to_string(),
            exchange_code: "NASDAQ".to_string(),
            instrument_type: "STOCK".to_string(),
            name: "Apple Inc.".to_string(),
            currency: "USD".to_string(),
            is_active: true,
        };

        // 創建
        let created = repo.create(instrument).await?;

        // 確認創建成功
        assert_eq!(created.symbol, "AAPL");
        assert_eq!(created.exchange_code, "NASDAQ");
        assert_eq!(created.instrument_type, "STOCK");
        assert_eq!(created.name, "Apple Inc.");
        assert_eq!(created.currency, "USD");
        assert!(created.is_active);
        assert_eq!(created.sync_version, 1);

        // 根據ID獲取
        let fetched = repo.get_by_id(created.instrument_id).await?;
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.symbol, "AAPL");
        assert_eq!(fetched.exchange_code, "NASDAQ");

        // 根據符號和交易所獲取
        let fetched_by_symbol = repo.get_by_symbol_and_exchange("AAPL", "NASDAQ").await?;
        assert!(fetched_by_symbol.is_some());
        let fetched_by_symbol = fetched_by_symbol.unwrap();
        assert_eq!(fetched_by_symbol.instrument_id, created.instrument_id);

        // 清理測試數據
        repo.delete(created.instrument_id).await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_active_instruments() -> Result<()> {
        let pool = setup_test_db().await;
        let repo = InstrumentReferenceRepository::new(pool);

        // 創建測試數據
        let instruments = vec![
            InstrumentReferenceInsert {
                instrument_id: 20001,
                symbol: "GOOGL".to_string(),
                exchange_code: "NASDAQ".to_string(),
                instrument_type: "STOCK".to_string(),
                name: "Alphabet Inc.".to_string(),
                currency: "USD".to_string(),
                is_active: true,
            },
            InstrumentReferenceInsert {
                instrument_id: 20002,
                symbol: "MSFT".to_string(),
                exchange_code: "NASDAQ".to_string(),
                instrument_type: "STOCK".to_string(),
                name: "Microsoft Corporation".to_string(),
                currency: "USD".to_string(),
                is_active: true,
            },
            InstrumentReferenceInsert {
                instrument_id: 20003,
                symbol: "INACTIVE".to_string(),
                exchange_code: "NASDAQ".to_string(),
                instrument_type: "STOCK".to_string(),
                name: "Inactive Stock".to_string(),
                currency: "USD".to_string(),
                is_active: false,
            },
        ];

        let mut created_ids = Vec::new();
        for instrument in instruments {
            let created = repo.create(instrument).await?;
            created_ids.push(created.instrument_id);
        }

        // 獲取活躍商品列表
        let active_list = repo.list_active(Some(10), Some(0)).await?;

        // 應該只有2個活躍商品
        assert!(active_list.len() >= 2);
        let our_instruments: Vec<_> = active_list
            .iter()
            .filter(|inst| created_ids.contains(&inst.instrument_id))
            .collect();
        assert_eq!(our_instruments.len(), 2);

        // 檢查排序（按符號和交易所）
        assert!(our_instruments[0].symbol <= our_instruments[1].symbol);

        // 清理測試數據
        for id in created_ids {
            repo.delete(id).await?;
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_instrument_reference() -> Result<()> {
        let pool = setup_test_db().await;
        let repo = InstrumentReferenceRepository::new(pool);

        // 創建金融商品參考
        let instrument = InstrumentReferenceInsert {
            instrument_id: 30001,
            symbol: "TEST".to_string(),
            exchange_code: "NYSE".to_string(),
            instrument_type: "STOCK".to_string(),
            name: "Test Company".to_string(),
            currency: "USD".to_string(),
            is_active: true,
        };

        let created = repo.create(instrument).await?;

        // 更新
        let updates = InstrumentReferenceInsert {
            instrument_id: 30001,
            symbol: "TEST".to_string(),
            exchange_code: "NYSE".to_string(),
            instrument_type: "STOCK".to_string(),
            name: "Updated Test Company".to_string(),
            currency: "EUR".to_string(),
            is_active: false,
        };

        let updated = repo.update(created.instrument_id, updates).await?;

        // 確認更新成功
        assert_eq!(updated.name, "Updated Test Company");
        assert_eq!(updated.currency, "EUR");
        assert!(!updated.is_active);
        assert!(updated.updated_at > created.updated_at);

        // 清理測試數據
        repo.delete(created.instrument_id).await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_sync_status() -> Result<()> {
        let pool = setup_test_db().await;
        let repo = InstrumentReferenceRepository::new(pool);

        // 創建金融商品參考
        let instrument = InstrumentReferenceInsert {
            instrument_id: 40001,
            symbol: "SYNC".to_string(),
            exchange_code: "NYSE".to_string(),
            instrument_type: "STOCK".to_string(),
            name: "Sync Test".to_string(),
            currency: "USD".to_string(),
            is_active: true,
        };

        let created = repo.create(instrument).await?;
        let original_sync_time = created.last_sync_at;

        // 等待一下以確保時間戳不同
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // 更新同步狀態
        repo.update_sync_status(created.instrument_id, 2).await?;

        // 獲取更新後的資料
        let updated = repo.get_by_id(created.instrument_id).await?.unwrap();

        // 確認同步狀態更新
        assert_eq!(updated.sync_version, 2);
        assert!(updated.last_sync_at > original_sync_time);

        // 清理測試數據
        repo.delete(created.instrument_id).await?;

        Ok(())
    }
}
