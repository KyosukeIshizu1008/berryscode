#!/bin/bash
#
# Run Rust WebDriver Rendering Accuracy Tests
#
# This script orchestrates the complete E2E test flow:
# 1. Starts geckodriver (Firefox WebDriver)
# 2. Starts Tauri dev server
# 3. Runs rendering accuracy tests
# 4. Cleans up processes
#
# Prerequisites:
#   brew install geckodriver  # macOS
#   apt install firefox-geckodriver  # Ubuntu

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🦀 BerryEditor Rendering Accuracy Tests${NC}"
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
if curl -s http://localhost:8081 > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Tauri dev server already running on port 8081${NC}"
else
    echo -e "${YELLOW}Starting Tauri dev server...${NC}"
    cd src-tauri
    cargo tauri dev > /dev/null 2>&1 &
    TAURI_PID=$!
    cd ..

    # Wait for Tauri to be ready (max 30 seconds)
    echo -e "${YELLOW}Waiting for Tauri app to start...${NC}"
    for i in {1..30}; do
        if curl -s http://localhost:8081 > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Tauri dev server ready${NC}"
            break
        fi
        sleep 1
    done

    if ! curl -s http://localhost:8081 > /dev/null 2>&1; then
        echo -e "${RED}❌ Tauri dev server failed to start${NC}"
        exit 1
    fi
fi

echo ""
echo -e "${GREEN}🧪 Running Rendering Accuracy Tests...${NC}"
echo ""

# Step 4: Run Rust WebDriver tests (with --ignored to run tests marked with #[ignore])
cargo test --test rendering_accuracy -- --ignored --test-threads=1 --nocapture

TEST_EXIT_CODE=$?

echo ""
if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✅ All rendering accuracy tests passed!${NC}"
else
    echo -e "${RED}❌ Some tests failed${NC}"
fi

exit $TEST_EXIT_CODE
