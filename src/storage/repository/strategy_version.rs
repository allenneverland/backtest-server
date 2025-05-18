use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::types::Json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use crate::storage::models::strategy_version::*;

/// 策略版本倉庫特性
#[async_trait]
pub trait StrategyVersionRepository: Send + Sync {
    /// 創建新的策略版本
    async fn create_version(&self, version: &StrategyVersion) -> Result<StrategyVersion, VersionError>;
    
    /// 獲取指定策略版本
    async fn get_version_by_id(&self, version_id: i32) -> Result<Option<StrategyVersion>, VersionError>;
    
    /// 獲取指定策略的所有版本
    async fn get_versions_by_strategy_id(&self, strategy_id: &str) -> Result<Vec<StrategyVersion>, VersionError>;
    
    /// 獲取指定策略的最新版本
    async fn get_latest_version(&self, strategy_id: &str) -> Result<Option<StrategyVersion>, VersionError>;
    
    /// 獲取指定策略的最新穩定版本
    async fn get_latest_stable_version(&self, strategy_id: &str) -> Result<Option<StrategyVersion>, VersionError>;
    
    /// 更新策略版本
    async fn update_version(&self, version_id: i32, is_stable: bool, metadata: Option<Json<HashMap<String, String>>>) -> Result<StrategyVersion, VersionError>;
    
    /// 刪除策略版本
    async fn delete_version(&self, version_id: i32) -> Result<bool, VersionError>;
    
    /// 比較兩個版本
    async fn compare_versions(&self, version1: &str, version2: &str) -> Result<VersionCompareResult, VersionError>;
}

/// PostgreSQL 實現的策略版本儲存庫
pub struct PgStrategyVersionRepository {
    /// 資料庫連接池
    pool: Arc<PgPool>,
}

impl PgStrategyVersionRepository {
    /// 創建新的 PostgreSQL 策略版本儲存庫
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    // 將版本字符串轉換為可比較的數字陣列
    fn parse_version(&self, version: &str) -> Result<Vec<i32>, VersionError> {
        let version = version.trim_start_matches('v');
        let parts: Vec<&str> = version.split('.').collect();
        
        if parts.len() != 3 {
            return Err(VersionError::InvalidVersionFormat(
                format!("版本號必須是三段式格式 (v1.2.3): {}", version)
            ));
        }
        
        let mut result = Vec::with_capacity(3);
        for part in parts {
            match part.parse::<i32>() {
                Ok(num) => result.push(num),
                Err(_) => return Err(VersionError::InvalidVersionFormat(
                    format!("版本號各段必須為數字: {}", version)
                )),
            }
        }
        
        Ok(result)
    }
}

#[async_trait]
impl StrategyVersionRepository for PgStrategyVersionRepository {
    async fn create_version(&self, version: &StrategyVersion) -> Result<StrategyVersion, VersionError> {
        let result = sqlx::query_as!(
            StrategyVersion,
            r#"
            INSERT INTO strategy_version (
                strategy_id, version, source_path, is_stable, 
                description, created_by, metadata, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9
            )
            RETURNING 
                version_id, strategy_id, version, source_path, is_stable, 
                description, created_by, metadata as "metadata: Json<HashMap<String, String>>", 
                created_at, updated_at
            "#,
            version.strategy_id,
            version.version,
            version.source_path,
            version.is_stable,
            version.description,
            version.created_by,
            version.metadata as _,
            version.created_at,
            version.updated_at
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| VersionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
    
    async fn get_version_by_id(&self, version_id: i32) -> Result<Option<StrategyVersion>, VersionError> {
        let result = sqlx::query_as!(
            StrategyVersion,
            r#"
            SELECT 
                version_id, strategy_id, version, source_path, is_stable, 
                description, created_by, metadata as "metadata: Json<HashMap<String, String>>", 
                created_at, updated_at
            FROM strategy_version
            WHERE version_id = $1
            "#,
            version_id
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| VersionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
    
    async fn get_versions_by_strategy_id(&self, strategy_id: &str) -> Result<Vec<StrategyVersion>, VersionError> {
        let result = sqlx::query_as!(
            StrategyVersion,
            r#"
            SELECT 
                version_id, strategy_id, version, source_path, is_stable, 
                description, created_by, metadata as "metadata: Json<HashMap<String, String>>", 
                created_at, updated_at
            FROM strategy_version
            WHERE strategy_id = $1
            ORDER BY created_at DESC
            "#,
            strategy_id.parse::<i32>().map_err(|_| VersionError::InvalidVersionFormat(format!("無效的策略ID格式: {}", strategy_id)))?
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| VersionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
    
    async fn get_latest_version(&self, strategy_id: &str) -> Result<Option<StrategyVersion>, VersionError> {
        let result = sqlx::query_as!(
            StrategyVersion,
            r#"
            SELECT 
                version_id, strategy_id, version, source_path, is_stable, 
                description, created_by, metadata as "metadata: Json<HashMap<String, String>>", 
                created_at, updated_at
            FROM strategy_version
            WHERE strategy_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            strategy_id.parse::<i32>().map_err(|_| VersionError::InvalidVersionFormat(format!("無效的策略ID格式: {}", strategy_id)))?
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| VersionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
    
    async fn get_latest_stable_version(&self, strategy_id: &str) -> Result<Option<StrategyVersion>, VersionError> {
        let result = sqlx::query_as!(
            StrategyVersion,
            r#"
            SELECT 
                version_id, strategy_id, version, source_path, is_stable, 
                description, created_by, metadata as "metadata: Json<HashMap<String, String>>", 
                created_at, updated_at
            FROM strategy_version
            WHERE strategy_id = $1 AND is_stable = true
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            strategy_id.parse::<i32>().map_err(|_| VersionError::InvalidVersionFormat(format!("無效的策略ID格式: {}", strategy_id)))?
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| VersionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
    
    async fn update_version(&self, version_id: i32, is_stable: bool, metadata: Option<Json<HashMap<String, String>>>) -> Result<StrategyVersion, VersionError> {
        let updated_at = Utc::now();
        
        let result = sqlx::query_as!(
            StrategyVersion,
            r#"
            UPDATE strategy_version
            SET 
                is_stable = $1,
                metadata = $2,
                updated_at = $3
            WHERE version_id = $4
            RETURNING 
                version_id, strategy_id, version, source_path, is_stable, 
                description, created_by, metadata as "metadata: Json<HashMap<String, String>>", 
                created_at, updated_at
            "#,
            is_stable,
            metadata as _,
            updated_at,
            version_id
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| VersionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
    
    async fn delete_version(&self, version_id: i32) -> Result<bool, VersionError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM strategy_version
            WHERE version_id = $1
            "#,
            version_id
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| VersionError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }
    
    async fn compare_versions(&self, version1: &str, version2: &str) -> Result<VersionCompareResult, VersionError> {
        let v1 = self.parse_version(version1)?;
        let v2 = self.parse_version(version2)?;
        
        for i in 0..3 {
            if v1[i] > v2[i] {
                return Ok(VersionCompareResult::Newer);
            } else if v1[i] < v2[i] {
                return Ok(VersionCompareResult::Older);
            }
        }
        
        Ok(VersionCompareResult::Equal)
    }
}