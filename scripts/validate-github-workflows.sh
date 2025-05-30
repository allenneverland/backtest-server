#!/bin/bash

# GitHub Workflows 驗證腳本
# 驗證 GitHub Actions 配置文件語法和結構

set -e

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

validate_yaml_syntax() {
    local file=$1
    
    if command -v yq &> /dev/null; then
        if yq eval '.' "$file" > /dev/null 2>&1; then
            print_status "pass" "YAML 語法正確: $file"
            return 0
        else
            print_status "fail" "YAML 語法錯誤: $file"
            return 1
        fi
    elif command -v python3 &> /dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$file'))" 2>/dev/null; then
            print_status "pass" "YAML 語法正確: $file"
            return 0
        else
            print_status "fail" "YAML 語法錯誤: $file"
            return 1
        fi
    else
        print_status "warn" "無法驗證 YAML 語法（缺少 yq 或 python3）: $file"
        return 0
    fi
}

check_workflow_structure() {
    local file=$1
    
    print_status "info" "檢查工作流程結構: $(basename "$file")"
    
    # 檢查必要的頂級鍵
    if grep -q "^name:" "$file"; then
        print_status "pass" "包含 name 欄位"
    else
        print_status "warn" "缺少 name 欄位"
    fi
    
    if grep -q "^on:" "$file"; then
        print_status "pass" "包含 on 欄位"
    else
        print_status "fail" "缺少 on 欄位"
        return 1
    fi
    
    if grep -q "^jobs:" "$file"; then
        print_status "pass" "包含 jobs 欄位"
    else
        print_status "fail" "缺少 jobs 欄位"
        return 1
    fi
    
    # 檢查作業是否有 runs-on
    if grep -A 10 "^jobs:" "$file" | grep -q "runs-on:"; then
        print_status "pass" "作業包含 runs-on 設定"
    else
        print_status "fail" "作業缺少 runs-on 設定"
        return 1
    fi
    
    return 0
}

check_ci_completeness() {
    local file=$1
    
    print_status "info" "檢查 CI 完整性: $(basename "$file")"
    
    # 檢查是否包含格式檢查
    if grep -q "fmt\|format" "$file"; then
        print_status "pass" "包含格式檢查"
    else
        print_status "warn" "建議添加格式檢查"
    fi
    
    # 檢查是否包含 Clippy
    if grep -q "clippy" "$file"; then
        print_status "pass" "包含 Clippy 檢查"
    else
        print_status "warn" "建議添加 Clippy 檢查"
    fi
    
    # 檢查是否包含測試
    if grep -q "test\|cargo test" "$file"; then
        print_status "pass" "包含測試步驟"
    else
        print_status "warn" "建議添加測試步驟"
    fi
    
    # 檢查是否設置了適當的環境變數
    if grep -q "DATABASE_URL\|RUST_LOG" "$file"; then
        print_status "pass" "設置了環境變數"
    else
        print_status "warn" "建議設置適當的環境變數"
    fi
    
    return 0
}

main() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}GitHub Workflows 驗證${NC}"
    echo -e "${BLUE}========================================${NC}"
    
    local total_files=0
    local valid_files=0
    
    # 檢查 .github/workflows 目錄
    if [[ ! -d ".github/workflows" ]]; then
        print_status "fail" ".github/workflows 目錄不存在"
        exit 1
    fi
    
    # 驗證所有 YAML 文件
    for file in .github/workflows/*.yml .github/workflows/*.yaml; do
        if [[ -f "$file" ]]; then
            ((total_files++))
            echo ""
            print_status "info" "驗證文件: $(basename "$file")"
            
            if validate_yaml_syntax "$file" && check_workflow_structure "$file"; then
                check_ci_completeness "$file"
                ((valid_files++))
                print_status "pass" "$(basename "$file") 驗證通過"
            else
                print_status "fail" "$(basename "$file") 驗證失敗"
            fi
        fi
    done
    
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}驗證結果: $valid_files/$total_files 文件通過${NC}"
    echo -e "${BLUE}========================================${NC}"
    
    if [[ $valid_files -eq $total_files ]]; then
        print_status "pass" "所有工作流程文件都有效"
        exit 0
    else
        print_status "fail" "部分工作流程文件有問題"
        exit 1
    fi
}

case "${1:-}" in
    --help)
        echo "使用方法: $0"
        echo ""
        echo "此腳本驗證 .github/workflows 目錄中的 GitHub Actions 配置文件"
        echo "檢查 YAML 語法、工作流程結構和 CI 完整性"
        exit 0
        ;;
    "")
        main
        ;;
    *)
        echo "未知選項: $1"
        echo "使用 --help 查看使用說明"
        exit 1
        ;;
esac