# ⚙️ Configuration Guide

## Overview

The Solana Arbitrage Bot uses a YAML configuration file (`config.yaml`) for all settings. This guide explains all available configuration options.

## Configuration File Structure

```yaml
# Main configuration file: config.yaml

# RPC Connection Settings
rpc:
  url: "https://api.mainnet-beta.solana.com"
  ws_url: "wss://api.mainnet-beta.solana.com"
  commitment: "confirmed"  # confirmed, finalized, processed
  timeout_seconds: 30
  max_retries: 3

# Wallet Configuration
wallet:
  path: "./wallet.json"
  use_ledger: false
  ledger_derivation_path: "44'/501'/0'/0'"

# DEX Settings
dex:
  raydium:
    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    api_url: "https://api.raydium.io"
  orca:
    program_id: "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP"
    api_url: "https://api.orca.so"
  jupiter:
    api_url: "https://quote-api.jup.ag"

# Trading Limits and Safety
limits:
  max_position_sol: 10.0
  min_profit_usd: 0.5
  max_slippage_percent: 1.0
  max_daily_loss_usd: 50.0
  max_daily_trades: 100
  min_liquidity_sol: 100.0
  max_gas_price_lamports: 50000

# Execution Settings
execution:
  priority_fee_lamports: 10000
  max_retries: 3
  retry_delay_ms: 1000
  simulation_enabled: true
  dry_run: false
  confirmation_timeout_seconds: 30

# Sniper Bot Configuration
sniper:
  enabled: false
  max_buy_amount_sol: 1.0
  min_liquidity_sol: 5.0
  max_buy_tax_percent: 10.0
  max_sell_tax_percent: 10.0
  honeypot_check: true
  rugcheck_enabled: true
  max_market_cap_usd: 100000.0
  min_holders: 10
  max_dev_percentage: 50.0
  max_token_age_minutes: 60
  blacklist_keywords:
    - "test"
    - "scam"
    - "rug"
    - "fake"
  blacklisted_creators: []
  safety_apis:
    honeypot_api: "https://api.honeypot.is"
    rugcheck_api: "https://api.rugcheck.xyz"

# Web Server Settings
web:
  enabled: true
  host: "0.0.0.0"
  port: 3001
  auth_token: "your-secret-token-here"
  cors_enabled: true
  rate_limit_per_minute: 100

# Discord Integration
discord:
  enabled: false
  webhook_url: ""
  alerts:
    trades: true
    errors: true
    startup: true
    daily_summary: true
  mention_role_id: ""  # Optional: Discord role ID to mention

# Logging Configuration
logging:
  level: "info"  # trace, debug, info, warn, error
  file_enabled: true
  file_path: "./logs/bot.log"
  max_file_size_mb: 100
  max_files: 10
  json_format: false
  console_enabled: true

# Monitoring and Metrics
monitoring:
  metrics_enabled: true
  health_check_interval_seconds: 60
  performance_tracking: true
  cache_ttl_seconds: 300
```

## Configuration Sections

### RPC Settings

Controls connection to Solana RPC nodes:

- `url`: Main RPC endpoint URL
- `ws_url`: WebSocket endpoint for real-time data
- `commitment`: Transaction commitment level
- `timeout_seconds`: Request timeout
- `max_retries`: Maximum retry attempts

### Wallet Configuration

Wallet and key management:

- `path`: Path to wallet keypair file
- `use_ledger`: Enable Ledger hardware wallet
- `ledger_derivation_path`: BIP44 derivation path for Ledger

### Trading Limits

Safety limits to prevent losses:

- `max_position_sol`: Maximum SOL per single trade
- `min_profit_usd`: Minimum profit threshold
- `max_slippage_percent`: Maximum acceptable slippage
- `max_daily_loss_usd`: Daily loss limit (emergency stop)
- `max_daily_trades`: Maximum trades per day
- `min_liquidity_sol`: Minimum pool liquidity required

### Sniper Bot Settings

Configuration for new token sniping:

- `enabled`: Enable/disable sniper functionality
- `max_buy_amount_sol`: Maximum SOL to spend per token
- `honeypot_check`: Enable honeypot detection
- `rugcheck_enabled`: Enable rug pull checks
- `blacklist_keywords`: Keywords to avoid in token names
- `safety_apis`: External API endpoints for safety checks

### Web Interface

Web dashboard configuration:

- `enabled`: Enable web server
- `host`: Bind address (0.0.0.0 for all interfaces)
- `port`: Port number
- `auth_token`: Authentication token for API access

### Discord Alerts

Discord webhook integration:

- `enabled`: Enable Discord notifications
- `webhook_url`: Discord webhook URL
- `alerts`: Types of alerts to send

## Environment Variables

Configuration can be overridden with environment variables:

```bash
# RPC Settings
export SOLANA_RPC_URL="https://your-rpc-endpoint.com"
export SOLANA_WS_URL="wss://your-ws-endpoint.com"

# Wallet
export WALLET_PATH="/secure/path/to/wallet.json"
export USE_LEDGER="true"

# Trading
export MAX_POSITION_SOL="5.0"
export MIN_PROFIT_USD="1.0"
export DRY_RUN="true"

# Web
export WEB_PORT="3001"
export WEB_AUTH_TOKEN="your-secret-token"

# Discord
export DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/..."

# Logging
export LOG_LEVEL="debug"
export LOG_FILE_PATH="/var/log/arbitrage-bot.log"
```

## Security Considerations

### Wallet Security

- Store wallet files in secure locations with restricted permissions
- Use hardware wallets (Ledger) for production
- Never commit wallet files to version control
- Consider using encrypted wallet files

### API Security

- Use strong, unique auth tokens
- Restrict web server access with firewalls
- Use HTTPS in production (reverse proxy)
- Regularly rotate auth tokens

### Configuration Security

- Protect config files with appropriate permissions
- Use environment variables for sensitive data
- Avoid hardcoding secrets in configuration files

## Validation

The bot validates all configuration on startup:

- Required fields must be present
- Numeric values must be within valid ranges
- URLs must be properly formatted
- File paths must be accessible

Invalid configuration will prevent startup with detailed error messages.

## Hot Reloading

Some configuration changes can be applied without restart:

- Trading limits
- Discord settings
- Logging levels
- Web server settings (except port)

Use the web API or send SIGHUP signal to reload configuration:

```bash
# Reload configuration
curl -X POST http://localhost:3001/api/config/reload

# Or send signal
kill -HUP $(pgrep arbitrage-bot)
```

## Examples

See `config.yaml.example` for a complete example configuration file.
