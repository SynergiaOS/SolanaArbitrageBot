#!/bin/bash

# ==============================================================================
#  AUTO-TDD WORKFLOW SCRIPT FOR RUST PROJECTS WITH CLAUDE CODE AGENTS
# ==============================================================================
#
#  This script automates the Test-Driven Development loop by orchestrating
#  a sequence of specialized AI agents.
#
#  Usage:
#  ./scripts/auto-tdd.sh "<developer_agent_handle>" "<task_description>"
#
#  Example:
#  ./scripts/auto-tdd.sh "@jito-mev-implementer" "Implement Jito bundle creation with MEV protection"
#

# --- Configuration ---
DEVELOPER_AGENT=$1
TASK_DESCRIPTION=$2
CLAUDE_COMMAND="claude" # Change if your Claude command is different

# --- Colors for better output ---
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# --- Helper function to run agent commands ---
run_agent_command() {
    local AGENT_PROMPT="$1"
    local DESCRIPTION="$2"

    echo -e "${BLUE}--- Stage: ${DESCRIPTION} ---${NC}"
    
    # Use -p for non-interactive mode
    if $CLAUDE_COMMAND -p "$AGENT_PROMPT"; then
        echo -e "${GREEN}--- Stage Completed Successfully ---${NC}\n"
    else
        echo -e "${RED}--- ERROR: Stage Failed! Aborting workflow. ---${NC}\n"
        exit 1
    fi
}

# --- Argument validation ---
if [ -z "$DEVELOPER_AGENT" ] || [ -z "$TASK_DESCRIPTION" ]; then
    echo "Usage: $0 \"<developer_agent_handle>\" \"<task_description>\""
    echo ""
    echo "Available Developer Agents:"
    echo "  @jito-mev-implementer    - MEV protection and Jito bundle logic"
    echo "  @onchain-monitor         - Event monitoring and sniper functionality"
    echo "  @transaction-engine      - Transaction building and execution"
    echo "  @solana-architect        - Architecture and boilerplate generation"
    echo "  @performance-optimizer   - Performance and optimization tasks"
    echo ""
    echo "Example:"
    echo "  $0 \"@jito-mev-implementer\" \"Implement bundle creation with priority fees\""
    exit 1
fi

# ==============================================================================
#  WORKFLOW START
# ==============================================================================

echo -e "${YELLOW}🚀 Starting Automated TDD Workflow for Solana Arbitrage Bot${NC}"
echo -e "${YELLOW}Task: '${TASK_DESCRIPTION}'${NC}"
echo -e "${YELLOW}Developer Agent: ${DEVELOPER_AGENT}${NC}\n"

# --- Step 1: Test Agent writes failing tests ---
TEST_PROMPT="@test-generator Please write comprehensive failing unit tests for the following task: '${TASK_DESCRIPTION}'. The tests should define the expected API and behavior following Rust best practices. Include edge cases and error scenarios."
run_agent_command "$TEST_PROMPT" "Generating Unit Tests"

# --- Step 2: Developer Agent writes code to make tests pass ---
CODE_PROMPT="${DEVELOPER_AGENT} Please implement the necessary Rust code to make the previously generated tests pass. The task is: '${TASK_DESCRIPTION}'. Follow the architecture guidelines in .bmad/ARCHITECTURE.md and ensure the implementation is efficient and secure."
run_agent_command "$CODE_PROMPT" "Implementing Code Logic"

# --- Step 3: Run tests to verify code works ---
echo -e "${BLUE}--- Stage: Running All Tests ---${NC}"
if cargo test --release --no-default-features --features monitor; then
    echo -e "${GREEN}--- All Tests Passed ---${NC}\n"
else
    echo -e "${RED}--- TESTS FAILED! Attempting automatic fix ---${NC}"
    
    # Try to fix with debug detective
    FIX_PROMPT="@debug-detective The tests are failing after implementing '${TASK_DESCRIPTION}'. Please analyze the test output and provide a fix."
    run_agent_command "$FIX_PROMPT" "Debugging Test Failures"
    
    # Retry tests
    if cargo test --release --no-default-features --features monitor; then
        echo -e "${GREEN}--- Tests Fixed and Passing ---${NC}\n"
    else
        echo -e "${RED}--- Tests Still Failing! Manual intervention required ---${NC}"
        exit 1
    fi
fi

# --- Step 4: Linter Agent reviews code quality ---
LINT_PROMPT="@rust-linter Please review and analyze the newly implemented code for quality issues. Run clippy and suggest improvements."
run_agent_command "$LINT_PROMPT" "Linting Code"

# --- Step 5: Check that clippy passes ---
echo -e "${BLUE}--- Stage: Running Clippy ---${NC}"
if cargo clippy --no-default-features --features monitor --all-targets -- -D warnings; then
    echo -e "${GREEN}--- Clippy Passed ---${NC}\n"
else
    echo -e "${RED}--- Clippy Issues Found! Auto-fixing... ---${NC}"
    # Most clippy issues can be auto-fixed
    cargo clippy --no-default-features --features monitor --all-targets --fix --allow-dirty -- -D warnings
    echo -e "${GREEN}--- Clippy Issues Fixed ---${NC}\n"
fi

# --- Step 6: Formatter ensures code style ---
FORMAT_PROMPT="@rust-formatter Please format the entire project."
run_agent_command "$FORMAT_PROMPT" "Formatting Code"

# --- Step 7: Final verification ---
echo -e "${BLUE}--- Stage: Final Build Verification ---${NC}"
if cargo build --release --no-default-features --features monitor; then
    echo -e "${GREEN}--- Build Successful ---${NC}\n"
else
    echo -e "${RED}--- Build Failed! Workflow incomplete ---${NC}"
    exit 1
fi

# --- Success summary ---
echo -e "${GREEN}✅ TDD Workflow Completed Successfully!${NC}"
echo -e "${GREEN}   ├─ Tests generated and passing${NC}"
echo -e "${GREEN}   ├─ Code implemented and reviewed${NC}" 
echo -e "${GREEN}   ├─ Linting passed${NC}"
echo -e "${GREEN}   ├─ Code formatted${NC}"
echo -e "${GREEN}   └─ Build verified${NC}"
echo ""
echo -e "${YELLOW}📝 Next Steps:${NC}"
echo -e "   • Review the generated code in your editor"
echo -e "   • Run integration tests if applicable"
echo -e "   • Consider security review with: ./scripts/run-security-audit.sh"
echo -e "   • Update .bmad/PROJECT_STATUS.md with progress"