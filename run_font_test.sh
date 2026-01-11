#!/bin/bash
# Run Font Verification E2E Test
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Font & Icon Loading Verification Test            ║${NC}"
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

# Step 3: Check if any server is already on port 8081
if lsof -ti:8081 > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Port 8081 is already in use (trunk serve likely running)${NC}"
else
    echo -e "${YELLOW}⚠️  Port 8081 is not in use. Starting from scratch...${NC}"
fi

# Step 4: Start Tauri dev server
echo -e "${YELLOW}Starting Tauri dev server...${NC}"
cd "$(dirname "$0")"
cargo tauri dev > /tmp/tauri_font_test.log 2>&1 &
TAURI_PID=$!

# Wait for Tauri to be ready (max 60 seconds) - check port 8081
echo -e "${YELLOW}Waiting for Tauri app to start...${NC}"
for i in {1..60}; do
    if curl -s http://localhost:8081 > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Tauri dev server ready (port 8081)${NC}"
        break
    fi
    sleep 1
    if [ $((i % 10)) -eq 0 ]; then
        echo -e "${YELLOW}  Still waiting... (${i}s)${NC}"
    fi
done

if ! curl -s http://localhost:8081 > /dev/null 2>&1; then
    echo -e "${RED}❌ Tauri dev server failed to start${NC}"
    echo "Check /tmp/tauri_font_test.log for details"
    exit 1
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Running Font Verification Test                   ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════╝${NC}"
echo ""

# Run the test
echo -e "${BLUE}▶ Running: Font & Icon Loading Test${NC}"
if cargo test --test style_verification_test test_icons_and_fonts_loaded -- --nocapture 2>&1 | tee /tmp/font_test.log; then
    echo -e "${GREEN}  ✅ TEST PASSED${NC}"
    echo ""
    echo -e "${GREEN}✨ Font verification test passed! ✨${NC}"
    exit 0
else
    echo -e "${RED}  ❌ TEST FAILED${NC}"
    echo ""
    echo -e "${RED}❌ Font verification test failed${NC}"
    echo ""
    echo "Check logs:"
    echo "  - Test output: /tmp/font_test.log"
    echo "  - Tauri logs: /tmp/tauri_font_test.log"
    exit 1
fi
