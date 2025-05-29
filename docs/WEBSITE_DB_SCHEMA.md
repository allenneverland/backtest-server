# StratPlat Website DB 架構設計

## 概述
本文檔描述 StratPlat 專案的 Website DB 架構設計。StratPlat 是三資料庫架構中的前端平台，只管理自己的 Website DB。系統採用 PostgreSQL 作為主要資料庫，用於儲存用戶資料、策略定義、回測結果等核心業務資料。

## 三資料庫架構中的位置
- **Website DB (PostgreSQL)**: 由 StratPlat 擁有和管理
- **History Data DB (TimescaleDB)**: 由外部市場數據專案維護，StratPlat 無直接連接
- **Backtest DB (TimescaleDB)**: 由 BacktestServer 專案管理，StratPlat 通過 RabbitMQ 間接獲取結果

## 資料庫選型理由
- **PostgreSQL (Website DB)**: 
  - 強大的 ACID 支持，確保數據一致性
  - JSON/JSONB 支持，適合儲存動態結構資料（如策略配置）
  - 良好的擴展性和性能
  - 豐富的索引類型支持
- **Redis**: 
  - 用於快取和會話管理
  - 支持發布/訂閱模式，適合即時通知
  - 快取回測結果和常用數據

## 核心資料表設計

### 1. users (用戶表)
儲存系統用戶的基本資訊和認證資料。

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(50) UNIQUE NOT NULL,
    hashed_password VARCHAR(255) NOT NULL,
    full_name VARCHAR(100),
    phone_number VARCHAR(20),  -- 用戶手機號碼
    is_active BOOLEAN DEFAULT FALSE,
    is_superuser BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_created_at ON users(created_at);
```

### 2. strategies (策略表)
儲存用戶創建的交易策略，包括 DSL 代碼和視覺化配置。

```sql
CREATE TABLE strategies (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name VARCHAR(200) NOT NULL,
    description TEXT,
    dsl_code TEXT NOT NULL,  -- DSL 策略代碼
    visual_config TEXT,       -- JSON 格式的視覺化編輯器配置
    owner_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_public BOOLEAN DEFAULT FALSE,  -- 是否公開分享
    tags TEXT[],              -- 策略標籤陣列
    version INTEGER DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_strategies_owner_id ON strategies(owner_id);
CREATE INDEX idx_strategies_is_public ON strategies(is_public);
CREATE INDEX idx_strategies_created_at ON strategies(created_at);
CREATE INDEX idx_strategies_tags ON strategies USING GIN(tags);
```

### 3. backtests (回測表)
儲存策略的回測任務和基本結果。詳細的交易記錄、持倉歷史和績效指標存在相關聯的資料表中。

```sql
CREATE TABLE backtests (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name VARCHAR(200) NOT NULL,
    strategy_id INTEGER NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
    owner_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    initial_capital DECIMAL(20, 2) DEFAULT 100000.00,
    status VARCHAR(20) DEFAULT 'pending',  -- pending, running, completed, failed
    results TEXT,            -- JSON 格式的摘要結果（總報酬率、夏普比率等）
    error_message TEXT,      -- 錯誤訊息（如果失敗）
    execution_time INTEGER,  -- 執行時間（毫秒）
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP WITH TIME ZONE
);

-- 索引
CREATE INDEX idx_backtests_strategy_id ON backtests(strategy_id);
CREATE INDEX idx_backtests_owner_id ON backtests(owner_id);
CREATE INDEX idx_backtests_status ON backtests(status);
CREATE INDEX idx_backtests_created_at ON backtests(created_at);
```

### 4. backtest_trades (回測交易記錄表)
儲存回測中產生的每筆交易詳細記錄。

```sql
CREATE TABLE backtest_trades (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    backtest_id INTEGER NOT NULL REFERENCES backtests(id) ON DELETE CASCADE,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    action VARCHAR(10) NOT NULL,  -- buy, sell
    quantity DECIMAL(20, 8) NOT NULL,
    price DECIMAL(20, 8) NOT NULL,
    commission DECIMAL(20, 8) DEFAULT 0,
    slippage DECIMAL(20, 8) DEFAULT 0,
    pnl DECIMAL(20, 8),  -- 實現損益
    position_value DECIMAL(20, 8),  -- 交易後的持倉價值
    cash_balance DECIMAL(20, 8),  -- 交易後的現金餘額
    notes TEXT  -- 交易備註或信號來源
);

-- 索引
CREATE INDEX idx_backtest_trades_backtest_id ON backtest_trades(backtest_id);
CREATE INDEX idx_backtest_trades_timestamp ON backtest_trades(timestamp);
CREATE INDEX idx_backtest_trades_symbol ON backtest_trades(symbol);
CREATE INDEX idx_backtest_trades_backtest_timestamp ON backtest_trades(backtest_id, timestamp);
```

### 5. backtest_positions (回測持倉記錄表)
儲存回測過程中的持倉快照，通常是每日結算時的持倉狀態。

```sql
CREATE TABLE backtest_positions (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    backtest_id INTEGER NOT NULL REFERENCES backtests(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    quantity DECIMAL(20, 8) NOT NULL,
    avg_price DECIMAL(20, 8) NOT NULL,  -- 平均成本價
    current_price DECIMAL(20, 8) NOT NULL,  -- 當前市價
    market_value DECIMAL(20, 8) NOT NULL,  -- 市值
    unrealized_pnl DECIMAL(20, 8),  -- 未實現損益
    weight DECIMAL(5, 4)  -- 持倉權重（佔總資產比例）
);

-- 索引
CREATE INDEX idx_backtest_positions_backtest_id ON backtest_positions(backtest_id);
CREATE INDEX idx_backtest_positions_date ON backtest_positions(date);
CREATE INDEX idx_backtest_positions_symbol ON backtest_positions(symbol);
CREATE INDEX idx_backtest_positions_backtest_date ON backtest_positions(backtest_id, date);
```

### 6. backtest_performance_metrics (回測績效指標表)
儲存回測的各項績效指標，包含風險調整後報酬等進階指標。

```sql
CREATE TABLE backtest_performance_metrics (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    backtest_id INTEGER NOT NULL REFERENCES backtests(id) ON DELETE CASCADE,
    -- 報酬指標
    total_return DECIMAL(10, 6),  -- 總報酬率
    annualized_return DECIMAL(10, 6),  -- 年化報酬率
    -- 風險指標
    volatility DECIMAL(10, 6),  -- 波動率
    max_drawdown DECIMAL(10, 6),  -- 最大回撤
    max_drawdown_duration INTEGER,  -- 最大回撤持續天數
    -- 風險調整報酬
    sharpe_ratio DECIMAL(10, 4),  -- 夏普比率
    sortino_ratio DECIMAL(10, 4),  -- 索提諾比率
    calmar_ratio DECIMAL(10, 4),  -- 卡瑪比率
    -- 交易統計
    total_trades INTEGER,  -- 總交易次數
    winning_trades INTEGER,  -- 獲利交易次數
    losing_trades INTEGER,  -- 虧損交易次數
    win_rate DECIMAL(5, 4),  -- 勝率
    profit_factor DECIMAL(10, 4),  -- 獲利因子
    avg_win DECIMAL(20, 8),  -- 平均獲利
    avg_loss DECIMAL(20, 8),  -- 平均虧損
    max_consecutive_wins INTEGER,  -- 最大連續獲利次數
    max_consecutive_losses INTEGER,  -- 最大連續虧損次數
    -- 其他指標
    final_equity DECIMAL(20, 2),  -- 最終權益
    peak_equity DECIMAL(20, 2),  -- 最高權益
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(backtest_id)
);

-- 索引
CREATE INDEX idx_backtest_metrics_backtest_id ON backtest_performance_metrics(backtest_id);
```

### 7. user_sessions (用戶會話表) - 未來擴展
儲存用戶登入會話資訊，用於安全審計和多設備管理。

```sql
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    device_info JSONB,       -- 設備資訊（瀏覽器、OS等）
    ip_address INET,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_activity TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_token_hash ON user_sessions(token_hash);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);
```

### 8. strategy_versions (策略版本控制表) - 未來擴展
追蹤策略的版本歷史，支持版本回溯。

```sql
CREATE TABLE strategy_versions (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    strategy_id INTEGER NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    dsl_code TEXT NOT NULL,
    visual_config TEXT,
    change_description TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(strategy_id, version_number)
);

-- 索引
CREATE INDEX idx_strategy_versions_strategy_id ON strategy_versions(strategy_id);
CREATE INDEX idx_strategy_versions_created_at ON strategy_versions(created_at);
```

### 9. user_favorites (用戶收藏表) - 未來擴展
允許用戶收藏公開的策略。

```sql
CREATE TABLE user_favorites (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    strategy_id INTEGER NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, strategy_id)
);

-- 索引
CREATE INDEX idx_user_favorites_user_id ON user_favorites(user_id);
CREATE INDEX idx_user_favorites_strategy_id ON user_favorites(strategy_id);
```

### 10. api_keys (API 金鑰表) - 未來擴展
用於第三方 API 整合和程式化交易。

```sql
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    permissions JSONB DEFAULT '{}',  -- API 權限配置
    last_used_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_is_active ON api_keys(is_active);
```

### 11. audit_logs (審計日誌表) - 未來擴展
記錄重要的系統操作和用戶活動。

```sql
CREATE TABLE audit_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50),
    resource_id INTEGER,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
```

### 12. subscription_plans (訂閱方案表)
定義系統提供的訂閱方案層級。

```sql
CREATE TABLE subscription_plans (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name VARCHAR(50) UNIQUE NOT NULL,  -- 方案名稱 (free, basic, pro, enterprise)
    display_name VARCHAR(100) NOT NULL,  -- 顯示名稱
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,  -- 月費
    currency VARCHAR(3) DEFAULT 'USD',
    -- 功能限制
    max_strategies INTEGER,  -- 最大策略數量
    max_backtests_per_month INTEGER,  -- 每月最大回測次數
    max_concurrent_backtests INTEGER DEFAULT 1,  -- 同時執行回測數
    backtest_priority INTEGER DEFAULT 1,  -- 回測優先級 (1-10)
    data_retention_days INTEGER DEFAULT 30,  -- 數據保留天數
    -- 功能開關
    advanced_analytics BOOLEAN DEFAULT FALSE,  -- 進階分析功能
    api_access BOOLEAN DEFAULT FALSE,  -- API 存取權限
    export_enabled BOOLEAN DEFAULT TRUE,  -- 匯出功能
    realtime_data BOOLEAN DEFAULT FALSE,  -- 即時數據
    -- 其他
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INTEGER DEFAULT 0,  -- 顯示順序
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_subscription_plans_is_active ON subscription_plans(is_active);
CREATE INDEX idx_subscription_plans_sort_order ON subscription_plans(sort_order);

-- 預設方案
INSERT INTO subscription_plans (name, display_name, price, max_strategies, max_backtests_per_month, max_concurrent_backtests, data_retention_days, sort_order)
VALUES 
    ('free', 'Free', 0, 3, 10, 1, 7, 1),
    ('basic', 'Basic', 9.99, 10, 50, 2, 30, 2),
    ('pro', 'Professional', 29.99, 50, 200, 5, 90, 3),
    ('enterprise', 'Enterprise', 99.99, -1, -1, 10, 365, 4);  -- -1 表示無限制
```

### 13. user_subscriptions (用戶訂閱記錄表)
追蹤用戶的訂閱歷史和當前狀態。

```sql
CREATE TABLE user_subscriptions (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_id INTEGER NOT NULL REFERENCES subscription_plans(id),
    status VARCHAR(20) NOT NULL DEFAULT 'active',  -- active, cancelled, expired, suspended
    -- 訂閱期間
    start_date TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    end_date TIMESTAMP WITH TIME ZONE,  -- NULL 表示持續訂閱
    cancelled_at TIMESTAMP WITH TIME ZONE,  -- 取消時間
    -- 付款資訊
    payment_method VARCHAR(50),  -- card, paypal, etc.
    last_payment_date TIMESTAMP WITH TIME ZONE,
    next_payment_date TIMESTAMP WITH TIME ZONE,
    amount_paid DECIMAL(10, 2),
    currency VARCHAR(3) DEFAULT 'USD',
    -- 其他
    auto_renew BOOLEAN DEFAULT TRUE,
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_user_subscriptions_user_id ON user_subscriptions(user_id);
CREATE INDEX idx_user_subscriptions_plan_id ON user_subscriptions(plan_id);
CREATE INDEX idx_user_subscriptions_status ON user_subscriptions(status);
CREATE INDEX idx_user_subscriptions_end_date ON user_subscriptions(end_date);

-- 確保用戶只有一個活躍訂閱
CREATE UNIQUE INDEX idx_user_subscriptions_active_unique ON user_subscriptions(user_id) WHERE status = 'active';
```

### 14. roles (角色表)
定義系統中的角色。

```sql
CREATE TABLE roles (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_roles_id ON roles(id);
```

### 15. permissions (權限表)
定義系統中的權限。

```sql
CREATE TABLE permissions (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name VARCHAR(100) UNIQUE NOT NULL,  -- 格式：resource:action
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_permissions_id ON permissions(id);
```

### 16. role_permissions (角色權限關聯表)
定義角色和權限的多對多關係。

```sql
CREATE TABLE role_permissions (
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id INTEGER NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (role_id, permission_id)
);
```

### 17. user_roles (用戶角色關聯表)
定義用戶和角色的多對多關係。

```sql
CREATE TABLE user_roles (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    assigned_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    PRIMARY KEY (user_id, role_id),
    UNIQUE(user_id, role_id)
);
```

## 資料關係圖

```mermaid
erDiagram
    users ||--o{ strategies : "擁有"
    users ||--o{ backtests : "執行"
    users ||--o{ user_roles : "擁有"
    roles ||--o{ user_roles : "分配給"
    roles ||--o{ role_permissions : "擁有"
    permissions ||--o{ role_permissions : "屬於"
    strategies ||--o{ backtests : "被測試"
    backtests ||--o{ backtest_trades : "包含"
    backtests ||--o{ backtest_positions : "記錄"
    backtests ||--|| backtest_performance_metrics : "產生"
    strategies ||--o{ strategy_versions : "有版本"
    users ||--o{ user_sessions : "登入"
    users ||--o{ user_favorites : "收藏"
    strategies ||--o{ user_favorites : "被收藏"
    users ||--o{ api_keys : "創建"
    users ||--o{ audit_logs : "產生"
    users ||--o{ user_subscriptions : "訂閱"
    subscription_plans ||--o{ user_subscriptions : "被訂閱"
    user_roles ||--| assigned_by : "分配者"

    users {
        int id PK
        string email UK
        string username UK
        string hashed_password
        string full_name
        string phone_number
        boolean is_active
        boolean is_superuser
        timestamp created_at
        timestamp updated_at
    }

    strategies {
        int id PK
        string name
        text description
        text dsl_code
        text visual_config
        int owner_id FK
        boolean is_public
        string[] tags
        int version
        timestamp created_at
        timestamp updated_at
    }

    backtests {
        int id PK
        string name
        int strategy_id FK
        int owner_id FK
        date start_date
        date end_date
        decimal initial_capital
        string status
        text results
        text error_message
        int execution_time
        timestamp created_at
        timestamp updated_at
        timestamp completed_at
    }

    backtest_trades {
        int id PK
        int backtest_id FK
        timestamp timestamp
        string symbol
        string action
        decimal quantity
        decimal price
        decimal commission
        decimal pnl
        decimal position_value
        decimal cash_balance
    }

    backtest_positions {
        int id PK
        int backtest_id FK
        date date
        string symbol
        decimal quantity
        decimal avg_price
        decimal current_price
        decimal market_value
        decimal unrealized_pnl
        decimal weight
    }

    backtest_performance_metrics {
        int id PK
        int backtest_id FK
        decimal total_return
        decimal annualized_return
        decimal volatility
        decimal max_drawdown
        decimal sharpe_ratio
        decimal sortino_ratio
        int total_trades
        int winning_trades
        decimal win_rate
        decimal profit_factor
    }

    subscription_plans {
        int id PK
        string name UK
        string display_name
        decimal price
        int max_strategies
        int max_backtests_per_month
        int max_concurrent_backtests
        int data_retention_days
        boolean advanced_analytics
        boolean api_access
        boolean is_active
    }

    user_subscriptions {
        int id PK
        int user_id FK
        int plan_id FK
        string status
        timestamp start_date
        timestamp end_date
        boolean auto_renew
        decimal amount_paid
    }

    roles {
        int id PK
        string name UK
        text description
        timestamp created_at
        timestamp updated_at
    }

    permissions {
        int id PK
        string name UK
        text description
        timestamp created_at
    }

    user_roles {
        int user_id PK,FK
        int role_id PK,FK
        timestamp assigned_at
        int assigned_by FK
    }

    role_permissions {
        int role_id PK,FK
        int permission_id PK,FK
        timestamp created_at
    }
```

## 資料庫設計原則

### 1. 正規化
- 遵循第三正規化形式（3NF），避免資料冗餘
- 使用外鍵約束確保資料完整性
- 適當使用 JSON 欄位儲存動態結構資料

### 2. 索引策略
- 主鍵自動建立唯一索引
- 外鍵欄位建立索引以提升 JOIN 效能
- 常用查詢條件欄位建立索引
- 使用 GIN 索引支持陣列和 JSON 查詢

### 3. 資料完整性
- 使用 NOT NULL 約束確保必要欄位
- 使用 UNIQUE 約束避免重複資料
- 使用 CHECK 約束驗證資料有效性
- 使用級聯刪除保持引用完整性

### 4. 效能優化
- 適當的欄位類型選擇（避免過度設計）
- 使用部分索引（Partial Index）優化特定查詢
- 考慮資料分區（Partitioning）應對大數據量
- 定期進行 VACUUM 和 ANALYZE 維護

### 5. 安全性考量
- 敏感資料（如密碼）使用雜湊儲存
- 使用 Row Level Security (RLS) 實現細粒度權限控制
- 定期備份和災難恢復計劃
- 審計日誌追蹤重要操作

## 資料遷移策略

### 使用 Alembic 進行版本控制
1. 每次架構變更都創建遷移腳本
2. 遷移腳本包含升級（upgrade）和降級（downgrade）邏輯
3. 在生產環境部署前充分測試遷移腳本
4. 保留完整的遷移歷史記錄

### 遷移最佳實踐
- 避免破壞性變更（如直接刪除欄位）
- 使用多步驟遷移處理複雜變更
- 確保遷移腳本的冪等性
- 為大型資料表遷移制定專門計劃

## 未來擴展考量

### 1. 與 BacktestServer 的整合
- 通過 RabbitMQ 傳送策略和回測參數
- 接收回測結果和狀態更新
- 將結果儲存在 Website DB 中

### 2. 分片和複製
- 水平擴展以應對用戶增長
- 讀寫分離提升查詢效能
- 地理分佈式部署

### 3. 快取層優化
- Redis 快取熱門查詢結果
- 實現寫入時快取失效策略
- 使用 Redis Pub/Sub 同步快取

### 4. 資料倉儲
- 定期將歷史資料歸檔
- 構建資料分析專用的數據倉儲
- 實現 ETL 流程支持商業智能

## 維護和監控

### 1. 定期維護任務
- 每日備份資料庫
- 每週執行 VACUUM ANALYZE
- 每月檢查索引使用情況
- 每季度審查慢查詢日誌

### 2. 監控指標
- 查詢響應時間
- 連接池使用率
- 資料表大小增長
- 索引命中率

### 3. 告警設置
- 磁碟空間不足
- 長時間運行的查詢
- 連接數異常
- 複製延遲（如有主從架構）

## 相關文檔
- [THREE_DB_ARCHITECTURE.md](./THREE_DB_ARCHITECTURE.md) - 三資料庫架構說明
- [PLANNING.md](./PLANNING.md) - 專案整體規劃
- [STRUCTURE.md](./STRUCTURE.md) - 專案結構說明
- Backend 模型定義：`backend/app/models/`
- Alembic 遷移腳本：`backend/alembic/`