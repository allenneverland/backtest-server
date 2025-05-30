#!/bin/bash

# 本地 CI 測試腳本 - 模擬 GitHub Actions CI 流程
# 執行與 CI 相同的檢查和測試，確保在推送前驗證所有內容

set -e  # 遇到錯誤立即退出
set -u  # 使用未定義變數時退出

# 顏色設定
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 函數：印出彩色訊息
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# 函數：檢查命令是否存在
check_command() {
    if ! command -v "$1" &> /dev/null; then
        print_error "缺少必要命令: $1"
        return 1
    fi
}

# 函數：檢查 Docker 服務狀態
check_docker_services() {
    print_info "檢查 Docker 服務狀態..."
    
    # 檢查是否有正在運行的服務
    running_count=$(docker compose ps | grep -c " Up " || echo "0")
    
    if [ "$running_count" -lt 4 ]; then
        print_warning "部分 Docker 服務未運行，嘗試啟動..."
        cargo make docker-up
        sleep 10
    fi
    
    # 檢查關鍵服務健康狀態（使用簡化的方法）
    services=("backtest-db-1" "redis-1" "rabbitmq-1" "dev-1")
    for service in "${services[@]}"; do
        if docker compose ps | grep "$service" | grep -q " Up "; then
            print_success "$service 服務正在運行"
        else
            print_error "$service 服務未運行"
            return 1
        fi
    done
}

# 函數：執行格式檢查
run_format_check() {
    print_header "執行代碼格式檢查"
    
    print_info "檢查 Rust 代碼格式..."
    if cargo make docker-c cargo fmt --all -- --check; then
        print_success "代碼格式檢查通過"
    else
        print_error "代碼格式檢查失敗"
        print_info "執行 'cargo make docker-c cargo fmt' 來修復格式"
        return 1
    fi
}

# 函數：執行 Clippy 檢查
run_clippy_check() {
    print_header "執行 Clippy 代碼質量檢查"
    
    print_info "執行 Clippy 分析..."
    if cargo make docker-c cargo clippy --all-targets --all-features -- -D warnings; then
        print_success "Clippy 檢查通過"
    else
        print_error "Clippy 檢查失敗"
        return 1
    fi
}

# 函數：構建測試二進制文件
build_test_binaries() {
    print_header "構建測試二進制文件"
    
    print_info "構建測試二進制文件（不運行測試）..."
    start_time=$(date +%s)
    
    if cargo make docker-c cargo test --no-run --all-features; then
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        print_success "測試二進制文件構建完成（耗時 ${duration}s）"
    else
        print_error "測試二進制文件構建失敗"
        return 1
    fi
}

# 函數：執行單元測試和集成測試
run_tests() {
    print_header "執行測試套件"
    
    print_info "執行所有測試（包含單元測試和集成測試）..."
    start_time=$(date +%s)
    
    # 設置測試環境變數（與 CI 一致）
    export RUST_LOG=debug
    export RUST_TEST_THREADS=2
    
    if cargo make docker-c cargo test --all-features -- --nocapture; then
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        print_success "所有測試通過（耗時 ${duration}s）"
    else
        print_error "測試失敗"
        return 1
    fi
}

# 函數：執行文檔測試
run_doc_tests() {
    print_header "執行文檔測試"
    
    print_info "執行文檔中的範例代碼測試..."
    if cargo make docker-c cargo test --doc; then
        print_success "文檔測試通過"
    else
        print_error "文檔測試失敗"
        return 1
    fi
}

# 函數：執行集成測試
run_integration_tests() {
    print_header "執行集成測試"
    
    print_info "執行專門的集成測試..."
    export RUST_LOG=debug
    
    if cargo make docker-c cargo test --test '*' -- --test-threads=1; then
        print_success "集成測試通過"
    else
        print_error "集成測試失敗"
        return 1
    fi
}

# 函數：檢查數據庫遷移
check_migrations() {
    print_header "檢查數據庫遷移"
    
    print_info "檢查數據庫遷移狀態..."
    if cargo make docker-c cargo run --bin migrate status; then
        print_success "數據庫遷移檢查通過"
    else
        print_warning "數據庫遷移檢查有問題，嘗試運行遷移..."
        if cargo make docker-c cargo run --bin migrate run; then
            print_success "數據庫遷移完成"
        else
            print_error "數據庫遷移失敗"
            return 1
        fi
    fi
}

# 函數：生成測試覆蓋率報告（可選）
generate_coverage() {
    print_header "生成測試覆蓋率報告（可選）"
    
    if check_command "cargo-tarpaulin"; then
        print_info "生成測試覆蓋率報告..."
        if cargo make docker-c cargo tarpaulin --out html --output-dir coverage; then
            print_success "覆蓋率報告已生成到 coverage/ 目錄"
        else
            print_warning "覆蓋率報告生成失敗（但不影響 CI）"
        fi
    else
        print_info "跳過覆蓋率報告（未安裝 cargo-tarpaulin）"
    fi
}

# 函數：顯示系統資源使用情況
show_system_info() {
    print_header "系統資源資訊"
    
    print_info "Docker 容器狀態："
    docker compose ps
    
    print_info "磁碟使用情況："
    df -h . | tail -1
    
    print_info "記憶體使用情況："
    free -h
}

# 主函數
main() {
    local start_time
    local end_time
    local total_duration
    local failed_checks=0
    
    start_time=$(date +%s)
    
    print_header "開始本地 CI 測試流程"
    print_info "模擬 GitHub Actions CI 環境..."
    
    # 檢查必要工具
    print_info "檢查必要工具..."
    for cmd in docker cargo jq; do
        if ! check_command "$cmd"; then
            print_error "請安裝 $cmd"
            exit 1
        fi
    done
    print_success "所有必要工具已安裝"
    
    # 檢查 Docker 服務
    if ! check_docker_services; then
        ((failed_checks++))
        print_error "Docker 服務檢查失敗"
    fi
    
    # 檢查數據庫遷移
    if ! check_migrations; then
        ((failed_checks++))
        print_error "數據庫遷移檢查失敗"
    fi
    
    # 執行格式檢查
    if ! run_format_check; then
        ((failed_checks++))
    fi
    
    # 構建測試二進制文件
    if ! build_test_binaries; then
        ((failed_checks++))
    fi
    
    # 執行 Clippy 檢查
    if ! run_clippy_check; then
        ((failed_checks++))
    fi
    
    # 執行測試
    if ! run_tests; then
        ((failed_checks++))
    fi
    
    # 執行文檔測試
    if ! run_doc_tests; then
        ((failed_checks++))
    fi
    
    # 執行集成測試
    if ! run_integration_tests; then
        ((failed_checks++))
    fi
    
    # 生成覆蓋率報告（可選）
    generate_coverage
    
    # 顯示系統資訊
    show_system_info
    
    end_time=$(date +%s)
    total_duration=$((end_time - start_time))
    
    print_header "本地 CI 測試完成"
    
    if [ $failed_checks -eq 0 ]; then
        print_success "所有檢查都通過！總耗時 ${total_duration}s"
        print_success "你的代碼已準備好推送到 CI"
    else
        print_error "有 $failed_checks 項檢查失敗，總耗時 ${total_duration}s"
        print_error "請修復問題後再推送到 CI"
        exit 1
    fi
}

# 函數：顯示使用說明
usage() {
    echo "使用方法: $0 [選項]"
    echo ""
    echo "選項:"
    echo "  --quick       快速模式（跳過集成測試和覆蓋率）"
    echo "  --format-only 只執行格式檢查"
    echo "  --test-only   只執行測試"
    echo "  --help        顯示此說明"
    echo ""
    echo "範例:"
    echo "  $0                    # 執行完整 CI 流程"
    echo "  $0 --quick           # 快速檢查（適合開發過程中）"
    echo "  $0 --format-only     # 只檢查格式"
    echo "  $0 --test-only       # 只執行測試"
}

# 處理命令行參數
case "${1:-}" in
    --quick)
        print_info "執行快速模式..."
        check_docker_services
        run_format_check
        run_clippy_check
        run_tests
        ;;
    --format-only)
        print_info "只執行格式檢查..."
        run_format_check
        ;;
    --test-only)
        print_info "只執行測試..."
        check_docker_services
        check_migrations
        run_tests
        ;;
    --help)
        usage
        exit 0
        ;;
    "")
        # 無參數，執行完整流程
        main
        ;;
    *)
        print_error "未知選項: $1"
        usage
        exit 1
        ;;
esac