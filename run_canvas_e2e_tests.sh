#!/bin/bash
#
# Canvas Editor E2E Test Runner
#
# Prerequisites:
#   brew install geckodriver  # macOS
#   apt install firefox-geckodriver  # Ubuntu

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🎨 Canvas Editor E2E Tests${NC}"
echo ""

# Cleanup function
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

# Check geckodriver
if ! command -v geckodriver &> /dev/null; then
    echo -e "${RED}❌ geckodriver not found!${NC}"
    echo ""
    echo "Install geckodriver:"
    echo "  macOS:  brew install geckodriver"
    echo "  Ubuntu: sudo apt install firefox-geckodriver"
    exit 1
fi

# Start geckodriver
echo -e "${YELLOW}Starting geckodriver on port 4444...${NC}"
geckodriver --port 4444 > /dev/null 2>&1 &
GECKODRIVER_PID=$!
sleep 2

if ! ps -p $GECKODRIVER_PID > /dev/null; then
    echo -e "${RED}❌ Failed to start geckodriver${NC}"
    exit 1
fi
echo -e "${GREEN}✅ geckodriver running (PID: $GECKODRIVER_PID)${NC}"

# Check if Tauri dev server is running
if curl -s http://localhost:8081 > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Tauri dev server already running on port 8081${NC}"
else
    echo -e "${YELLOW}Starting Tauri dev server...${NC}"
    cargo tauri dev > /tmp/tauri_dev.log 2>&1 &
    TAURI_PID=$!

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
        echo "Check logs: /tmp/tauri_dev.log"
        exit 1
    fi
fi

echo ""
echo -e "${GREEN}🧪 Running Canvas E2E Tests...${NC}"
echo ""

# Run E2E tests
cargo test --test canvas_full_e2e_test -- --test-threads=1 --nocapture

TEST_EXIT_CODE=$?

echo ""
if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✅ All Canvas E2E tests passed!${NC}"
else
    echo -e "${RED}❌ Some tests failed${NC}"
fi

exit $TEST_EXIT_CODE
