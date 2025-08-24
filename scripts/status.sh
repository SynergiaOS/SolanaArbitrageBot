#!/bin/bash
# 📊 Status Check - Solana Arbitrage Bot

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PROD_PID_FILE="$PROJECT_ROOT/production.pid"
PAPER_PID_FILE="$PROJECT_ROOT/paper.pid"
PROD_LOG_FILE="$PROJECT_ROOT/logs/production.log"
PAPER_LOG_FILE="$PROJECT_ROOT/logs/paper.log"

echo -e "${BLUE}📊 Solana Arbitrage Bot - Status Check${NC}"
echo "========================================"

# Function to check process status
check_process() {
    local pid_file=$1
    local log_file=$2
    local mode=$3
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "${GREEN}✅ $mode bot is RUNNING (PID: $pid)${NC}"
            
            # Show uptime
            local start_time=$(ps -o lstart= -p "$pid" 2>/dev/null | xargs -I {} date -d "{}" +%s 2>/dev/null || echo "0")
            local current_time=$(date +%s)
            local uptime=$((current_time - start_time))
            
            if [ $uptime -gt 0 ]; then
                local hours=$((uptime / 3600))
                local minutes=$(((uptime % 3600) / 60))
                echo "   Uptime: ${hours}h ${minutes}m"
            fi
            
            # Show resource usage
            local cpu_mem=$(ps -o %cpu,%mem -p "$pid" --no-headers 2>/dev/null || echo "N/A N/A")
            echo "   CPU/Memory: $cpu_mem"
            
            # Show recent activity
            if [ -f "$log_file" ]; then
                echo "   Recent activity:"
                tail -n 3 "$log_file" 2>/dev/null | sed 's/^/     /' || echo "     No recent logs"
            fi
            
            return 0
        else
            echo -e "${RED}❌ $mode bot is NOT RUNNING (stale PID file)${NC}"
            rm -f "$pid_file"
            return 1
        fi
    else
        echo -e "${YELLOW}⚠️  $mode bot is NOT RUNNING (no PID file)${NC}"
        return 1
    fi
}

# Check production bot
echo -e "\n${BLUE}🚀 Production Status:${NC}"
check_process "$PROD_PID_FILE" "$PROD_LOG_FILE" "Production"

# Check paper trading bot
echo -e "\n${BLUE}📄 Paper Trading Status:${NC}"
check_process "$PAPER_PID_FILE" "$PAPER_LOG_FILE" "Paper trading"

# System health checks
echo -e "\n${BLUE}🖥️  System Health:${NC}"

# Check disk space
DISK_USAGE=$(df -h "$PROJECT_ROOT" | awk 'NR==2{print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 90 ]; then
    echo -e "${RED}❌ Disk usage critical: ${DISK_USAGE}%${NC}"
elif [ "$DISK_USAGE" -gt 80 ]; then
    echo -e "${YELLOW}⚠️  Disk usage high: ${DISK_USAGE}%${NC}"
else
    echo -e "${GREEN}✅ Disk usage OK: ${DISK_USAGE}%${NC}"
fi

# Check memory usage
MEMORY_USAGE=$(free | awk 'NR==2{printf "%.0f", $3*100/$2}')
if [ "$MEMORY_USAGE" -gt 90 ]; then
    echo -e "${RED}❌ Memory usage critical: ${MEMORY_USAGE}%${NC}"
elif [ "$MEMORY_USAGE" -gt 80 ]; then
    echo -e "${YELLOW}⚠️  Memory usage high: ${MEMORY_USAGE}%${NC}"
else
    echo -e "${GREEN}✅ Memory usage OK: ${MEMORY_USAGE}%${NC}"
fi

# Check load average
LOAD_AVG=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')
LOAD_THRESHOLD="4.0"
if (( $(echo "$LOAD_AVG > $LOAD_THRESHOLD" | bc -l) )); then
    echo -e "${YELLOW}⚠️  Load average high: $LOAD_AVG${NC}"
else
    echo -e "${GREEN}✅ Load average OK: $LOAD_AVG${NC}"
fi

# Network connectivity check
echo -e "\n${BLUE}🌐 Network Connectivity:${NC}"

# Check Solana RPC
if curl -s --max-time 5 "https://api.mainnet-beta.solana.com" > /dev/null; then
    echo -e "${GREEN}✅ Solana RPC reachable${NC}"
else
    echo -e "${RED}❌ Solana RPC unreachable${NC}"
fi

# Check internet connectivity
if ping -c 1 8.8.8.8 > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Internet connectivity OK${NC}"
else
    echo -e "${RED}❌ Internet connectivity issues${NC}"
fi

# Log file analysis
echo -e "\n${BLUE}📋 Log Analysis:${NC}"

analyze_logs() {
    local log_file=$1
    local mode=$2
    
    if [ -f "$log_file" ]; then
        echo "   $mode logs:"
        
        # Count recent errors
        local error_count=$(tail -n 100 "$log_file" 2>/dev/null | grep -c "ERROR" || echo "0")
        if [ "$error_count" -gt 5 ]; then
            echo -e "     ${RED}❌ High error count: $error_count errors in last 100 lines${NC}"
        elif [ "$error_count" -gt 0 ]; then
            echo -e "     ${YELLOW}⚠️  Some errors: $error_count errors in last 100 lines${NC}"
        else
            echo -e "     ${GREEN}✅ No recent errors${NC}"
        fi
        
        # Check for warnings
        local warning_count=$(tail -n 100 "$log_file" 2>/dev/null | grep -c "WARN" || echo "0")
        if [ "$warning_count" -gt 10 ]; then
            echo -e "     ${YELLOW}⚠️  Many warnings: $warning_count warnings${NC}"
        fi
        
        # Show file size
        local file_size=$(du -h "$log_file" | cut -f1)
        echo "     Log size: $file_size"
        
    else
        echo "     No log file found"
    fi
}

analyze_logs "$PROD_LOG_FILE" "Production"
analyze_logs "$PAPER_LOG_FILE" "Paper trading"

# Performance summary
echo -e "\n${BLUE}📈 Performance Summary:${NC}"

# Check if any bot is running for performance stats
if [ -f "$PROD_PID_FILE" ] || [ -f "$PAPER_PID_FILE" ]; then
    # Try to extract performance metrics from logs
    for log_file in "$PROD_LOG_FILE" "$PAPER_LOG_FILE"; do
        if [ -f "$log_file" ]; then
            # Look for performance metrics in logs
            local calc_rate=$(tail -n 50 "$log_file" 2>/dev/null | grep -o "calculations/sec: [0-9.]*" | tail -n 1 | cut -d' ' -f2 || echo "N/A")
            local success_rate=$(tail -n 50 "$log_file" 2>/dev/null | grep -o "success rate: [0-9.]*%" | tail -n 1 | cut -d' ' -f3 || echo "N/A")
            
            if [ "$calc_rate" != "N/A" ]; then
                echo "   Calculation rate: $calc_rate/sec"
            fi
            if [ "$success_rate" != "N/A" ]; then
                echo "   Success rate: $success_rate"
            fi
            break
        fi
    done
else
    echo "   No bots running - no performance data"
fi

# Quick actions
echo -e "\n${BLUE}🔧 Quick Actions:${NC}"
echo "   Start paper trading: ./scripts/start_paper.sh"
echo "   Start production: ./scripts/start_production.sh"
echo "   Stop production: ./scripts/stop_production.sh"
echo "   View logs: tail -f logs/production.log"
echo "   Monitor: ./scripts/monitor.sh"
echo "   Dashboard: http://localhost:8080 (production) or :8081 (paper)"

echo -e "\n${GREEN}📊 Status check completed${NC}"
