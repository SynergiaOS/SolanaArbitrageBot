# Deployment Guide

This document summarizes production deployment steps for Solana Arbitrage Bot with secure Docker runtime and essential API usage.

## Docker Image

Build multi-stage image (runtime runs as non-root `arbitrage`):

```bash
# From repo root
docker build -t solana-arb-bot:prod .
```

Run container:
```bash
docker run --rm -p 3001:3001 \
  -e RUST_LOG=info \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  --name arb solana-arb-bot:prod
```

Notes:
- Runtime user is `arbitrage` and owns `/app`, `/app/data`, `/app/logs`; the app can write logs/DB.
- Default config path: `/app/config.production.yaml` (copied into image). Provide your own if needed:
  ```bash
  -v $(pwd)/myconfig.yaml:/app/config.production.yaml:ro
  ```

## Systemd (optional)

Example unit file:
```ini
[Unit]
Description=Solana Arbitrage Bot
After=network-online.target

[Service]
User=ubuntu
WorkingDirectory=/opt/solana-arb
ExecStart=/usr/bin/docker run --rm -p 3001:3001 \
  -e RUST_LOG=info \
  -v /opt/solana-arb/data:/app/data \
  -v /opt/solana-arb/logs:/app/logs \
  --name arb solana-arb-bot:prod
ExecStop=/usr/bin/docker stop arb
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## API Usage

Auth: if enabled, include `Authorization: Bearer <TOKEN>`.

- GET /api/status
  ```bash
  curl -s http://127.0.0.1:3001/api/status | jq
  ```

- GET current config
  ```bash
  curl -s http://127.0.0.1:3001/api/config | jq
  ```

- POST update config (validation: returns 400 on invalid values)
  ```bash
  curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"min_profit_usd":0.5,"max_position_sol":0.02,"max_daily_trades":50,"max_daily_loss_usd":10,"enabled":true}' \
    http://127.0.0.1:3001/api/config | jq
  ```

- POST emergency stop (optional JSON body)
  ```bash
  curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"reason":"manual stop","source":"ops"}' \
    http://127.0.0.1:3001/api/control/emergency | jq
  ```

## WebSocket

Subscribe for live updates (e.g., `config_update` after successful /api/config):
```bash
websocat ws://127.0.0.1:3001/ws
```

## Tests

Run unit/integration tests:
```bash
cargo test
```

Network-dependent tests are ignored in CI. Run locally with:
```bash
cargo test --test integration -- --ignored --nocapture
```

Smoke tests for CI/CD:
```bash
scripts/smoke_test.sh
scripts/ws_smoke_test.sh
```

