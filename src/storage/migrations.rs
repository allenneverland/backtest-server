use anyhow::Result;
use sqlx::{migrate::Migrator, PgPool};
use tracing::info;

// 靜態嵌入遷移目錄（此目錄應放在專案根目錄）
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// 執行數據庫遷移
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("開始執行數據庫遷移...");

    // 使用 sqlx::migrate!() 自動執行所有嵌入的 SQL 檔案
    MIGRATOR.run(pool).await?;
    info!("SQLx 遷移完成");
    Ok(())
}
