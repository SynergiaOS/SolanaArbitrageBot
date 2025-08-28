# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Solana arbitrage bot written in Rust with advanced features including sniping, MEV protection, and multi-DEX arbitrage. The bot operates on Solana mainnet/devnet and supports trading across Raydium, Orca, Jupiter, and other DEXs.

## Build System & Commands

### Core Build Commands
```bash
# Monitor-only build (production default)
cargo build --release --no-default-features --features monitor

# Full feature build (development)
cargo build --release --all-features

# Testing
cargo test --no-default-features --features monitor --workspace --verbose

# Linting
cargo fmt --all -- --check
cargo clippy --no-default-features --features monitor --all-targets -- -D warnings
```

### Scripts
- `./scripts/start_monitor_only.sh` - Start monitor-only mode with network selection
- `./scripts/smoke_test.sh` - API endpoint tests
- `./scripts/run_safety_tests.sh` - Safety checker tests
- `./scripts/setup_devnet.sh` - Devnet environment setup
- `./scripts/deploy_production.sh` - Production deployment

### Docker
```bash
# Build monitor-only image
docker build --build-arg CARGO_FEATURES=monitor -t solana-arbitrage-bot .

# Run with volumes for data and logs
docker run --rm -p 3001:3001 \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  solana-arbitrage-bot
```

## Architecture

### Feature-Based Architecture
The codebase uses Rust feature flags for modular compilation:
- `monitor` - Core monitoring and basic functionality (production default)
- `full` - All features enabled (development)
- `sniper` - New token sniping functionality
- `web` - HTTP API and WebSocket dashboard
- `gepa` - Genetic algorithm optimization
- `kestra` - Workflow orchestration

### Core Modules
- `src/main.rs` - Entry point with CLI argument parsing
- `src/config_manager.rs` - Centralized configuration with type safety
- `src/monitor.rs` - DEX price monitoring and WebSocket handling
- `src/calculator.rs` - Profit calculations with gas and slippage
- `src/executor.rs` - Transaction building and execution
- `src/safety.rs` - Risk management and safety checks

### Specialized Modules
- `src/sniper/` - New token detection and sniping logic
- `src/web/` - HTTP API server and WebSocket handlers
- `src/security/` - MEV protection and rate limiting
- `src/performance/` - Connection pooling and optimization
- `src/gepa/` - Genetic algorithm parameter optimization

## Configuration

### Environment-Specific Configs
- `config.yaml` - Base configuration
- `config_devnet.yaml` - Devnet settings
- `config.production.yaml` - Production settings
- `config_micro_16usd.yaml` - Micro capital mode

### Key Configuration Sections
- `limits` - Trading limits and risk management
- `sniper` - New token sniping parameters
- `dex` - DEX-specific endpoints and program IDs
- `web` - Dashboard and API settings
- `monitoring` - Alerts and notifications

## Testing

### Test Suites
- Unit tests: `cargo test --workspace`
- Integration tests: `tests/integration.rs`
- Safety tests: `./scripts/run_safety_tests.sh`
- API tests: `./scripts/smoke_test.sh`

### Test Environments
- Devnet for safe testing with fake SOL
- Dry-run mode for production config testing
- Mock services for unit testing DEX interactions

## Database

- SQLite for local data storage (`data/dashboard.db`)
- PostgreSQL support for production deployments
- Trades table for profit/loss tracking
- Real-time metrics via WebSocket

## Security Considerations

### Safety Mechanisms
- Position size limits (max_position_sol)
- Daily loss limits (max_daily_loss_usd)
- Circuit breakers for anomalous conditions
- Rug pull detection for new tokens

### MEV Protection
- Jito bundles for private mempools
- Transaction priority fee optimization
- Slippage protection and monitoring

## Development Guidelines

### Code Style
- Follow existing Rust patterns and error handling
- Use feature gates to keep monitor-only build lightweight
- Maintain backwards compatibility for config structures
- Add comprehensive error messages for operational issues

### Dependencies
- Keep monitor-only build free of heavy dependencies (no axum, rusqlite, etc.)
- Use async/await patterns with Tokio runtime
- Prefer rust_decimal for financial calculations
- Use anyhow for error handling

### Performance
- Connection pooling for RPC endpoints
- Parallel request processing where possible
- Efficient WebSocket handling for real-time data
- Memory-conscious data structures for high-frequency updates

## Common Development Tasks

### Adding New DEX Support
1. Update `src/dex/` module with new DEX client
2. Add program IDs and endpoints to config
3. Implement price monitoring WebSocket connection
4. Add transaction building logic in executor

### Modifying Safety Limits
1. Update `SafetyConfig` in `config_manager.rs`
2. Implement validation logic in safety checkers
3. Add tests for new limit conditions
4. Update config examples

### Dashboard Features
1. Add new handlers in `src/web/handlers.rs`
2. Update WebSocket message types in `websocket.rs`
3. Add database migrations if needed
4. Update frontend components in `dashboard-frontend/`[byterover-mcp]

# important 
always use byterover-retrieve-knowledge tool to get the related context before any tasks 
always use byterover-store-knowledge to store all the critical informations after sucessful tasks