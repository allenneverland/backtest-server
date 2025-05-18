#!/bin/bash
set -e

# 顏色定義
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}===== 開始執行數據庫遷移 =====${NC}"

# 檢查遷移文件是否存在
if [ ! -d "migrations" ] || [ -z "$(ls -A migrations 2>/dev/null)" ]; then
    echo -e "${YELLOW}警告: 遷移目錄不存在或為空 (migrations)${NC}"
    echo -e "${GREEN}✅ 沒有遷移需要執行${NC}"
    echo -e "${YELLOW}===== 遷移程序結束 =====${NC}"
    exit 0
fi

# 創建擴展
echo -e "${YELLOW}創建必要的擴展...${NC}"

# 檢查是否在 Docker 環境中
if [[ -n "$(docker-compose ps -q db 2>/dev/null)" ]] && [[ "$(docker-compose ps -q db 2>/dev/null)" != "" ]]; then
    # 確保用戶名和密碼
    if [ -z "$POSTGRES_USER" ]; then
        POSTGRES_USER="postgres"
    fi
    
    if [ -z "$POSTGRES_DB" ]; then
        POSTGRES_DB="postgres"
    fi
    
    docker-compose exec db psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
        CREATE EXTENSION IF NOT EXISTS timescaledb;
EOSQL
else
    # 確保環境變數存在
    if [ -z "$POSTGRES_USER" ]; then
        echo -e "${YELLOW}警告: POSTGRES_USER 環境變數未設置，使用默認值 'postgres'${NC}"
        POSTGRES_USER="postgres"
    fi
    
    if [ -z "$POSTGRES_DB" ]; then
        echo -e "${YELLOW}警告: POSTGRES_DB 環境變數未設置，使用默認值 'postgres'${NC}"
        POSTGRES_DB="postgres"
    fi
    
    # 嘗試創建擴展，但忽略錯誤（本地開發環境可能沒有安裝這些擴展）
    set +e
    
    # 嘗試創建 timescaledb 擴展
    psql -v ON_ERROR_STOP=0 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -c "CREATE EXTENSION IF NOT EXISTS timescaledb;" 2>/dev/null
    if [ $? -ne 0 ]; then
        echo -e "${YELLOW}警告: 無法創建 timescaledb 擴展，這在本地開發環境中是正常的${NC}"
    fi
    
    set -e
fi

echo -e "${GREEN}✅ 擴展設置完成${NC}"

# 運行遷移
echo -e "${YELLOW}🔄 運行 SQL 遷移...${NC}"

cd migrations
for migration in $(ls -v *.sql 2>/dev/null)
do
    echo -e "${YELLOW}🔄 執行遷移: $migration${NC}"
    
    # 檢查是否在 Docker 環境中
    if [[ -n "$(docker-compose ps -q db 2>/dev/null)" ]] && [[ "$(docker-compose ps -q db 2>/dev/null)" != "" ]]; then
        if ! docker-compose exec -T db psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -f "/app/migrations/$migration"; then
            echo -e "${RED}❌ 遷移 $migration 失敗${NC}"
            exit 1
        fi
    else
        if ! psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -f "$migration"; then
            echo -e "${RED}❌ 遷移 $migration 失敗${NC}"
            exit 1
        fi
    fi
done
cd ..

echo -e "${GREEN}✅ 數據庫遷移完成${NC}"
echo -e "${YELLOW}===== 遷移程序結束 =====${NC}" 