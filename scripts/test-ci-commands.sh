#!/bin/bash
# Test script to validate CI/CD commands work locally
# This ensures our GitHub Actions workflow will succeed

set -e

echo "=== Testing CI/CD Commands ==="

# Color codes for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to test a command
test_command() {
    local desc="$1"
    local cmd="$2"
    
    echo -n "Testing: $desc... "
    if eval "$cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC}"
        echo "  Command failed: $cmd"
        return 1
    fi
}

# Track failures
FAILED=0

# Test Rust toolchain
test_command "Rust compiler" "rustc --version" || ((FAILED++))
test_command "Cargo" "cargo --version" || ((FAILED++))
test_command "Rustfmt" "rustfmt --version" || ((FAILED++))
test_command "Clippy" "cargo clippy --version" || ((FAILED++))

# Test cargo-make
test_command "Cargo make" "cargo make --version" || ((FAILED++))

# Test Docker commands (for CI environment setup)
test_command "Docker" "docker --version" || ((FAILED++))
test_command "Docker Compose" "docker compose version" || ((FAILED++))

# Test project-specific commands
echo ""
echo "=== Testing Project Commands ==="

# These should be run in Docker environment
test_command "Format check" "cargo make docker-c cargo format-check" || ((FAILED++))
test_command "Clippy check" "cargo make docker-c cargo lint" || ((FAILED++))
test_command "Build (debug)" "cargo make docker-c cargo build" || ((FAILED++))
test_command "Build (release)" "cargo make docker-c cargo build-release" || ((FAILED++))

echo ""
echo "=== Testing Database Commands ==="
test_command "Migration binary" "cargo make docker-c cargo build --bin migrate" || ((FAILED++))

# Summary
echo ""
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED tests failed!${NC}"
    exit 1
fi