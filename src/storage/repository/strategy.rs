use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use async_trait::async_trait;

use crate::storage::models::strategy::{Strategy, StrategyInsert};
use crate::storage::repository::{DbExecutor, Page, PageQuery};

/// 策略相關錯誤
#[derive(Debug, thiserror::Error)]
pub enum StrategyError {
    /// 資料庫錯誤
    #[error("資料庫錯誤: {0}")]
    DatabaseError(String),
    
    /// 策略不存在
    #[error("策略不存在: ID {0}")]
    StrategyNotFound(i32),
    
    /// 名稱衝突
    #[error("策略名稱已存在: {0}")]
    NameConflict(String),
    
    /// 其他錯誤
    #[error("其他錯誤: {0}")]
    Other(String),
}

/// 策略儲存庫特徵
#[async_trait]
pub trait StrategyRepository: Send + Sync {
    /// 創建新策略
    async fn create_strategy(&self, strategy: &StrategyInsert) -> Result<Strategy, StrategyError>;
    
    /// 根據ID獲取策略
    async fn get_strategy_by_id(&self, strategy_id: i32) -> Result<Option<Strategy>, StrategyError>;
    
    /// 根據名稱和版本獲取策略
    async fn get_strategy_by_name_version(&self, name: &str, version: &str) -> Result<Option<Strategy>, StrategyError>;
    
    /// 獲取策略列表
    async fn list_strategies(&self, page: PageQuery, active_only: bool) -> Result<Page<Strategy>, StrategyError>;
    
    /// 更新策略
    async fn update_strategy(&self, strategy_id: i32, strategy: &StrategyInsert) -> Result<Strategy, StrategyError>;
    
    /// 更新策略啟用狀態
    async fn update_strategy_active_status(&self, strategy_id: i32, active: bool) -> Result<Strategy, StrategyError>;
    
    /// 刪除策略
    async fn delete_strategy(&self, strategy_id: i32) -> Result<bool, StrategyError>;
    
    /// 根據標籤搜尋策略
    async fn search_strategies_by_tags(&self, tags: &[String], page: PageQuery) -> Result<Page<Strategy>, StrategyError>;
}

/// PostgreSQL 策略儲存庫實現
pub struct PgStrategyRepository {
    pool: Arc<PgPool>,
}

impl PgStrategyRepository {
    /// 創建新的策略儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgStrategyRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl StrategyRepository for PgStrategyRepository {
    async fn create_strategy(&self, strategy: &StrategyInsert) -> Result<Strategy, StrategyError> {
        // 檢查名稱和版本是否已存在
        let existing = sqlx::query!(
            r#"
            SELECT strategy_id FROM strategy
            WHERE name = $1 AND version = $2
            "#,
            strategy.name,
            strategy.version
        )
        .fetch_optional(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        if existing.is_some() {
            return Err(StrategyError::NameConflict(format!(
                "策略 '{}' 版本 '{}' 已存在",
                strategy.name, strategy.version
            )));
        }
        
        let now = Utc::now();
        
        let result = sqlx::query_as!(
            Strategy,
            r#"
            INSERT INTO strategy (
                name, description, version, code, code_path, 
                parameters, active, author, tags, dependencies, 
                metadata, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
            RETURNING 
                strategy_id, name, description, version, code, 
                code_path, parameters as "parameters!: _", active as "active!", 
                author, tags as "tags!: _", dependencies as "dependencies!: _", 
                metadata as "metadata!: _", created_at, updated_at
            "#,
            strategy.name,
            strategy.description,
            strategy.version,
            strategy.code,
            strategy.code_path,
            strategy.parameters as _,
            strategy.active,
            strategy.author,
            &strategy.tags as &[String],
            &strategy.dependencies as &[String],
            strategy.metadata as _,
            now,
            now
        )
        .fetch_one(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    async fn get_strategy_by_id(&self, strategy_id: i32) -> Result<Option<Strategy>, StrategyError> {
        let result = sqlx::query_as!(
            Strategy,
            r#"
            SELECT 
                strategy_id, name, description, version, code, 
                code_path, parameters as "parameters!: _", active as "active!", 
                author, tags as "tags!: _", dependencies as "dependencies!: _", 
                metadata as "metadata!: _", created_at, updated_at
            FROM strategy
            WHERE strategy_id = $1
            "#,
            strategy_id
        )
        .fetch_optional(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    async fn get_strategy_by_name_version(&self, name: &str, version: &str) -> Result<Option<Strategy>, StrategyError> {
        let result = sqlx::query_as!(
            Strategy,
            r#"
            SELECT 
                strategy_id, name, description, version, code, 
                code_path, parameters as "parameters!: _", active as "active!", 
                author, tags as "tags!: _", dependencies as "dependencies!: _", 
                metadata as "metadata!: _", created_at, updated_at
            FROM strategy
            WHERE name = $1 AND version = $2
            "#,
            name,
            version
        )
        .fetch_optional(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    async fn list_strategies(&self, page: PageQuery, active_only: bool) -> Result<Page<Strategy>, StrategyError> {
        let offset = (page.page - 1) * page.page_size;
        
        // 取得總數
        let total = if active_only {
            sqlx::query!(
                "SELECT COUNT(*) as count FROM strategy WHERE active = true"
            )
            .fetch_one(self.get_pool())
            .await
            .map_err(|e| StrategyError::DatabaseError(e.to_string()))?
            .count
            .unwrap_or(0) as i64
        } else {
            sqlx::query!(
                "SELECT COUNT(*) as count FROM strategy"
            )
            .fetch_one(self.get_pool())
            .await
            .map_err(|e| StrategyError::DatabaseError(e.to_string()))?
            .count
            .unwrap_or(0) as i64
        };
        
        // 如果沒有策略，直接返回空頁
        if total == 0 {
            return Ok(Page::empty(page.page, page.page_size));
        }
        
        // 取得策略列表
        let strategies = if active_only {
            sqlx::query_as!(
                Strategy,
                r#"
                SELECT 
                    strategy_id, name, description, version, code, 
                    code_path, parameters as "parameters!: _", active as "active!", 
                    author, tags as "tags!: _", dependencies as "dependencies!: _", 
                    metadata as "metadata!: _", created_at, updated_at
                FROM strategy
                WHERE active = true
                ORDER BY name, version DESC
                LIMIT $1 OFFSET $2
                "#,
                page.page_size,
                offset
            )
            .fetch_all(self.get_pool())
            .await
            .map_err(|e| StrategyError::DatabaseError(e.to_string()))?
        } else {
            sqlx::query_as!(
                Strategy,
                r#"
                SELECT 
                    strategy_id, name, description, version, code, 
                    code_path, parameters as "parameters!: _", active as "active!", 
                    author, tags as "tags!: _", dependencies as "dependencies!: _", 
                    metadata as "metadata!: _", created_at, updated_at
                FROM strategy
                ORDER BY name, version DESC
                LIMIT $1 OFFSET $2
                "#,
                page.page_size,
                offset
            )
            .fetch_all(self.get_pool())
            .await
            .map_err(|e| StrategyError::DatabaseError(e.to_string()))?
        };
        
        Ok(Page::new(strategies, total, page.page, page.page_size))
    }
    
    async fn update_strategy(&self, strategy_id: i32, strategy: &StrategyInsert) -> Result<Strategy, StrategyError> {
        // 檢查策略是否存在
        let existing = self.get_strategy_by_id(strategy_id).await?;
        
        if existing.is_none() {
            return Err(StrategyError::StrategyNotFound(strategy_id));
        }
        
        // 檢查名稱和版本是否與其他策略衝突
        let name_conflict = sqlx::query!(
            r#"
            SELECT strategy_id FROM strategy
            WHERE name = $1 AND version = $2 AND strategy_id != $3
            "#,
            strategy.name,
            strategy.version,
            strategy_id
        )
        .fetch_optional(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        if name_conflict.is_some() {
            return Err(StrategyError::NameConflict(format!(
                "策略 '{}' 版本 '{}' 已存在",
                strategy.name, strategy.version
            )));
        }
        
        let now = Utc::now();
        
        let result = sqlx::query_as!(
            Strategy,
            r#"
            UPDATE strategy SET
                name = $1,
                description = $2,
                version = $3,
                code = $4,
                code_path = $5,
                parameters = $6,
                active = $7,
                author = $8,
                tags = $9,
                dependencies = $10,
                metadata = $11,
                updated_at = $12
            WHERE strategy_id = $13
            RETURNING 
                strategy_id, name, description, version, code, 
                code_path, parameters as "parameters!: _", active as "active!", 
                author, tags as "tags!: _", dependencies as "dependencies!: _", 
                metadata as "metadata!: _", created_at, updated_at
            "#,
            strategy.name,
            strategy.description,
            strategy.version,
            strategy.code,
            strategy.code_path,
            strategy.parameters as _,
            strategy.active,
            strategy.author,
            &strategy.tags as &[String],
            &strategy.dependencies as &[String],
            strategy.metadata as _,
            now,
            strategy_id
        )
        .fetch_one(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    async fn update_strategy_active_status(&self, strategy_id: i32, active: bool) -> Result<Strategy, StrategyError> {
        // 檢查策略是否存在
        let existing = self.get_strategy_by_id(strategy_id).await?;
        
        if existing.is_none() {
            return Err(StrategyError::StrategyNotFound(strategy_id));
        }
        
        let now = Utc::now();
        
        let result = sqlx::query_as!(
            Strategy,
            r#"
            UPDATE strategy SET
                active = $1,
                updated_at = $2
            WHERE strategy_id = $3
            RETURNING 
                strategy_id, name, description, version, code, 
                code_path, parameters as "parameters!: _", active as "active!", 
                author, tags as "tags!: _", dependencies as "dependencies!: _", 
                metadata as "metadata!: _", created_at, updated_at
            "#,
            active,
            now,
            strategy_id
        )
        .fetch_one(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    async fn delete_strategy(&self, strategy_id: i32) -> Result<bool, StrategyError> {
        // 檢查策略是否存在
        let existing = self.get_strategy_by_id(strategy_id).await?;
        
        if existing.is_none() {
            return Err(StrategyError::StrategyNotFound(strategy_id));
        }
        
        let result = sqlx::query!(
            "DELETE FROM strategy WHERE strategy_id = $1",
            strategy_id
        )
        .execute(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        Ok(result.rows_affected() > 0)
    }
    
    async fn search_strategies_by_tags(&self, tags: &[String], page: PageQuery) -> Result<Page<Strategy>, StrategyError> {
        if tags.is_empty() {
            return self.list_strategies(page, false).await;
        }
        
        let offset = (page.page - 1) * page.page_size;
        
        // 取得總數
        let total = sqlx::query!(
            r#"
            SELECT COUNT(*) as count 
            FROM strategy
            WHERE tags && $1
            "#,
            tags
        )
        .fetch_one(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?
        .count
        .unwrap_or(0) as i64;
        
        // 如果沒有策略，直接返回空頁
        if total == 0 {
            return Ok(Page::empty(page.page, page.page_size));
        }
        
        // 取得策略列表
        let strategies = sqlx::query_as!(
            Strategy,
            r#"
            SELECT 
                strategy_id, name, description, version, code, 
                code_path, parameters as "parameters!: _", active as "active!", 
                author, tags as "tags!: _", dependencies as "dependencies!: _", 
                metadata as "metadata!: _", created_at, updated_at
            FROM strategy
            WHERE tags && $1
            ORDER BY name, version DESC
            LIMIT $2 OFFSET $3
            "#,
            tags,
            page.page_size,
            offset
        )
        .fetch_all(self.get_pool())
        .await
        .map_err(|e| StrategyError::DatabaseError(e.to_string()))?;
        
        Ok(Page::new(strategies, total, page.page, page.page_size))
    }
}
