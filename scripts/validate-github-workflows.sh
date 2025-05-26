#!/bin/bash
# Validate GitHub Actions workflow files

set -e

echo "=== Validating GitHub Actions Workflows ==="

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

WORKFLOW_DIR=".github/workflows"
FAILED=0

# Check if workflow directory exists
if [ ! -d "$WORKFLOW_DIR" ]; then
    echo -e "${YELLOW}Warning: $WORKFLOW_DIR directory does not exist yet${NC}"
    echo "This is expected for initial setup"
    exit 0
fi

# Find all YAML files in workflows directory
WORKFLOW_FILES=$(find "$WORKFLOW_DIR" -name "*.yml" -o -name "*.yaml" 2>/dev/null || true)

if [ -z "$WORKFLOW_FILES" ]; then
    echo -e "${YELLOW}No workflow files found in $WORKFLOW_DIR${NC}"
    exit 0
fi

# Validate each workflow file
for file in $WORKFLOW_FILES; do
    echo -n "Validating $file... "
    
    # Basic YAML syntax check using Python
    if python3 -c "import yaml; yaml.safe_load(open('$file'))" 2>/dev/null; then
        echo -e "${GREEN}✓ Valid YAML${NC}"
        
        # Check for required GitHub Actions fields
        if grep -q "^name:" "$file" && grep -q "^on:" "$file" && grep -q "^jobs:" "$file"; then
            echo -e "  ${GREEN}✓ Has required fields (name, on, jobs)${NC}"
        else
            echo -e "  ${RED}✗ Missing required fields${NC}"
            ((FAILED++))
        fi
    else
        echo -e "${RED}✗ Invalid YAML syntax${NC}"
        ((FAILED++))
    fi
done

# Summary
echo ""
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All workflow files are valid!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED workflow files have issues!${NC}"
    exit 1
fi