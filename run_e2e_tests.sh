#!/bin/bash
#
# Run ALL Rust E2E Tests for Desktop App
#
# This script runs comprehensive end-to-end tests including:
# - Rendering accuracy
# - Syntax highlighting (HTML rendering)
# - Editor integration
# - Input handling
#
# Prerequisites:
#   brew install geckodriver  # macOS
#   apt install firefox-geckodriver  # Ubuntu

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  BerryEditor E2E Tests (Desktop App)               ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to cleanup background processes
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    if [ ! -z "$GECKODRIVER_PID" ]; then
        kill $GECKODRIVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$TAURI_PID" ]; then
        kill $TAURI_PID 2>/dev/null || true
    fi
}

trap cleanup EXIT

# Step 1: Check geckodriver is installed
if ! command -v geckodriver &> /dev/null; then
    echo -e "${RED}❌ geckodriver not found!${NC}"
    echo ""
    echo "Please install geckodriver:"
    echo "  macOS:  brew install geckodriver"
    echo "  Ubuntu: sudo apt install firefox-geckodriver"
    exit 1
fi

# Step 2: Start geckodriver in background
echo -e "${YELLOW}Starting geckodriver on port 4444...${NC}"
geckodriver --port 4444 > /dev/null 2>&1 &
GECKODRIVER_PID=$!
sleep 2

# Verify geckodriver is running
if ! ps -p $GECKODRIVER_PID > /dev/null; then
    echo -e "${RED}❌ Failed to start geckodriver${NC}"
    exit 1
fi
echo -e "${GREEN}✅ geckodriver running (PID: $GECKODRIVER_PID)${NC}"

# Step 3: Check if Tauri dev server is already running
if curl -s http://localhost:8080 > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Tauri dev server already running on port 8080${NC}"
else
    echo -e "${YELLOW}Starting Tauri dev server...${NC}"
    cargo tauri dev > /tmp/tauri_dev.log 2>&1 &
    TAURI_PID=$!

    # Wait for Tauri to be ready (max 60 seconds for cold start)
    echo -e "${YELLOW}Waiting for Tauri app to start...${NC}"
    for i in {1..60}; do
        if curl -s http://localhost:8080 > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Tauri dev server ready${NC}"
            break
        fi
        sleep 1
        if [ $((i % 10)) -eq 0 ]; then
            echo -e "${YELLOW}  Still waiting... (${i}s)${NC}"
        fi
    done

    if ! curl -s http://localhost:8080 > /dev/null 2>&1; then
        echo -e "${RED}❌ Tauri dev server failed to start${NC}"
        echo "Check /tmp/tauri_dev.log for details"
        exit 1
    fi
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Running E2E Test Suite                           ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════╝${NC}"
echo ""

# Test list
TESTS_PASSED=0
TESTS_FAILED=0

run_test() {
    local test_name=$1
    local test_file=$2

    echo -e "${BLUE}▶ Running: $test_name${NC}"
    if cargo test --test "$test_file" -- --ignored --test-threads=1 --nocapture 2>&1 | tee "/tmp/test_${test_file}.log"; then
        echo -e "${GREEN}  ✅ PASSED${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}  ❌ FAILED${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    echo ""
}

# Critical: Syntax Highlighting HTML Rendering
run_test "Syntax HTML Rendering (prop:innerHTML)" "syntax_html_rendering_test"

# Rendering Accuracy
run_test "Rendering Accuracy" "rendering_accuracy"

# Editor Integration
if [ -f "tests/editor_integration_e2e_test.rs" ]; then
    run_test "Editor Integration" "editor_integration_e2e_test"
fi

# Syntax Highlighting Colors
if [ -f "tests/syntax_highlighting_colors_test.rs" ]; then
    run_test "Syntax Highlighting Colors" "syntax_highlighting_colors_test"
fi

# Codicon Font Loading (Regression Prevention)
run_test "Codicon Font Loading (CDN)" "codicon_font_loading_test"

# Database Panel E2E
run_test "Database Panel E2E" "database_panel_e2e_test"

# Terminal E2E
if [ -f "tests/terminal_e2e_test.rs" ]; then
    run_test "Terminal Panel E2E" "terminal_e2e_test"
fi

# Panel Resize Layout (TDD regression test)
run_test "Panel Resize Layout" "panel_resize_layout_test"

# Summary
TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  E2E Test Summary                                  ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════╝${NC}"
echo ""
printf "%-30s %s\n" "Tests Passed:" "${GREEN}${TESTS_PASSED}${NC}"
printf "%-30s %s\n" "Tests Failed:" "${RED}${TESTS_FAILED}${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
printf "%-30s %s\n" "TOTAL:" "${TOTAL_TESTS} tests"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✨ ALL E2E TESTS PASSED! ✨${NC}"
    echo -e "${GREEN}Desktop app is production-ready!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some E2E tests failed${NC}"
    echo ""
    echo "Check logs in /tmp/test_*.log for details"
    exit 1
fi
