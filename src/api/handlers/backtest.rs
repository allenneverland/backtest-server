use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::HashMap;

use axum::{
    extract::Multipart,
    response::IntoResponse,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tokio::fs as tokio_fs;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use serde_yaml_bw::{Value,from_str};
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use tokio::task;
use tracing;

use crate::strategy::{
    StrategyId, StrategyType, StrategyDefinition, 
    StrategyCode, BacktestProgress, config_watcher::StrategyConfigFile,
    config_watcher::StrategyParameterConfig
};
use crate::storage::{
    get_db_pool, 
    repository::PgStrategyConfigRepository,
    repository::strategy::StrategyConfigRepository,
};
use crate::runtime::sandbox::{Sandbox, SandboxBuilder, SandboxConfig};
use crate::runtime::resource::ResourceLimit;

// 回測任務相關結構
#[derive(Debug)]
pub struct BacktestTask {
    pub backtest_id: String,
    pub strategy_id: StrategyId,
    pub instruments: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub initial_capital: f64,
    pub parameters: std::collections::HashMap<String, String>,
    pub sandbox: Arc<Mutex<Sandbox>>,
    pub status: String,
}

// 回測任務管理器
#[derive(Clone, Debug)]
pub struct BacktestTaskManager {
    tasks: Arc<Mutex<std::collections::HashMap<String, Arc<BacktestTask>>>>,
}

// 實現回測任務管理器的單例模式
impl BacktestTaskManager {
    pub fn global() -> &'static Self {
        static INSTANCE: once_cell::sync::OnceCell<BacktestTaskManager> = once_cell::sync::OnceCell::new();
        INSTANCE.get_or_init(|| {
            BacktestTaskManager {
                tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }
        })
    }

    // 添加回測任務
    pub async fn add_task(&self, task: BacktestTask) -> Arc<BacktestTask> {
        let backtest_id = task.backtest_id.clone();
        let task = Arc::new(task);
        let mut tasks = self.tasks.lock().await;
        tasks.insert(backtest_id, task.clone());
        task
    }
    
    // 獲取回測任務
    pub async fn get_task(&self, backtest_id: &str) -> Option<Arc<BacktestTask>> {
        let tasks = self.tasks.lock().await;
        tasks.get(backtest_id).cloned()
    }
    
    // 更新任務狀態
    pub async fn update_task_status(&self, backtest_id: &str, status: &str) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks.get_mut(backtest_id).ok_or_else(|| anyhow!("回測任務不存在"))?;
        
        // 使用as_mut方法來獲得可變引用，但是Arc無法直接修改內部值
        // 所以我們這裡創建一個新的任務來替換原來的
        let mut_task = BacktestTask {
            backtest_id: task.backtest_id.clone(),
            strategy_id: task.strategy_id.clone(),
            instruments: task.instruments.clone(),
            start_time: task.start_time,
            end_time: task.end_time,
            initial_capital: task.initial_capital,
            parameters: task.parameters.clone(),
            sandbox: task.sandbox.clone(),
            status: status.to_string(),
        };
        
        *task = Arc::new(mut_task);
        Ok(())
    }
}

// 執行回測任務的函數
async fn execute_backtest(task: Arc<BacktestTask>) -> Result<()> {
    tracing::info!(
        "開始執行回測任務: id={}, 策略={}, 區間={}~{}",
        task.backtest_id, task.strategy_id, task.start_time, task.end_time
    );

    // 更新狀態為運行中
    BacktestTaskManager::global().update_task_status(&task.backtest_id, "running").await?;
    
    // 這裡應該實現實際的回測邏輯
    // 1. 初始化資料提供者
    // 2. 設置回測參數
    // 3. 執行策略
    // 4. 收集結果
    
    // TODO: 實際的回測執行邏輯
    // 由於我們不做模擬，這裡只實現架構，實際邏輯需要另外開發
    
    // 模擬回測過程，每10%更新一次進度
    for i in 1..=10 {
        // 等待一小段時間以模擬執行過程
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // 創建進度物件
        let progress = BacktestProgress {
            current_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            percent_complete: i as f64 * 10.0,
            last_processed_bar_id: Some(i * 1000),
            processing_stage: format!("處理數據 {}/10", i),
            processed_events: i * 5000,
            last_checkpoint_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            estimated_remaining_secs: Some(((10 - i) * 500) as u64),
        };
        
        // 序列化並存入沙箱
        let progress_json = serde_json::to_vec(&progress)?;
        let sandbox = task.sandbox.lock().await;
        sandbox.context().set_data("backtest_progress".to_string(), progress_json).await
            .map_err(|e| anyhow!("無法設置回測進度: {}", e))?;
            
        tracing::info!(
            "回測進度更新: id={}, 進度={:.1}%", 
            task.backtest_id, progress.percent_complete
        );
    }
    
    // 更新狀態為完成
    BacktestTaskManager::global().update_task_status(&task.backtest_id, "completed").await?;
    
    
    tracing::info!(
        "回測任務已完成: id={}, 策略={}", 
        task.backtest_id, task.strategy_id
    );
    
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct BacktestRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_id: Option<String>,
    pub instruments: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub initial_capital: f64,
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BacktestResponse {
    pub backtest_id: String,
    pub strategy_id: String,
    pub status: String,
    pub result_url: Option<String>,
}

#[derive(Debug)]
struct ExtractedStrategyInfo {
    name: String,
    description: Option<String>,
}

// 解析策略 DSL 文件，提取策略名稱和描述
fn extract_strategy_info(content: &[u8]) -> Result<ExtractedStrategyInfo> {
    let content_str = std::str::from_utf8(content)?;
    let yaml_value: Value = from_str(content_str)?;
    
    // 檢查是否存在 strategy 區塊
    let strategy = yaml_value.get("strategy")
        .ok_or_else(|| anyhow!("無效的策略 DSL 格式: 缺少 strategy 區塊"))?;
    
    // 提取策略名稱
    let name = strategy.get("name")
        .ok_or_else(|| anyhow!("無效的策略 DSL 格式: 缺少策略名稱"))?
        .as_str()
        .ok_or_else(|| anyhow!("策略名稱必須是字串類型"))?
        .to_string();
    
    // 提取描述（如果有）
    let description = strategy.get("description")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());
    
    Ok(ExtractedStrategyInfo { name, description })
}

// 建立策略資料夾並保存策略文件
async fn save_strategy_file(
    name: &str, 
    content: &[u8], 
    strategy_id: &StrategyId
) -> Result<PathBuf> {
    // 保存策略文件的根目錄
    let strategies_dir = Path::new("strategies");
    
    // 將策略名稱轉換為合法的資料夾名稱
    let folder_name = name.to_lowercase()
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();
    
    // 策略資料夾路徑
    let strategy_dir = strategies_dir.join(&folder_name);
    
    // 確保目錄存在
    tokio_fs::create_dir_all(&strategy_dir).await?;
    
    // 建立檔案名稱（使用策略 ID）
    let file_name = format!("{}.dsl", strategy_id.as_str());
    let file_path = strategy_dir.join(&file_name);
    
    // 寫入文件內容
    tokio_fs::write(&file_path, content).await?;
    
    Ok(file_path)
}

// 產生回測結果 HTML
async fn generate_backtest_html(
    backtest_id: &str,
    strategy_name: &str,
    instruments: &[String],
    start_time: &DateTime<Utc>,
    end_time: &DateTime<Utc>
) -> Result<PathBuf> {
    // 回測結果資料夾
    let results_dir = Path::new("results/backtests").join(backtest_id);
    
    // 確保目錄存在
    tokio_fs::create_dir_all(&results_dir).await?;
    
    // HTML 檔案路徑
    let html_path = results_dir.join("report.html");
    
    // 產生簡易的 HTML 報告（實際情況會更複雜）
    let html_content = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>回測報告 - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        h1 {{ color: #333; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; }}
        th {{ background-color: #f2f2f2; text-align: left; }}
        tr:nth-child(even) {{ background-color: #f9f9f9; }}
    </style>
</head>
<body>
    <h1>回測報告 - {}</h1>
    <p><strong>回測 ID:</strong> {}</p>
    <p><strong>回測時間區間:</strong> {} 至 {}</p>
    <h2>回測標的</h2>
    <ul>
        {}
    </ul>
    <h2>回測結果</h2>
    <p>回測正在進行中，結果將在完成後顯示...</p>
</body>
</html>"#,
        strategy_name,
        strategy_name,
        backtest_id,
        start_time.format("%Y-%m-%d %H:%M:%S"),
        end_time.format("%Y-%m-%d %H:%M:%S"),
        instruments.iter()
            .map(|i| format!("<li>{}</li>", i))
            .collect::<String>()
    );
    
    // 寫入 HTML 文件
    tokio_fs::write(&html_path, html_content).await?;
    
    Ok(html_path)
}

pub async fn create_backtest(
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut dsl_content = None;
    let mut request_data = None;
    let mut _filename = None;

    // 解析 multipart 表單數據
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "strategy_file" {
            // 讀取策略文件
            _filename = field.file_name().map(|s| s.to_string());
            dsl_content = Some(field.bytes().await.unwrap_or_default().to_vec());
        } else if name == "data" {
            // 解析回測參數
            let data = field.bytes().await.unwrap_or_default();
            request_data = serde_json::from_slice::<BacktestRequest>(&data).ok();
        }
    }

    // 檢查必要數據是否存在
    let dsl_content = match dsl_content {
        Some(content) => content,
        None => return (StatusCode::BAD_REQUEST, "缺少策略文件").into_response(),
    };
    
    let request = match request_data {
        Some(req) => req,
        None => return (StatusCode::BAD_REQUEST, "缺少回測參數").into_response(),
    };

    // 解析策略 DSL 獲取名稱和描述
    let strategy_info = match extract_strategy_info(&dsl_content) {
        Ok(info) => info,
        Err(e) => return (
            StatusCode::BAD_REQUEST, 
            format!("策略 DSL 解析錯誤: {}", e)
        ).into_response(),
    };

    // 生成新的策略 ID（如果沒有提供）
    let strategy_id = match request.strategy_id {
        Some(id) => StrategyId::from_string(id),
        None => StrategyId::new(),
    };

    // 儲存策略文件
    let file_path = match save_strategy_file(
        &strategy_info.name, 
        &dsl_content,
        &strategy_id
    ).await {
        Ok(path) => path,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR, 
            format!("儲存策略文件失敗: {}", e)
        ).into_response(),
    };

    // 建立策略定義
    let _strategy_def = StrategyDefinition::new(
        strategy_id.clone(), 
        strategy_info.name.clone(), 
        StrategyType::DSL
    )
    .with_description(strategy_info.description.clone().unwrap_or_default())
    .with_code(StrategyCode::from_binary(StrategyType::DSL, dsl_content.clone()));

    // 生成回測 ID
    let backtest_id = Uuid::new_v4().to_string();

    // 將策略資訊寫入資料庫
    let db_result = async {
        let pool = get_db_pool(false).await?;
        let repo = PgStrategyConfigRepository::new(Arc::new(pool.clone()));
        
        // 將策略參數轉換為配置格式
        let mut parameters = HashMap::new();
        for (key, value) in &request.parameters {
            parameters.insert(key.clone(), StrategyParameterConfig {
                value: serde_json::Value::String(value.clone()),
                required: false,
                description: None,
            });
        }
        
        // 創建配置文件對象
        let config_file = StrategyConfigFile {
            id: strategy_id.to_string(),
            name: strategy_info.name.clone(),
            description: strategy_info.description,
            version: "1.0.0".to_string(),
            parameters,
            code_path: Some(file_path.to_string_lossy().to_string()),
            enabled: true,
            author: None,
            tags: request.tags.clone(),
            dependencies: Vec::new(),
            metadata: HashMap::new(),
            last_modified: Some(Utc::now().timestamp() as u64),
            config_path: None,
        };
        
        repo.save_config(&config_file).await?;
        
        anyhow::Result::<()>::Ok(())
    }.await;
    
    if let Err(e) = db_result {
        return (
            StatusCode::INTERNAL_SERVER_ERROR, 
            format!("保存策略資訊到資料庫失敗: {}", e)
        ).into_response();
    }

    // 啟動回測任務
    let task_result = async {
        // 創建沙箱配置
        let mut sandbox_config = SandboxConfig::default();
        
        // 設置資源限制
        let resource_limit = ResourceLimit::new()
            .with_max_memory(1024) // 1GB 記憶體限制
            .with_max_cpu_time(50); // 50% CPU 時間限制
            
        sandbox_config.resource_limit = resource_limit;
        
        // 創建沙箱
        let sandbox_builder = SandboxBuilder::new(strategy_id.to_string())
            .with_config(sandbox_config)
            .with_code(dsl_content.clone())
            .with_parameters(request.parameters.clone());
            
        let sandbox = sandbox_builder.build().await
            .map_err(|e| anyhow!("創建沙箱失敗: {}", e))?;
        
        // 初始化沙箱
        let mut initialized_sandbox = sandbox;
        initialized_sandbox.initialize().await
            .map_err(|e| anyhow!("初始化沙箱失敗: {}", e))?;
        
        // 創建回測任務
        let task = BacktestTask {
            backtest_id: backtest_id.clone(),
            strategy_id: strategy_id.clone(),
            instruments: request.instruments.clone(),
            start_time: request.start_time,
            end_time: request.end_time,
            initial_capital: request.initial_capital,
            parameters: request.parameters.clone(),
            sandbox: Arc::new(Mutex::new(initialized_sandbox)),
            status: "pending".to_string(),
        };
        
        // 添加任務到管理器
        let task = BacktestTaskManager::global().add_task(task).await;
        
        // 非同步執行回測
        task::spawn(async move {
            if let Err(e) = execute_backtest(task).await {
                tracing::error!("回測任務執行失敗: {}", e);
            }
        });
        
        anyhow::Result::<()>::Ok(())
    }.await;
    
    if let Err(e) = task_result {
        return (
            StatusCode::INTERNAL_SERVER_ERROR, 
            format!("啟動回測任務失敗: {}", e)
        ).into_response();
    }

    // 生成初始回測報告 HTML
    let html_result = generate_backtest_html(
        &backtest_id,
        &strategy_info.name,
        &request.instruments,
        &request.start_time,
        &request.end_time
    ).await;
    
    if let Err(e) = html_result {
        return (
            StatusCode::INTERNAL_SERVER_ERROR, 
            format!("生成回測報告失敗: {}", e)
        ).into_response();
    }

    // 返回回測資訊
    let response = BacktestResponse {
        backtest_id: backtest_id.clone(),
        strategy_id: strategy_id.to_string(),
        status: "pending".to_string(),
        result_url: Some(format!("/api/backtest/results/{}", backtest_id)),
    };

    // 返回成功響應
    (StatusCode::CREATED, Json(response)).into_response()
} 