#!/bin/bash
# GitLab CI/CD 配置驗證腳本

echo "驗證 .gitlab-ci.yml 配置..."

# 檢查必要的檔案是否存在
if [ ! -f ".gitlab-ci.yml" ]; then
    echo "❌ 錯誤：.gitlab-ci.yml 檔案不存在"
    exit 1
fi

echo "✅ .gitlab-ci.yml 檔案存在"

# 檢查 YAML 語法（如果有 yq 工具）
if command -v yq &> /dev/null; then
    echo "驗證 YAML 語法..."
    if yq eval '.' .gitlab-ci.yml > /dev/null 2>&1; then
        echo "✅ YAML 語法正確"
    else
        echo "❌ YAML 語法錯誤"
        exit 1
    fi
else
    echo "⚠️  警告：未安裝 yq，跳過 YAML 語法檢查"
fi

# 檢查必要的階段
echo "檢查 CI/CD 階段..."
REQUIRED_STAGES=("prepare" "build" "test" "quality" "coverage" "deploy")
for stage in "${REQUIRED_STAGES[@]}"; do
    if grep -q "\- $stage" .gitlab-ci.yml; then
        echo "✅ 找到階段：$stage"
    else
        echo "❌ 缺少階段：$stage"
        exit 1
    fi
done

# 檢查必要的任務
echo "檢查 CI/CD 任務..."
REQUIRED_JOBS=("prepare:verify" "build:check" "test:unit" "quality:format" "quality:lint")
for job in "${REQUIRED_JOBS[@]}"; do
    if grep -q "^$job:" .gitlab-ci.yml; then
        echo "✅ 找到任務：$job"
    else
        echo "❌ 缺少任務：$job"
        exit 1
    fi
done

echo ""
echo "✅ GitLab CI/CD 配置驗證通過！"
echo ""
echo "建議："
echo "1. 將此 branch 推送到 GitLab 以觸發 pipeline"
echo "2. 在 GitLab 專案設定中配置必要的環境變數"
echo "3. 確保 GitLab Runner 已正確設置"