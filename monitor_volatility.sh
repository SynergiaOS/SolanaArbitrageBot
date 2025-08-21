#!/bin/bash
# 📊 Solana Arbitrage Bot - Market Monitor & Alert System
# Monitors price spreads and alerts when opportunities appear

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
MIN_SPREAD_ALERT=0.3  # Alert when spread > 0.3%
LOG_FILE="arbitrage_monitor.log"
SOUND_ALERT=true      # Play sound on opportunity

print_header() {
    clear
    echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║      🚀 SOLANA ARBITRAGE MONITOR - LIVE MODE 🚀       ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

check_spread() {
    # Extract prices from logs
    LAST_LOG=$(tail -n 100 "$LOG_FILE" 2>/dev/null | grep "Price update" | tail -n 2 || true)
    
    if [ -n "$LAST_LOG" ]; then
        RAYDIUM_PRICE=$(echo "$LAST_LOG" | grep "Raydium" | sed -E 's/.*\$([0-9.]+).*/\1/' | head -n 1)
        ORCA_PRICE=$(echo "$LAST_LOG" | grep "Orca" | sed -E 's/.*\$([0-9.]+).*/\1/' | head -n 1)
        
        if [ -n "$RAYDIUM_PRICE" ] && [ -n "$ORCA_PRICE" ]; then
            # Calculate spread
            SPREAD=$(echo "scale=4; abs = ($RAYDIUM_PRICE - $ORCA_PRICE); if (abs < 0) abs = -abs; (abs / $RAYDIUM_PRICE) * 100" | bc -l 2>/dev/null || echo "0")
            
            # Display current status
            echo -e "${GREEN}📊 CURRENT MARKET STATUS:${NC}"
            echo -e "  Raydium:  ${YELLOW}$$RAYDIUM_PRICE${NC}"
            echo -e "  Orca:     ${YELLOW}$$ORCA_PRICE${NC}"
            echo -e "  Spread:   ${YELLOW}${SPREAD}%${NC}"
            echo ""
            
            # Check for opportunity
            if (( $(echo "$SPREAD > $MIN_SPREAD_ALERT" | bc -l) )); then
                echo -e "${GREEN}🎯 ARBITRAGE OPPORTUNITY DETECTED!${NC}"
                echo -e "${GREEN}  Potential profit from ${SPREAD}% spread${NC}"
                
                # Sound alert if enabled
                if [ "$SOUND_ALERT" = true ]; then
                    echo -e "\a" # Terminal bell
                fi
                
                # Log opportunity
                echo "[$(date)] OPPORTUNITY: Spread ${SPREAD}% - Raydium: $RAYDIUM_PRICE, Orca: $ORCA_PRICE" >> opportunities.log
            fi
        fi
    fi
}

show_statistics() {
    echo -e "${BLUE}📈 SESSION STATISTICS:${NC}"
    
    # Count opportunities
    if [ -f "opportunities.log" ]; then
        OPP_COUNT=$(wc -l < opportunities.log)
        echo -e "  Opportunities found: ${GREEN}$OPP_COUNT${NC}"
        
        if [ "$OPP_COUNT" -gt 0 ]; then
            echo -e "  Last opportunity: ${YELLOW}$(tail -n 1 opportunities.log)${NC}"
        fi
    else
        echo -e "  Opportunities found: ${YELLOW}0${NC}"
    fi
    
    echo ""
}

show_recommendations() {
    echo -e "${BLUE}💡 RECOMMENDATIONS:${NC}"
    echo -e "  1. ${YELLOW}Watch for news events${NC} - they create volatility"
    echo -e "  2. ${YELLOW}Peak trading hours${NC} - 14:00-22:00 UTC"
    echo -e "  3. ${YELLOW}Market opens/closes${NC} - increased spreads"
    echo -e "  4. ${YELLOW}Network congestion${NC} - can create opportunities"
    echo ""
}

# Main monitoring loop
main() {
    print_header
    
    # Start bot in background if not running
    if ! pgrep -f "solana-arbitrage-bot" > /dev/null; then
        echo -e "${YELLOW}Starting arbitrage bot in background...${NC}"
        nohup ./target/release/solana-arbitrage-bot --dry-run > "$LOG_FILE" 2>&1 &
        sleep 5
    fi
    
    echo -e "${GREEN}✅ Monitoring started!${NC}"
    echo -e "${YELLOW}Watching for spreads > ${MIN_SPREAD_ALERT}%${NC}"
    echo ""
    
    while true; do
        print_header
        check_spread
        show_statistics
        show_recommendations
        
        echo -e "${BLUE}──────────────────────────────────────────────${NC}"
        echo -e "Press ${RED}Ctrl+C${NC} to stop monitoring"
        echo -e "Refreshing in 5 seconds..."
        
        sleep 5
    done
}

# Cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Stopping monitor...${NC}"
    # Optionally stop the bot
    # pkill -f solana-arbitrage-bot || true
    echo -e "${GREEN}Monitor stopped. Check opportunities.log for history.${NC}"
    exit 0
}

trap cleanup INT

# Run
main
