-- 策略定義表（整合所有策略相關信息）
CREATE TABLE strategy (
    strategy_id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    version VARCHAR(50) NOT NULL,
    code TEXT NOT NULL,
    code_path VARCHAR(255),
    parameters JSONB NOT NULL DEFAULT '{}',
    active BOOLEAN DEFAULT true,
    author VARCHAR(100),
    tags VARCHAR[] NOT NULL DEFAULT '{}',
    dependencies VARCHAR[] NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name, version)
);

-- 創建索引
CREATE INDEX idx_strategy_name_version ON strategy(name, version);
CREATE INDEX idx_strategy_active ON strategy(active);
CREATE INDEX idx_strategy_tags ON strategy USING GIN (tags);

-- 策略版本表（簡化版本管理）
CREATE TABLE strategy_version (
    version_id SERIAL PRIMARY KEY,
    strategy_id INTEGER NOT NULL REFERENCES strategy(strategy_id),
    version VARCHAR(50) NOT NULL,
    source_path VARCHAR(255) NOT NULL,
    description TEXT,
    is_stable BOOLEAN NOT NULL DEFAULT false,
    created_by VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(strategy_id, version)
);

-- 創建索引
CREATE INDEX idx_strategy_versions_strategy_id ON strategy_version(strategy_id);
CREATE INDEX idx_strategy_versions_is_stable ON strategy_version(is_stable); 