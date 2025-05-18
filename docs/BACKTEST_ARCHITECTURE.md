# FinRust 回測執行器協作架構

## 目錄

- [1. 簡介](#1-簡介)
- [2. 回測執行流程全景](#2-回測執行流程全景)
- [3. 詳細協作流程](#3-詳細協作流程)
  - [3.1 初始化階段](#31-初始化階段)
  - [3.2 數據準備階段](#32-數據準備階段)
  - [3.3 策略執行階段](#33-策略執行階段)
  - [3.4 結果收集階段](#34-結果收集階段)
  - [3.5 結果分析階段](#35-結果分析階段)
- [4. 關鍵協作機制](#4-關鍵協作機制)
  - [4.1 策略模組與DSL模組協作](#41-策略模組與dsl模組協作)
  - [4.2 運行時模組與沙箱隔離](#42-運行時模組與沙箱隔離)
  - [4.3 數據提供模組與延遲加載](#43-數據提供模組與延遲加載)
  - [4.4 執行模擬器與交易處理](#44-執行模擬器與交易處理)
  - [4.5 風險管理模組的整合](#45-風險管理模組的整合)
- [5. 高級特性](#5-高級特性)
  - [5.1 快取與批處理機制](#51-快取與批處理機制)
  - [5.2 多策略並行回測](#52-多策略並行回測)
  - [5.3 回測任務調度](#53-回測任務調度)
- [6. 錯誤處理與恢復](#6-錯誤處理與恢復)
  - [6.1 錯誤偵測與分類](#61-錯誤偵測與分類)
  - [6.2 策略回滾機制](#62-策略回滾機制)
  - [6.3 資源清理與釋放](#63-資源清理與釋放)

## 1. 簡介

FinRust回測執行器是系統的核心引擎，負責協調各功能模組共同執行策略回測。它以回測模組為中心，整合了數據提供、策略管理、運行時隔離、執行模擬、風險控制等多個子系統，實現了從回測任務提交到結果分析的完整流程。

本文檔詳細描述了回測執行器的架構設計和各模組間的協作關係，以及關鍵流程的實現細節。

## 2. 回測執行流程全景

回測執行流程分為五個主要階段，每個階段涉及不同的模組協作：

```
┌─── 初始化階段 ───┐   ┌─── 數據準備階段 ───┐   ┌─── 策略執行階段 ───┐   ┌─── 結果收集階段 ───┐   ┌─── 結果分析階段 ───┐
│  使用模組:        │   │  使用模組:          │   │  使用模組:          │   │  使用模組:          │   │  使用模組:          │
│  - 回測模組       │   │  - 數據提供模組     │   │  - 策略模組         │   │  - 執行模擬器模組   │   │  - 回測模組         │
│  - 配置管理模組   │   │  - 運行時模組       │   │  - DSL模組          │   │  - 風險管理模組     │   │  - 事件處理模組     │
│                  │   │  - 回測模組         │   │  - 運行時模組       │   │  - 回測模組         │   │                    │
│                  │   │                    │   │  - 執行模擬器模組    │   │                    │   │                    │
└──────────────────┘   └────────────────────┘   └────────────────────┘   └────────────────────┘   └────────────────────┘
```

**核心協調模組：**
- **回測模組**作為整個執行過程的中央協調器，負責貫穿所有階段的任務管理、狀態追蹤和結果處理。
- **回測引擎**是回測模組的核心組件，實現各階段的具體邏輯並協調其他模組的調用關係。

## 3. 詳細協作流程

### 3.1 初始化階段

**主要任務：** 配置加載、回測環境準備、任務初始化

**參與模組：**
- **回測模組 (backtest)**：創建回測上下文，初始化進度跟踪器，協調其他模組
- **配置管理模組 (config)**：加載回測配置，驗證設置有效性
- **事件處理模組 (event)**：設置回測事件監聽器

**協作流程：**

```rust
// backtest/engine.rs
async fn initialize_backtest(&self, task: &BacktestTask) -> Result<BacktestContext, Error> {
    // 1. [配置管理模組] 加載回測配置
    let config = self.config_service.load_backtest_config(&task.config_id).await?;
    
    // 2. [回測模組] 創建回測上下文
    let context = BacktestContext::new(task.id.clone(), config);
    
    // 3. [回測模組] 通過任務管理器更新任務狀態
    self.task_manager.update_task_status(&task.id, TaskStatus::Initializing).await?;
    
    // 4. [事件處理模組] 設置事件監聽器
    let event_listener = self.event_bus.create_listener(&task.id, 
        vec![EventType::StrategyError, EventType::SystemError])?;
    context.set_event_listener(event_listener);
    
    // 5. [回測模組] 初始化快取系統
    let cache = BacktestCache::new(&task.id, self.cache_config.clone())?;
    context.set_cache(cache);
    
    // 6. [回測模組] 創建進度追蹤器
    let progress_tracker = ProgressTracker::new(&task.id);
    context.set_progress_tracker(progress_tracker);
    
    log::info!("[初始化階段] 回測任務初始化完成: {}", task.id);
    
    Ok(context)
}
```

### 3.2 數據準備階段

**主要任務：** 請求歷史數據、設置數據加載器、預計算技術指標

**參與模組：**
- **回測模組 (backtest)**：協調整個數據準備流程，更新任務進度
- **數據提供模組 (data_provider)**：創建延遲加載器，預取初始數據，準備技術指標
- **運行時模組 (runtime)**：為沙箱準備數據訪問環境
- **領域類型模組 (domain_types)**：提供數據結構和類型定義

**協作流程：**

```rust
// backtest/engine.rs
async fn prepare_data(&self, context: &mut BacktestContext) -> Result<(), Error> {
    // 1. [回測模組] 更新任務狀態
    self.task_manager.update_task_status(&context.task_id, TaskStatus::PreparingData).await?;
    
    // 2. [領域類型模組] 獲取回測時間範圍和資產類型
    let start_time = context.config.start_time;
    let end_time = context.config.end_time;
    let frequency = context.config.data_frequency;
    
    log::info!("[數據準備階段] 開始準備數據, 時間範圍: {} - {}, 頻率: {:?}", 
        start_time, end_time, frequency);
    
    // 3. [數據提供模組] 為每個金融工具創建數據加載器
    for instrument in &context.config.instruments {
        // 3.1 [數據提供模組] 創建數據請求
        let request = DataRequest::new(
            instrument.clone(),
            start_time,
            end_time,
            frequency,
        );
        
        log::debug!("[數據準備階段] 建立數據加載器: {}", instrument);
        
        // 3.2 [數據提供模組] 使用LazyLoader延遲加載機制
        let data_loader = self.data_provider.create_lazy_loader(request)?;
        
        // 3.3 [回測模組] 存儲數據加載器到上下文
        context.add_data_loader(instrument.clone(), data_loader);
        
        // 3.4 [數據提供模組] 預加載一小部分數據以驗證可用性
        data_loader.prefetch_head(100)?;
    }
    
    // 4. [數據提供模組] 初始化技術指標預計算器（如需要）
    if !context.config.indicators.is_empty() {
        log::info!("[數據準備階段] 設置技術指標預計算: {:?}", context.config.indicators);
        let precalculator = self.data_provider.create_precalculator(&context.config.indicators)?;
        context.set_precalculator(Some(precalculator));
    }
    
    // 5. [運行時模組] 準備沙箱數據環境
    if let Some(sandbox) = &context.sandbox {
        log::debug!("[數據準備階段] 配置沙箱數據環境");
        self.setup_sandbox_data_environment(sandbox, &context.config.instruments)?;
    }
    
    // 6. [回測模組] 更新進度
    context.progress_tracker.update_stage("資料準備完成", 10.0)?;
    
    log::info!("[數據準備階段] 數據準備完成");
    
    Ok(())
}
```

### 3.3 策略執行階段

**主要任務：** 策略加載與編譯、沙箱創建、主回測循環執行

**參與模組：**
- **回測模組 (backtest)**：協調策略執行，管理回測進度，回測主循環控制
- **策略模組 (strategy)**：加載策略定義，管理策略生命週期
- **DSL模組 (dsl)**：解析和編譯策略代碼
- **運行時模組 (runtime)**：提供沙箱隔離環境，執行策略代碼
- **執行模擬器模組 (execution)**：處理訂單模擬和倉位管理
- **風險管理模組 (risk)**：進行風險評估和限制檢查
- **數據提供模組 (data_provider)**：按時間序提供市場數據

**協作流程：**

```rust
// backtest/engine.rs
async fn execute_strategy(&self, context: &mut BacktestContext) -> Result<(), Error> {
    // 1. [回測模組] 更新任務狀態
    self.task_manager.update_task_status(&context.task_id, TaskStatus::Running).await?;
    
    log::info!("[策略執行階段] 開始執行策略: {}", context.config.strategy_id);
    
    // 2. [策略模組] 從策略模組加載策略
    let strategy = self.strategy_service
        .load_strategy(&context.config.strategy_id, context.config.strategy_version.as_deref())
        .await?;
        
    log::debug!("[策略執行階段] 策略成功加載: {} 版本: {:?}", 
        strategy.id, strategy.version);
    
    // 3. [DSL模組] 編譯策略代碼
    let compiled_strategy = self.strategy_service.compile_strategy(&strategy)?;
    
    // 4. [運行時模組] 從運行時模組創建沙箱
    let sandbox_config = SandboxConfig::from(&context.config);
    let sandbox = self.runtime_service.create_sandbox(&context.task_id, sandbox_config)?;
    context.set_sandbox(sandbox.clone());
    
    log::debug!("[策略執行階段] 沙箱創建完成, 加載策略");
    
    // 5. [運行時模組] 將策略加載到沙箱
    sandbox.load_strategy(compiled_strategy)?;
    
    // 6. [數據提供模組] 創建市場數據迭代器
    let market_data_iterator = self.create_market_data_iterator(context)?;
    
    // 7. [執行模擬器模組] 創建執行引擎
    let execution_engine = self.execution_service.create_engine(
        context.config.initial_capital,
        &context.config.execution_settings,
    )?;
    context.set_execution_engine(execution_engine.clone());
    
    log::info!("[策略執行階段] 開始回測主循環");
    
    // 8. [回測模組] 主回測循環
    while let Some((current_time, market_data)) = market_data_iterator.next().await? {
        // 8.1 [運行時模組] 更新策略上下文
        sandbox.update_market_data(current_time, &market_data)?;
        
        // 8.2 [運行時模組 + DSL模組] 執行策略獲取信號
        let signals = sandbox.execute_strategy_step()?;
        
        // 8.3 [風險管理模組] 風險檢查
        let validated_signals = self.risk_service.validate_signals(
            signals, 
            &context.config.risk_limits,
            execution_engine.get_portfolio_state()?,
        )?;
        
        // 8.4 [執行模擬器模組] 執行交易
        for signal in validated_signals {
            let order = self.convert_signal_to_order(signal)?;
            execution_engine.process_order(current_time, order, &market_data)?;
        }
        
        // 8.5 [回測模組] 快取交易結果
        let position_snapshot = execution_engine.get_position_snapshot(current_time)?;
        context.cache.add_position_snapshot(current_time, position_snapshot)?;
        
        // 8.6 [回測模組] 更新進度（每100個時間點更新一次）
        if context.progress_counter % 100 == 0 {
            let progress_percent = self.calculate_progress_percent(current_time, context)?;
            context.progress_tracker.update_progress(progress_percent)?;
            
            // 持久化進度到沙箱
            let progress_data = context.progress_tracker.to_json()?;
            sandbox.set_context_data("progress", progress_data)?;
            
            log::debug!("[策略執行階段] 進度更新: {:.2}%", progress_percent);
        }
        
        context.progress_counter += 1;
    }
    
    // 9. [運行時模組] 最後一步執行
    sandbox.execute_final_step()?;
    
    // 10. [回測模組] 更新任務狀態
    self.task_manager.update_task_status(&context.task_id, TaskStatus::CollectingResults).await?;
    
    log::info!("[策略執行階段] 策略執行完成");
    
    Ok(())
}
```

### 3.4 結果收集階段

**主要任務：** 收集和整理交易記錄，計算績效指標，保存結果

**參與模組：**
- **回測模組 (backtest)**：協調結果收集和指標計算，保存回測結果
- **執行模擬器模組 (execution)**：提供交易和持倉數據
- **風險管理模組 (risk)**：計算風險相關指標
- **存儲模組 (storage)**：將結果持久化到數據庫

**協作流程：**

```rust
// backtest/engine.rs
async fn collect_results(&self, context: &mut BacktestContext) -> Result<BacktestResult, Error> {
    log::info!("[結果收集階段] 開始收集回測結果: {}", context.task_id);
    
    // 1. [回測模組] 從快取中獲取所有交易記錄和持倉數據
    let trades = context.cache.get_all_trades()?;
    let positions = context.cache.get_all_positions()?;
    
    log::debug!("[結果收集階段] 收集到 {} 筆交易, {} 個持倉快照", 
        trades.len(), positions.len());
    
    // 2. [回測模組 + 風險管理模組] 計算各種績效指標
    let metrics_calculator = self.get_metrics_calculator();
    let metrics = metrics_calculator.calculate_metrics(
        &trades,
        &positions,
        context.config.initial_capital,
        context.config.start_time,
        context.config.end_time,
    )?;
    
    log::debug!("[結果收集階段] 計算結果: 總收益率: {:.2}%, 年化收益: {:.2}%, 夏普比率: {:.2}, 最大回撤: {:.2}%", 
        metrics.total_return * 100.0, 
        metrics.annualized_return * 100.0, 
        metrics.sharpe_ratio, 
        metrics.max_drawdown * 100.0);
    
    // 3. [回測模組] 創建回測結果
    let result = BacktestResult {
        backtest_id: context.task_id.clone(),
        strategy_id: context.config.strategy_id.clone(),
        strategy_version: context.config.strategy_version.clone(),
        start_time: context.config.start_time,
        end_time: context.config.end_time,
        initial_capital: context.config.initial_capital,
        final_capital: metrics.final_capital,
        total_return: metrics.total_return,
        annualized_return: metrics.annualized_return,
        sharpe_ratio: metrics.sharpe_ratio,
        max_drawdown: metrics.max_drawdown,
        win_rate: metrics.win_rate,
        trades_count: trades.len(),
        // ... 其他指標 ...
        execution_time: context.execution_time(),
    };
    
    // 4. [存儲模組] 持久化結果
    self.result_storage.save_result(&result).await?;
    
    log::debug!("[結果收集階段] 基本結果已保存");
    
    // 5. [存儲模組] 如果設置了保存詳細數據，將交易和持倉數據存儲到資料庫
    if context.config.save_detailed_results {
        log::debug!("[結果收集階段] 保存詳細交易和持倉數據");
        self.result_storage.save_detailed_data(
            &context.task_id,
            &trades,
            &positions,
        ).await?;
    }
    
    // 6. [回測模組] 更新任務狀態
    self.task_manager.update_task_status(&context.task_id, TaskStatus::Completed).await?;
    
    log::info!("[結果收集階段] 回測結果收集完成");
    
    Ok(result)
}
```

### 3.5 結果分析階段

**主要任務：** 生成報告和圖表，進行基準對比，結果可視化

**參與模組：**
- **回測模組 (backtest)**：協調結果分析流程，生成報告和圖表
- **事件處理模組 (event)**：發布回測完成事件

**協作流程：**

```rust
// backtest/engine.rs
async fn analyze_results(&self, context: &BacktestContext, result: &BacktestResult) -> Result<(), Error> {
    // 檢查是否啟用分析功能
    if !context.config.enable_analysis {
        log::info!("[結果分析階段] 分析功能未啟用，跳過此階段");
        return Ok(());
    }
    
    log::info!("[結果分析階段] 開始結果分析");
    
    // 1. [回測模組] 生成績效報告
    if context.config.generate_performance_report {
        log::debug!("[結果分析階段] 生成績效報告");
        self.report_generator.generate_performance_report(&result).await?;
    }
    
    // 2. [回測模組] 生成圖表
    if context.config.generate_charts {
        log::debug!("[結果分析階段] 生成圖表");
        self.chart_generator.generate_equity_curve(&context.task_id).await?;
        self.chart_generator.generate_drawdown_chart(&context.task_id).await?;
        self.chart_generator.generate_monthly_returns_heatmap(&context.task_id).await?;
    }
    
    // 3. [回測模組] 如果有比較基準，計算相對績效
    if let Some(benchmark) = &context.config.benchmark {
        log::debug!("[結果分析階段] 進行基準比較: {}", benchmark);
        self.benchmark_analyzer.analyze_relative_performance(
            &result,
            benchmark,
            context.config.start_time,
            context.config.end_time,
        ).await?;
    }
    
    // 4. [事件處理模組] 發布回測分析完成事件
    self.event_bus.publish(Event::new(
        EventType::BacktestAnalysisCompleted,
        context.task_id.clone(),
        Some(json!({
            "backtest_id": context.task_id,
            "success": true
        }))
    )).await?;
    
    log::info!("[結果分析階段] 結果分析完成");
    
    Ok(())
}
```

## 4. 關鍵協作機制

### 4.1 策略模組與DSL模組協作

**參與模組：**
- **策略模組 (strategy)**
- **DSL模組 (dsl)**

**協作內容：** 策略加載、編譯與運行時擴展

```rust
// strategy/service.rs
impl StrategyService {
    // 加載並編譯策略
    pub async fn compile_strategy(&self, strategy: &Strategy) -> Result<CompiledStrategy, Error> {
        log::debug!("開始編譯策略: {}", strategy.id);
        
        // 1. [DSL模組] 使用DSL模組解析策略代碼
        let parsed = self.dsl_service.parse(&strategy.source_code)?;
        
        // 2. [DSL模組] 驗證和編譯策略
        let compiled = self.dsl_service.compile(parsed)?;
        
        // 3. [策略模組 -> DSL模組] 註冊回測專用的DSL函數
        self.register_backtest_functions(&compiled)?;
        
        log::debug!("策略編譯完成: {}", strategy.id);
        
        Ok(compiled)
    }
    
    // 註冊回測專用函數
    fn register_backtest_functions(&self, compiled: &CompiledStrategy) -> Result<(), Error> {
        // [DSL模組] 註冊市場數據訪問函數
        self.dsl_service.register_function("get_price", |args| {
            // 實現從回測數據中獲取價格的邏輯
            // ...
        })?;
        
        // [DSL模組] 註冊技術指標計算函數
        self.dsl_service.register_function("sma", |args| {
            // 實現簡單移動平均計算邏輯
            // ...
        })?;
        
        // [DSL模組] 註冊交易函數
        self.dsl_service.register_function("place_order", |args| {
            // 實現下單邏輯
            // ...
        })?;
        
        Ok(())
    }
}
```

### 4.2 運行時模組與沙箱隔離

**參與模組：**
- **運行時模組 (runtime)**
- **回測模組 (backtest)**
- **策略模組 (strategy)**

**協作內容：** 安全的策略執行環境

```rust
// runtime/service.rs
impl RuntimeService {
    // 創建隔離的策略執行沙箱
    pub fn create_sandbox(&self, id: &str, config: SandboxConfig) -> Result<StrategySandbox, Error> {
        log::debug!("創建策略沙箱: {}", id);
        
        // 1. [運行時模組] 初始化沙箱
        let sandbox = StrategySandbox::new(id, config)?;
        
        // 2. [運行時模組] 設置資源限制
        sandbox.set_memory_limit(config.memory_limit)?;
        sandbox.set_execution_timeout(config.execution_timeout)?;
        
        // 3. [運行時模組] 初始化沙箱上下文
        let context = SandboxContext::new();
        sandbox.set_context(context)?;
        
        // 4. [運行時模組] 設置錯誤處理
        sandbox.set_error_handler(|error| {
            // 處理策略執行中的錯誤
            // 如果錯誤嚴重，觸發回滾機制
            if error.is_critical() {
                log::error!("策略執行嚴重錯誤: {}", error);
                return ErrorAction::Rollback;
            }
            log::warn!("策略執行非嚴重錯誤: {}", error);
            ErrorAction::Continue
        })?;
        
        log::debug!("沙箱創建完成: {}", id);
        
        Ok(sandbox)
    }
}

// runtime/sandbox.rs
impl StrategySandbox {
    // 在沙箱中執行策略步驟
    pub fn execute_strategy_step(&self) -> Result<Vec<Signal>, Error> {
        // [運行時模組] 使用受控的執行環境運行策略代碼
        self.runtime.execute_with_timeout(|| {
            // 獲取當前市場數據
            let market_data = self.context.get_current_market_data()?;
            
            // 執行策略邏輯
            let signals = self.strategy.execute_step(&market_data)?;
            
            Ok(signals)
        }, self.config.step_timeout)
    }
}
```

### 4.3 數據提供模組與延遲加載

**參與模組：**
- **數據提供模組 (data_provider)**
- **回測模組 (backtest)**
- **存儲模組 (storage)**

**協作內容：** 高效的歷史數據訪問

```rust
// data_provider/service.rs
impl DataProviderService {
    // 創建延遲加載器
    pub fn create_lazy_loader(&self, request: DataRequest) -> Result<Box<dyn LazyLoader>, Error> {
        log::debug!("創建延遲加載器: {:?}", request);
        
        match request.data_type {
            DataType::OHLCV => {
                // [數據提供模組] 創建OHLCV數據加載器
                let loader = LazyOHLCVLoader::new(
                    request.instrument,
                    request.start_time,
                    request.end_time,
                    request.frequency,
                    self.db_pool.clone(),
                )?;
                Ok(Box::new(loader))
            },
            DataType::Tick => {
                // [數據提供模組] 創建Tick數據加載器
                let loader = LazyTickLoader::new(
                    request.instrument,
                    request.start_time,
                    request.end_time,
                    self.db_pool.clone(),
                )?;
                Ok(Box::new(loader))
            },
            // ... 其他數據類型 ...
        }
    }
}

// data_provider/lazy_loader/ohlcv_loader.rs
impl LazyOHLCVLoader {
    // 預取數據頭部（驗證可用性）
    pub fn prefetch_head(&self, count: usize) -> Result<Vec<OHLCVPoint>, Error> {
        log::debug!("預取數據頭部: {} 筆", count);
        
        // [數據提供模組 -> 存儲模組] 從數據庫加載一小部分數據
        let data = self.db.query_ohlcv_head(
            &self.instrument,
            self.start_time,
            count,
        )?;
        
        // [數據提供模組] 存入內部快取
        self.cache.insert_head(data.clone());
        
        log::debug!("預取完成，獲取到 {} 筆數據", data.len());
        
        Ok(data)
    }
    
    // 按時間窗口獲取數據
    pub async fn get_window(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<OHLCVPoint>, Error> {
        // 1. [數據提供模組] 先查詢快取
        if let Some(cached_data) = self.cache.get_window(start, end) {
            return Ok(cached_data);
        }
        
        log::debug!("快取未命中，從數據庫加載時間窗口: {} - {}", start, end);
        
        // 2. [數據提供模組 -> 存儲模組] 從數據庫加載
        let data = self.db.query_ohlcv_window(
            &self.instrument,
            start,
            end,
        )?;
        
        // 3. [數據提供模組] 存入快取
        self.cache.insert_window(start, end, data.clone());
        
        log::debug!("時間窗口數據加載完成: {} 筆", data.len());
        
        Ok(data)
    }
}
```

### 4.4 執行模擬器與交易處理

**參與模組：**
- **執行模擬器模組 (execution)**
- **回測模組 (backtest)**
- **領域類型模組 (domain_types)**

**協作內容：** 訂單執行與倉位管理

```rust
// execution/engine.rs
impl ExecutionEngine {
    // 處理訂單
    pub fn process_order(&mut self, time: DateTime<Utc>, order: Order, market_data: &MarketData) -> Result<Option<Trade>, Error> {
        log::debug!("處理訂單: {}, 方向: {:?}, 數量: {}, 請求價格: {}", 
            order.id, order.direction, order.quantity, order.price);
        
        // 1. [執行模擬器模組] 檢查訂單有效性
        self.validate_order(&order)?;
        
        // 2. [執行模擬器模組] 根據市場數據計算實際成交價格（考慮滑點等）
        let execution_price = self.calculate_execution_price(order.price, &order.direction, &market_data);
        
        // 3. [執行模擬器模組] 計算交易費用
        let commission = self.calculate_commission(&order, execution_price);
        
        // 4. [執行模擬器模組 + 領域類型模組] 創建交易記錄
        let trade = Trade {
            id: format!("T-{}-{}", order.id, Uuid::new_v4()),
            time,
            instrument_id: order.instrument_id.clone(),
            direction: order.direction.clone(),
            order_type: order.order_type.clone(),
            requested_price: order.price,
            execution_price,
            quantity: order.quantity,
            commission,
            slippage: (execution_price - order.price).abs(),
            order_id: order.id.clone(),
        };
        
        // 5. [執行模擬器模組] 更新投資組合
        self.portfolio.apply_trade(&trade)?;
        
        // 6. [執行模擬器模組] 更新持倉
        self.positions.update_from_trade(&trade)?;
        
        // 7. [執行模擬器模組] 記錄交易
        self.trade_history.push(trade.clone());
        
        log::debug!("訂單處理完成: {}, 成交價格: {}, 交易費用: {}", 
            order.id, execution_price, commission);
        
        Ok(Some(trade))
    }
    
    // 獲取持倉快照
    pub fn get_position_snapshot(&self, time: DateTime<Utc>) -> Result<PositionSnapshot, Error> {
        // [執行模擬器模組 + 領域類型模組] 創建當前時間點的持倉快照
        let snapshot = PositionSnapshot {
            time,
            positions: self.positions.get_current_positions()?,
            portfolio_value: self.portfolio.total_value()?,
            cash_balance: self.portfolio.cash_balance,
        };
        
        Ok(snapshot)
    }
}
```

### 4.5 風險管理模組的整合

**參與模組：**
- **風險管理模組 (risk)**
- **回測模組 (backtest)**
- **執行模擬器模組 (execution)**

**協作內容：** 交易信號風險評估

```rust
// risk/service.rs
impl RiskService {
    // 驗證交易信號
    pub fn validate_signals(
        &self, 
        signals: Vec<Signal>, 
        risk_limits: &RiskLimits,
        portfolio_state: PortfolioState
    ) -> Result<Vec<Signal>, Error> {
        log::debug!("進行風險驗證: {} 個信號", signals.len());
        
        let mut validated_signals = Vec::new();
        
        for signal in signals {
            // 1. [風險管理模組] 檢查單筆交易限額
            if !self.check_single_trade_limit(&signal, risk_limits, &portfolio_state) {
                log::warn!("信號被拒: 超出單筆交易限額, 商品: {}, 數量: {}", 
                    signal.instrument_id, signal.quantity);
                continue; // 超出限額，跳過該信號
            }
            
            // 2. [風險管理模組] 檢查集中度限制
            if !self.check_concentration_limit(&signal, risk_limits, &portfolio_state) {
                log::warn!("信號被拒: 超出集中度限制, 商品: {}", signal.instrument_id);
                continue; // 超出集中度限制，跳過該信號
            }
            
            // 3. [風險管理模組] 檢查交易頻率限制
            if !self.check_frequency_limit(&signal, risk_limits, &portfolio_state) {
                log::warn!("信號被拒: 超出交易頻率限制");
                continue; // 超出交易頻率限制，跳過該信號
            }
            
            // 4. [風險管理模組] 檢查最大回撤限制
            if !self.check_drawdown_limit(&signal, risk_limits, &portfolio_state) {
                log::warn!("信號被拒: 接近最大回撤限制");
                continue; // 超出最大回撤限制，跳過該信號
            }
            
            // 所有檢查都通過，保留信號
            validated_signals.push(signal);
        }
        
        log::debug!("風險驗證完成: 通過 {} 個信號，拒絕 {} 個信號", 
            validated_signals.len(), signals.len() - validated_signals.len());
        
        Ok(validated_signals)
    }
}
```

## 5. 高級特性

### 5.1 快取與批處理機制

**參與模組：**
- **回測模組 (backtest)**
- **存儲模組 (storage)**

**協作內容：** 高效數據處理與存儲

```rust
// backtest/cache.rs
pub struct BacktestCache {
    backtest_id: String,
    positions: Vec<PositionSnapshot>,
    trades: Vec<Trade>,
    max_memory_items: usize,
    position_secondary_cache: Option<TempPositionStorage>,
    trade_secondary_cache: Option<TempTradeStorage>,
}

impl BacktestCache {
    // 添加持倉快照
    pub fn add_position_snapshot(&mut self, time: DateTime<Utc>, snapshot: PositionSnapshot) -> Result<(), Error> {
        self.positions.push(snapshot);
        
        // 檢查是否需要溢出到二級快取
        self.check_position_overflow()?;
        
        Ok(())
    }
    
    // 檢查持倉溢出
    fn check_position_overflow(&mut self) -> Result<(), Error> {
        if self.positions.len() >= self.max_memory_items {
            log::debug!("持倉資料達到記憶體閾值 ({}), 溢出到二級快取", self.max_memory_items);
            
            // 初始化二級快取（如果尚未初始化）
            if self.position_secondary_cache.is_none() {
                self.position_secondary_cache = Some(TempPositionStorage::new(&self.backtest_id)?);
            }
            
            // 將一半數據溢出到二級快取
            let overflow_batch: Vec<_> = self.positions.drain(0..self.max_memory_items/2).collect();
            self.position_secondary_cache.as_mut().unwrap().store_batch(overflow_batch)?;
            
            log::debug!("已將 {} 筆持倉資料溢出到二級快取", self.max_memory_items/2);
        }
        
        Ok(())
    }
    
    // 獲取所有持倉數據
    pub fn get_all_positions(&mut self) -> Result<Vec<PositionSnapshot>, Error> {
        log::debug!("獲取所有持倉資料");
        
        let mut all_positions = Vec::new();
        
        // 從二級快取獲取數據
        if let Some(secondary_cache) = &mut self.position_secondary_cache {
            log::debug!("從二級快取讀取資料");
            let secondary_positions = secondary_cache.retrieve_all()?;
            all_positions.extend(secondary_positions);
        }
        
        // 添加主快取中的數據
        log::debug!("從主快取讀取 {} 筆資料", self.positions.len());
        all_positions.extend(std::mem::take(&mut self.positions));
        
        log::debug!("共獲取 {} 筆持倉資料", all_positions.len());
        
        Ok(all_positions)
    }
}

// storage/backtest_storage.rs
impl BacktestResultStorage {
    // 保存詳細回測數據
    pub async fn save_detailed_data(
        &self,
        backtest_id: &str,
        trades: &[Trade],
        positions: &[PositionSnapshot],
    ) -> Result<(), Error> {
        log::info!("開始保存詳細回測數據: {}", backtest_id);
        
        // 使用數據批處理優化寫入效能
        
        // 1. [存儲模組] 批量寫入交易數據
        log::debug!("批量寫入 {} 筆交易資料", trades.len());
        for trade_batch in trades.chunks(self.batch_size) {
            self.db.batch_insert_trades(backtest_id, trade_batch).await?;
        }
        
        // 2. [存儲模組] 批量寫入持倉數據
        log::debug!("批量寫入 {} 筆持倉資料", positions.len());
        for position_batch in positions.chunks(self.batch_size) {
            self.db.batch_insert_positions(backtest_id, position_batch).await?;
        }
        
        log::info!("詳細回測數據保存完成: {}", backtest_id);
        
        Ok(())
    }
}
```

### 5.2 多策略並行回測

**參與模組：**
- **回測模組 (backtest)**
- **API服務模組 (api)**

**協作內容：** 並行處理多個回測任務

```rust
// backtest/executor.rs
pub struct BacktestExecutor {
    engine: Arc<BacktestEngine>,
    max_concurrent_tasks: usize,
    current_tasks: AtomicUsize,
}

impl BacktestExecutor {
    // 執行多個回測任務
    pub async fn execute_batch_tasks(&self, tasks: Vec<BacktestTask>) -> Result<Vec<BacktestResult>, Error> {
        log::info!("開始批量執行 {} 個回測任務", tasks.len());
        
        let mut results = Vec::with_capacity(tasks.len());
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_tasks));
        
        // 創建任務future集合
        let task_futures: Vec<_> = tasks.into_iter().map(|task| {
            let engine = self.engine.clone();
            let semaphore = semaphore.clone();
            
            async move {
                // 獲取信號量許可
                let _permit = semaphore.acquire().await?;
                log::debug!("開始執行回測任務: {}", task.id);
                
                // 執行回測
                let result = engine.execute_backtest(task).await?;
                
                log::debug!("回測任務完成: {}", result.backtest_id);
                // 釋放許可（通過_permit的Drop特性自動實現）
                Ok::<_, Error>(result)
            }
        }).collect();
        
        log::debug!("創建了 {} 個任務Future, 最大並行數: {}", 
            task_futures.len(), self.max_concurrent_tasks);
        
        // 同時執行所有任務，但受信號量控制
        for task_result in futures::future::join_all(task_futures).await {
            match task_result {
                Ok(result) => {
                    log::debug!("收集回測結果: {}", result.backtest_id);
                    results.push(result);
                },
                Err(err) => {
                    log::error!("回測任務執行失敗: {}", err);
                }
            }
        }
        
        log::info!("批量回測完成, 成功: {}, 失敗: {}", 
            results.len(), task_futures.len() - results.len());
        
        Ok(results)
    }
}
```

### 5.3 回測任務調度

**參與模組：**
- **回測模組 (backtest)**
- **配置管理模組 (config)**
- **API服務模組 (api)**

**協作內容：** 管理回測任務的生命週期

```rust
// backtest/task.rs
pub struct BacktestTaskManager {
    db: Arc<dyn BacktestDatabase>,
    event_bus: Arc<dyn EventBus>,
    config_service: Arc<dyn ConfigService>,
}

impl BacktestTaskManager {
    // 創建新的回測任務
    pub async fn create_task(&self, request: BacktestRequest) -> Result<BacktestTask, Error> {
        log::info!("創建新的回測任務: 策略={}", request.strategy_id);
        
        // 1. [回測模組 -> 配置管理模組] 驗證和加載配置
        let config = self.config_service.validate_backtest_config(&request.config)?;
        
        // 2. [回測模組] 生成任務ID
        let backtest_id = format!("bt_{}", Uuid::new_v4());
        
        // 3. [回測模組] 創建任務
        let task = BacktestTask {
            id: backtest_id.clone(),
            strategy_id: request.strategy_id,
            strategy_version: request.strategy_version,
            config,
            status: TaskStatus::Created,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // 4. [回測模組 -> 存儲模組] 保存任務到數據庫
        self.db.save_task(&task).await?;
        
        // 5. [回測模組 -> 事件處理模組] 發布任務創建事件
        self.event_bus.publish(Event::new(
            EventType::BacktestTaskCreated,
            backtest_id,
            Some(json!({ "strategy_id": task.strategy_id }))
        )).await?;
        
        log::info!("回測任務創建完成: {}", task.id);
        
        Ok(task)
    }
    
    // 更新任務狀態
    pub async fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<(), Error> {
        log::debug!("更新回測任務狀態: {} -> {:?}", task_id, status);
        
        // 1. [回測模組 -> 存儲模組] 更新數據庫中的任務狀態
        self.db.update_task_status(task_id, status).await?;
        
        // 2. [回測模組 -> 事件處理模組] 發布任務狀態更新事件
        self.event_bus.publish(Event::new(
            EventType::BacktestTaskStatusChanged,
            task_id.to_string(),
            Some(json!({ "status": status.to_string() }))
        )).await?;
        
        Ok(())
    }
    
    // 獲取任務列表
    pub async fn get_tasks(&self, filter: &TaskFilter) -> Result<Vec<BacktestTask>, Error> {
        log::debug!("獲取回測任務列表: {:?}", filter);
        
        // [回測模組 -> 存儲模組] 從數據庫查詢任務
        let tasks = self.db.get_tasks(filter).await?;
        
        log::debug!("查詢到 {} 個回測任務", tasks.len());
        
        Ok(tasks)
    }
}
```

## 6. 錯誤處理與恢復

### 6.1 錯誤偵測與分類

**參與模組：**
- **回測模組 (backtest)**
- **運行時模組 (runtime)**

**協作內容：** 監測和分類策略執行錯誤

```rust
// runtime/error.rs
// 運行時錯誤類型
#[derive(Debug)]
pub enum RuntimeError {
    // 策略代碼解析錯誤
    SyntaxError { line: usize, column: usize, message: String },
    
    // 運行時錯誤
    ExecutionError { message: String, is_critical: bool },
    
    // 資源限制錯誤
    ResourceLimitExceeded { resource_type: String, limit: u64, actual: u64 },
    
    // 超時錯誤
    TimeoutError { timeout_ms: u64, operation: String },
    
    // 其他系統錯誤
    SystemError { source: Box<dyn Error + Send + Sync>, message: String },
}

impl RuntimeError {
    // 判斷是否為嚴重錯誤
    pub fn is_critical(&self) -> bool {
        match self {
            RuntimeError::SyntaxError { .. } => true, // 語法錯誤總是嚴重的
            RuntimeError::ExecutionError { is_critical, .. } => *is_critical,
            RuntimeError::ResourceLimitExceeded { .. } => true, // 資源限制錯誤總是嚴重的
            RuntimeError::TimeoutError { .. } => true, // 超時錯誤總是嚴重的
            RuntimeError::SystemError { .. } => true, // 系統錯誤總是嚴重的
        }
    }
}

// runtime/sandbox.rs
impl StrategySandbox {
    // 監控策略執行
    fn monitor_execution<F, T>(&self, operation: F) -> Result<T, RuntimeError>
    where
        F: FnOnce() -> Result<T, RuntimeError>,
    {
        // 1. [運行時模組] 設置資源監控
        let resource_monitor = self.setup_resource_monitor()?;
        
        // 2. [運行時模組] 建立超時監控
        let timeout = Timeout::new(self.config.operation_timeout);
        
        // 3. 嘗試執行操作
        let result = match timeout.run(operation) {
            Ok(result) => result,
            Err(TimeoutError { timeout_ms }) => {
                log::error!("策略執行超時: {}ms", timeout_ms);
                return Err(RuntimeError::TimeoutError { 
                    timeout_ms, 
                    operation: "strategy_execution".to_string() 
                });
            }
        };
        
        // 4. [運行時模組] 檢查資源使用
        if let Some(violation) = resource_monitor.check_violations() {
            log::error!("策略執行超出資源限制: {:?}", violation);
            return Err(RuntimeError::ResourceLimitExceeded { 
                resource_type: violation.resource_type, 
                limit: violation.limit, 
                actual: violation.actual 
            });
        }
        
        result
    }
}
```

### 6.2 策略回滾機制

**參與模組：**
- **策略模組 (strategy)**
- **運行時模組 (runtime)**
- **回測模組 (backtest)**

**協作內容：** 處理策略執行錯誤並回滾到穩定版本

```rust
// runtime/sandbox.rs
impl StrategySandbox {
    // 執行策略步驟，包含錯誤處理和回滾機制
    pub fn execute_strategy_step(&self) -> Result<Vec<Signal>, Error> {
        // 嘗試執行策略
        match self.monitor_execution(|| self.try_execute_strategy_step()) {
            Ok(signals) => Ok(signals),
            Err(err) => {
                // 判斷錯誤嚴重程度
                if err.is_critical() {
                    // 嚴重錯誤，執行回滾
                    log::error!("策略執行嚴重錯誤，觸發回滾: {}", err);
                    self.execute_rollback()?;
                    return Err(err.into());
                } else {
                    // 非嚴重錯誤，記錄後繼續
                    log::warn!("策略執行遇到非嚴重錯誤: {}", err);
                    Ok(Vec::new()) // 返回空信號集
                }
            }
        }
    }
    
    // 執行回滾
    fn execute_rollback(&self) -> Result<(), Error> {
        log::info!("開始執行策略回滾");
        
        // 1. [運行時模組] 停止當前版本
        self.runtime.stop_execution()?;
        
        // 2. [策略模組] 獲取上一個穩定版本
        let previous_version = self.version_manager.get_last_stable_version()?;
        log::info!("獲取到上一個穩定版本: {}", previous_version);
        
        // 3. [運行時模組 + 策略模組] 加載上一個版本
        let strategy = self.strategy_service.load_strategy_version(&previous_version).await?;
        self.runtime.load_strategy(strategy)?;
        
        // 4. [運行時模組] 恢復策略狀態
        if let Some(checkpoint) = self.last_checkpoint.as_ref() {
            log::debug!("從檢查點恢復策略狀態: {}", checkpoint.id);
            self.runtime.restore_checkpoint(checkpoint)?;
        }
        
        // 5. [回測模組 + 事件處理模組] 記錄回滾事件
        self.event_recorder.record_rollback_event(
            &self.id,
            &previous_version,
            "策略執行錯誤，自動回滾到上一個穩定版本"
        )?;
        
        log::info!("策略回滾完成");
        
        Ok(())
    }
}
```

### 6.3 資源清理與釋放

**參與模組：**
- **回測模組 (backtest)**
- **運行時模組 (runtime)**
- **存儲模組 (storage)**

**協作內容：** 確保資源正確釋放

```rust
// backtest/engine.rs
impl BacktestEngine {
    // 資源清理
    async fn cleanup_resources(&self, context: &mut BacktestContext) -> Result<(), Error> {
        log::info!("開始清理回測資源: {}", context.task_id);
        
        // 1. [運行時模組] 關閉沙箱
        if let Some(sandbox) = context.sandbox.take() {
            log::debug!("關閉策略沙箱");
            sandbox.shutdown().await?;
        }
        
        // 2. [回測模組] 清理快取
        log::debug!("清理回測快取");
        if let Some(cache) = context.cache.as_mut() {
            // 如果設置了保留快取，則跳過清理
            if !context.config.retain_cache {
                cache.clear()?;
            }
        }
        
        // 3. [存儲模組] 清理臨時存儲
        if !context.config.retain_temp_files {
            log::debug!("清理臨時文件");
            self.storage.cleanup_temp_files(&context.task_id).await?;
        }
        
        log::info!("回測資源清理完成: {}", context.task_id);
        
        Ok(())
    }
    
    // 執行完整回測，包括錯誤處理和資源清理
    pub async fn execute_backtest(&self, task: BacktestTask) -> Result<BacktestResult, Error> {
        let mut context = BacktestContext::new(task.id.clone(), task.config.clone());
        
        // 使用 finally 模式確保資源清理
        let result = self.execute_backtest_internal(&mut context).await;
        
        // 無論成功失敗都清理資源
        if let Err(e) = self.cleanup_resources(&mut context).await {
            log::warn!("資源清理過程中出錯: {}", e);
        }
        
        // 返回原始結果
        result
    }
}
```

## 總結

FinRust回測執行器採用模組化設計，通過回測模組作為中央協調器，整合了數據提供、策略管理、運行時隔離、執行模擬和風險管理等核心模組，實現了高效、穩定、可擴展的策略回測系統。

回測流程分為初始化、數據準備、策略執行、結果收集和結果分析五個主要階段，每個階段都有明確的模組協作關係。系統支持多策略並行回測、數據延遲加載、快取與批處理、錯誤恢復機制等高級特性，確保了在大規模回測場景下的性能和穩定性。

通過細緻的協作設計和清晰的責任劃分，回測系統充分利用了Rust語言的安全性和高效性，為策略開發和驗證提供了堅實的基礎。