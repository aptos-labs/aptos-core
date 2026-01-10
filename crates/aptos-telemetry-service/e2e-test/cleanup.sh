#!/bin/bash
# Cleanup script for E2E test environment
# Stops all services and optionally removes data

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_DIR="$SCRIPT_DIR/test-data"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}E2E Test Environment Cleanup${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Stop Docker services
echo -e "${YELLOW}[1/3]${NC} Stopping Docker services..."
cd "$SCRIPT_DIR"
docker-compose down
echo -e "${GREEN}✓ Docker services stopped${NC}"
echo ""

# Stop aptos-node
echo -e "${YELLOW}[2/3]${NC} Stopping Aptos node..."
if [ -f "$TEST_DIR/node.pid" ]; then
    NODE_PID=$(cat "$TEST_DIR/node.pid")
    if ps -p $NODE_PID > /dev/null 2>&1; then
        kill $NODE_PID
        echo -e "${GREEN}✓ Aptos node stopped (PID: $NODE_PID)${NC}"
    else
        echo -e "${YELLOW}Node process not running${NC}"
    fi
    rm "$TEST_DIR/node.pid"
else
    # Try to find and kill any aptos-node process on port 8080
    if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null 2>&1; then
        kill $(lsof -t -i:8080) 2>/dev/null || true
        echo -e "${GREEN}✓ Stopped process on port 8080${NC}"
    else
        echo -e "${YELLOW}No node process found${NC}"
    fi
fi

# Stop telemetry service
if [ -f "$TEST_DIR/telemetry.pid" ]; then
    TELEMETRY_PID=$(cat "$TEST_DIR/telemetry.pid")
    if ps -p $TELEMETRY_PID > /dev/null 2>&1; then
        kill $TELEMETRY_PID
        echo -e "${GREEN}✓ Telemetry service stopped (PID: $TELEMETRY_PID)${NC}"
    else
        echo -e "${YELLOW}Telemetry process not running${NC}"
    fi
    rm "$TEST_DIR/telemetry.pid"
else
    # Try to find and kill any telemetry service process on port 8082
    if lsof -Pi :8082 -sTCP:LISTEN -t >/dev/null 2>&1; then
        kill $(lsof -t -i:8082) 2>/dev/null || true
        echo -e "${GREEN}✓ Stopped process on port 8082${NC}"
    else
        echo -e "${YELLOW}No telemetry service process found${NC}"
    fi
fi
echo ""

# Ask about data removal
echo -e "${YELLOW}[3/3]${NC} Data cleanup..."
read -p "Remove test data directory? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf "$TEST_DIR"
    echo -e "${GREEN}✓ Test data removed${NC}"
else
    echo -e "${YELLOW}Test data preserved at: $TEST_DIR${NC}"
fi

read -p "Remove Docker volumes? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cd "$SCRIPT_DIR"
    docker-compose down -v
    echo -e "${GREEN}✓ Docker volumes removed${NC}"
else
    echo -e "${YELLOW}Docker volumes preserved${NC}"
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}✓ Cleanup Complete${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

