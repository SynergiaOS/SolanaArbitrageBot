#!/usr/bin/env bash
set -euo pipefail
HOST=${HOST:-127.0.0.1}
PORT=${PORT:-3001}
TOKEN=${TOKEN:-change-me}
BASE="http://${HOST}:${PORT}"

which websocat >/dev/null 2>&1 || { echo "websocat is required for WS test"; exit 2; }

# Open WS and expect a config_update shortly after valid config POST
{
  sleep 1
  curl -s -X POST -H "Authorization: Bearer ${TOKEN}" -H "Content-Type: application/json" \
    -d '{"min_profit_usd":0.5,"max_position_sol":0.02,"max_daily_trades":50,"max_daily_loss_usd":10,"enabled":true}' \
    "${BASE}/api/config" >/dev/null
} &

# Connect and wait 5s for a message
out=$(websocat -H "Authorization: Bearer ${TOKEN}" "ws://${HOST}:${PORT}/ws" --exit-on-eof --ping-interval=2 --linemode --max-messages=1 --timeout=5 || true)

echo "$out"
if echo "$out" | grep -q '"type":"config_update"'; then
  echo "[OK] Received config_update via WebSocket"
  exit 0
else
  echo "[FAIL] Did not receive config_update"
  exit 1
fi

