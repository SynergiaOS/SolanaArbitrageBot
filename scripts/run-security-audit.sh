#!/bin/bash

# ==============================================================================
#  AUTOMATED SECURITY AUDIT WORKFLOW FOR SOLANA ARBITRAGE BOT
# ==============================================================================
#
#  This script initiates a comprehensive, autonomous security audit by
#  leveraging advanced MCP-powered agents.
#
#  Usage:
#  ./scripts/run-security-audit.sh [--quick|--full]
#
#  Options:
#  --quick  : Basic security scan (faster)
#  --full   : Comprehensive audit with web research (slower)
#

# --- Configuration ---
AUDIT_TYPE=${1:-"--full"}
CLAUDE_COMMAND="claude"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
AUDIT_DIR="audit_reports"

# --- Colors ---
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# --- Create audit reports directory ---
mkdir -p "$AUDIT_DIR"

# --- Helper function ---
run_audit_stage() {
    local PROMPT="$1"
    local STAGE_NAME="$2"
    local OUTPUT_FILE="$3"

    echo -e "${BLUE}🔍 Stage: ${STAGE_NAME}${NC}"
    
    if [ ! -z "$OUTPUT_FILE" ]; then
        echo "📋 Output will be saved to: $OUTPUT_FILE"
        $CLAUDE_COMMAND -p "$PROMPT" | tee "$OUTPUT_FILE"
    else
        $CLAUDE_COMMAND -p "$PROMPT"
    fi
    
    echo -e "${GREEN}✓ ${STAGE_NAME} completed${NC}\n"
}

# --- Display usage if invalid option ---
if [[ "$AUDIT_TYPE" != "--quick" && "$AUDIT_TYPE" != "--full" ]]; then
    echo "Usage: $0 [--quick|--full]"
    echo ""
    echo "Options:"
    echo "  --quick  : Basic security scan (5-10 minutes)"
    echo "  --full   : Comprehensive audit with research (30+ minutes)"
    echo ""
    echo "Examples:"
    echo "  $0 --quick"
    echo "  $0 --full"
    exit 1
fi

# ==============================================================================
#  SECURITY AUDIT WORKFLOW
# ==============================================================================

echo -e "${YELLOW}🛡️  SOLANA ARBITRAGE BOT SECURITY AUDIT${NC}"
echo -e "${YELLOW}════════════════════════════════════════${NC}"
echo -e "Audit Type: $AUDIT_TYPE"
echo -e "Timestamp: $TIMESTAMP"
echo -e "Reports Dir: $AUDIT_DIR"
echo ""

# --- Stage 1: Code Analysis ---
echo -e "${BLUE}📊 STAGE 1: STATIC CODE ANALYSIS${NC}"
STATIC_ANALYSIS_PROMPT="@rust-linter Please perform a comprehensive static analysis of the entire codebase focusing on security vulnerabilities. Pay special attention to:
- Unsafe code blocks
- Input validation
- Error handling
- Cryptographic operations
- Network operations
Generate a detailed security report."

run_audit_stage "$STATIC_ANALYSIS_PROMPT" "Static Code Analysis" "$AUDIT_DIR/static_analysis_$TIMESTAMP.txt"

# --- Stage 2: Dependency Audit ---
echo -e "${BLUE}📦 STAGE 2: DEPENDENCY SECURITY AUDIT${NC}"
echo "Running cargo audit for known vulnerabilities..."
if command -v cargo-audit &> /dev/null; then
    cargo audit --json > "$AUDIT_DIR/dependency_audit_$TIMESTAMP.json" 2>&1
    echo -e "${GREEN}✓ Dependency audit completed${NC}\n"
else
    echo -e "${YELLOW}⚠️  cargo-audit not installed. Installing...${NC}"
    cargo install cargo-audit
    cargo audit --json > "$AUDIT_DIR/dependency_audit_$TIMESTAMP.json" 2>&1
fi

# --- Stage 3: Architecture Security Review ---
ARCH_REVIEW_PROMPT="@solana-architect Please review the system architecture from .bmad/ARCHITECTURE.md for security vulnerabilities. Focus on:
- Data flow security
- Authentication and authorization
- Communication security
- Trust boundaries
- Attack surface analysis"

run_audit_stage "$ARCH_REVIEW_PROMPT" "Architecture Security Review" "$AUDIT_DIR/architecture_review_$TIMESTAMP.txt"

if [ "$AUDIT_TYPE" = "--full" ]; then
    # --- Stage 4: Advanced Security Audit (Full mode only) ---
    echo -e "${BLUE}🔬 STAGE 4: ADVANCED SECURITY RESEARCH${NC}"
    ADVANCED_AUDIT_PROMPT="@solana-security-auditor-pro Please perform a comprehensive security audit using your web research capabilities. Research the latest Solana and DeFi vulnerabilities, analyze our specific implementation, and provide actionable security recommendations. Focus on:
    - Recent Solana exploit patterns
    - DeFi-specific attack vectors
    - MEV-related security issues
    - Flash loan attack prevention
    - Private key security
    Create a detailed security report with prioritized recommendations."

    run_audit_stage "$ADVANCED_AUDIT_PROMPT" "Advanced Security Research" "$AUDIT_DIR/advanced_audit_$TIMESTAMP.txt"

    # --- Stage 5: UI Security Testing ---
    if [ -d "dashboard-frontend" ]; then
        echo -e "${BLUE}🌐 STAGE 5: UI SECURITY TESTING${NC}"
        UI_SECURITY_PROMPT="@ui-responsive-tester Please perform security testing of the dashboard frontend. Check for:
        - XSS vulnerabilities
        - CSRF protection
        - Input sanitization
        - Authentication bypass
        - Sensitive data exposure
        Use browser automation to test various scenarios."

        run_audit_stage "$UI_SECURITY_PROMPT" "UI Security Testing" "$AUDIT_DIR/ui_security_$TIMESTAMP.txt"
    fi
fi

# --- Stage 6: Smart Contract Interaction Security ---
echo -e "${BLUE}⚡ STAGE 6: SMART CONTRACT SECURITY${NC}"
CONTRACT_SECURITY_PROMPT="@jito-mev-implementer @onchain-monitor Please analyze all smart contract interactions in the codebase for security issues:
- Transaction signing security
- Account validation
- Program interaction safety
- MEV protection effectiveness
- Slippage protection
- Reentrancy protection"

run_audit_stage "$CONTRACT_SECURITY_PROMPT" "Smart Contract Security" "$AUDIT_DIR/contract_security_$TIMESTAMP.txt"

# --- Final Report Generation ---
echo -e "${BLUE}📋 GENERATING FINAL AUDIT REPORT${NC}"

FINAL_REPORT="$AUDIT_DIR/SECURITY_AUDIT_REPORT_$TIMESTAMP.md"

cat > "$FINAL_REPORT" << EOF
# Solana Arbitrage Bot Security Audit Report

**Audit Date:** $(date)
**Audit Type:** $AUDIT_TYPE
**Auditor:** Automated AI Security Team

## Executive Summary

This report contains the findings from an automated security audit of the Solana Arbitrage Bot project.

## Audit Scope

- Static code analysis
- Dependency vulnerability assessment  
- Architecture security review
$([ "$AUDIT_TYPE" = "--full" ] && echo "- Advanced security research with latest threat intelligence")
$([ "$AUDIT_TYPE" = "--full" ] && [ -d "dashboard-frontend" ] && echo "- UI security testing")
- Smart contract interaction security

## Detailed Findings

### 1. Static Code Analysis
See: [static_analysis_$TIMESTAMP.txt](./static_analysis_$TIMESTAMP.txt)

### 2. Dependency Audit
See: [dependency_audit_$TIMESTAMP.json](./dependency_audit_$TIMESTAMP.json)

### 3. Architecture Review
See: [architecture_review_$TIMESTAMP.txt](./architecture_review_$TIMESTAMP.txt)

$([ "$AUDIT_TYPE" = "--full" ] && echo "### 4. Advanced Security Research")
$([ "$AUDIT_TYPE" = "--full" ] && echo "See: [advanced_audit_$TIMESTAMP.txt](./advanced_audit_$TIMESTAMP.txt)")

$([ "$AUDIT_TYPE" = "--full" ] && [ -d "dashboard-frontend" ] && echo "### 5. UI Security Testing")
$([ "$AUDIT_TYPE" = "--full" ] && [ -d "dashboard-frontend" ] && echo "See: [ui_security_$TIMESTAMP.txt](./ui_security_$TIMESTAMP.txt)")

### $([ "$AUDIT_TYPE" = "--full" ] && echo "6" || echo "4"). Smart Contract Security
See: [contract_security_$TIMESTAMP.txt](./contract_security_$TIMESTAMP.txt)

## Recommendations

Please review all individual reports and implement the recommended security measures.

## Next Steps

1. Review all findings in detail
2. Prioritize fixes based on severity
3. Implement security improvements
4. Re-run audit after fixes
5. Consider periodic security reviews

---
*This audit was generated automatically by the Solana Arbitrage Bot security framework.*
EOF

# --- Success Summary ---
echo -e "${GREEN}✅ SECURITY AUDIT COMPLETED SUCCESSFULLY!${NC}"
echo ""
echo -e "${YELLOW}📊 AUDIT SUMMARY${NC}"
echo -e "   ├─ Audit Type: $AUDIT_TYPE"
echo -e "   ├─ Reports Generated: $(ls -1 $AUDIT_DIR/*$TIMESTAMP* | wc -l)"
echo -e "   ├─ Main Report: $FINAL_REPORT"
echo -e "   └─ All Reports: $AUDIT_DIR/"
echo ""
echo -e "${YELLOW}📋 NEXT STEPS${NC}"
echo -e "   1. Review the main report: cat $FINAL_REPORT"
echo -e "   2. Check individual findings in: $AUDIT_DIR/"
echo -e "   3. Implement security recommendations"
echo -e "   4. Update .bmad/PROJECT_STATUS.md with security status"
echo -e "   5. Re-run audit after fixes: $0 $AUDIT_TYPE"
echo ""

# --- Open main report if possible ---
if command -v code &> /dev/null; then
    echo -e "${BLUE}📖 Opening main report in VS Code...${NC}"
    code "$FINAL_REPORT"
elif command -v cat &> /dev/null; then
    echo -e "${BLUE}📖 Displaying main report:${NC}"
    echo ""
    cat "$FINAL_REPORT"
fi