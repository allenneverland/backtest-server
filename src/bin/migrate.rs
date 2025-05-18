use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use backtest_server::storage;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Parser)]
#[command(name = "migrate", about = "BacktestServer 數據庫遷移工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 運行所有未應用的遷移
    Run,
    
    /// 檢查遷移狀態
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌系統
    tracing_subscriber::fmt()
        .with_env_filter("backtest_server=info")
        .with_span_events(FmtSpan::CLOSE)
        .init();
    
    // 解析命令行參數
    let cli = Cli::parse();
    
    // 初始化數據庫連接
    let db_pool = storage::database::get_db_pool(true).await.context("無法初始化數據庫連接池")?;
    
    // 執行命令
    match cli.command {
        Commands::Run => {
            info!("開始運行數據庫遷移...");
            storage::run_migrations(db_pool).await.context("遷移執行失敗")?;
            info!("遷移完成！");
        },
        Commands::Status => {
            info!("檢查遷移狀態...");
        },
    }
    
    Ok(())
} 