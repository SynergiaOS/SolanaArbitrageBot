#!/usr/bin/env bash
set -euo pipefail
HOST=${HOST:-127.0.0.1}
PORT=${PORT:-3001}
TOKEN=${TOKEN:-change-me}
BASE="http://${HOST}:${PORT}"

red() { echo -e "\033[31m$*\033[0m"; }
green() { echo -e "\033[32m$*\033[0m"; }

check() {
  local name="$1"; shift
  if "$@"; then
    green "[OK] $name"
  else
    red "[FAIL] $name"
    exit 1
  fi
}

check "GET /api/status" curl -sSf -H "Authorization: Bearer ${TOKEN}" "${BASE}/api/status" >/dev/null

# Get current config
CFG=$(curl -sSf -H "Authorization: Bearer ${TOKEN}" "${BASE}/api/config")
echo "Current config: ${CFG}" | head -c 200

# Invalid update (expect 400)
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST -H "Authorization: Bearer ${TOKEN}" -H "Content-Type: application/json" \
  -d '{"min_profit_usd":0,"max_position_sol":0.02,"max_daily_trades":50,"max_daily_loss_usd":10,"enabled":true}' \
  "${BASE}/api/config")
if [[ "$code" != "400" ]]; then red "Expected 400 for invalid /api/config"; exit 1; else green "Invalid /api/config returned 400"; fi

# Valid update (expect 200)
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST -H "Authorization: Bearer ${TOKEN}" -H "Content-Type: application/json" \
  -d '{"min_profit_usd":0.5,"max_position_sol":0.02,"max_daily_trades":50,"max_daily_loss_usd":10,"enabled":true}' \
  "${BASE}/api/config")
if [[ "$code" != "200" ]]; then red "Expected 200 for valid /api/config"; exit 1; else green "Valid /api/config returned 200"; fi

# Emergency stop with reason/source
check "POST /api/control/emergency" curl -sSf -X POST -H "Authorization: Bearer ${TOKEN}" -H "Content-Type: application/json" \
  -d '{"reason":"smoke test","source":"script"}' "${BASE}/api/control/emergency" >/dev/null

green "Smoke tests completed successfully."

