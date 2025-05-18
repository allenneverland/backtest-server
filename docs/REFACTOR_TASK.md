# BacktestServer 回測系統重構任務清單

## 概述

根據 BACKTEST_ARCHITECTURE.md 與當前程式碼的對比分析，本文檔詳細規劃了 BacktestServer 回測系統的重構任務，按優先級分類並提供詳細的實現指南。重構目標是實現一個符合五階段協作架構的完整回測系統，實現從配置設置到結果分析的完整流程。

## 任務分類及優先級

每個任務都標記了以下屬性：
- **優先級**: P0 (最高/阻塞) - P3 (最低/可選)
- **複雜度**: C0 (簡單) - C3 (複雜)
- **預計工時**: 以工作日計
- **前置依賴**: 需要先完成的任務

---

## P0: 核心執行模組 (最高優先級)

### 任務 1: 執行引擎實現 (P0, C3, 3天)

**目標文件**: `src/execution/engine.rs`

**功能描述**: 
實現執行引擎核心，負責處理訂單執行、倉位管理和交易模擬，是回測系統的基礎組件。

**實現要點**:
1. 創建 `ExecutionEngine` 結構體，管理訂單、倉位和交易記錄
2. 實現訂單處理功能 `process_order()`，模擬實際市場執行
3. 實現價格計算系統，支持滑點模擬和不同市場條件
4. 實現倉位更新邏輯，處理開倉、平倉和調整倉位
5. 整合交易費用和滑點計算
6. 實現投資組合估值功能
7. 提供快照機制，用於記錄回測過程中的狀態

**API 設計**:
```rust
pub struct ExecutionEngine {
    // 內部狀態
    settings: ExecutionSettings,
    portfolio: Portfolio,
    orders: HashMap<Uuid, Order>,
    trades: Vec<Trade>,
    // ...其他
}

impl ExecutionEngine {
    // 核心方法
    pub fn new(initial_capital: f64, settings: &ExecutionSettings) -> Self { ... }
    pub fn process_order(&mut self, time: DateTime<Utc>, order: Order, market_data: &MarketData) -> Result<Option<Trade>, Error> { ... }
    pub fn get_position_snapshot(&self, time: DateTime<Utc>) -> Result<PositionSnapshot, Error> { ... }
    pub fn get_portfolio_state(&self) -> Result<PortfolioState, Error> { ... }
    
    // 實用方法
    fn calculate_execution_price(&self, order_price: f64, direction: &OrderDirection, market_data: &MarketData) -> f64 { ... }
    fn calculate_commission(&self, order: &Order, execution_price: f64) -> f64 { ... }
}
```

**測試策略**:
- 單元測試市價訂單執行
- 單元測試限價訂單執行
- 測試滑點計算
- 測試佣金計算
- 測試資產組合估值

---

### 任務 2: 訂單模擬器實現 (P0, C2, 2天)
**前置依賴**: 任務 1

**目標文件**: `src/execution/simulator.rs`

**功能描述**:
實現更高級的訂單執行模擬，包括不同市場條件下的滑點模擬、成交概率模擬等，提供比 ExecutionEngine 更真實的市場模擬。

**實現要點**:
1. 創建 `OrderSimulator` 結構體，支持多種模擬策略
2. 實現基於成交量的滑點模型
3. 實現流動性模擬邏輯
4. 支持多種訂單類型的真實模擬（市價、限價、止損等）
5. 實現部分成交邏輯
6. 支持實時和延遲執行模擬

**API 設計**:
```rust
pub struct OrderSimulator {
    settings: SimulatorSettings,
    market_models: HashMap<String, MarketModel>,
}

impl OrderSimulator {
    pub fn new(settings: SimulatorSettings) -> Self { ... }
    
    pub fn simulate_execution(
        &self,
        order: &Order,
        market_data: &MarketData,
        time: DateTime<Utc>
    ) -> SimulationResult { ... }
    
    fn calculate_slippage(&self, order: &Order, market_data: &MarketData) -> f64 { ... }
    
    fn calculate_fill_probability(&self, order: &Order, market_data: &MarketData) -> f64 { ... }
}

pub struct SimulationResult {
    pub executed: bool,
    pub partial_fill: bool,
    pub filled_quantity: f64,
    pub execution_price: f64,
    pub slippage: f64,
    pub execution_time: DateTime<Utc>,
}
```

**測試策略**:
- 測試各種市場條件下的滑點計算
- 測試不同訂單類型的成交概率
- 測試部分成交邏輯
- 測試大單拆分執行模擬

---

### 任務 3: 訂單匹配引擎 (P0, C2, 2天)
**前置依賴**: 任務 1, 任務 2

**目標文件**: `src/execution/matching.rs`

**功能描述**:
實現訂單匹配引擎，模擬交易所撮合系統，處理買賣訂單之間的匹配和成交，尤其適用於複雜策略和多策略回測場景。

**實現要點**:
1. 創建 `OrderBook` 結構體，實現價格優先、時間優先的撮合機制
2. 實現買賣盤維護邏輯
3. 實現撮合演算法
4. 支持限價訂單、市價訂單
5. 實現部分成交和取消邏輯
6. 產生交易記錄和成交回報

**API 設計**:
```rust
pub struct OrderBook {
    symbol: String,
    bids: BTreeMap<Decimal, Vec<Order>>, // 買盤，按價格降序排列
    asks: BTreeMap<Decimal, Vec<Order>>, // 賣盤，按價格升序排列
    last_trade_price: Option<Decimal>,
    trades: Vec<Trade>,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self { ... }
    
    pub fn add_order(&mut self, order: Order) -> Vec<Trade> { ... }
    
    pub fn cancel_order(&mut self, order_id: Uuid) -> Result<(), Error> { ... }
    
    pub fn get_best_bid(&self) -> Option<Decimal> { ... }
    
    pub fn get_best_ask(&self) -> Option<Decimal> { ... }
    
    pub fn get_depth(&self, levels: usize) -> (Vec<Level>, Vec<Level>) { ... }
    
    fn match_orders(&mut self) -> Vec<Trade> { ... }
}
```

**測試策略**:
- 測試基本撮合機制
- 測試市價訂單撮合
- 測試限價訂單撮合
- 測試部分成交情況
- 測試訂單取消

---

### 任務 4: 回測引擎五階段流程實現 (P0, C3, 3天)
**前置依賴**: 任務 1 (可與任務 2、3 並行)

**目標文件**: `src/backtest/engine.rs`

**功能描述**:
重構現有回測引擎，實現完整的五階段回測流程（初始化、數據準備、策略執行、結果收集、結果分析），確保各模組間的協作符合架構設計。

**實現要點**:
1. 重構 `BacktestEngine::run()` 為明確的五階段流程
2. 實現 `initialize_backtest()` - 配置回測環境和創建上下文
3. 實現 `prepare_data()` - 請求和預加載必要的歷史數據
4. 實現 `execute_strategy()` - 運行主回測循環
5. 實現 `collect_results()` - 收集和整合交易結果
6. 實現 `analyze_results()` - 計算績效指標和生成報告
7. 確保各階段有良好的進度報告和錯誤處理
8. 實現資源清理和優雅關閉機制

**API 設計**:
```rust
impl BacktestEngine {
    pub async fn run(&mut self, task: BacktestTask, strategy_id: StrategyId) -> BacktestResult {
        let start_time = Instant::now();
        
        // 1. 初始化階段
        let mut context = self.initialize_backtest(&task).await?;
        
        // 2. 數據準備階段
        self.prepare_data(&mut context).await?;
        
        // 3. 策略執行階段
        self.execute_strategy(&mut context).await?;
        
        // 4. 結果收集階段
        let mut result = self.collect_results(&mut context).await?;
        
        // 5. 結果分析階段
        self.analyze_results(&context, &mut result).await?;
        
        // 清理資源
        self.cleanup_resources(&mut context).await?;
        
        result.set_duration(start_time.elapsed().as_millis() as u64);
        result
    }
    
    async fn initialize_backtest(&self, task: &BacktestTask) -> Result<BacktestContext, Error> { ... }
    
    async fn prepare_data(&self, context: &mut BacktestContext) -> Result<(), Error> { ... }
    
    async fn execute_strategy(&self, context: &mut BacktestContext) -> Result<(), Error> { ... }
    
    async fn collect_results(&self, context: &mut BacktestContext) -> Result<BacktestResult, Error> { ... }
    
    async fn analyze_results(&self, context: &BacktestContext, result: &mut BacktestResult) -> Result<(), Error> { ... }
    
    async fn cleanup_resources(&self, context: &mut BacktestContext) -> Result<(), Error> { ... }
}
```

**測試策略**:
- 為每個階段創建單元測試
- 測試數據預加載機制
- 測試策略信號處理
- 測試結果計算準確性
- 測試資源釋放機制

---

### 任務 5: 回測上下文優化 (P0, C2, 2天)
**前置依賴**: 任務 4

**目標文件**: `src/backtest/context.rs`

**功能描述**:
優化回測上下文實現，確保能高效管理回測狀態、儲存回測數據，並提供一致的接口供回測引擎和策略使用。

**實現要點**:
1. 增強 `BacktestContext` 結構體，支持更多回測相關狀態
2. 實現高效的數據存取機制
3. 添加策略沙箱整合功能
4. 實現交易和倉位管理 
5. 實現權益曲線記錄功能
6. 添加進度追蹤能力
7. 實現可配置的數據快取機制
8. 添加事件接收和處理能力

**API 設計**:
```rust
pub struct BacktestContext {
    pub task_id: String,
    pub config: BacktestConfig,
    pub sandbox: Option<Sandbox>,
    pub data_loaders: HashMap<String, Box<dyn LazyLoader>>,
    pub execution_engine: Option<Box<dyn ExecutionEngine>>,
    pub cache: Option<BacktestCache>,
    pub progress_tracker: ProgressTracker,
    // ... 其他成員
}

impl BacktestContext {
    pub fn new(task_id: String, config: BacktestConfig) -> Self { ... }
    
    pub fn set_sandbox(&mut self, sandbox: Sandbox) { ... }
    
    pub fn add_data_loader(&mut self, symbol: String, loader: Box<dyn LazyLoader>) { ... }
    
    pub fn set_execution_engine(&mut self, engine: Box<dyn ExecutionEngine>) { ... }
    
    pub fn submit_order(&mut self, order: Order) -> Result<Uuid, Error> { ... }
    
    pub fn get_data(&self, symbol: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<OHLCVPoint>, Error> { ... }
}
```

**測試策略**:
- 測試上下文初始化
- 測試數據加載功能
- 測試訂單提交和處理流程
- 測試倉位和權益計算
- 測試進度追蹤功能

---

## P1: 核心協作功能 (高優先級)

### 任務 6: 風險管理信號驗證實現 (P1, C2, 2天)

**目標文件**: `src/risk/checker.rs`

**功能描述**:
實現風險管理模組中的信號驗證功能，確保策略產生的交易信號符合風險限制規則，防止策略產生過度冒險的交易決策。

**實現要點**:
1. 創建 `RiskChecker` 結構體，管理不同類型的風險檢查
2. 實現信號驗證主函數 `validate_signals()`
3. 實現多種風險檢查器：
   - 單筆交易限額檢查
   - 集中度限制檢查
   - 交易頻率限制檢查
   - 最大回撤限制檢查
4. 實現風險報告生成功能
5. 設計可配置的風險參數系統

**API 設計**:
```rust
pub struct RiskChecker {
    config: RiskConfig,
    limits: RiskLimits,
}

impl RiskChecker {
    pub fn new(config: RiskConfig) -> Self { ... }
    
    pub fn validate_signals(
        &self, 
        signals: Vec<Signal>, 
        portfolio_state: &PortfolioState
    ) -> Vec<ValidatedSignal> { ... }
    
    fn check_single_trade_limit(&self, signal: &Signal, portfolio_state: &PortfolioState) -> bool { ... }
    
    fn check_concentration_limit(&self, signal: &Signal, portfolio_state: &PortfolioState) -> bool { ... }
    
    fn check_frequency_limit(&self, signal: &Signal, portfolio_state: &PortfolioState) -> bool { ... }
    
    fn check_drawdown_limit(&self, signal: &Signal, portfolio_state: &PortfolioState) -> bool { ... }
}
```

**測試策略**:
- 測試單筆交易限額檢查
- 測試集中度限制
- 測試頻率限制
- 測試回撤限制
- 測試多風險條件組合情況

---

### 任務 7: 回測快取機制實現 (P1, C2, 2天)
**前置依賴**: 任務 5

**目標文件**: `src/backtest/cache.rs`

**功能描述**:
實現基本的回測數據和結果快取機制，專注於內存快取，滿足一般規模回測的需求。

**實現要點**:
1. 創建 BacktestCache 結構體，管理基本的交易和持倉資料
2. 實現簡單的內存快取邏輯，使用固定容量限制
3. 使用基本的環形緩衝區策略（當達到容量上限時移除最舊數據）
4. 提供基本的資料存取功能

**API 設計**:
```rust
pub struct BacktestCache {
    backtest_id: String,
    positions: Vec<PositionSnapshot>,
    trades: Vec<Trade>,
    equity_points: Vec<EquityCurvePoint>,
    max_items_per_collection: usize,  // 每種集合的最大項目數
}

impl BacktestCache {
    pub fn new(backtest_id: &str, max_items_per_collection: usize) -> Self { ... }
    
    pub fn add_position_snapshot(&mut self, time: DateTime<Utc>, snapshot: PositionSnapshot) -> Result<(), Error> { ... }
    
    pub fn add_trade(&mut self, trade: Trade) -> Result<(), Error> { ... }
    
    pub fn add_equity_point(&mut self, point: EquityCurvePoint) -> Result<(), Error> { ... }
    
    pub fn get_all_positions(&self) -> &[PositionSnapshot] { ... }
    
    pub fn get_all_trades(&self) -> &[Trade] { ... }
    
    pub fn get_all_equity_points(&self) -> &[EquityCurvePoint] { ... }
    
    pub fn clear(&mut self) { ... }
}
```

**測試策略**:
- 測試基本的資料添加和獲取功能
- 測試容量管理（達到最大容量時的行為）
- 測試快取清理功能

---

### 任務 8: DSL運行時回測擴展 (P1, C2, 2天)
**前置依賴**: 任務 4

**目標文件**: `src/dsl/runtime.rs`

**功能描述**:
擴展DSL運行時，添加回測專用函數和數據訪問能力，確保策略代碼能夠在回測環境中有效執行。

**實現要點**:
1. 設計回測專用DSL擴展函數庫
2. 實現回測數據訪問函數
   - 歷史數據查詢
   - 技術指標計算  
3. 實現交易執行函數
   - 下單函數
   - 持倉查詢
   - 資金查詢
4. 實現回測環境互動函數
   - 時間查詢
   - 參數訪問
5. 確保函數安全性和錯誤處理
6. 與沙箱環境整合

**API 設計**:
```rust
// 在 stdlib.rs 中添加
pub fn register_backtest_functions(engine: &mut DslEngine) -> Result<(), Error> {
    // 時間和數據訪問
    engine.register_function("current_date", |args| { ... })?;
    engine.register_function("get_bars", |args| { ... })?;
    engine.register_function("get_price", |args| { ... })?;
    
    // 技術指標
    engine.register_function("sma", |args| { ... })?;
    engine.register_function("ema", |args| { ... })?;
    engine.register_function("rsi", |args| { ... })?;
    engine.register_function("macd", |args| { ... })?;
    
    // 交易下單
    engine.register_function("buy", |args| { ... })?;
    engine.register_function("sell", |args| { ... })?;
    engine.register_function("order", |args| { ... })?;
    
    // 倉位和資金查詢
    engine.register_function("get_position", |args| { ... })?;
    engine.register_function("get_positions", |args| { ... })?;
    engine.register_function("get_cash", |args| { ... })?;
    
    Ok(())
}
```

**測試策略**:
- 測試各時間函數
- 測試數據訪問函數
- 測試指標計算函數
- 測試下單函數
- 測試資產查詢函數

---

### 任務 9: 沙箱回測整合 (P1, C2, 2天)
**前置依賴**: 任務 4, 任務 8

**目標文件**: `src/runtime/sandbox.rs`

**功能描述**:
擴展沙箱模組，添加回測專用功能，確保策略能在受控環境中與回測引擎協作。

**實現要點**:
1. 創建回測專用沙箱配置和建構器
2. 實現市場數據注入機制
4. 實現策略步驟執行與監控
5. 添加回測專用錯誤處理
6. 與事件發布/訂閱系統整合
7. 實現回測狀態和進度報告機制

**API 設計**:
```rust
// 擴展現有Sandbox實現
impl Sandbox {
    // 回測專用方法
    pub fn update_market_data(&self, time: DateTime<Utc>, data: &MarketData) -> RuntimeResult<()> { ... }
    
    pub fn execute_strategy_step(&self) -> RuntimeResult<Vec<Signal>> { ... }
    
    pub fn execute_final_step(&self) -> RuntimeResult<()> { ... }
    
    // 監控執行的輔助方法
    fn monitor_execution<F, T>(&self, operation: F) -> RuntimeResult<T>
    where
        F: FnOnce() -> Result<T, RuntimeError>,
    { ... }
}
```

**測試策略**:
- 測試市場數據注入
- 測試策略步驟執行
- 測試錯誤處理
- 測試與回測引擎的協作

---

## P2: 輔助功能模組 (中優先級)

### 任務 10: 回測結果存儲實現 (P2, C2, 2天)
**前置依賴**: 任務 5, 任務 7

**目標文件**: `src/backtest/storage.rs`

**功能描述**:
實現回測結果的持久化存儲，包括交易記錄、持倉快照、權益曲線和績效指標，符合數據庫結構設計。

**實現要點**:
1. 創建 `ResultStorage` 結構體，管理回測結果的存儲
2. 實現基本結果保存功能
3. 實現詳細交易數據的高效批量存儲
4. 實現持倉快照和權益曲線的存儲
5. 提供結果查詢和加載功能
6. 整合 TimescaleDB 批量插入優化
7. 實現數據壓縮儲存選項

**API 設計**:
```rust
pub struct ResultStorage {
    db_pool: PgPool,
    batch_size: usize,
}

impl ResultStorage {
    pub fn new(db_pool: PgPool) -> Self { ... }
    
    pub async fn save_result(&self, result: &BacktestResult) -> Result<(), Error> { ... }
    
    pub async fn save_detailed_data(
        &self,
        backtest_id: &str,
        trades: &[Trade],
        positions: &[PositionSnapshot],
    ) -> Result<(), Error> { ... }
    
    pub async fn load_result(&self, backtest_id: &str) -> Result<Option<BacktestResult>, Error> { ... }
    
    pub async fn load_trades(&self, backtest_id: &str) -> Result<Vec<Trade>, Error> { ... }
    
    pub async fn load_equity_curve(&self, backtest_id: &str) -> Result<Vec<EquityCurvePoint>, Error> { ... }
}
```

**測試策略**:
- 測試基本結果存儲
- 測試批量數據儲存
- 測試結果加載功能
- 測試數據庫連接池使用效率
- 測試錯誤處理和恢復

---

### 任務 11: 回測指標計算增強 (P2, C1, 1天)

**目標文件**: `src/backtest/metrics.rs`

**功能描述**:
增強回測指標計算模組，提供更全面的績效評估指標，包括夏普比率、最大回撤、年化收益等，以及自定義指標計算框架。

**實現要點**:
1. 完善 `MetricsCalculator` 類，添加更多指標計算
2. 實現更多標準金融指標：
   - 索提諾比率
   - 卡爾馬比率
   - 資訊比率
   - 阿爾法和貝塔
   - 最大連續虧損
3. 支持自定義指標註冊
4. 支持回測結果與基準的比較
5. 實現統計顯著性檢驗

**API 設計**:
```rust
pub struct MetricsCalculator {
    config: MetricsConfig,
}

impl MetricsCalculator {
    pub fn new(config: MetricsConfig) -> Self { ... }
    
    // 計算所有標準指標
    pub fn calculate_all_metrics(
        &self,
        equity_curve: &[EquityCurvePoint],
        trades: &[Trade],
        initial_capital: f64,
    ) -> HashMap<String, f64> { ... }
    
    // 計算特定指標
    pub fn calculate_sharpe_ratio(&self, returns: &[f64], risk_free_rate: f64) -> f64 { ... }
    
    pub fn calculate_max_drawdown(&self, equity: &[f64]) -> f64 { ... }
    
    pub fn calculate_alpha_beta(&self, returns: &[f64], benchmark_returns: &[f64]) -> (f64, f64) { ... }
    
    // 註冊自定義指標
    pub fn register_custom_metric<F>(&mut self, name: &str, calculator: F) -> Result<(), Error>
    where
        F: Fn(&[EquityCurvePoint], &[Trade], f64) -> f64 + 'static,
    { ... }
}
```

**測試策略**:
- 測試各個指標計算準確性
- 測試基準比較功能
- 測試自定義指標註冊和計算
- 測試極端情況下的指標計算

---

### 任務 12: 回測任務調度系統 (P2, C2, 2天)
**前置依賴**: 任務 4

**目標文件**: `src/backtest/executor.rs`

**功能描述**:
實現回測任務調度系統，支持多個回測任務的並行處理、暫停/繼續、取消等功能，提高系統整體吞吐量。

**實現要點**:
1. 實現 `BacktestExecutor` 結構體，管理多個回測任務的執行
2. 創建任務佇列和優先級機制
3. 實現任務狀態管理和監控
4. 添加並行執行控制，包括並行度限制
5. 設計任務取消和暫停/恢復機制
6. 實現任務結果收集和錯誤處理
7. 添加進度報告機制

**API 設計**:
```rust
pub struct BacktestExecutor {
    engine: Arc<BacktestEngine>,
    task_queue: Arc<Mutex<VecDeque<BacktestTask>>>,
    max_concurrent_tasks: usize,
    current_tasks: AtomicUsize,
}

impl BacktestExecutor {
    pub fn new(engine: Arc<BacktestEngine>, max_concurrent_tasks: usize) -> Self { ... }
    
    pub async fn submit_task(&self, task: BacktestTask) -> Result<String, Error> { ... }
    
    pub async fn execute_batch_tasks(&self, tasks: Vec<BacktestTask>) -> Result<Vec<BacktestResult>, Error> { ... }
    
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), Error> { ... }
    
    pub async fn pause_task(&self, task_id: &str) -> Result<(), Error> { ... }
    
    pub async fn resume_task(&self, task_id: &str) -> Result<(), Error> { ... }
    
    pub async fn get_task_status(&self, task_id: &str) -> Result<TaskStatus, Error> { ... }
}
```

**測試策略**:
- 測試單一任務執行
- 測試並行任務執行
- 測試任務取消功能
- 測試暫停/恢復功能
- 測試錯誤恢復和報告

---

## P3: 高級功能與優化 (較低優先級)

### 任務 13: 回測報告生成器 (P3, C2, 2天)
**前置依賴**: 任務 11

**目標文件**: `src/backtest/report.rs`

**功能描述**:
實現回測結果可視化和報告生成系統，將回測結果轉換為易於理解的報告格式（HTML、JSON、CSV等）。

**實現要點**:
1. 創建 `ReportGenerator` 結構體，支持不同格式的報告生成
2. 實現 HTML 報告模板
3. 實現 JSON 和 CSV 導出
4. 生成權益曲線、回撤、每月回報等圖表
5. 實現交易明細和績效指標匯總
6. 支持自定義報告模板
7. 比較多策略回測結果

**API 設計**:
```rust
pub struct ReportGenerator {
    config: ReportConfig,
    template_engine: TemplateEngine,
}

impl ReportGenerator {
    pub fn new(config: ReportConfig) -> Self { ... }
    
    pub async fn generate_html_report(&self, result: &BacktestResult) -> Result<String, Error> { ... }
    
    pub fn generate_json_report(&self, result: &BacktestResult) -> Result<String, Error> { ... }
    
    pub fn generate_csv_report(&self, result: &BacktestResult) -> Result<String, Error> { ... }
    
    pub async fn generate_charts(&self, result: &BacktestResult, output_dir: &Path) -> Result<Vec<String>, Error> { ... }
    
    pub async fn compare_strategies(&self, results: &[BacktestResult], output_file: &Path) -> Result<(), Error> { ... }
}
```

**測試策略**:
- 測試 HTML 報告生成
- 測試 JSON 和 CSV 導出
- 測試圖表生成
- 測試多策略比較報告
- 測試自定義模板系統

---

### (暫緩) 任務 14: 回測資料並行處理器 (P3, C2, 2天)
**前置依賴**: 任務 7

**目標文件**: `src/backtest/parallel.rs`

**功能描述**:
實現資料並行處理器，提高大規模回測數據處理效率，特別是針對大量金融商品的多參數回測場景。

**實現要點**:
1. 創建 `ParallelDataProcessor` 結構體
2. 實現自動數據分片算法
3. 使用 `rayon` 或 `tokio` 實現並行數據處理
4. 優化記憶體使用和數據傳輸
5. 設計可配置的平衡策略（CPU使用率 vs. 記憶體使用率）
6. 實現進度追蹤和中間結果合併

**API 設計**:
```rust
pub struct ParallelDataProcessor<T> {
    config: ParallelConfig,
    process_fn: Arc<dyn Fn(&[T]) -> Result<Vec<R>, Error> + Send + Sync>,
}

impl<T: Send + Sync + 'static> ParallelDataProcessor<T> {
    pub fn new(config: ParallelConfig) -> Self { ... }
    
    pub fn with_processor<F>(mut self, process_fn: F) -> Self 
    where
        F: Fn(&[T]) -> Result<Vec<R>, Error> + Send + Sync + 'static,
    { ... }
    
    pub async fn process(&self, data: Vec<T>) -> Result<Vec<R>, Error> { ... }
    
    pub async fn process_chunks(&self, data: Vec<T>, chunk_size: usize) -> Result<Vec<R>, Error> { ... }
    
    fn optimal_chunk_size(&self, data_len: usize) -> usize { ... }
}
```

**測試策略**:
- 測試不同數據量下的處理效率
- 測試不同並行度設置的效果
- 測試進度報告功能
- 測試記憶體使用優化效果
- 測試錯誤處理邏輯

---
---

## 綜合測試計劃

### 任務 16: 整合測試系統 (P2, C2, 2天)
**前置依賴**: 任務 1, 任務 4

**目標文件**: `tests/backtest_integration_tests.rs`

**功能描述**:
建立整合測試系統，確保各個模組協同工作正常，驗證完整的回測流程。

**實現要點**:
1. 創建回測完整流程的集成測試
2. 設計測試策略和市場數據
3. 實現自動化測試流程
4. 添加效能測試基準
5. 測試不同市場條件和策略行為
6. 驗證回測結果的一致性

**測試案例**:
1. 簡單移動平均策略回測
2. 動態倉位調整策略回測
3. 多資產組合策略回測
4. 高頻交易策略回測
5. 極端市場條件測試

---

## 依賴關係圖

```
任務1 (執行引擎) ─┬─→ 任務2 (訂單模擬器) ──┬─→ 任務3 (訂單匹配引擎)
                 │                       │
                 └─→ 任務4 (五階段流程) ─┬─→ 任務5 (回測上下文) ─┬─→ 任務7 (快取機制) ──┬─→ 任務10 (結果存儲)
                                        │                       │                      │
                                        └─→ 任務8 (DSL擴展) ─────┬─→ 任務9 (沙箱整合)   └─→ 任務14 (並行處理)
                                                                │
                                        任務6 (風險管理) ────────┘
                                                                │
                                        任務11 (指標計算) ──────→ 任務13 (報告生成)
                                                                │
                                        任務12 (任務調度) ───────┘
                                                                │
                                        任務15 (版本回滾) ───────┘
                                                                │
                                                              任務16 (整合測試)
```

## 實施計劃

1. **第一階段 (1-2週)**:
   - 完成任務1-5：核心執行模組和回測流程
   - 初步整合測試

2. **第二階段 (1-2週)**:
   - 完成任務6-9：風險管理、快取、DSL擴展和沙箱整合
   - 更全面的功能測試

3. **第三階段 (1週)**:
   - 完成任務10-12：結果存儲、指標計算和任務調度
   - 效能測試和優化

4. **第四階段 (1週)**:
   - 完成任務13-16：報告生成、並行處理、版本回滾和整合測試
   - 系統壓力測試和使用者測試