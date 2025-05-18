#!/bin/bash
set -e

# 顏色定義
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 確保提供了遷移名稱
if [ "$#" -ne 1 ]; then
    echo -e "${RED}錯誤: 請提供遷移名稱${NC}"
    echo "用法: $0 <遷移名稱>"
    echo "示例: $0 add_user_table"
    exit 1
fi

# 遷移名稱
MIGRATION_NAME=$1

# 確保遷移目錄存在
mkdir -p migrations

# 獲取最新的版本號
LATEST_VERSION=$(ls -1 migrations/ 2>/dev/null | grep -E '^V[0-9]+__.*\.sql$' | sed -E 's/V([0-9]+)__.*/\1/' | sort -n | tail -1)

# 如果沒有找到遷移，從1開始
if [ -z "$LATEST_VERSION" ]; then
    NEXT_VERSION=1
else
    NEXT_VERSION=$((LATEST_VERSION + 1))
fi

# 創建遷移文件
MIGRATION_FILE="migrations/${NEXT_VERSION}__${MIGRATION_NAME}.sql"

echo -e "${YELLOW}創建新的遷移: ${MIGRATION_FILE}${NC}"

# 創建文件並寫入模板
cat > "$MIGRATION_FILE" << EOF
-- 遷移: ${MIGRATION_NAME}
-- 版本: ${NEXT_VERSION}
-- 創建時間: $(date)

-- 在此處添加您的 SQL 語句
-- 範例:
-- CREATE TABLE example (
--     id SERIAL PRIMARY KEY,
--     name VARCHAR(100) NOT NULL,
--     created_at TIMESTAMPTZ NOT NULL DEFAULT now()
-- );

EOF

echo -e "${GREEN}遷移文件已創建: ${MIGRATION_FILE}${NC}"
echo "您現在可以編輯此文件添加 SQL 語句。" 