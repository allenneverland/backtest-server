#!/bin/bash

# CI 準備狀態檢查腳本
# 快速檢查本地環境是否準備好運行 CI 測試

set -e
set -u

# 顏色設定
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() {
    local status=$1
    local message=$2
    
    case $status in
        "pass")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "fail")
            echo -e "${RED}❌ $message${NC}"
            ;;
        "warn")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "info")
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
    esac
}

check_docker_env() {
    print_status "info" "檢查 Docker 環境..."
    
    if ! command -v docker &> /dev/null; then
        print_status "fail" "Docker 未安裝"
        return 1
    fi
    
    if ! docker info &> /dev/null; then
        print_status "fail" "Docker 服務未運行"
        return 1
    fi
    
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        print_status "fail" "Docker Compose 未安裝"
        return 1
    fi
    
    print_status "pass" "Docker 環境正常"
    return 0
}

check_rust_toolchain() {
    print_status "info" "檢查 Rust 工具鏈..."
    
    if ! command -v cargo &> /dev/null; then
        print_status "fail" "Cargo 未安裝"
        return 1
    fi
    
    if ! cargo --version | grep -q "1\."; then
        print_status "fail" "Cargo 版本異常"
        return 1
    fi
    
    # 檢查 rustfmt
    if ! cargo fmt --version &> /dev/null; then
        print_status "warn" "rustfmt 未安裝，執行: rustup component add rustfmt"
    else
        print_status "pass" "rustfmt 可用"
    fi
    
    # 檢查 clippy
    if ! cargo clippy --version &> /dev/null; then
        print_status "warn" "clippy 未安裝，執行: rustup component add clippy"
    else
        print_status "pass" "clippy 可用"
    fi
    
    print_status "pass" "Rust 工具鏈基本正常"
    return 0
}

check_project_structure() {
    print_status "info" "檢查專案結構..."
    
    required_files=(
        "Cargo.toml"
        "Makefile.toml"
        "docker-compose.yml"
        "src/lib.rs"
        "migrations"
        "scripts/init-market-db.sql"
    )
    
    for file in "${required_files[@]}"; do
        if [[ ! -e "$file" ]]; then
            print_status "fail" "缺少必要文件: $file"
            return 1
        fi
    done
    
    print_status "pass" "專案結構完整"
    return 0
}

check_environment_variables() {
    print_status "info" "檢查環境變數..."
    
    # 檢查是否在開發環境中定義了必要的配置
    if [[ -f ".env" ]]; then
        print_status "pass" "找到 .env 文件"
    else
        print_status "warn" "未找到 .env 文件（CI 會使用默認值）"
    fi
    
    if [[ -f "config/development.toml" ]]; then
        print_status "pass" "開發配置文件存在"
    else
        print_status "fail" "缺少開發配置文件"
        return 1
    fi
    
    return 0
}

check_dependencies() {
    print_status "info" "檢查系統依賴..."
    
    # 檢查是否有 cargo-make
    if ! command -v cargo-make &> /dev/null; then
        print_status "warn" "cargo-make 未安裝，執行: cargo install cargo-make"
    else
        print_status "pass" "cargo-make 可用"
    fi
    
    # 檢查是否有 jq（用於解析 JSON）
    if ! command -v jq &> /dev/null; then
        print_status "warn" "jq 未安裝（推薦安裝用於腳本）"
    else
        print_status "pass" "jq 可用"
    fi
    
    return 0
}

check_git_status() {
    print_status "info" "檢查 Git 狀態..."
    
    if ! git rev-parse --is-inside-work-tree &> /dev/null; then
        print_status "fail" "不在 Git 倉庫中"
        return 1
    fi
    
    # 檢查是否有未提交的更改
    if ! git diff --quiet; then
        print_status "warn" "有未暫存的更改"
    fi
    
    if ! git diff --cached --quiet; then
        print_status "warn" "有已暫存但未提交的更改"
    fi
    
    # 檢查當前分支
    current_branch=$(git branch --show-current)
    print_status "info" "當前分支: $current_branch"
    
    return 0
}

check_disk_space() {
    print_status "info" "檢查磁碟空間..."
    
    # 檢查可用空間（至少需要 2GB）
    available_space=$(df . | tail -1 | awk '{print $4}')
    available_gb=$((available_space / 1024 / 1024))
    
    if [[ $available_gb -lt 2 ]]; then
        print_status "warn" "可用磁碟空間不足 2GB (當前: ${available_gb}GB)"
    else
        print_status "pass" "磁碟空間充足 (${available_gb}GB 可用)"
    fi
    
    return 0
}

quick_connectivity_test() {
    print_status "info" "快速連接測試..."
    
    # 嘗試啟動 Docker 服務並檢查連接性
    if docker compose ps &> /dev/null; then
        services_running=$(docker compose ps | grep -c " Up " || echo "0")
        
        if [[ $services_running -gt 0 ]]; then
            print_status "pass" "Docker 服務運行中 ($services_running 個服務)"
        else
            print_status "warn" "Docker 服務未運行，建議執行: cargo make docker-up"
        fi
    else
        print_status "warn" "無法檢查 Docker 服務狀態"
    fi
    
    return 0
}

show_recommendations() {
    echo ""
    print_status "info" "建議的 CI 測試流程："
    echo "  1. 快速檢查:     ./scripts/test-ci-commands.sh --quick"
    echo "  2. 完整測試:     ./scripts/test-ci-commands.sh"
    echo "  3. 只檢查格式:   ./scripts/test-ci-commands.sh --format-only"
    echo "  4. 只執行測試:   ./scripts/test-ci-commands.sh --test-only"
    echo ""
    print_status "info" "常用修復命令："
    echo "  - 修復格式:      cargo make docker-c cargo fmt"
    echo "  - 啟動服務:      cargo make docker-up"
    echo "  - 查看日誌:      cargo make docker-logs"
    echo "  - 重建環境:      cargo make docker-clean && cargo make docker-build"
}

main() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}CI 準備狀態檢查${NC}"
    echo -e "${BLUE}========================================${NC}"
    
    local checks_passed=0
    local total_checks=0
    
    # 執行所有檢查
    for check_func in check_docker_env check_rust_toolchain check_project_structure check_environment_variables check_dependencies check_git_status check_disk_space quick_connectivity_test; do
        ((total_checks++))
        if $check_func; then
            ((checks_passed++))
        fi
        echo ""
    done
    
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}檢查結果: $checks_passed/$total_checks 通過${NC}"
    echo -e "${BLUE}========================================${NC}"
    
    if [[ $checks_passed -eq $total_checks ]]; then
        print_status "pass" "環境準備就緒，可以運行 CI 測試"
        show_recommendations
        exit 0
    else
        print_status "warn" "部分檢查未通過，建議修復後再運行 CI 測試"
        show_recommendations
        exit 1
    fi
}

# 處理命令行參數
case "${1:-}" in
    --help)
        echo "使用方法: $0"
        echo ""
        echo "此腳本檢查本地環境是否準備好運行 CI 測試"
        echo "包括 Docker、Rust 工具鏈、專案結構等檢查"
        exit 0
        ;;
    "")
        main
        ;;
    *)
        print_status "fail" "未知選項: $1"
        echo "使用 --help 查看使用說明"
        exit 1
        ;;
esac