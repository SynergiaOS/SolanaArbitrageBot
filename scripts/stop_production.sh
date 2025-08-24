#!/bin/bash
# 🛑 Stop Production Trading - Solana Arbitrage Bot

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PID_FILE="$PROJECT_ROOT/production.pid"
LOG_FILE="$PROJECT_ROOT/logs/production.log"

echo -e "${BLUE}🛑 Stopping Solana Arbitrage Bot - PRODUCTION MODE${NC}"
echo "=================================================="

# Check if PID file exists
if [ ! -f "$PID_FILE" ]; then
    echo -e "${YELLOW}⚠️  No PID file found. Bot may not be running.${NC}"
    
    # Check for running processes anyway
    PIDS=$(pgrep -f "solana-arbitrage-bot" || true)
    if [ -n "$PIDS" ]; then
        echo -e "${YELLOW}Found running bot processes: $PIDS${NC}"
        read -p "Kill these processes? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo $PIDS | xargs kill
            echo -e "${GREEN}✅ Processes killed${NC}"
        fi
    else
        echo -e "${GREEN}✅ No bot processes found${NC}"
    fi
    exit 0
fi

# Read PID
BOT_PID=$(cat "$PID_FILE")

# Check if process is running
if ! kill -0 "$BOT_PID" 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Process $BOT_PID is not running${NC}"
    rm -f "$PID_FILE"
    exit 0
fi

echo -e "${YELLOW}🔍 Found running bot (PID: $BOT_PID)${NC}"

# Show recent performance
if [ -f "$LOG_FILE" ]; then
    echo -e "\n${BLUE}📊 Recent activity:${NC}"
    tail -n 5 "$LOG_FILE" || echo "No recent logs"
fi

# Confirmation
echo -e "\n${YELLOW}⚠️  STOP CONFIRMATION${NC}"
read -p "Stop the production bot? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Stop cancelled"
    exit 0
fi

# Graceful shutdown
echo -e "\n${BLUE}🔄 Attempting graceful shutdown...${NC}"
kill -TERM "$BOT_PID"

# Wait for graceful shutdown
TIMEOUT=30
for i in $(seq 1 $TIMEOUT); do
    if ! kill -0 "$BOT_PID" 2>/dev/null; then
        echo -e "${GREEN}✅ Bot stopped gracefully${NC}"
        rm -f "$PID_FILE"
        
        # Show final stats
        if [ -f "$LOG_FILE" ]; then
            echo -e "\n${BLUE}📈 Final log entries:${NC}"
            tail -n 3 "$LOG_FILE" || echo "No final logs"
        fi
        
        echo -e "\n${GREEN}🎉 Production bot stopped successfully${NC}"
        exit 0
    fi
    
    if [ $i -eq 10 ]; then
        echo -e "${YELLOW}⏳ Still waiting for graceful shutdown...${NC}"
    fi
    
    sleep 1
done

# Force kill if graceful shutdown failed
echo -e "${RED}⚠️  Graceful shutdown timed out. Force killing...${NC}"
kill -KILL "$BOT_PID" 2>/dev/null || true

# Wait a bit more
sleep 2

if kill -0 "$BOT_PID" 2>/dev/null; then
    echo -e "${RED}❌ Failed to stop bot process${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Bot force-stopped${NC}"
    rm -f "$PID_FILE"
fi

echo -e "\n${GREEN}🎉 Production bot stopped${NC}"
