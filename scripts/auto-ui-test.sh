#!/bin/bash

# ==============================================================================
#  AUTOMATED UI TESTING WORKFLOW WITH PLAYWRIGHT MCP
# ==============================================================================
#
#  This script uses Playwright MCP to automatically test the dashboard UI,
#  detect responsive design issues, and generate fix recommendations.
#
#  Usage:
#  ./scripts/auto-ui-test.sh [dashboard_url] [--fix]
#
#  Options:
#  --fix    : Automatically apply fixes to detected issues
#
#  Examples:
#  ./scripts/auto-ui-test.sh http://localhost:3001
#  ./scripts/auto-ui-test.sh http://localhost:3001 --fix
#

# --- Configuration ---
DASHBOARD_URL=${1:-"http://localhost:3001"}
AUTO_FIX=${2:-""}
CLAUDE_COMMAND="claude"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
TEST_REPORTS_DIR="ui_test_reports"

# --- Colors ---
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
PURPLE='\033[0;35m'
NC='\033[0m'

# --- Create test reports directory ---
mkdir -p "$TEST_REPORTS_DIR"

# --- Helper functions ---
run_ui_test_stage() {
    local PROMPT="$1"
    local STAGE_NAME="$2"
    local OUTPUT_FILE="$3"

    echo -e "${BLUE}🧪 ${STAGE_NAME}${NC}"
    
    if [ ! -z "$OUTPUT_FILE" ]; then
        echo "📊 Results will be saved to: $OUTPUT_FILE"
        $CLAUDE_COMMAND -p "$PROMPT" | tee "$OUTPUT_FILE"
    else
        $CLAUDE_COMMAND -p "$PROMPT"
    fi
    
    echo -e "${GREEN}✓ ${STAGE_NAME} completed${NC}\n"
}

check_dashboard_running() {
    echo -e "${BLUE}🔍 Checking if dashboard is accessible...${NC}"
    if curl -s "$DASHBOARD_URL" > /dev/null; then
        echo -e "${GREEN}✓ Dashboard is accessible at $DASHBOARD_URL${NC}\n"
        return 0
    else
        echo -e "${RED}✗ Dashboard not accessible at $DASHBOARD_URL${NC}"
        echo -e "${YELLOW}💡 Please start the dashboard first:${NC}"
        echo -e "   cd dashboard-frontend && npm run dev"
        echo -e "   OR"
        echo -e "   ./scripts/start_monitor_only.sh"
        return 1
    fi
}

# ==============================================================================
#  UI TESTING WORKFLOW
# ==============================================================================

echo -e "${PURPLE}🎭 AUTOMATED UI TESTING WITH PLAYWRIGHT${NC}"
echo -e "${PURPLE}═══════════════════════════════════════${NC}"
echo -e "Dashboard URL: $DASHBOARD_URL"
echo -e "Auto-Fix Mode: $([ "$AUTO_FIX" = "--fix" ] && echo "ENABLED" || echo "DISABLED")"
echo -e "Timestamp: $TIMESTAMP"
echo ""

# --- Pre-flight checks ---
if ! check_dashboard_running; then
    exit 1
fi

# --- Stage 1: Visual Regression Testing ---
echo -e "${BLUE}📸 STAGE 1: VISUAL REGRESSION TESTING${NC}"
VISUAL_TEST_PROMPT="@ui-responsive-tester Please perform comprehensive visual regression testing on the dashboard at $DASHBOARD_URL. Take screenshots at different viewport sizes:
- Desktop (1920x1080)
- Tablet (768x1024) 
- Mobile (375x667)
- Ultra-wide (2560x1440)

Test all major pages and components:
- Main dashboard
- Trading charts
- Position management
- Configuration panel
- System health status

Generate a visual test report with screenshots and any detected issues."

run_ui_test_stage "$VISUAL_TEST_PROMPT" "Visual Regression Testing" "$TEST_REPORTS_DIR/visual_regression_$TIMESTAMP.txt"

# --- Stage 2: Responsive Design Validation ---
echo -e "${BLUE}📱 STAGE 2: RESPONSIVE DESIGN VALIDATION${NC}"
RESPONSIVE_TEST_PROMPT="@ui-responsive-tester Please validate responsive design compliance across all viewport sizes. Check for:
- Layout breakpoints
- Text readability
- Button accessibility  
- Navigation usability
- Chart responsiveness
- Table overflow handling
- Modal dialog scaling

Use browser automation to interact with elements at different screen sizes and identify any UX issues."

run_ui_test_stage "$RESPONSIVE_TEST_PROMPT" "Responsive Design Validation" "$TEST_REPORTS_DIR/responsive_validation_$TIMESTAMP.txt"

# --- Stage 3: Accessibility Testing ---
echo -e "${BLUE}♿ STAGE 3: ACCESSIBILITY TESTING${NC}"  
ACCESSIBILITY_TEST_PROMPT="@ui-responsive-tester Please perform accessibility testing on the dashboard:
- ARIA labels and roles
- Keyboard navigation
- Color contrast ratios
- Focus management
- Screen reader compatibility
- Alternative text for images
- Form accessibility

Generate an accessibility compliance report with specific fixes needed."

run_ui_test_stage "$ACCESSIBILITY_TEST_PROMPT" "Accessibility Testing" "$TEST_REPORTS_DIR/accessibility_$TIMESTAMP.txt"

# --- Stage 4: Performance Testing ---
echo -e "${BLUE}⚡ STAGE 4: PERFORMANCE TESTING${NC}"
PERFORMANCE_TEST_PROMPT="@ui-responsive-tester Please test dashboard performance metrics:
- Initial page load time
- Time to interactive (TTI)
- Largest contentful paint (LCP)
- First input delay (FID)
- Cumulative layout shift (CLS)
- WebSocket connection performance
- Chart rendering performance
- Real-time data update efficiency

Use Playwright to measure and report performance metrics."

run_ui_test_stage "$PERFORMANCE_TEST_PROMPT" "Performance Testing" "$TEST_REPORTS_DIR/performance_$TIMESTAMP.txt"

# --- Stage 5: Functional Testing ---
echo -e "${BLUE}🔧 STAGE 5: FUNCTIONAL TESTING${NC}"
FUNCTIONAL_TEST_PROMPT="@ui-responsive-tester Please perform end-to-end functional testing:
- Dashboard data loading
- Real-time updates
- User interactions (buttons, forms, filters)
- Navigation between sections
- Configuration changes
- Error handling and user feedback
- WebSocket reconnection behavior

Automate user scenarios and report any functional issues."

run_ui_test_stage "$FUNCTIONAL_TEST_PROMPT" "Functional Testing" "$TEST_REPORTS_DIR/functional_$TIMESTAMP.txt"

# --- Stage 6: Cross-browser Testing ---
echo -e "${BLUE}🌐 STAGE 6: CROSS-BROWSER TESTING${NC}"
BROWSER_TEST_PROMPT="@ui-responsive-tester Please test the dashboard across different browsers:
- Chrome/Chromium
- Firefox  
- Safari (if available)
- Edge

Check for browser-specific issues:
- CSS compatibility
- JavaScript functionality
- WebSocket support
- Chart rendering differences
- Performance variations

Report any browser-specific problems."

run_ui_test_stage "$BROWSER_TEST_PROMPT" "Cross-browser Testing" "$TEST_REPORTS_DIR/browser_compatibility_$TIMESTAMP.txt"

# --- Stage 7: Security Testing (UI-focused) ---
echo -e "${BLUE}🛡️ STAGE 7: UI SECURITY TESTING${NC}"
UI_SECURITY_TEST_PROMPT="@ui-responsive-tester Please perform UI-focused security testing:
- XSS vulnerability testing
- Input validation on forms
- CSRF token validation
- Sensitive data exposure
- Authentication bypass attempts
- Session management
- Content Security Policy validation

Use browser automation to test various attack scenarios safely."

run_ui_test_stage "$UI_SECURITY_TEST_PROMPT" "UI Security Testing" "$TEST_REPORTS_DIR/ui_security_$TIMESTAMP.txt"

# --- Auto-fix stage (if enabled) ---
if [ "$AUTO_FIX" = "--fix" ]; then
    echo -e "${YELLOW}🔧 STAGE 8: AUTOMATED ISSUE FIXING${NC}"
    AUTO_FIX_PROMPT="@ui-responsive-tester Based on all the test results generated, please automatically fix the detected issues where possible. Focus on:
    - CSS responsive design fixes
    - Accessibility improvements  
    - Performance optimizations
    - Cross-browser compatibility fixes
    
    Generate the necessary code changes and apply them to the dashboard-frontend directory. Create a summary of fixes applied."

    run_ui_test_stage "$AUTO_FIX_PROMPT" "Automated Issue Fixing" "$TEST_REPORTS_DIR/auto_fixes_$TIMESTAMP.txt"
fi

# --- Generate comprehensive test report ---
echo -e "${BLUE}📋 GENERATING COMPREHENSIVE TEST REPORT${NC}"

MAIN_REPORT="$TEST_REPORTS_DIR/UI_TEST_REPORT_$TIMESTAMP.md"

cat > "$MAIN_REPORT" << EOF
# Dashboard UI Test Report

**Test Date:** $(date)
**Dashboard URL:** $DASHBOARD_URL
**Auto-Fix Mode:** $([ "$AUTO_FIX" = "--fix" ] && echo "ENABLED" || echo "DISABLED")
**Test Framework:** Playwright MCP with AI Agents

## Test Summary

This comprehensive UI testing report covers visual regression, responsive design, accessibility, performance, functionality, cross-browser compatibility, and security aspects of the Solana Arbitrage Bot dashboard.

## Test Results

### 1. Visual Regression Testing 📸
**Status:** $([ -f "$TEST_REPORTS_DIR/visual_regression_$TIMESTAMP.txt" ] && echo "✅ Completed" || echo "❌ Failed")
**Report:** [visual_regression_$TIMESTAMP.txt](./visual_regression_$TIMESTAMP.txt)

### 2. Responsive Design Validation 📱
**Status:** $([ -f "$TEST_REPORTS_DIR/responsive_validation_$TIMESTAMP.txt" ] && echo "✅ Completed" || echo "❌ Failed")
**Report:** [responsive_validation_$TIMESTAMP.txt](./responsive_validation_$TIMESTAMP.txt)

### 3. Accessibility Testing ♿
**Status:** $([ -f "$TEST_REPORTS_DIR/accessibility_$TIMESTAMP.txt" ] && echo "✅ Completed" || echo "❌ Failed")
**Report:** [accessibility_$TIMESTAMP.txt](./accessibility_$TIMESTAMP.txt)

### 4. Performance Testing ⚡
**Status:** $([ -f "$TEST_REPORTS_DIR/performance_$TIMESTAMP.txt" ] && echo "✅ Completed" || echo "❌ Failed")
**Report:** [performance_$TIMESTAMP.txt](./performance_$TIMESTAMP.txt)

### 5. Functional Testing 🔧
**Status:** $([ -f "$TEST_REPORTS_DIR/functional_$TIMESTAMP.txt" ] && echo "✅ Completed" || echo "❌ Failed")
**Report:** [functional_$TIMESTAMP.txt](./functional_$TIMESTAMP.txt)

### 6. Cross-browser Testing 🌐
**Status:** $([ -f "$TEST_REPORTS_DIR/browser_compatibility_$TIMESTAMP.txt" ] && echo "✅ Completed" || echo "❌ Failed")
**Report:** [browser_compatibility_$TIMESTAMP.txt](./browser_compatibility_$TIMESTAMP.txt)

### 7. UI Security Testing 🛡️
**Status:** $([ -f "$TEST_REPORTS_DIR/ui_security_$TIMESTAMP.txt" ] && echo "✅ Completed" || echo "❌ Failed")
**Report:** [ui_security_$TIMESTAMP.txt](./ui_security_$TIMESTAMP.txt)

$([ "$AUTO_FIX" = "--fix" ] && echo "### 8. Automated Fixes 🔧")
$([ "$AUTO_FIX" = "--fix" ] && echo "**Status:** $([ -f "$TEST_REPORTS_DIR/auto_fixes_$TIMESTAMP.txt" ] && echo "✅ Applied" || echo "❌ Failed")")
$([ "$AUTO_FIX" = "--fix" ] && echo "**Report:** [auto_fixes_$TIMESTAMP.txt](./auto_fixes_$TIMESTAMP.txt)")

## Recommendations

1. Review all individual test reports
2. Prioritize fixes based on severity and user impact
3. Implement recommended changes
4. Re-run tests to verify fixes
5. Consider adding automated UI tests to CI/CD pipeline

## Next Steps

- [ ] Fix high-priority issues identified in reports
- [ ] Update responsive design breakpoints
- [ ] Improve accessibility compliance
- [ ] Optimize performance bottlenecks
- [ ] Add automated regression tests

---
*This report was generated automatically by the Solana Arbitrage Bot UI testing framework.*
EOF

# --- Success Summary ---
echo -e "${GREEN}✅ UI TESTING WORKFLOW COMPLETED!${NC}"
echo ""
echo -e "${PURPLE}📊 TEST SUMMARY${NC}"
echo -e "   ├─ Dashboard URL: $DASHBOARD_URL"
echo -e "   ├─ Test Reports: $(ls -1 $TEST_REPORTS_DIR/*$TIMESTAMP* | wc -l)"
echo -e "   ├─ Main Report: $MAIN_REPORT"
echo -e "   $([ "$AUTO_FIX" = "--fix" ] && echo "├─ Auto-fixes: Applied" || echo "├─ Auto-fixes: Not requested")"
echo -e "   └─ All Reports: $TEST_REPORTS_DIR/"
echo ""
echo -e "${YELLOW}📋 NEXT STEPS${NC}"
echo -e "   1. Review main report: cat $MAIN_REPORT"
echo -e "   2. Check individual findings: ls $TEST_REPORTS_DIR/"
echo -e "   3. Implement UI improvements"
echo -e "   4. Re-run with fixes: $0 $DASHBOARD_URL --fix"
echo -e "   5. Update .bmad/PROJECT_STATUS.md"
echo ""

# --- Open report if possible ---
if command -v code &> /dev/null; then
    echo -e "${BLUE}📖 Opening test report in VS Code...${NC}"
    code "$MAIN_REPORT"
fi