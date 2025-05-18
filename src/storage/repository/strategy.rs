use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use chrono::Utc;

use crate::strategy::types::StrategyId;
use crate::strategy::config_watcher::{StrategyConfigFile, StrategyParameterConfig, ConfigUpdateError};
use crate::storage::repository::DbExecutor;

use crate::storage::models::strategy::DbStrategyConfig;

/// 策略配置資料庫儲存庫特性
#[async_trait]
pub trait StrategyConfigRepository: Send + Sync {
    /// 獲取所有策略配置
    async fn get_all_configs(&self) -> Result<Vec<StrategyConfigFile>, ConfigUpdateError>;
    
    /// 獲取單個策略配置
    async fn get_config(&self, id: &StrategyId) -> Result<Option<StrategyConfigFile>, ConfigUpdateError>;
    
    /// 保存策略配置
    async fn save_config(&self, config: &StrategyConfigFile) -> Result<(), ConfigUpdateError>;
    
    /// 刪除策略配置
    async fn delete_config(&self, id: &StrategyId) -> Result<bool, ConfigUpdateError>;
    
    /// 獲取上次更新時間後變更的所有配置
    async fn get_updated_configs(&self, since: u64) -> Result<Vec<StrategyConfigFile>, ConfigUpdateError>;
}

/// PostgreSQL 策略配置儲存庫實現
pub struct PgStrategyConfigRepository {
    pool: Arc<PgPool>,
}

impl PgStrategyConfigRepository {
    /// 建立新的 PostgreSQL 策略配置儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    
    /// 解析 JSON 參數
    fn parse_parameters(&self, json: serde_json::Value) -> Result<HashMap<String, StrategyParameterConfig>, ConfigUpdateError> {
        serde_json::from_value(json).map_err(|e| ConfigUpdateError::JsonParseError(e))
    }
    
    /// 解析 JSON 元數據
    fn parse_metadata(&self, json: serde_json::Value) -> Result<HashMap<String, String>, ConfigUpdateError> {
        serde_json::from_value(json).map_err(|e| ConfigUpdateError::JsonParseError(e))
    }
    
    /// 將資料庫結構轉換為配置文件
    fn db_to_config_file(&self, db_config: DbStrategyConfig) -> Result<StrategyConfigFile, ConfigUpdateError> {
        Ok(StrategyConfigFile {
            id: db_config.strategy_id,
            name: db_config.name,
            description: db_config.description,
            version: db_config.version,
            parameters: self.parse_parameters(db_config.parameters)?,
            code_path: db_config.code_path,
            enabled: db_config.enabled,
            author: db_config.author,
            tags: db_config.tags,
            dependencies: db_config.dependencies,
            metadata: self.parse_metadata(db_config.metadata)?,
            last_modified: Some(db_config.updated_at.timestamp() as u64),
            config_path: None,
        })
    }
}

impl DbExecutor for PgStrategyConfigRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl StrategyConfigRepository for PgStrategyConfigRepository {
    async fn get_all_configs(&self) -> Result<Vec<StrategyConfigFile>, ConfigUpdateError> {
        let db_configs = sqlx::query_as!(
            DbStrategyConfig,
            r#"
            SELECT 
                strategy_id, name, description, version, parameters, code_path, 
                enabled, author, tags, dependencies, metadata, updated_at
            FROM strategy_configs 
            WHERE enabled = true
            "#
        )
        .fetch_all(self.get_pool())
        .await
        .map_err(|e| ConfigUpdateError::Other(format!("數據庫錯誤: {}", e)))?;
        
        let mut configs = Vec::with_capacity(db_configs.len());
        for db_config in db_configs {
            configs.push(self.db_to_config_file(db_config)?);
        }
        
        Ok(configs)
    }
    
    async fn get_config(&self, id: &StrategyId) -> Result<Option<StrategyConfigFile>, ConfigUpdateError> {
        let db_config = sqlx::query_as!(
            DbStrategyConfig,
            r#"
            SELECT 
                strategy_id, name, description, version, parameters, code_path, 
                enabled, author, tags, dependencies, metadata, updated_at
            FROM strategy_configs 
            WHERE strategy_id = $1
            "#,
            id.as_str()
        )
        .fetch_optional(self.get_pool())
        .await
        .map_err(|e| ConfigUpdateError::Other(format!("數據庫錯誤: {}", e)))?;
        
        match db_config {
            Some(db_config) => Ok(Some(self.db_to_config_file(db_config)?)),
            None => Ok(None),
        }
    }
    
    async fn save_config(&self, config: &StrategyConfigFile) -> Result<(), ConfigUpdateError> {
        sqlx::query!(
            r#"
            INSERT INTO strategy_configs 
            (strategy_id, name, description, version, parameters, code_path, enabled, author, tags, dependencies, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (strategy_id) 
            DO UPDATE SET
            name = $2, description = $3, version = $4, parameters = $5, code_path = $6, 
            enabled = $7, author = $8, tags = $9, dependencies = $10, metadata = $11
            "#,
            &config.id,
            &config.name,
            config.description.as_deref(),
            &config.version,
            serde_json::to_value(&config.parameters).map_err(|e| 
                ConfigUpdateError::JsonParseError(e))?,
            config.code_path.as_deref(),
            config.enabled,
            config.author.as_deref(),
            &config.tags,
            &config.dependencies,
            serde_json::to_value(&config.metadata).map_err(|e| 
                ConfigUpdateError::JsonParseError(e))?
        )
        .execute(self.get_pool())
        .await
        .map_err(|e| ConfigUpdateError::Other(format!("數據庫錯誤: {}", e)))?;
        
        Ok(())
    }
    
    async fn delete_config(&self, id: &StrategyId) -> Result<bool, ConfigUpdateError> {
        let result = sqlx::query!(
            "DELETE FROM strategy_configs WHERE strategy_id = $1",
            id.as_str()
        )
        .execute(self.get_pool())
        .await
        .map_err(|e| ConfigUpdateError::Other(format!("數據庫錯誤: {}", e)))?;
        
        Ok(result.rows_affected() > 0)
    }
    
    async fn get_updated_configs(&self, since: u64) -> Result<Vec<StrategyConfigFile>, ConfigUpdateError> {
        // 將 u64 轉換為 timestamp
        let since_time = chrono::DateTime::<Utc>::from_timestamp(since as i64, 0)
            .ok_or_else(|| ConfigUpdateError::Other("無效的時間戳".to_string()))?;
        
        let db_configs = sqlx::query_as!(
            DbStrategyConfig,
            r#"
            SELECT 
                strategy_id, name, description, version, parameters, code_path, 
                enabled, author, tags, dependencies, metadata, updated_at
            FROM strategy_configs 
            WHERE updated_at > $1 AND enabled = true
            "#,
            since_time
        )
        .fetch_all(self.get_pool())
        .await
        .map_err(|e| ConfigUpdateError::Other(format!("數據庫錯誤: {}", e)))?;
        
        let mut configs = Vec::with_capacity(db_configs.len());
        for db_config in db_configs {
            configs.push(self.db_to_config_file(db_config)?);
        }
        
        Ok(configs)
    }
} 