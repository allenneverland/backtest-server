use tracing::{info, error};
use tokio::signal;
use backtest_server::config;
use backtest_server::storage::{database, run_migrations, repository};
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use tracing::Level;
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use anyhow::{Result, anyhow};
use backtest_server::strategy::loader::DefaultStrategyLoader;
use backtest_server::strategy::registry::InMemoryRegistry;
use backtest_server::strategy::lifecycle::LifecycleManager;
use backtest_server::strategy::types::RollbackPolicy;
use backtest_server::strategy::snapshot::{DefaultSnapshotManager, SnapshotConfig, SnapshotStorage, CleanupPolicy};
use backtest_server::strategy::config_watcher::ConfigurationWatcher;
use backtest_server::strategy::version::VersionManagerConfig;
use backtest_server::api::rest::RestApi;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化配置
    let app_config = config::init_config()?;
    
    // 初始化日誌系統
    init_logging(&app_config.log)?;
    
    // 獲取全局資料庫連線池
    let db_pool = database::get_db_pool(true).await?;
    
    // 執行資料庫遷移
    run_migrations(db_pool).await?;
    
    // 創建策略目錄（如果不存在）
    let strategy_dir = PathBuf::from(&app_config.strategy.directory);
    if !strategy_dir.exists() {
        fs::create_dir_all(&strategy_dir)
            .map_err(|e| anyhow!("無法創建策略目錄: {}", e))?;
    }
    
    // 初始化策略子系統
    let strategy_config = app_config.strategy.clone();
    
    // 創建加載器和註冊表
    let strategy_loader = DefaultStrategyLoader::new(&strategy_dir);
    let registry = InMemoryRegistry::new();
    
    // 將它們包裝在Arc中以便共享
    let registry_arc = Arc::new(registry);
    let loader_arc = Arc::new(strategy_loader);
    
    // 創建快照配置和管理器
    let snapshot_config = SnapshotConfig {
        auto_snapshot_enabled: true,
        auto_snapshot_interval_secs: 300, // 5分鐘
        max_snapshots: 10,
        storage: SnapshotStorage::FileSystem {
            directory: strategy_dir.join("snapshots"),
        },
        compress_snapshots: true,
        encrypt_snapshots: false,
        rollback_policy: RollbackPolicy::RollbackOnError,
        checkpoint_interval_percent: 25.0,
        min_checkpoint_interval_secs: 60,
        compression_level: 3,
        cache_size_limit_mb: 100,
        filename_format: String::from("snapshot_{strategy_id}_{timestamp}_{type}"),
        save_calculation_cache: true,
        cleanup_policy: CleanupPolicy::KeepLatest(10),
        auto_recover_on_crash: true,
        max_concurrent_snapshots: 3,
    };
    let snapshot_manager = DefaultSnapshotManager::new(snapshot_config);
    
    // 創建生命週期管理器
    let mut lifecycle_manager = LifecycleManager::new(
        registry_arc.clone(),
        loader_arc.clone(),
    );
    
    // 配置生命週期管理器
    lifecycle_manager = lifecycle_manager.with_snapshot_manager(Arc::new(snapshot_manager));
    
    // 將生命週期管理器包裝在 Arc 中以便共享
    let lifecycle_arc = Arc::new(lifecycle_manager);
    
    // 創建資料庫策略配置儲存庫
    let strategy_repo = Arc::new(
        repository::PgStrategyConfigRepository::new(Arc::new(db_pool.clone()))
    );
    
    // 創建配置監視器（使用資料庫來源）
    let mut config_watcher = backtest_server::strategy::config_watcher::ConfigWatcher::new(
        strategy_repo,
        lifecycle_arc.clone(),
        loader_arc.clone(),
        strategy_config
    );
    
    // 啟動配置監視器
    info!("啟動策略配置監視器...");
    config_watcher.start().await.map_err(|e| anyhow!("無法啟動策略配置監視器: {}", e))?;
    info!("策略配置監視器已啟動，使用資料庫配置來源");
    
    // 初始化版本管理器和服務
    let version_repo = Arc::new(repository::strategy_version::PgStrategyVersionRepository::new(Arc::new(db_pool.clone())));
    let _version_manager = Arc::new(backtest_server::strategy::version::DefaultVersionManager::new(
        version_repo,
        lifecycle_arc.clone(),
        loader_arc.clone()
    ).with_config(VersionManagerConfig {
        root_directory: PathBuf::from("./strategies"),
        max_retained_versions: 5,
        auto_backup: true,
        validate_before_update: true,
        auto_rollback_on_failure: true,
        keep_backup_copies: true,
        max_backup_copies: 3,
        create_symlink_to_latest: true,
    }));

    // 初始化REST API
    let rest_api = RestApi::new(app_config.server.clone(), app_config.rest_api.clone());
    rest_api.start().await?;
    
    info!("伺服器初始化完成，等待請求...");
    info!("監聽端口: {}", app_config.server.port);
    
    // 等待關閉信號
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("接收到關閉信號，正在退出...");
            // 停止配置監視器
            config_watcher.stop().await.ok();
            Ok(())
        },
        Err(err) => {
            error!("無法監聽關閉信號: {}", err);
            Err(anyhow!("無法監聽關閉信號: {}", err))
        },
    }
}

// 初始化日誌系統
fn init_logging(log_config: &crate::config::LogConfig) -> Result<()> {
    let level = match log_config.level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO, // 默認為INFO
    };


    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_max_level(level)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow!("設置日誌系統失敗: {}", e))?;
    
    info!("日誌系統初始化完成");
    Ok(())
}
