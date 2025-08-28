#!/bin/bash

# ==============================================================================
#  MASTER WORKFLOW ORCHESTRATOR FOR SOLANA ARBITRAGE BOT
# ==============================================================================
#
#  This script orchestrates all automated workflows and provides a central
#  command center for AI-driven development processes.
#
#  Usage:
#  ./scripts/master-workflow.sh <command> [options]
#
#  Commands:
#  tdd <agent> <task>     - Run Test-Driven Development workflow
#  security [quick|full]  - Run security audit
#  ui-test [url] [--fix]  - Run UI testing workflow
#  full-cycle <agent> <task> - Complete development cycle (TDD + Security + UI)
#  status                 - Show project status and available workflows
#  help                   - Show this help message
#

# --- Configuration ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# --- Colors ---
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# --- Helper Functions ---
show_banner() {
    echo -e "${PURPLE}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${PURPLE}║           🤖 SOLANA ARBITRAGE BOT AI ORCHESTRATOR            ║${NC}"
    echo -e "${PURPLE}║                  Master Workflow Manager                     ║${NC}"
    echo -e "${PURPLE}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

show_help() {
    show_banner
    echo -e "${CYAN}Available Commands:${NC}"
    echo ""
    echo -e "${YELLOW}Development Workflows:${NC}"
    echo -e "  ${GREEN}tdd${NC} <agent> <task>          Run Test-Driven Development"
    echo -e "                               Agent examples: @jito-mev-implementer, @onchain-monitor"
    echo ""
    echo -e "${YELLOW}Quality Assurance:${NC}"
    echo -e "  ${GREEN}security${NC} [quick|full]        Run security audit (default: full)"
    echo -e "  ${GREEN}ui-test${NC} [url] [--fix]        Run UI testing (default: http://localhost:3001)"
    echo ""
    echo -e "${YELLOW}Complete Workflows:${NC}"
    echo -e "  ${GREEN}full-cycle${NC} <agent> <task>    Complete dev cycle (TDD + Security + UI)"
    echo -e "  ${GREEN}pre-deploy${NC}                   Full pre-deployment validation"
    echo ""
    echo -e "${YELLOW}Utilities:${NC}"
    echo -e "  ${GREEN}status${NC}                       Show project status and metrics"
    echo -e "  ${GREEN}clean${NC}                        Clean up generated reports"
    echo -e "  ${GREEN}help${NC}                         Show this help message"
    echo ""
    echo -e "${CYAN}Examples:${NC}"
    echo -e '  ./scripts/master-workflow.sh tdd "@jito-mev-implementer" "Create bundle with MEV protection"'
    echo -e "  ./scripts/master-workflow.sh security quick"
    echo -e "  ./scripts/master-workflow.sh ui-test http://localhost:3001 --fix"
    echo -e '  ./scripts/master-workflow.sh full-cycle "@onchain-monitor" "Add new DEX support"'
    echo ""
}

show_status() {
    show_banner
    echo -e "${CYAN}📊 PROJECT STATUS DASHBOARD${NC}"
    echo ""
    
    # Project info
    echo -e "${YELLOW}Project Information:${NC}"
    echo -e "  📁 Location: $PROJECT_ROOT"
    echo -e "  🦀 Language: Rust ($(rustc --version 2>/dev/null || echo "Not installed"))"
    echo -e "  📦 Package: $(grep '^name = ' Cargo.toml 2>/dev/null | cut -d'"' -f2 || echo "Unknown")"
    echo ""
    
    # Build status
    echo -e "${YELLOW}Build Status:${NC}"
    if [ -d "target" ]; then
        echo -e "  ✅ Target directory exists"
        if [ -f "target/release/solana-arbitrage-bot" ]; then
            echo -e "  ✅ Release binary built"
        else
            echo -e "  ⚠️  Release binary not found"
        fi
    else
        echo -e "  ❌ Project not built yet"
    fi
    echo ""
    
    # Test status
    echo -e "${YELLOW}Test Coverage:${NC}"
    TEST_COUNT=$(find . -name "*.rs" -exec grep -l "#\[test\]" {} \; 2>/dev/null | wc -l)
    echo -e "  📝 Test files found: $TEST_COUNT"
    echo ""
    
    # BMAD Method status
    echo -e "${YELLOW}BMAD Method Organization:${NC}"
    if [ -d ".bmad" ]; then
        echo -e "  ✅ BMAD structure exists"
        echo -e "     📋 Requirements: $([ -f ".bmad/PROJECT_REQUIREMENTS.md" ] && echo "✅" || echo "❌")"
        echo -e "     🏗️  Architecture: $([ -f ".bmad/ARCHITECTURE.md" ] && echo "✅" || echo "❌")"
        echo -e "     📖 Stories: $([ -f ".bmad/DEVELOPMENT_STORIES.md" ] && echo "✅" || echo "❌")"
        echo -e "     📊 Status: $([ -f ".bmad/PROJECT_STATUS.md" ] && echo "✅" || echo "❌")"
    else
        echo -e "  ❌ BMAD structure missing"
    fi
    echo ""
    
    # Agents status
    echo -e "${YELLOW}AI Agents:${NC}"
    if [ -f "AGENTS.md" ]; then
        echo -e "  ✅ Agent definitions available"
        AGENT_COUNT=$(grep -c "^## @" AGENTS.md 2>/dev/null || echo "0")
        echo -e "  🤖 Configured agents: $AGENT_COUNT"
    else
        echo -e "  ❌ Agent definitions missing"
    fi
    echo ""
    
    # Reports status  
    echo -e "${YELLOW}Recent Reports:${NC}"
    if [ -d "audit_reports" ]; then
        AUDIT_REPORTS=$(find audit_reports -name "*.txt" -o -name "*.md" -o -name "*.json" 2>/dev/null | wc -l)
        echo -e "  🛡️  Security reports: $AUDIT_REPORTS"
    fi
    if [ -d "ui_test_reports" ]; then
        UI_REPORTS=$(find ui_test_reports -name "*.txt" -o -name "*.md" 2>/dev/null | wc -l)
        echo -e "  🎭 UI test reports: $UI_REPORTS"
    fi
    echo ""
    
    # Available workflows
    echo -e "${YELLOW}Available Workflows:${NC}"
    echo -e "  🔧 TDD Workflow: $([ -x "scripts/auto-tdd.sh" ] && echo "✅" || echo "❌")"
    echo -e "  🛡️  Security Audit: $([ -x "scripts/run-security-audit.sh" ] && echo "✅" || echo "❌")"
    echo -e "  🎭 UI Testing: $([ -x "scripts/auto-ui-test.sh" ] && echo "✅" || echo "❌")"
    echo ""
}

clean_reports() {
    echo -e "${YELLOW}🧹 Cleaning up generated reports...${NC}"
    
    CLEANED=0
    
    if [ -d "audit_reports" ]; then
        OLD_AUDITS=$(find audit_reports -type f -mtime +7 2>/dev/null | wc -l)
        if [ "$OLD_AUDITS" -gt 0 ]; then
            find audit_reports -type f -mtime +7 -delete 2>/dev/null
            echo -e "  🗑️  Removed $OLD_AUDITS old audit reports (>7 days)"
            CLEANED=$((CLEANED + OLD_AUDITS))
        fi
    fi
    
    if [ -d "ui_test_reports" ]; then
        OLD_UI_TESTS=$(find ui_test_reports -type f -mtime +7 2>/dev/null | wc -l)
        if [ "$OLD_UI_TESTS" -gt 0 ]; then
            find ui_test_reports -type f -mtime +7 -delete 2>/dev/null
            echo -e "  🗑️  Removed $OLD_UI_TESTS old UI test reports (>7 days)"
            CLEANED=$((CLEANED + OLD_UI_TESTS))
        fi
    fi
    
    if [ "$CLEANED" -eq 0 ]; then
        echo -e "  ✨ No old reports to clean"
    else
        echo -e "${GREEN}✅ Cleaned up $CLEANED old report files${NC}"
    fi
}

run_tdd_workflow() {
    local AGENT="$1"
    local TASK="$2"
    
    if [ -z "$AGENT" ] || [ -z "$TASK" ]; then
        echo -e "${RED}❌ Error: TDD workflow requires agent and task parameters${NC}"
        echo -e "Usage: $0 tdd \"<agent>\" \"<task>\""
        echo -e 'Example: $0 tdd "@jito-mev-implementer" "Create Jito bundle logic"'
        exit 1
    fi
    
    echo -e "${BLUE}🔧 Starting TDD Workflow...${NC}"
    "$SCRIPT_DIR/auto-tdd.sh" "$AGENT" "$TASK"
}

run_security_audit() {
    local AUDIT_TYPE="${1:-full}"
    
    echo -e "${BLUE}🛡️  Starting Security Audit ($AUDIT_TYPE)...${NC}"
    "$SCRIPT_DIR/run-security-audit.sh" "--$AUDIT_TYPE"
}

run_ui_testing() {
    local URL="${1:-http://localhost:3001}"
    local FIX_FLAG="$2"
    
    echo -e "${BLUE}🎭 Starting UI Testing...${NC}"
    if [ "$FIX_FLAG" = "--fix" ]; then
        "$SCRIPT_DIR/auto-ui-test.sh" "$URL" "--fix"
    else
        "$SCRIPT_DIR/auto-ui-test.sh" "$URL"
    fi
}

run_full_cycle() {
    local AGENT="$1"
    local TASK="$2"
    
    if [ -z "$AGENT" ] || [ -z "$TASK" ]; then
        echo -e "${RED}❌ Error: Full cycle requires agent and task parameters${NC}"
        exit 1
    fi
    
    show_banner
    echo -e "${CYAN}🔄 FULL DEVELOPMENT CYCLE${NC}"
    echo -e "Agent: $AGENT"
    echo -e "Task: $TASK"
    echo ""
    
    # Step 1: TDD
    echo -e "${PURPLE}━━━ PHASE 1: TEST-DRIVEN DEVELOPMENT ━━━${NC}"
    run_tdd_workflow "$AGENT" "$TASK"
    
    # Step 2: Security Audit  
    echo -e "${PURPLE}━━━ PHASE 2: SECURITY AUDIT ━━━${NC}"
    run_security_audit "quick"
    
    # Step 3: UI Testing (if dashboard exists)
    if [ -d "dashboard-frontend" ]; then
        echo -e "${PURPLE}━━━ PHASE 3: UI TESTING ━━━${NC}"
        run_ui_testing "http://localhost:3001"
    fi
    
    echo -e "${GREEN}✅ FULL DEVELOPMENT CYCLE COMPLETED!${NC}"
}

run_pre_deploy() {
    show_banner
    echo -e "${CYAN}🚀 PRE-DEPLOYMENT VALIDATION${NC}"
    echo ""
    
    echo -e "${PURPLE}━━━ PHASE 1: BUILD VERIFICATION ━━━${NC}"
    if cargo build --release --no-default-features --features monitor; then
        echo -e "${GREEN}✅ Build successful${NC}"
    else
        echo -e "${RED}❌ Build failed - deployment blocked${NC}"
        exit 1
    fi
    
    echo -e "${PURPLE}━━━ PHASE 2: COMPREHENSIVE SECURITY AUDIT ━━━${NC}"
    run_security_audit "full"
    
    echo -e "${PURPLE}━━━ PHASE 3: FULL TEST SUITE ━━━${NC}"
    if cargo test --release --no-default-features --features monitor; then
        echo -e "${GREEN}✅ All tests passed${NC}"
    else
        echo -e "${RED}❌ Tests failed - deployment blocked${NC}"
        exit 1
    fi
    
    if [ -d "dashboard-frontend" ]; then
        echo -e "${PURPLE}━━━ PHASE 4: UI VALIDATION ━━━${NC}"
        run_ui_testing "http://localhost:3001"
    fi
    
    echo -e "${GREEN}✅ PRE-DEPLOYMENT VALIDATION COMPLETED!${NC}"
    echo -e "${YELLOW}📋 Ready for production deployment${NC}"
}

# ==============================================================================
#  MAIN COMMAND DISPATCHER
# ==============================================================================

cd "$PROJECT_ROOT"

case "${1:-help}" in
    "tdd")
        run_tdd_workflow "$2" "$3"
        ;;
    "security")
        run_security_audit "$2"
        ;;
    "ui-test")
        run_ui_testing "$2" "$3"
        ;;
    "full-cycle")
        run_full_cycle "$2" "$3"
        ;;
    "pre-deploy")
        run_pre_deploy
        ;;
    "status")
        show_status
        ;;
    "clean")
        clean_reports
        ;;
    "help"|*)
        show_help
        ;;
esac