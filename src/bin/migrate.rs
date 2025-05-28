use anyhow::{Context, Result};
use backtest_server::storage;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Parser)]
#[command(name = "migrate", about = "backtest-server 數據庫遷移工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 運行所有未應用的遷移
    Run {
        /// 目標資料庫 (market 或 backtest，預設為 both)
        #[arg(short, long, default_value = "both")]
        target: String,
    },

    /// 檢查遷移狀態
    Status {
        /// 目標資料庫 (market 或 backtest，預設為 both)
        #[arg(short, long, default_value = "both")]
        target: String,
    },
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

    // 執行命令
    match cli.command {
        Commands::Run { target } => {
            match target.as_str() {
                "market" => {
                    info!("開始運行市場數據資料庫遷移...");
                    let market_pool = storage::database::get_market_data_pool(true)
                        .await
                        .context("無法初始化市場數據資料庫連接池")?;
                    storage::run_migrations(market_pool)
                        .await
                        .context("市場數據資料庫遷移執行失敗")?;
                    info!("市場數據資料庫遷移完成！");
                }
                "backtest" => {
                    info!("開始運行回測資料庫遷移...");
                    let backtest_pool = storage::database::get_backtest_pool(true)
                        .await
                        .context("無法初始化回測資料庫連接池")?;
                    storage::run_migrations(backtest_pool)
                        .await
                        .context("回測資料庫遷移執行失敗")?;
                    info!("回測資料庫遷移完成！");
                }
                "both" => {
                    // 運行市場數據資料庫遷移
                    info!("開始運行市場數據資料庫遷移...");
                    let market_pool = storage::database::get_market_data_pool(true)
                        .await
                        .context("無法初始化市場數據資料庫連接池")?;
                    storage::run_migrations(market_pool)
                        .await
                        .context("市場數據資料庫遷移執行失敗")?;
                    info!("市場數據資料庫遷移完成！");

                    // 運行回測資料庫遷移
                    info!("開始運行回測資料庫遷移...");
                    let backtest_pool = storage::database::get_backtest_pool(true)
                        .await
                        .context("無法初始化回測資料庫連接池")?;
                    storage::run_migrations(backtest_pool)
                        .await
                        .context("回測資料庫遷移執行失敗")?;
                    info!("回測資料庫遷移完成！");
                }
                _ => {
                    anyhow::bail!("無效的目標資料庫：{}。請使用 'market'、'backtest' 或 'both'", target);
                }
            }
        }
        Commands::Status { target } => {
            match target.as_str() {
                "market" => {
                    info!("檢查市場數據資料庫遷移狀態...");
                    // TODO: 實作狀態檢查
                }
                "backtest" => {
                    info!("檢查回測資料庫遷移狀態...");
                    // TODO: 實作狀態檢查
                }
                "both" => {
                    info!("檢查兩個資料庫的遷移狀態...");
                    // TODO: 實作狀態檢查
                }
                _ => {
                    anyhow::bail!("無效的目標資料庫：{}。請使用 'market'、'backtest' 或 'both'", target);
                }
            }
        }
    }

    Ok(())
}